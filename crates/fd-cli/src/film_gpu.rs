//! wgpu compute shading and tent resampling for keyframe films (SHAD-07): the WGSL
//! prepare, shade and resample passes, f32 and integer arithmetic only (DEC-25, DEC-28).
//! All camera and fractal arithmetic remains in the CPU renderer.
use crate::keyframe::Resample;
use fd_samples::Samples;
use fd_shade::{Appearance, Look};
use std::sync::mpsc;

const GROUP: u32 = 8;
pub(crate) const PREPARE: &str = concat!(
    include_str!("../shaders/soft_f64.wgsl"),
    include_str!("../shaders/film_prepare.wgsl")
);
pub(crate) const SHADE: &str = concat!(
    include_str!("../shaders/soft_f64.wgsl"),
    include_str!("../shaders/film_shade.wgsl")
);
const RESAMPLE: &str = include_str!("../shaders/film_resample.wgsl");

type Rect = (usize, usize, usize, usize);
type Source<'a> = (&'a Samples, (usize, usize), Rect);

fn groups(n: u32) -> u32 {
    n.div_ceil(GROUP)
}

/// The shade (and prepare) parameter block: nine vec4s, 144 bytes.
#[derive(Clone, Copy)]
pub(crate) struct ShadePush {
    dims: [u32; 4],
    rect: [u32; 4],
    tone: [f32; 4],
    light: [f32; 4],
    color: [f32; 4],
    extra: [f32; 4],
    density64: [u32; 4],
    lines: [f32; 4],
    haze: [f32; 4],
}

impl ShadePush {
    /// The block as native-endian bytes.
    pub(crate) fn bytes(&self) -> Vec<u8> {
        let words = |w: [u32; 4]| w.into_iter().flat_map(u32::to_ne_bytes);
        let floats = |f: &[[f32; 4]]| -> Vec<u8> {
            f.iter().flatten().flat_map(|v| v.to_ne_bytes()).collect()
        };
        words(self.dims)
            .chain(words(self.rect))
            .chain(floats(&[self.tone, self.light, self.color, self.extra]))
            .chain(words(self.density64))
            .chain(floats(&[self.lines, self.haze]))
            .collect()
    }

    /// The f64 density these parameters band with.
    fn density(&self) -> u64 {
        u64::from(self.density64[0]) | u64::from(self.density64[1]) << 32
    }
}

pub(crate) fn linear(rgb: u32) -> [f32; 3] {
    [16, 8, 0].map(|shift| {
        let c = ((rgb >> shift) & 255) as f32 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    })
}

pub(crate) fn shade_push(
    l: &Look,
    a: &Appearance,
    full: (usize, usize),
    rect: (usize, usize, usize, usize),
) -> ShadePush {
    let stops: Vec<_> = l.stops.iter().map(|&s| linear(s)).collect();
    let mean: [f32; 3] =
        [0, 1, 2].map(|c| stops.iter().map(|v| v[c]).sum::<f32>() / stops.len() as f32);
    let inside = linear(l.interior);
    let (sin, cos) = l.light.to_radians().sin_cos();
    // As fd-shade's Studio: the breathing density in f64, its f32 for band spacing.
    let density = l.density * f64::from(a.density());
    let bits = density.to_bits();
    let haze = l.haze.map_or([0.0; 4], |h| {
        let c = linear(h);
        [c[0], c[1], c[2], 1.0]
    });
    ShadePush {
        dims: [full.0 as u32, full.1 as u32, rect.0 as u32, rect.1 as u32],
        rect: [rect.2 as u32, rect.3 as u32, 0, 0],
        tone: [a.phase(), density as f32, l.terrace, l.slope],
        light: [cos, -sin, inside[0], inside[1]],
        color: [inside[2], mean[0], mean[1], mean[2]],
        extra: [
            if l.terrace > 0.0 {
                1.0 - l.terrace.max(0.0) * 0.4
            } else {
                1.0
            },
            stops.len() as f32,
            u8::from(a.aa) as f32,
            u8::from(a.unresolved_interior) as f32,
        ],
        density64: [bits as u32, (bits >> 32) as u32, 0, 0],
        lines: [l.lines, l.line_px, 0.0, 0.0],
        haze,
    }
}

