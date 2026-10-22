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
    let choice = super::adapters::choice()?;
    let adapters = if choice.is_some() {
        super::adapters::enumerate()
    } else {
        super::adapters::native()
    };
    let adapter = if let Some(index) = choice {
        super::adapters::selected(&adapters, index)?
    } else {
        adapters
            .iter()
            .min_by_key(|a| rank(a))
            .ok_or("no wgpu adapter")?
    };
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
    dispatch_inner(label, source, entry, bindings, groups, None)
}

/// A bounded1D dispatch: update the sample-base word between completed submissions.
/// One device, pipeline and set of buffers are reused within this attempt.
#[derive(Clone, Copy, Debug)]
pub struct DispatchBatch {
    pub groups_per_batch: u32,
    pub invocations_per_group: u32,
    pub offset_binding: usize,
    pub offset_word: usize,
}

/// Dispatch bounded work submissions without changing the authored WGSL maths.
/// Timeout/device/map/validation failures abort the attempt; this helper never retries.
pub fn dispatch_batched(
    label: &str,
    source: &str,
    entry: &str,
    bindings: &[Binding],
    groups: (u32, u32),
    batch: DispatchBatch,
) -> Result<(String, Vec<u8>), String> {
    dispatch_inner(label, source, entry, bindings, groups, Some(batch))
}

fn dispatch_inner(
    label: &str,
    source: &str,
    entry: &str,
    bindings: &[Binding],
    groups: (u32, u32),
    batch: Option<DispatchBatch>,
) -> Result<(String, Vec<u8>), String> {
    if let Some(b) = batch {
        if groups.1 != 1 || b.groups_per_batch == 0 || b.invocations_per_group == 0 {
            return Err("bounded dispatch needs positive1D groups".into());
        }
        match bindings.get(b.offset_binding) {
            Some(Binding::Read(bytes))
                if (b.offset_word + 1)
                    .checked_mul(4)
                    .is_some_and(|n| n <= bytes.len()) => {}
            _ => {
                return Err("bounded dispatch sample-base word is outside its read binding".into())
            }
        }
        groups
            .0
            .checked_mul(b.invocations_per_group)
            .ok_or("bounded dispatch sample index overflow")?;
    }
    let (name, device, queue) = open_device(label)?;
    let mut scopes = vec![device.push_error_scope(wgpu::ErrorFilter::Validation)];
    let lost = std::sync::Arc::new(std::sync::Mutex::new(None));
    if batch.is_some() {
        scopes.push(device.push_error_scope(wgpu::ErrorFilter::OutOfMemory));
        scopes.push(device.push_error_scope(wgpu::ErrorFilter::Internal));
        let signal = lost.clone();
        device.set_device_lost_callback(move |reason, message| {
            *signal.lock().unwrap() = Some(format!("{reason:?}: {message}"));
        });
        let limits = device.limits();
        for binding in bindings {
            let size = match binding {
                Binding::Read(b) | Binding::Uniform(b) => b.len() as u64,
                Binding::Out(n) => *n,
            };
            if size > limits.max_buffer_size
                || (!matches!(binding, Binding::Uniform(_))
                    && size > limits.max_storage_buffer_binding_size)
            {
                return Err(format!(
                    "bounded dispatch buffer{size} exceeds device buffer/storage limits {}/{}",
                    limits.max_buffer_size, limits.max_storage_buffer_binding_size
                ));
            }
        }
        if groups.0 > limits.max_compute_workgroups_per_dimension {
            return Err("bounded dispatch total groups exceed supported1D index grid".into());
        }
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    let wait_submission = |submission| -> Result<(), String> {
        let timeout = deadline
            .saturating_duration_since(std::time::Instant::now())
            .min(std::time::Duration::from_secs(5));
        if timeout.is_zero() {
            return Err("bounded dispatch120s attempt deadline exceeded".into());
        }
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(timeout),
            })
            .map_err(|e| format!("bounded dispatch submission completion: {e}"))?;
        if let Some(why) = lost.lock().unwrap().as_ref() {
            return Err(format!("bounded dispatch device lost: {why}"));
        }
        Ok(())
    };
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
    if let Some(b) = batch {
        let mut base = 0u32;
        while base < groups.0 {
            let n = b.groups_per_batch.min(groups.0 - base);
            queue.write_buffer(
                &buffers[b.offset_binding],
                (b.offset_word * 4) as u64,
                &(base * b.invocations_per_group).to_ne_bytes(),
            );
            let mut encoder = device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_compute_pass(&Default::default());
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &group, &[]);
                pass.dispatch_workgroups(n, 1, 1);
            }
            let submission = queue.submit([encoder.finish()]);
            wait_submission(submission)?;
            base += n;
        }
    }
    let mut encoder = device.create_command_encoder(&Default::default());
    if batch.is_none() {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(groups.0, groups.1, 1);
    }
    encoder.copy_buffer_to_buffer(&buffers[out_index], 0, &staging, 0, staging.size());
    let submission = queue.submit([encoder.finish()]);
    if batch.is_some() {
        wait_submission(submission)?;
    }
    let (tx, rx) = mpsc::channel();
    staging.map_async(wgpu::MapMode::Read, .., move |r| {
        let _ = tx.send(r);
    });
    device
        .poll(if batch.is_some() {
            wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(5)),
            }
        } else {
            wgpu::PollType::wait_indefinitely()
        })
        .map_err(|e| format!("wgpu poll: {e}"))?;
    (if batch.is_some() {
        rx.recv_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| format!("wgpu map callback: {e}"))
    } else {
        rx.recv().map_err(|e| format!("wgpu map callback: {e}"))
    })?
    .map_err(|e| format!("wgpu map: {e}"))?;
    for scope in scopes.into_iter().rev() {
        if let Some(e) = pollster::block_on(scope.pop()) {
            staging.unmap();
            return Err(format!("wgpu {label} rejected: {e}"));
        }
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

#[test]
fn batched_dispatch_preserves_partial_chunk_ids() {
    let shader = r#"
@group(0) @binding(0) var<storage,read> args:array<u32>;
@group(0) @binding(1) var<storage,read_write> result:array<vec4<u32>>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>){let i=id.x+args[1];if(i>=args[0]){return;}result[i]=vec4<u32>(i,i+1u,0x47505502u,0u);}
"#;
    let count = 67u32;
    let args = bytes(&[count, 0]);
    let Some((_, data)) = or_skip(
        dispatch_batched(
            "partial chunk IDs",
            shader,
            "main",
            &[Binding::Read(&args), Binding::Out(u64::from(count) * 16)],
            (2, 1),
            DispatchBatch {
                groups_per_batch: 1,
                invocations_per_group: 64,
                offset_binding: 0,
                offset_word: 1,
            },
        ),
        "partial chunk IDs",
    ) else {
        return;
    };
    let output = words(&data);
    let mut iterations = 0u64;
    for (i, row) in output.chunks_exact(4).enumerate() {
        assert_eq!(row, [i as u32, i as u32 + 1, 0x47505502, 0]);
        iterations += u64::from(row[1]);
    }
    assert_eq!(iterations, u64::from(count) * (u64::from(count) + 1) / 2);
}
