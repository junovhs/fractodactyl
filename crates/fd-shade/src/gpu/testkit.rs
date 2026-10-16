//! wgpu device selection for the studio passes, plus the plumbing their tests use (here
//! and in fd-cli): a one-shot compute dispatch (bind buffers in declaration order,
//! dispatch, read the output back) and the no-adapter skip rule.

use std::sync::mpsc;

/// The best adapter wgpu offers, opened with the adapter's own limits (keyframe buffers
/// exceed wgpu's conservative defaults): a discrete GPU, else an integrated one, else a
/// software adapter (lavapipe on Linux, WARP on Windows), so a machine without a GPU still
/// runs the WGSL (DEC-28). Vulkan is preferred; DX12 serves Windows without a Vulkan
/// driver. Returns the adapter name, device and queue.
pub fn open_device(label: &str) -> Result<(String, wgpu::Device, wgpu::Queue), String> {
    let backends = wgpu::Backends::VULKAN | wgpu::Backends::DX12;
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let rank = |a: &wgpu::Adapter| {
        let info = a.get_info();
        let kind = match info.device_type {
            wgpu::DeviceType::DiscreteGpu => 0,
            wgpu::DeviceType::IntegratedGpu => 1,
            wgpu::DeviceType::VirtualGpu => 2,
            wgpu::DeviceType::Other => 3,
            wgpu::DeviceType::Cpu => 4,
        };
        (kind, info.backend != wgpu::Backend::Vulkan)
    };
    let adapter = pollster::block_on(instance.enumerate_adapters(backends))
        .into_iter()
        .min_by_key(rank)
        .ok_or("no wgpu adapter")?;
    let name = adapter.get_info().name;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some(label),
        required_limits: adapter.limits(),
        ..Default::default()
    }))
    .map_err(|e| format!("wgpu device: {e}"))?;
    Ok((name, device, queue))
}

/// A buffer bound at the binding number equal to its position in the list.
pub enum Binding<'a> {
    /// Read-only storage, uploaded from these bytes.
    Read(&'a [u8]),
    /// Read-write storage of this many bytes; the one buffer read back.
    Out(u64),
    /// Uniform block, uploaded from these bytes.
    Uniform(&'a [u8]),
}

/// Run `entry` of the WGSL `source` over `groups` workgroups on the first Vulkan
/// adapter wgpu offers, preferring a discrete GPU. Returns the adapter name and the
/// bytes of the single `Out` binding.
pub fn dispatch(
    label: &str,
    source: &str,
    entry: &str,
    bindings: &[Binding],
    groups: (u32, u32),
) -> Result<(String, Vec<u8>), String> {
    let (name, device, queue) = open_device(label)?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(label),
        layout: None,
        module: &module,
        entry_point: Some(entry),
        compilation_options: Default::default(),
        cache: None,
    });
    let buffer = |size: u64, usage: wgpu::BufferUsages| {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            // Zero-sized bindings are invalid; pad to one word.
            size: size.max(4).next_multiple_of(4),
            usage,
            mapped_at_creation: false,
        })
    };
    let mut out = None;
    let buffers: Vec<wgpu::Buffer> = bindings
        .iter()
        .map(|b| match *b {
            Binding::Read(bytes) => {
                let buf = buffer(
                    bytes.len() as u64,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                );
                queue.write_buffer(&buf, 0, bytes);
                buf
            }
            Binding::Uniform(bytes) => {
                let buf = buffer(
                    bytes.len() as u64,
                    wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                );
                queue.write_buffer(&buf, 0, bytes);
                buf
            }
            Binding::Out(size) => {
                out = Some(size);
                buffer(
                    size,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                )
            }
        })
        .collect();
    let out_bytes = out.ok_or("no output binding")?;
    let out_index = bindings
        .iter()
        .position(|b| matches!(b, Binding::Out(_)))
        .ok_or("no output binding")?;
    let staging = buffer(
        out_bytes,
        wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
    );
    let entries: Vec<wgpu::BindGroupEntry> = buffers
        .iter()
        .enumerate()
        .map(|(i, b)| wgpu::BindGroupEntry {
            binding: i as u32,
            resource: b.as_entire_binding(),
        })
        .collect();
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &entries,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(groups.0, groups.1, 1);
    }
    encoder.copy_buffer_to_buffer(&buffers[out_index], 0, &staging, 0, staging.size());
    queue.submit([encoder.finish()]);
    let (tx, rx) = mpsc::channel();
    staging.map_async(wgpu::MapMode::Read, .., move |r| {
        let _ = tx.send(r);
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| format!("wgpu poll: {e}"))?;
    rx.recv()
        .map_err(|e| format!("wgpu map callback: {e}"))?
        .map_err(|e| format!("wgpu map: {e}"))?;
    if let Some(e) = pollster::block_on(scope.pop()) {
        staging.unmap();
        return Err(format!("wgpu {label} rejected: {e}"));
    }
    let bytes = staging
        .get_mapped_range(..)
        .map_err(|e| format!("wgpu mapped range: {e}"))?[..out_bytes as usize]
        .to_vec();
    staging.unmap();
    Ok((name, bytes))
}

/// A GPU test's result, or `None` after printing a skip when this machine has no wgpu
/// adapter at all. With FD_REQUIRE_GPU set (CI's Linux runner,
/// on lavapipe) a missing adapter fails instead; any other error always fails.
pub fn or_skip<T>(result: Result<T, String>, what: &str) -> Option<T> {
    match result {
        Ok(v) => Some(v),
        Err(e) if e.contains("no wgpu adapter") => {
            assert!(
                std::env::var_os("FD_REQUIRE_GPU").is_none(),
                "{what}: {e}, but FD_REQUIRE_GPU is set"
            );
            eprintln!("{what} skipped: {e}");
            None
        }
        Err(e) => panic!("{what}: {e}"),
    }
}

/// Native-endian bytes of a word slice, for upload.
pub fn bytes<T: Copy + Into<u32>>(words: &[T]) -> Vec<u8> {
    words
        .iter()
        .flat_map(|&w| Into::<u32>::into(w).to_ne_bytes())
        .collect()
}

/// Words of a read-back byte buffer.
pub fn words(bytes: &[u8]) -> Vec<u32> {
    bytes
        .chunks_exact(4)
        .map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}