/// Pack the raw samples for the prepare pass: words 0..1 the bitwise f64 `nu` of
/// escaped samples (0 otherwise; the pass forms density * nu in exact f64, so the
/// palette phase base is never rounded to f32, DEC-29), word 2 DE, word 3 the u16
/// normal + class kind.
#[cfg(test)]
pub(crate) fn pack_samples(s: &Samples, out: &mut [u32]) -> Result<(), String> {
    if out.len() != s.class.len() * 4 {
        return Err("GPU sample-column lengths differ".into());
    }
    pack_range(s, 0, out)
}

/// Pack samples `first..first + out.len() / 4`, as pack_samples.
fn pack_range(s: &Samples, first: usize, out: &mut [u32]) -> Result<(), String> {
    let nu = s.nu.as_deref().ok_or("GPU needs nu")?;
    let de = s.de.as_deref().ok_or("GPU needs de")?;
    let normal = s.normal.as_deref().ok_or("GPU needs normal")?;
    let n = out.len() / 4;
    if s.class.len() != nu.len()
        || nu.len() != de.len()
        || de.len() != normal.len()
        || !out.len().is_multiple_of(4)
        || first + n > nu.len()
    {
        return Err("GPU sample-column lengths differ".into());
    }
    for (j, word) in out.chunks_exact_mut(4).enumerate() {
        let i = first + j;
        let bits = if s.class[i].escaped() { nu[i] } else { 0.0 }.to_bits();
        word.copy_from_slice(&[
            bits as u32,
            (bits >> 32) as u32,
            de[i].to_bits(),
            u32::from(normal[i]) | (u32::from(s.class[i].0 & 3) << 16),
        ]);
    }
    Ok(())
}

/// One keyframe's raw samples and prepared words (palette base, relief, band AA, haze
/// and line terms), prepared on upload and again only when the density changes.
struct Key {
    id: usize,
    full: (usize, usize),
    samples: wgpu::Buffer,
    prepared: wgpu::Buffer,
    density: Option<u64>,
}

/// A GPU film keeps only the current pair's prepared keyframes resident.
/// An unavailable device or unsupported look is a normal CPU fallback.
pub(crate) struct FilmGpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    prepare: wgpu::ComputePipeline,
    shade: wgpu::ComputePipeline,
    resample: wgpu::ComputePipeline,
    keys: Vec<Key>,
    shaded: [Option<wgpu::Buffer>; 2],
    palette: wgpu::Buffer,
    lut: wgpu::Buffer,
    output: wgpu::Buffer,
    readback: wgpu::Buffer,
    host_output: Vec<u8>,
    pub(crate) name: String,
    look: Look,
}

impl FilmGpu {
    pub(crate) fn supported(l: &Look, a: &Appearance) -> bool {
        // The band select multiplies by the stop count exactly up to 2047.
        (2..=64).contains(&l.stops.len())
            && l.density.is_finite()
            && l.density > 0.0
            && f64::from(a.density()).is_finite()
    }

