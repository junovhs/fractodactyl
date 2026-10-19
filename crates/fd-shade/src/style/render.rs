//! Persistent opt-in WGSL renderer. No kernel link and no CPU color evaluation.
use super::Compiled;
use crate::{gpu, Rgb8};
use fd_samples::{Column, ColumnSet, Header, Samples};
use std::{sync::mpsc, time::Instant};

/// Exact columns required by this graph (class is always needed).
pub fn columns(c: &Compiled) -> ColumnSet {
    let mut set = ColumnSet::of(&[Column::Class]);
    for source in &c.sources {
        set = match source.as_str() {
            "nu" | "phase" => set.with(Column::Nu),
            "de" => set.with(Column::De),
            "gradient" => set.with(Column::De).with(Column::Normal),
            "normal" | "tangent" => set.with(Column::Normal),
            _ => set,
        };
    }
    set
}

/// Resource/cost facts, not a claim of total driver VRAM or GPU timestamp duration.
#[derive(Clone, Debug)]
pub struct Stats {
    pub adapter: String,
    pub initialization_seconds: f64,
    pub frame_seconds: f64,
    pub dispatches: u32,
    pub working_bytes: u64,
    pub pipelines: u32,
    pub sample_uploads: u32,
    pub allocations: u32,
    pub frames: u64,
}

/// One graph and sample raster on a device; uniforms update without rebuilding
/// pipelines, bind groups, sample uploads, output or readback buffers (DEC-33).
pub struct StyleRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    shade: wgpu::ComputePipeline,
    finish: wgpu::ComputePipeline,
    shade_group: wgpu::BindGroup,
    finish_group: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    output: wgpu::Buffer,
    readback: wgpu::Buffer,
    params: Vec<u32>,
    graph: Compiled,
    width: usize,
    height: usize,
    last_shade: Option<Vec<u32>>,
    pub stats: Stats,
}

fn buffer(
    device: &wgpu::Device,
    label: &str,
    size: u64,
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: size.max(4),
        usage,
        mapped_at_creation: false,
    })
}

fn pipeline(
    device: &wgpu::Device,
    label: &str,
    source: &str,
    entry: &str,
) -> wgpu::ComputePipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(label),
        layout: None,
        module: &module,
        entry_point: Some(entry),
        compilation_options: Default::default(),
        cache: None,
    })
}

fn group(
    device: &wgpu::Device,
    pipeline: &wgpu::ComputePipeline,
    buffers: &[&wgpu::Buffer],
) -> wgpu::BindGroup {
    let entries: Vec<_> = buffers
        .iter()
        .enumerate()
        .map(|(i, b)| wgpu::BindGroupEntry {
            binding: i as u32,
            resource: b.as_entire_binding(),
        })
        .collect();
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("style reusable bindings"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &entries,
    })
}

impl StyleRenderer {
    /// Open the existing adapter-selection route; callers already owning a GPU device
    /// can use `with_device` to avoid another device and queue.
    pub fn new(graph: Compiled, h: &Header, s: &Samples) -> Result<Self, String> {
        let started = Instant::now();
        let (adapter, device, queue) = gpu::testkit::open_device("fd style")?;
        let mut renderer = Self::with_device(graph, h, s, adapter, device, queue)?;
        renderer.stats.initialization_seconds = started.elapsed().as_secs_f64();
        Ok(renderer)
    }

