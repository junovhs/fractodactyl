//! Bounded optics and output-space grain configuration. No external resources.
use super::render::{buffer, pipeline};

/// Display-referred grain approximation and HDR bloom. Grain is photographic
/// texture, independently controlled from quantization dither (DEC-34).
#[derive(Clone, Debug)]
pub struct Finishing {
    pub bloom: f32,
    pub bloom_radius: u32,
    pub grain: f32,
    pub grain_size: f32,
    pub grain_color: f32,
    pub grain_tonal: [f32; 3],
    pub seed: u32,
    pub exposure: u32,
    pub dither: bool,
}
impl Default for Finishing {
    fn default() -> Self {
        Self {
            bloom: 0.0,
            bloom_radius: 12,
            grain: 0.0,
            grain_size: 1.0,
            grain_color: 0.0,
            grain_tonal: [0.25, 1.0, 0.3],
            seed: 1,
            exposure: 0,
            dither: false,
        }
    }
}
impl Finishing {
    /// Validate predictable bounds before issuing GPU commands.
    pub fn validate(&self) -> Result<(), String> {
        for (name, x, lo, hi) in [
            ("bloom", self.bloom, 0.0, 16.0),
            ("grain", self.grain, 0.0, 0.25),
            ("grain size", self.grain_size, 0.3, 8.0),
            ("grain color", self.grain_color, 0.0, 1.0),
        ] {
            if !x.is_finite() || x < lo || x > hi {
                return Err(format!("{name} must be in {lo}..{hi}"));
            }
        }
        if !(1..=32).contains(&self.bloom_radius)
            || self
                .grain_tonal
                .iter()
                .any(|x| !x.is_finite() || *x < 0.0 || *x > 4.0)
        {
            return Err("bloom radius must be 1..32 and grain tonal response 0..4".into());
        }
        Ok(())
    }
}

pub(super) struct Bloom {
    pub a: wgpu::Buffer,
    down: wgpu::ComputePipeline,
    blur: wgpu::ComputePipeline,
    extract_group: wgpu::BindGroup,
    h_group: wgpu::BindGroup,
    v_group: wgpu::BindGroup,
    h_uniform: wgpu::Buffer,
    v_uniform: wgpu::Buffer,
    width: u32,
    height: u32,
    full: (u32, u32),
    pub radius: u32,
    pub dirty: bool,
    pub bytes: u64,
}

fn bindings(
    device: &wgpu::Device,
    pipeline: &wgpu::ComputePipeline,
    buffers: &[(u32, &wgpu::Buffer)],
) -> wgpu::BindGroup {
    let entries: Vec<_> = buffers
        .iter()
        .map(|(binding, b)| wgpu::BindGroupEntry {
            binding: *binding,
            resource: b.as_entire_binding(),
        })
        .collect();
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("style optics bindings"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &entries,
    })
}

fn weights(w: u32, h: u32, axis: u32, radius: u32, full: (u32, u32)) -> Vec<u8> {
    let mut words = vec![w, h, axis, radius, full.0, full.1, 0, 0];
    let mut kernel = Vec::new();
    for k in 0..=radius {
        let x = k as f64;
        let value = [(0.6, 0.12), (0.3, 0.35), (0.1, 0.7)]
            .iter()
            .map(|(gain, scale)| {
                let sigma = (radius as f64 * scale).max(0.6);
                gain * (-0.5 * (x / sigma).powi(2)).exp() / sigma
            })
            .sum::<f64>();
        kernel.push(value);
    }
    let norm = kernel[0] + 2.0 * kernel[1..].iter().sum::<f64>();
    for k in 0..33 {
        words.extend([
            ((kernel.get(k).copied().unwrap_or(0.0) / norm) as f32).to_bits(),
            0,
            0,
            0,
        ]);
    }
    crate::gpu::testkit::bytes(&words)
}

impl Bloom {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        hdr: &wgpu::Buffer,
        w: u32,
        h: u32,
        radius: u32,
    ) -> Self {
        let (width, height) = (w.div_ceil(2), h.div_ceil(2));
        let bytes = width as u64 * height as u64 * 16;
        let usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        let a = buffer(device, "style half-res bloom A", bytes, usage);
        let b = buffer(device, "style half-res bloom B", bytes, usage);
        let hw = weights(width, height, 0, radius, (w, h));
        let vw = weights(width, height, 1, radius, (w, h));
        let uniform = wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST;
        let h_uniform = buffer(
            device,
            "style horizontal bloom kernel",
            hw.len() as u64,
            uniform,
        );
        let v_uniform = buffer(
            device,
            "style vertical bloom kernel",
            vw.len() as u64,
            uniform,
        );
        queue.write_buffer(&h_uniform, 0, &hw);
        queue.write_buffer(&v_uniform, 0, &vw);
        let down = pipeline(
            device,
            "style emission extraction",
            include_str!("bloom.wgsl"),
            "extract",
        );
        let blur = pipeline(
            device,
            "style separable bloom",
            include_str!("bloom.wgsl"),
            "blur",
        );
        let extract_group = bindings(device, &down, &[(0, hdr), (1, &a), (2, &h_uniform)]);
        let h_group = bindings(device, &blur, &[(1, &b), (2, &h_uniform), (3, &a)]);
        let v_group = bindings(device, &blur, &[(1, &a), (2, &v_uniform), (3, &b)]);
        Self {
            a,
            down,
            blur,
            extract_group,
            h_group,
            v_group,
            h_uniform,
            v_uniform,
            width,
            height,
            full: (w, h),
            radius,
            dirty: true,
            bytes: 2 * bytes + 2 * hw.len() as u64,
        }
    }
    pub fn update(&mut self, queue: &wgpu::Queue, radius: u32) {
        if radius != self.radius {
            queue.write_buffer(
                &self.h_uniform,
                0,
                &weights(self.width, self.height, 0, radius, self.full),
            );
            queue.write_buffer(
                &self.v_uniform,
                0,
                &weights(self.width, self.height, 1, radius, self.full),
            );
            self.radius = radius;
            self.dirty = true;
        }
    }
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        for (pipeline, group) in [
            (&self.down, &self.extract_group),
            (&self.blur, &self.h_group),
            (&self.blur, &self.v_group),
        ] {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.dispatch_workgroups(self.width.div_ceil(8), self.height.div_ceil(8), 1);
        }
    }
}