    pub(crate) fn new(
        l: &Look,
        a: &Appearance,
        out: (usize, usize),
        multiple: usize,
    ) -> Result<Self, String> {
        if !Self::supported(l, a) {
            return Err("studio look density or stop count is out of range".into());
        }
        if multiple == 0 || out.0 == 0 || out.1 == 0 {
            return Err("invalid keyframe dimensions".into());
        }
        let (name, device, queue) = crate::wgpu_compute::open_device("fd film")?;
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let pipeline = |source: &str| {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("fd film"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("fd film"),
                layout: None,
                module: &module,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let (prepare, shade, resample) = (pipeline(PREPARE), pipeline(SHADE), pipeline(RESAMPLE));
        let colors: Vec<u8> = l
            .stops
            .iter()
            .flat_map(|&stop| linear(stop).into_iter().chain([0.0]))
            .flat_map(f32::to_ne_bytes)
            .collect();
        let palette = storage(&device, colors.len() as u64, false);
        queue.write_buffer(&palette, 0, &colors);
        // The exact CPU tent gamma-decode table.
        let table: Vec<u8> = (0..256)
            .flat_map(|i| (i as f32 / 255.0).powf(2.2).to_ne_bytes())
            .collect();
        let lut = storage(&device, table.len() as u64, false);
        queue.write_buffer(&lut, 0, &table);
        let bytes = (out.0 * out.1 * 4) as u64;
        let output = storage(&device, bytes, true);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fd film readback"),
            size: bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if let Some(e) = pollster::block_on(scope.pop()) {
            return Err(format!("wgpu film pipelines: {e}"));
        }
        Ok(Self {
            device,
            queue,
            prepare,
            shade,
            resample,
            keys: Vec::new(),
            shaded: [None, None],
            palette,
            lut,
            output,
            readback,
            host_output: vec![0; out.0 * out.1 * 3],
            name,
            look: l.clone(),
        })
    }

    fn group(
        &self,
        pipeline: &wgpu::ComputePipeline,
        buffers: &[&wgpu::Buffer],
    ) -> wgpu::BindGroup {
        let entries: Vec<wgpu::BindGroupEntry> = buffers
            .iter()
            .enumerate()
            .map(|(i, b)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: b.as_entire_binding(),
            })
            .collect();
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fd film"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &entries,
        })
    }

    fn uniform(&self, bytes: &[u8]) -> wgpu::Buffer {
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fd film parameters"),
            size: bytes.len() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&buffer, 0, bytes);
        buffer
    }

    /// Upload one keyframe's raw samples, once.
    fn ensure_key(&mut self, id: usize, s: &Samples, full: (usize, usize)) -> Result<(), String> {
        if self.keys.iter().any(|k| k.id == id) {
            return Ok(());
        }
        if s.class.len() != full.0 * full.1 {
            return Err("GPU keyframe dimensions differ from samples".into());
        }
        let len = (s.class.len() as u64)
            .checked_mul(32)
            .ok_or("GPU keyframe byte count overflow")?;
        let limits = self.device.limits();
        if len > limits.max_storage_buffer_binding_size || len > limits.max_buffer_size {
            return Err("keyframe exceeds the adapter's storage buffer limit".into());
        }
        let samples = storage(&self.device, len / 2, false);
        // Pack and upload in bounded chunks: no keyframe-sized second CPU copy.
        const CHUNK: usize = 1 << 20;
        let mut words = vec![0u32; CHUNK.min(s.class.len()) * 4];
        for first in (0..s.class.len()).step_by(CHUNK) {
            let n = CHUNK.min(s.class.len() - first);
            pack_range(s, first, &mut words[..n * 4])?;
            let bytes: Vec<u8> = words[..n * 4]
                .iter()
                .flat_map(|w| w.to_ne_bytes())
                .collect();
            self.queue.write_buffer(&samples, first as u64 * 16, &bytes);
        }
        let prepared = storage(&self.device, len, true);
        self.keys.push(Key {
            id,
            full,
            samples,
            prepared,
            density: None,
        });
        Ok(())
    }

    /// Run the prepare pass for keyframe `id` unless it is current for this density.
    fn prepare_key(&mut self, id: usize, a: &Appearance) {
        let k = self.keys.iter().position(|k| k.id == id).expect("uploaded");
        let full = self.keys[k].full;
        let push = shade_push(&self.look, a, full, (0, 0, full.0, full.1));
        if self.keys[k].density == Some(push.density()) {
            return;
        }
        let params = self.uniform(&push.bytes());
        let key = &self.keys[k];
        let group = self.group(&self.prepare, &[&key.samples, &key.prepared, &params]);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.prepare);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(groups(full.0 as u32), groups(full.1 as u32), 1);
        }
        self.queue.submit([encoder.finish()]);
        self.keys[k].density = Some(push.density());
    }

    /// Shade both keyframes' needed rectangles, tent blend (rotation included), then
    /// read only RGB8 back for ffmpeg.
    pub(crate) fn frame(
        &mut self,
        outer_id: usize,
        outer: Source<'_>,
        inner_id: usize,
        inner: Source<'_>,
        a: &Appearance,
        r: &Resample,
    ) -> Result<&[u8], String> {
        if !Self::supported(&self.look, a) {
            return Err("GPU look became unsupported".into());
        }
        if r.out.0 * r.out.1 * 3 != self.host_output.len() {
            return Err("GPU output size differs from the film".into());
        }
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        self.ensure_key(outer_id, outer.0, outer.1)?;
        self.ensure_key(inner_id, inner.0, inner.1)?;
        self.keys.retain(|k| k.id >= outer_id);
        self.prepare_key(outer_id, a);
        self.prepare_key(inner_id, a);
        for (i, (_, full, _)) in [&outer, &inner].into_iter().enumerate() {
            let bytes = (full.0 * full.1 * 4) as u64;
            if self.shaded[i].as_ref().is_none_or(|b| b.size() < bytes) {
                self.shaded[i] = Some(storage(&self.device, bytes, true));
            }
        }
        let key = |id: usize| {
            &self
                .keys
                .iter()
                .find(|k| k.id == id)
                .expect("uploaded")
                .prepared
        };
        let shaded = [
            self.shaded[0].as_ref().unwrap(),
            self.shaded[1].as_ref().unwrap(),
        ];
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let mut groups_kept = Vec::new();
        for (i, (id, (_, full, rect))) in [(outer_id, &outer), (inner_id, &inner)]
            .into_iter()
            .enumerate()
        {
            let params = self.uniform(&shade_push(&self.look, a, *full, *rect).bytes());
            let group = self.group(&self.shade, &[key(id), shaded[i], &self.palette, &params]);
            {
                let mut pass = encoder.begin_compute_pass(&Default::default());
                pass.set_pipeline(&self.shade);
                pass.set_bind_group(0, &group, &[]);
                pass.dispatch_workgroups(groups(rect.2 as u32), groups(rect.3 as u32), 1);
            }
            groups_kept.push((group, params));
        }
        let params = self.uniform(&r.uniform());
        let group = self.group(
            &self.resample,
            &[shaded[0], shaded[1], &self.output, &self.lut, &params],
        );
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.resample);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(groups(r.out.0 as u32), groups(r.out.1 as u32), 1);
        }
        encoder.copy_buffer_to_buffer(&self.output, 0, &self.readback, 0, self.readback.size());
        self.queue.submit([encoder.finish()]);
        let (tx, rx) = mpsc::channel();
        self.readback.map_async(wgpu::MapMode::Read, .., move |m| {
            let _ = tx.send(m);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| format!("wgpu poll: {e}"))?;
        rx.recv()
            .map_err(|e| format!("wgpu map callback: {e}"))?
            .map_err(|e| format!("wgpu map: {e}"))?;
        if let Some(e) = pollster::block_on(scope.pop()) {
            self.readback.unmap();
            return Err(format!("wgpu film frame: {e}"));
        }
        {
            let view = self
                .readback
                .get_mapped_range(..)
                .map_err(|e| format!("wgpu mapped range: {e}"))?;
            for (rgb, pixel) in self
                .host_output
                .chunks_exact_mut(3)
                .zip(view.chunks_exact(4))
            {
                rgb.copy_from_slice(&pixel[..3]);
            }
        }
        self.readback.unmap();
        Ok(&self.host_output)
    }
}