    /// Attach to an existing device, sharing the GPU infrastructure. This POC uploads
    /// stored CPU samples once; direct device-buffer adoption is a later integration.
    pub fn with_device(
        graph: Compiled,
        h: &Header,
        s: &Samples,
        adapter: String,
        device: wgpu::Device,
        queue: wgpu::Queue,
    ) -> Result<Self, String> {
        let started = Instant::now();
        if h.nx == 0
            || h.ny == 0
            || h.ss == 0
            || !h.nx.is_multiple_of(h.ss)
            || !h.ny.is_multiple_of(h.ss)
        {
            return Err("style requires a nonempty complete sample grid".into());
        }
        let n = (h.nx as usize)
            .checked_mul(h.ny as usize)
            .ok_or("style sample size overflow")?;
        if s.class.len() != n {
            return Err("style sample count differs from header".into());
        }
        for column in columns(&graph).iter() {
            let len = match column {
                Column::Class => Some(s.class.len()),
                Column::Nu => s.nu.as_ref().map(Vec::len),
                Column::De => s.de.as_ref().map(Vec::len),
                Column::Normal => s.normal.as_ref().map(Vec::len),
                _ => None,
            };
            if len != Some(n) {
                return Err(format!("style needs complete {column:?} column"));
            }
        }
        if s.nu.as_ref().is_some_and(|v| v.len() != n)
            || s.de.as_ref().is_some_and(|v| v.len() != n)
            || s.normal.as_ref().is_some_and(|v| v.len() != n)
        {
            return Err("inconsistent style sample columns".into());
        }
        let (width, height) = h.pixels();
        let pixels = (width as u64)
            .checked_mul(height as u64)
            .ok_or("style image size overflow")?;
        let input_bytes = (n as u64)
            .checked_mul(16)
            .ok_or("style sample bytes overflow")?;
        let hdr_bytes = pixels * 32;
        let out_bytes = pixels * 4;
        let limits = device.limits();
        for bytes in [input_bytes, hdr_bytes, out_bytes] {
            if bytes > limits.max_storage_buffer_binding_size as u64
                || bytes > limits.max_buffer_size
            {
                return Err(
                    "style exceeds GPU buffer limits; tiled styling is not implemented".into(),
                );
            }
        }
        if width.div_ceil(8) > limits.max_compute_workgroups_per_dimension as usize
            || height.div_ceil(8) > limits.max_compute_workgroups_per_dimension as usize
        {
            return Err("style exceeds GPU dispatch limits".into());
        }
        let mut raw = Vec::with_capacity(n * 4);
        let origin = s
            .nu
            .as_ref()
            .and_then(|v| {
                v.iter()
                    .enumerate()
                    .filter_map(|(i, &nu)| (s.class[i].0 & 3 == 0 && nu.is_finite()).then_some(nu))
                    .reduce(f64::min)
            })
            .unwrap_or(0.0)
            .to_bits();
        for i in 0..n {
            let nu = s.nu.as_ref().map_or(0.0, |v| v[i]).to_bits();
            let de = s.de.as_ref().map_or(0.0, |v| v[i]);
            let normal = s.normal.as_ref().map_or(0, |v| v[i]);
            raw.extend([
                nu as u32,
                (nu >> 32) as u32,
                de.to_bits(),
                normal as u32 | ((s.class[i].0 as u32 & 3) << 16),
            ]);
        }
        let mut params = vec![0u32; 12 + 64 * 4];
        params[..4].copy_from_slice(&[h.nx, h.ny, h.ss, 0]);
        params[8..10].copy_from_slice(&[origin as u32, (origin >> 32) as u32]);
        for (i, (_, v)) in graph.parameters.iter().enumerate() {
            params[12 + i * 4] = v.to_bits();
        }
        let u = gpu::testkit::bytes(&params);
        let normals = gpu::unit_table();
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let storage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        let samples = buffer(&device, "style samples", input_bytes, storage);
        queue.write_buffer(&samples, 0, &gpu::testkit::bytes(&raw));
        let unit = buffer(
            &device,
            "shared normal table",
            normals.len() as u64,
            storage,
        );
        queue.write_buffer(&unit, 0, &normals);
        let hdr = buffer(&device, "style linear HDR layers", hdr_bytes, storage);
        let uniform = buffer(
            &device,
            "style uniforms",
            u.len() as u64,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        queue.write_buffer(&uniform, 0, &u);
        let output = buffer(
            &device,
            "style final pixels",
            out_bytes,
            storage | wgpu::BufferUsages::COPY_SRC,
        );
        let readback = buffer(
            &device,
            "style final readback",
            out_bytes,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        );
        let generated = include_str!("shade.wgsl")
            .replace("// GRAPH_BODY", &graph.body)
            .replace(
                "// GRAPH_RETURN",
                &format!(
                    "return Layer(vec4<f32>(clean_color({}),1.0), vec4<f32>(clean_color({}),0.0));",
                    graph.surface, graph.emission
                ),
            );
        let shade = pipeline(
            &device,
            "style fused pointwise",
            &format!(
                "{}\n{}",
                include_str!("../../shaders/soft_f64.wgsl"),
                generated
            ),
            "shade",
        );
        let finish = pipeline(
            &device,
            "style output",
            include_str!("finish.wgsl"),
            "finish",
        );
        let shade_group = group(&device, &shade, &[&samples, &hdr, &uniform, &unit]);
        let finish_group = group(&device, &finish, &[&hdr, &output, &uniform]);
        if let Some(e) = pollster::block_on(scope.pop()) {
            return Err(format!("style GPU setup: {e}"));
        }
        let stats = Stats {
            adapter,
            initialization_seconds: started.elapsed().as_secs_f64(),
            frame_seconds: 0.0,
            dispatches: 0,
            working_bytes: input_bytes
                + hdr_bytes
                + 2 * out_bytes
                + u.len() as u64
                + normals.len() as u64,
            pipelines: 2,
            sample_uploads: 1,
            allocations: 6,
            frames: 0,
        };
        Ok(Self {
            device,
            queue,
            shade,
            finish,
            shade_group,
            finish_group,
            uniform,
            output,
            readback,
            params,
            graph,
            width,
            height,
            last_shade: None,
            stats,
        })
    }

    /// Update a named uniform without recompiling or reallocating resources.
    pub fn set_parameter(&mut self, name: &str, value: f32) -> Result<(), String> {
        if !value.is_finite() {
            return Err("style parameter must be finite".into());
        }
        let i = self
            .graph
            .parameters
            .iter()
            .position(|(n, _)| n == name)
            .ok_or_else(|| format!("unknown style parameter {name}"))?;
        self.params[12 + i * 4] = value.to_bits();
        Ok(())
    }

    /// Evaluate one timestamp. A time-independent graph reuses its HDR output until
    /// parameters or class policy change; final pixels are the only readback.
    pub fn frame(&mut self, time: f32, unresolved_interior: bool) -> Result<Rgb8, String> {
        if !time.is_finite() {
            return Err("style time must be finite".into());
        }
        let started = Instant::now();
        self.params[3] = u32::from(unresolved_interior);
        self.params[4] = time.to_bits();
        let mut key = self.params.clone();
        if !self.graph.sources.contains("time") {
            key[4] = 0;
        }
        let shade_needed = self.last_shade.as_ref() != Some(&key);
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        self.queue
            .write_buffer(&self.uniform, 0, &gpu::testkit::bytes(&self.params));
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let groups = (
            self.width.div_ceil(8) as u32,
            self.height.div_ceil(8) as u32,
        );
        for (needed, pipeline, group) in [
            (shade_needed, &self.shade, &self.shade_group),
            (true, &self.finish, &self.finish_group),
        ] {
            if !needed {
                continue;
            }
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.dispatch_workgroups(groups.0, groups.1, 1);
        }
        encoder.copy_buffer_to_buffer(&self.output, 0, &self.readback, 0, self.readback.size());
        self.queue.submit([encoder.finish()]);
        let (tx, rx) = mpsc::channel();
        self.readback.map_async(wgpu::MapMode::Read, .., move |r| {
            let _ = tx.send(r);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| e.to_string())?;
        rx.recv()
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
        if let Some(e) = pollster::block_on(scope.pop()) {
            self.readback.unmap();
            return Err(format!("style frame: {e}"));
        }
        let mut data = Vec::with_capacity(self.width * self.height * 3);
        {
            let view = self
                .readback
                .get_mapped_range(..)
                .map_err(|e| e.to_string())?;
            for pixel in view.chunks_exact(4) {
                data.extend_from_slice(&pixel[..3]);
            }
        }
        self.readback.unmap();
        self.last_shade = Some(key);
        self.stats.dispatches = 1 + u32::from(shade_needed);
        self.stats.frames += 1;
        self.stats.frame_seconds = started.elapsed().as_secs_f64();
        Ok(Rgb8 {
            w: self.width,
            h: self.height,
            data,
        })
    }
}