/// A storage buffer; `out` buffers are also copy sources.
fn storage(device: &wgpu::Device, size: u64, out: bool) -> wgpu::Buffer {
    let usage = if out {
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC
    } else {
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST
    };
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("fd film"),
        size: size.max(4).next_multiple_of(4),
        usage,
        mapped_at_creation: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_layout_and_dispatch_are_portable() {
        let a = Appearance::STILL;
        let look = Look::builtin("ice").unwrap();
        assert_eq!(
            shade_push(&look, &a, (16, 9), (0, 0, 16, 9)).bytes().len(),
            144
        );
        assert_eq!(groups(1), 1);
        assert_eq!(groups(8), 1);
        assert_eq!(groups(16), 2);
        assert_eq!(groups(17), 3);
        assert_eq!((groups(3840), groups(2160)), (480, 270));
        assert_eq!((groups(7680), groups(4320)), (960, 540));
    }

    #[test]
    fn source_packing_and_look_guard() {
        use fd_samples::{Class, Evidence, Kind};
        let s = Samples {
            class: vec![
                Class::new(Kind::Escaped, Evidence::Heuristic),
                Class::new(Kind::Interior, Evidence::Certified),
            ],
            nu: Some(vec![2000.25, 0.0]),
            de: Some(vec![1.5, 0.0]),
            normal: Some(vec![32768, 0]),
            ..Default::default()
        };
        let mut words = vec![0u32; 8];
        pack_samples(&s, &mut words).unwrap();
        assert_eq!(words.len(), 8);
        let nu = f64::from_bits(u64::from(words[0]) | (u64::from(words[1]) << 32));
        assert_eq!(nu, 2000.25);
        assert_eq!(f32::from_bits(words[2]), 1.5);
        assert_eq!(words[3], 32768);
        assert_eq!(words[7] >> 16, 1);
        // Lines, haze and breathing run on the GPU (SHAD-05); stop counts cap at 64.
        let mut look = Look::builtin("ice").unwrap();
        let a = Appearance::STILL;
        assert!(FilmGpu::supported(&look, &a));
        assert!(FilmGpu::supported(&look, &Appearance { breathe: 0.1, ..a }));
        look.stops = vec![0x102030; 65];
        assert!(!FilmGpu::supported(&look, &a));
    }

    #[test]
    fn palette_phase_wrap_regression() {
        // Animated master-ice terrace: the old GPU reduction rounds a value
        // just *below* a whole palette cycle up to 1.0, then fract() makes it
        // 0.0. CPU x is 8.0 (clamped to stop 7, fr=1), not stop 0, fr=0.
        let phase = (-0.11_f64 * (2.0 / 30.0)).rem_euclid(1.0) as f32;
        let t = 2048.0_f64 + 1.0 - f64::from(phase) - 1e-8;
        let cpu = ((t + f64::from(phase)).rem_euclid(1.0) * 8.0) as f32;
        let truncated = (t.rem_euclid(1.0) as f32 + phase).fract() * 8.0;
        assert_eq!(cpu, 8.0);
        assert_eq!(truncated, 0.0);
        let mut packed = vec![0u32; 4];
        let s = fd_samples::Samples {
            class: vec![fd_samples::Class::new(
                fd_samples::Kind::Escaped,
                fd_samples::Evidence::Heuristic,
            )],
            nu: Some(vec![t / 0.05]),
            de: Some(vec![1.0]),
            normal: Some(vec![1234]),
            ..Default::default()
        };
        pack_samples(&s, &mut packed).unwrap();
        let reconstructed = f64::from_bits(u64::from(packed[0]) | (u64::from(packed[1]) << 32));
        assert_eq!(reconstructed, s.nu.as_ref().unwrap()[0]);
        assert_eq!(packed[3], 1234);
    }

    #[test]
    fn optional_device_skips_cleanly() {
        if let Ok(gpu) = FilmGpu::new(
            &Look {
                lines: 0.0,
                ..Look::builtin("ice").unwrap()
            },
            &Appearance::STILL,
            (8, 8),
            2,
        ) {
            assert!(!gpu.name.is_empty());
        }
    }
}
