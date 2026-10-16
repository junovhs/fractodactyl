//! wgpu zone samples (GPU-13). The CPU preflight owns decimal camera
//! arithmetic and the zone validity gate; a rejected GPU path retains the
//! exact CPU renderer.
use fd_kernel::{Params, Zone};
use fd_samples::{Class, Column, Evidence, Header, Kind, Samples, View};
use std::time::Instant;

/// Views of one dispatch's output columns.
struct Columns<'a> {
    nu: &'a [f64],
    de: &'a [f32],
    word: &'a [u32],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kernel {
    F64,
    F32,
    Mixed,
    /// No f64 or i64 anywhere (GPU-28): float-float and f32+exponent (DEC-25).
    F32Only,
}

impl Kernel {
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        match value {
            "f64" => Ok(Self::F64),
            "f32" => Ok(Self::F32),
            "mixed" => Ok(Self::Mixed),
            "f32only" => Ok(Self::F32Only),
            _ => Err(format!(
                "--gpu-kernel: expected f64, f32, mixed or f32only, got {value:?}"
            )),
        }
    }

    fn kernel_tag(self) -> &'static str {
        match self {
            Self::F64 => "wgpu-f64",
            Self::F32 => "wgpu-f32",
            Self::Mixed => "wgpu-mixed",
            Self::F32Only => "wgpu-f32only",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::F64 => "FP64",
            Self::F32 => "f32",
            Self::Mixed => "mixed",
            Self::F32Only => "f32only",
        }
    }

    /// The WGSL module: zone.wgsl plus the guarded f32 finish, or a stub
    /// that keeps the f64 kernel free of subgroup operations.
    /// The mid-band module (GPU-22).
    fn mid_source(self) -> String {
        if self == Self::F32Only {
            return format!(
                "{}\n{}\n{}\n{}",
                crate::zone_ff::FF,
                include_str!("../../fd-shade/shaders/soft_f64.wgsl"),
                include_str!("../shaders/zone32_lib.wgsl"),
                include_str!("../shaders/zone_mid32.wgsl")
            );
        }
        format!(
            "{}\n{}",
            include_str!("../shaders/zone_f64_lib.wgsl"),
            include_str!("../shaders/zone_mid.wgsl")
        )
    }

    fn source(self) -> String {
        if self == Self::F32Only {
            return format!(
                "{}\n{}\n{}\n{}",
                crate::zone_ff::FF,
                include_str!("../../fd-shade/shaders/soft_f64.wgsl"),
                include_str!("../shaders/zone32_lib.wgsl"),
                include_str!("../shaders/zone32.wgsl")
            );
        }
        const F64_TAIL: &str = "const F32_FINISH: bool = false;\n\
            fn wave_all(ready: bool) -> bool { return false; }\n\
            fn finish32(gid: i32, z: C, du: C, n: i32, errpx: f64) -> bool { return false; }\n";
        let tail = match self {
            Self::F64 => F64_TAIL,
            Self::F32 | Self::Mixed | Self::F32Only => {
                include_str!("../shaders/zone_finish32.wgsl")
            }
        };
        format!(
            "{}\n{}\n{tail}",
            include_str!("../shaders/zone_f64_lib.wgsl"),
            include_str!("../shaders/zone.wgsl")
        )
    }

    fn features(self) -> wgpu::Features {
        let base = wgpu::Features::SHADER_F64 | wgpu::Features::SHADER_INT64;
        match self {
            Self::F64 => base,
            Self::F32 | Self::Mixed => base | wgpu::Features::SUBGROUP,
            Self::F32Only => wgpu::Features::empty(),
        }
    }
}

/// Holds the device, pipeline and buffers across successive keyframes.
pub(crate) struct Renderer {
    mode: Kernel,
    opened: Option<Opened>,
    gpu: Option<Gpu>,
    layout: Option<(Stage, usize, usize, [i32; 3])>,
}

/// Which zone operator a frame uses, as the CPU zone decides it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    /// Return map, approach, Koenigs jump or patch, finish (zone.wgsl / zone32.wgsl).
    Deep,
    /// KERN-02's co-moving two-cycle jump (zone_mid.wgsl / zone_mid32.wgsl, GPU-22).
    Mid,
}

impl Stage {
    /// Slot of r^2 in the stage's constants.
    fn escape_slot(self) -> usize {
        match self {
            Self::Deep => 21,
            Self::Mid => 3,
        }
    }
}

/// The frame's constants for whichever operator the CPU zone would use on it.
fn inputs(view: &View, p: &Params, zone: &Zone) -> Result<(Stage, Header, Vec<f64>), String> {
    match zone.gpu_f64_input(view, p) {
        Ok((h, v)) => Ok((Stage::Deep, h, v)),
        Err(deep) => zone
            .gpu_mid_input(view, p)
            .map(|(h, v)| (Stage::Mid, h, v))
            .map_err(|_| deep),
    }
}

impl Renderer {
    pub(crate) fn new(mode: Kernel) -> Self {
        Self {
            mode,
            opened: None,
            gpu: None,
            layout: None,
        }
    }

    /// The zone kernel's device for `count`-sample keyframes (opened on first use), so
    /// the studio look can shade its output in place (GPU-21): adapter name, device
    /// and queue.
    pub(crate) fn device(
        &mut self,
        count: usize,
    ) -> Result<(String, wgpu::Device, wgpu::Queue), String> {
        let o = self.open(count)?;
        Ok((o.name.clone(), o.device.clone(), o.queue.clone()))
    }

    fn open(&mut self, count: usize) -> Result<&Opened, String> {
        if self.opened.is_none() {
            // Without an f64-capable GPU, the f32-only kernel runs instead (GPU-28),
            // on a software adapter if need be (DEC-28).
            let opened = match Opened::new(count, self.mode) {
                Err(e) if self.mode != Kernel::F32Only => {
                    let o = Opened::new(count, Kernel::F32Only)?;
                    eprintln!("fd: {e}; using the f32-only zone kernel on {}", o.name);
                    self.mode = Kernel::F32Only;
                    o
                }
                other => other?,
            };
            self.opened = Some(opened);
        }
        Ok(self.opened.as_ref().expect("opened"))
    }
}

/// The FP64 wgpu device a mode needs, with its adapter's limits.
struct Opened {
    name: String,
    device: wgpu::Device,
    queue: wgpu::Queue,
    limits: wgpu::Limits,
}

impl Opened {
    fn new(count: usize, mode: Kernel) -> Result<Self, String> {
        let required = mode.features();
        let column_bytes = (count as u64).checked_mul(8).ok_or("GPU grid overflow")?;
        let groups = count.div_ceil(64) as u64;
        // The binary64 kernels are proven on Vulkan only (GPU-12/19/20); other
        // backends arrive with the f32-only path (GPU-05).
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let mut adapters: Vec<_> =
            pollster::block_on(instance.enumerate_adapters(wgpu::Backends::VULKAN))
                .into_iter()
                .filter(|a| {
                    let limits = a.limits();
                    let x = groups.min(u64::from(limits.max_compute_workgroups_per_dimension));
                    (mode == Kernel::F32Only || a.get_info().device_type != wgpu::DeviceType::Cpu)
                        && a.features().contains(required)
                        && x > 0
                        && groups.div_ceil(x)
                            <= u64::from(limits.max_compute_workgroups_per_dimension)
                        && column_bytes <= limits.max_storage_buffer_binding_size
                        && 2 * column_bytes <= limits.max_buffer_size
                })
                .collect();
        adapters.sort_by_key(|a| match a.get_info().device_type {
            wgpu::DeviceType::DiscreteGpu => 0,
            wgpu::DeviceType::IntegratedGpu => 1,
            wgpu::DeviceType::Cpu => 3,
            _ => 2,
        });
        let Some(adapter) = adapters.into_iter().next() else {
            return Err(format!(
                "no wgpu Vulkan GPU with {} and sufficient buffer limits",
                if mode == Kernel::F32Only {
                    "no f64 features (f32only)"
                } else if mode == Kernel::F64 {
                    "SHADER_F64 and SHADER_INT64"
                } else {
                    "SHADER_F64, SHADER_INT64 and SUBGROUP (f32/mixed)"
                }
            ));
        };
        let name = adapter.get_info().name;
        let limits = adapter.limits();
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("fd zone"),
            required_features: required,
            required_limits: limits.clone(),
            ..Default::default()
        }))
        .map_err(|e| format!("wgpu device: {e}"))?;
        Ok(Self {
            name,
            device,
            queue,
            limits,
        })
    }
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    group: wgpu::BindGroup,
    constants: wgpu::Buffer,
    /// Sample columns as the host stores them: nu f64, de f32, normal | class << 16.
    outputs: [wgpu::Buffer; 3],
    /// The three columns back to back, mapped for reading after each dispatch.
    staging: wgpu::Buffer,
    name: String,
    mode: Kernel,
    dispatch: (u32, u32),
}

impl Gpu {
    fn new(
        opened: &Opened,
        count: usize,
        constants: &[f64],
        mode: Kernel,
        stage: Stage,
    ) -> Result<Self, String> {
        let const_bytes = std::mem::size_of_val(constants) as u64;
        let column_bytes = (count as u64).checked_mul(8).ok_or("GPU grid overflow")?;
        let groups = count.div_ceil(64) as u64;
        let limits = &opened.limits;
        let per_dim = u64::from(limits.max_compute_workgroups_per_dimension);
        let x = groups.min(per_dim);
        if x == 0
            || groups.div_ceil(x) > per_dim
            || column_bytes > limits.max_storage_buffer_binding_size
            || const_bytes > limits.max_storage_buffer_binding_size
            || 2 * column_bytes > limits.max_buffer_size
        {
            return Err("zone keyframe exceeds the wgpu device's limits".into());
        }
        let dispatch = (x as u32, groups.div_ceil(x) as u32);
        let (name, device, queue) = (
            opened.name.clone(),
            opened.device.clone(),
            opened.queue.clone(),
        );
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fd zone kernel"),
            source: wgpu::ShaderSource::Wgsl(
                match stage {
                    Stage::Deep => mode.source(),
                    Stage::Mid => mode.mid_source(),
                }
                .into(),
            ),
        });
        let shape = match stage {
            Stage::Deep => specialisation(constants),
            Stage::Mid => [0; 3],
        };
        let all = [
            ("DEG", f64::from(shape[0])),
            ("NPHI", f64::from(shape[1])),
            ("NPSI", f64::from(shape[2])),
            ("F32_FRONT", if mode == Kernel::F32 { 1.0 } else { 0.0 }),
        ];
        let overrides = match (stage, mode) {
            (Stage::Mid, _) => &all[..0],
            (Stage::Deep, Kernel::F32Only) => &all[..3],
            (Stage::Deep, _) => &all[..],
        };
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("fd zone kernel"),
            layout: None,
            module: &module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: overrides,
                ..Default::default()
            },
            cache: None,
        });
        let storage = |label, size, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        let out = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC;
        let constants_buffer = storage(
            "zone constants",
            const_bytes,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let outputs = [
            storage("zone nu", column_bytes, out),
            storage("zone de", column_bytes / 2, out),
            storage("zone normal+class", column_bytes / 2, out),
        ];
        let staging = storage(
            "zone readback",
            2 * column_bytes,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        );
        let entries: Vec<_> = std::iter::once(&constants_buffer)
            .chain(&outputs)
            .enumerate()
            .map(|(binding, buffer)| wgpu::BindGroupEntry {
                binding: binding as u32,
                resource: buffer.as_entire_binding(),
            })
            .collect();
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fd zone"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        if let Some(e) = pollster::block_on(scope.pop()) {
            return Err(format!("wgpu zone pipeline rejected: {e}"));
        }
        Ok(Self {
            device,
            queue,
            pipeline,
            group,
            constants: constants_buffer,
            outputs,
            staging,
            name,
            mode,
            dispatch,
        })
    }

    /// Record one keyframe's kernel dispatch after uploading its constants.
    fn encode(&self, constants: &[f64]) -> wgpu::CommandEncoder {
        let bytes: Vec<u8> = constants.iter().flat_map(|v| v.to_ne_bytes()).collect();
        self.queue.write_buffer(&self.constants, 0, &bytes);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.group, &[]);
            pass.dispatch_workgroups(self.dispatch.0, self.dispatch.1, 1);
        }
        encoder
    }

    /// Run one keyframe, leaving its columns on the device for the studio look.
    fn run(&self, constants: &[f64]) -> Result<(), String> {
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        self.queue.submit([self.encode(constants).finish()]);
        match pollster::block_on(scope.pop()) {
            Some(e) => Err(format!("wgpu zone dispatch rejected: {e}")),
            None => Ok(()),
        }
    }

    /// Run one keyframe and hand the mapped columns to `read`.
    fn dispatch<R>(
        &self,
        constants: &[f64],
        count: usize,
        read: impl FnOnce(&Columns) -> R,
    ) -> Result<R, String> {
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut encoder = self.encode(constants);
        let n = count as u64;
        let mut at = 0;
        for (buffer, size) in self.outputs.iter().zip([8 * n, 4 * n, 4 * n]) {
            encoder.copy_buffer_to_buffer(buffer, 0, &self.staging, at, size);
            at += size;
        }
        self.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        self.staging.map_async(wgpu::MapMode::Read, ..at, move |r| {
            let _ = tx.send(r);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| format!("wgpu poll: {e}"))?;
        rx.recv()
            .map_err(|e| format!("wgpu map callback: {e}"))?
            .map_err(|e| format!("wgpu map: {e}"))?;
        if let Some(e) = pollster::block_on(scope.pop()) {
            self.staging.unmap();
            return Err(format!("wgpu zone dispatch rejected: {e}"));
        }
        let result = {
            let view = self
                .staging
                .get_mapped_range(..at)
                .map_err(|e| format!("wgpu mapped range: {e}"))?;
            let ptr = view.as_ptr();
            if ptr.align_offset(8) != 0 {
                return Err("wgpu readback is not 8-byte aligned".into());
            }
            // SAFETY: the view is 16 * count initialised bytes laid out as
            // count f64, then count f32, then count u32; the base is 8-aligned.
            let columns = unsafe {
                Columns {
                    nu: std::slice::from_raw_parts(ptr.cast(), count),
                    de: std::slice::from_raw_parts(ptr.add(8 * count).cast(), count),
                    word: std::slice::from_raw_parts(ptr.add(12 * count).cast(), count),
                }
            };
            read(&columns)
        };
        self.staging.unmap();
        Ok(result)
    }
}

/// Probe the FP64 wgpu stack; unavailable devices are not correctness failures.
#[cfg(test)]
fn probe() -> Result<String, String> {
    // A real binary64 device, not the f32-only fallback.
    Ok(Opened::new(1, Kernel::F64)?.name)
}

/// Sample all columns on the FP64 compute device.
pub(crate) fn render(view: &View, p: &Params, zone: &Zone) -> Result<(Header, Samples), String> {
    Renderer::new(Kernel::F64).render(view, p, zone)
}

impl Renderer {
    pub(crate) fn render(
        &mut self,
        view: &View,
        p: &Params,
        zone: &Zone,
    ) -> Result<(Header, Samples), String> {
        let start = Instant::now();
        let (stage, mut header, constants) = inputs(view, p, zone)?;
        header.kernel = header.kernel.replace("vulkan-f64", self.mode.kernel_tag());
        let prep = start.elapsed();
        let count = (p.nx as usize)
            .checked_mul(p.ny as usize)
            .ok_or("GPU grid overflow")?;
        // Keyframes share a sample grid and a zone packing shape.
        let t = Instant::now();
        self.prepare(stage, count, &constants)?;
        let init = t.elapsed();
        let gpu = self.gpu.as_ref().unwrap();
        let mut samples = Samples::alloc(count, p.columns.with(Column::Class));
        let t = Instant::now();
        let (execute, unpack) = gpu.dispatch(&constants, count, |raw| {
            let execute = t.elapsed();
            let t = Instant::now();
            unpack(raw, &mut samples, p.threads).map(|()| (execute, t.elapsed()))
        })??;
        eprintln!(
            "fd: wgpu {} zone on {}: constants {:.3} ms, setup {:.3} ms, dispatch/readback {:.3} ms, unpack {:.3} ms, whole-frame {:.3} ms",
            gpu.mode.label(),
            gpu.name,
            prep.as_secs_f64() * 1e3,
            init.as_secs_f64() * 1e3,
            execute.as_secs_f64() * 1e3,
            unpack.as_secs_f64() * 1e3,
            start.elapsed().as_secs_f64() * 1e3,
        );
        Ok((header, samples))
    }
}

impl Renderer {
    /// Build (or keep) the pipeline and buffers for this keyframe's grid and zone shape.
    fn prepare(&mut self, stage: Stage, count: usize, constants: &[f64]) -> Result<(), String> {
        let shape = match stage {
            Stage::Deep => specialisation(constants),
            Stage::Mid => [0; 3],
        };
        let layout = (stage, count, constants.len(), shape);
        if self.layout != Some(layout) || self.gpu.is_none() {
            self.open(count)?;
            // The f32-only finish holds |z|^2 as a float-float: radius^2 must stay
            // inside f32's range with room for one more squaring.
            let r2 = constants[stage.escape_slot()];
            if self.mode == Kernel::F32Only && (r2.is_nan() || r2 >= 1e36) {
                return Err("the f32-only zone kernel needs an escape radius <= 1e18".into());
            }
            let opened = self.opened.as_ref().expect("opened");
            self.gpu = Some(Gpu::new(opened, count, constants, self.mode, stage)?);
            self.layout = Some(layout);
        }
        Ok(())
    }

    /// Render keyframe `id` and hand its columns to `studio` on the device, with no
    /// sample readback (GPU-21): `studio` must be on this renderer's device
    /// ([`Renderer::device`]). `None` when the columns failed the validity check.
    pub(crate) fn render_resident(
        &mut self,
        view: &View,
        p: &Params,
        zone: &Zone,
        studio: &mut fd_shade::gpu::StudioGpu,
        id: usize,
    ) -> Result<Option<Header>, String> {
        let start = Instant::now();
        let (stage, mut header, constants) = inputs(view, p, zone)?;
        header.kernel = header.kernel.replace("vulkan-f64", self.mode.kernel_tag());
        let count = (p.nx as usize)
            .checked_mul(p.ny as usize)
            .ok_or("GPU grid overflow")?;
        self.prepare(stage, count, &constants)?;
        let gpu = self.gpu.as_ref().unwrap();
        gpu.run(&constants)?;
        let [nu, de, word] = &gpu.outputs;
        let valid = studio.adopt_key(id, [nu, de, word], (p.nx as usize, p.ny as usize))?;
        eprintln!(
            "fd: wgpu {} zone on {} (resident): whole-frame {:.3} ms",
            gpu.mode.label(),
            gpu.name,
            start.elapsed().as_secs_f64() * 1e3,
        );
        Ok(valid.then_some(header))
    }
}

/// Specialisation constants: biseries degree and the two Koenigs series lengths.
fn specialisation(constants: &[f64]) -> [i32; 3] {
    [constants[4], constants[24], constants[25]].map(|v| v as i32)
}

/// Copy the shader's columns into samples, split across threads; the shader
/// already wrote host formats, so each span is plain copies plus a class check.
fn unpack(raw: &Columns, samples: &mut Samples, threads: usize) -> Result<(), String> {
    let count = samples.class.len();
    let parts = threads.max(1).min(count.max(1));
    let span = count.div_ceil(parts).max(1);
    fn split<T>(col: Option<&mut [T]>, span: usize, parts: usize) -> Vec<Option<&mut [T]>> {
        match col {
            Some(c) => c.chunks_mut(span).map(Some).collect(),
            None => (0..parts).map(|_| None).collect(),
        }
    }
    let mut nu = split(samples.nu.as_deref_mut(), span, parts);
    let mut de = split(samples.de.as_deref_mut(), span, parts);
    let mut normal = split(samples.normal.as_deref_mut(), span, parts);
    std::thread::scope(|scope| {
        let mut jobs = Vec::new();
        for (k, class) in samples.class.chunks_mut(span).enumerate() {
            let (nu, de, normal) = (nu[k].take(), de[k].take(), normal[k].take());
            let at = k * span..k * span + class.len();
            let src = Columns {
                nu: &raw.nu[at.clone()],
                de: &raw.de[at.clone()],
                word: &raw.word[at],
            };
            jobs.push(scope.spawn(move || unpack_span(&src, k * span, class, nu, de, normal)));
        }
        jobs.into_iter().try_for_each(|j| {
            j.join()
                .unwrap_or_else(|_| Err("GPU unpack panicked".into()))
        })
    })
}

fn unpack_span(
    src: &Columns,
    first: usize,
    class: &mut [Class],
    nu: Option<&mut [f64]>,
    de: Option<&mut [f32]>,
    normal: Option<&mut [u16]>,
) -> Result<(), String> {
    for (i, &word) in src.word.iter().enumerate() {
        let kind = match word >> 16 {
            0 => Kind::Escaped,
            1 => Kind::Interior,
            2 => Kind::Unresolved,
            _ => {
                return Err(format!(
                    "GPU returned invalid class for pixel {}",
                    first + i
                ))
            }
        };
        if kind == Kind::Escaped && !(src.nu[i].is_finite() && src.de[i].is_finite()) {
            return Err(format!(
                "GPU returned non-finite columns at pixel {}",
                first + i
            ));
        }
        class[i] = Class::new(kind, Evidence::Heuristic);
    }
    if let Some(col) = nu {
        col.copy_from_slice(src.nu);
    }
    if let Some(col) = de {
        col.copy_from_slice(src.de);
    }
    if let Some(col) = normal {
        for (n, &w) in col.iter_mut().zip(src.word) {
            *n = w as u16;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use fd_samples::Column;
    use std::time::Instant;

    #[test]
    fn gpu_02_modes_and_dispatch_grid() {
        assert_eq!(Kernel::parse("f32").unwrap(), Kernel::F32);
        assert_eq!(Kernel::parse("mixed").unwrap(), Kernel::Mixed);
        assert!(Kernel::parse("half").is_err());
        // 2x 1080p keyframes span two rows on 65535-group devices.
        let groups = (3840usize * 2160).div_ceil(64);
        let x = groups.min(65_535);
        assert_eq!(groups.div_ceil(x), 2);
    }

    /// All three kernels parse and validate without a GPU, so CI catches WGSL
    /// errors on machines with no binary64 device.
    #[test]
    fn zone_wgsl_validates() {
        use naga::valid::{Capabilities, ValidationFlags, Validator};
        let kernels = [Kernel::F64, Kernel::Mixed, Kernel::F32, Kernel::F32Only];
        let deep = kernels.map(|mode| (mode, mode.source()));
        let mid = [Kernel::F64, Kernel::F32Only].map(|mode| (mode, mode.mid_source()));
        for (mode, source) in deep.into_iter().chain(mid) {
            let module = naga::front::wgsl::parse_str(&source)
                .unwrap_or_else(|e| panic!("{mode:?}: {}", e.emit_to_string(&source)));
            // f32only must validate with no f64/i64 capability at all (DEC-25).
            let mut caps = Capabilities::FLOAT64 | Capabilities::SHADER_INT64;
            if mode == Kernel::F32Only {
                caps = Capabilities::empty();
            } else if mode != Kernel::F64 {
                caps |= Capabilities::SUBGROUP;
            }
            Validator::new(ValidationFlags::all(), caps)
                .validate(&module)
                .unwrap_or_else(|e| panic!("{mode:?}: {}", e.emit_to_string(&source)));
        }
    }

    #[test]
    fn gpu_zone_f64_parity() {
        match probe() {
            Ok(name) => eprintln!("GPU-13 wgpu FP64 device: {name}"),
            Err(e) if e.contains("no wgpu") => {
                eprintln!("GPU-03 SKIP: {e}");
                return;
            }
            Err(e) => panic!("GPU-13 wgpu init failed: {e}"),
        }
        let zone = Zone::parse(include_str!("../../../bench/zones/v0-core.zone")).unwrap();
        let centre_re =
            "-0.743291890852430202931624158508904039462544013087723088341335644672284698593566";
        let centre_im =
            "0.131240552308797604770845873815964848019374249266625134372668873249132305318161";
        let p = Params {
            nx: 480,
            ny: 270,
            ss: 1,
            max_iter: 20_000,
            escape_radius: 1e10,
            columns: fd_samples::ColumnSet::of(&[Column::Nu, Column::De, Column::Normal]),
            threads: 4,
            tier: None,
        };
        for width in ["1e-35", "1e-38", "1e-40", "1e-43", "1e-46", "2e-48"] {
            let view = View {
                center_re: centre_re.into(),
                center_im: centre_im.into(),
                width: width.into(),
                rotation: 0.0,
            };
            let t = Instant::now();
            let (_, cpu, _, _) = fd_kernel::render_zone(&view, &p, &zone).unwrap();
            let cpu_ms = t.elapsed().as_secs_f64() * 1e3;
            let t = Instant::now();
            let (_, gpu) = render(&view, &p, &zone).unwrap();
            let gpu_ms = t.elapsed().as_secs_f64() * 1e3;
            let (a, b) = (cpu.nu.as_ref().unwrap(), gpu.nu.as_ref().unwrap());
            let (ad, bd) = (cpu.de.as_ref().unwrap(), gpu.de.as_ref().unwrap());
            let (an, bn) = (cpu.normal.as_ref().unwrap(), gpu.normal.as_ref().unwrap());
            let mut maxima = [0.0f64; 3];
            for i in 0..cpu.class.len() {
                assert_eq!(
                    cpu.class[i].kind(),
                    gpu.class[i].kind(),
                    "{width}: class mismatch at ({},{})",
                    i % p.nx as usize,
                    i / p.nx as usize
                );
                if !cpu.class[i].escaped() {
                    continue;
                }
                let de = f64::from(ad[i]);
                let errors = [
                    (a[i] - b[i]).abs() * std::f64::consts::LN_2 * de / 2.0,
                    (f64::from(ad[i]) - f64::from(bd[i])).abs(),
                    f64::from(an[i].wrapping_sub(bn[i]).min(bn[i].wrapping_sub(an[i])))
                        * std::f64::consts::TAU
                        / 65536.0
                        * de,
                ];
                for c in 0..3 {
                    maxima[c] = maxima[c].max(errors[c]);
                    assert!(
                        errors[c].is_finite() && errors[c] <= 1e-3,
                        "{width}: pixel {i}, column {c}, error {} px (CPU/GPU DE {} / {})",
                        errors[c],
                        ad[i],
                        bd[i]
                    );
                }
            }
            eprintln!("GPU-03 width {width}: CPU {cpu_ms:.2} ms/frame, GPU {gpu_ms:.2} ms/frame; max nu/de/normal px = {maxima:?}");
        }
    }

    /// GPU-02: the f32 and mixed kernels across the v0 film's depth range,
    /// including the 1e-38..3e-39 band and the post-1e-42 range that broke
    /// round 1. Class agrees with the CPU zone (up to the f64 kernel's own
    /// count) and positions stay within 0.05 px; under 0.5% of samples may
    /// exceed the strict 1e-3 px, all of them in f32-tail or front
    /// derivative rounding the guard does not bound (DE/normal only).
    #[test]
    fn gpu_zone_f32_guarded_parity() {
        if let Err(e) = probe() {
            eprintln!("GPU-02 SKIP: {e}");
            return;
        }
        let zone = Zone::parse(include_str!("../../../bench/zones/v0-core.zone")).unwrap();
        let p = Params {
            nx: 480,
            ny: 270,
            ss: 1,
            max_iter: 100_000,
            escape_radius: 1e10,
            columns: fd_samples::ColumnSet::of(&[Column::Nu, Column::De, Column::Normal]),
            threads: 8,
            tier: None,
        };
        let widths = [
            "1e-37", "1e-38", "6e-39", "3e-39", "1e-39", "1e-41", "1e-42", "1e-45", "2e-49",
        ];
        for mode in [Kernel::Mixed, Kernel::F32] {
            let mut gpu = Renderer::new(mode);
            for width in widths {
                let view = View {
                    center_re: "-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502".into(),
                    center_im: "0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922".into(),
                    width: width.into(),
                    rotation: 0.0,
                };
                let (_, cpu, _, _) = fd_kernel::render_zone(&view, &p, &zone).unwrap();
                let (_, got) = gpu.render(&view, &p, &zone).unwrap();
                let (a, b) = (cpu.nu.as_ref().unwrap(), got.nu.as_ref().unwrap());
                let (ad, bd) = (cpu.de.as_ref().unwrap(), got.de.as_ref().unwrap());
                let (an, bn) = (cpu.normal.as_ref().unwrap(), got.normal.as_ref().unwrap());
                let (mut mismatched, mut strict, mut worst) = (0, 0, 0.0f64);
                for i in 0..cpu.class.len() {
                    if cpu.class[i].kind() != got.class[i].kind() {
                        mismatched += 1;
                        continue;
                    }
                    if !cpu.class[i].escaped() {
                        continue;
                    }
                    let de = f64::from(ad[i]);
                    let e = [
                        (a[i] - b[i]).abs() * std::f64::consts::LN_2 * de / 2.0,
                        (de - f64::from(bd[i])).abs(),
                        f64::from(an[i].wrapping_sub(bn[i]).min(bn[i].wrapping_sub(an[i])))
                            * std::f64::consts::TAU
                            / 65536.0
                            * de,
                    ]
                    .into_iter()
                    .fold(0.0, f64::max);
                    assert!(e.is_finite(), "{mode:?} {width}: pixel {i} non-finite");
                    worst = worst.max(e);
                    strict += usize::from(e > 1e-3);
                }
                eprintln!("GPU-02 {mode:?} {width}: >1e-3 px {strict}, worst {worst:.2e} px, class mismatches {mismatched}");
                assert!(
                    mismatched <= 1,
                    "{mode:?} {width}: {mismatched} class mismatches"
                );
                assert!(worst <= 0.05, "{mode:?} {width}: worst {worst} px");
                assert!(
                    strict * 200 <= cpu.class.len(),
                    "{mode:?} {width}: {strict} samples > 1e-3 px"
                );
            }
        }
    }

    /// GPU-28: the f32-only kernel (no f64 or i64 requested) at the GPU-02 parity widths,
    /// judged by DEC-29 film-equivalence: the studio look (master-ice, film settings)
    /// shaded from its samples is within one level of the exact CPU zone's on >=99.9%
    /// of pixels. Class, position, DE and normal differences against the CPU reference
    /// are reported (DEC-17, not gating), with ms per keyframe next to mixed.
    #[test]
    fn gpu_zone_f32only_parity() {
        use fd_shade::Pass as _;
        let zone = Zone::parse(include_str!("../../../bench/zones/v0-core.zone")).unwrap();
        let p = Params {
            nx: 480,
            ny: 270,
            ss: 1,
            max_iter: 100_000,
            escape_radius: 1e10,
            columns: fd_samples::ColumnSet::of(&[Column::Nu, Column::De, Column::Normal]),
            threads: 8,
            tier: None,
        };
        let look = fd_shade::Look::parse(include_str!("../../../looks/master-ice.look")).unwrap();
        let film = fd_shade::Appearance {
            time: 1.0,
            flow: look.flow,
            aa: true,
            dither: true,
            unresolved_interior: true,
            ..fd_shade::Appearance::STILL
        };
        let studio = fd_shade::Studio(look);
        let mut f32only = Renderer::new(Kernel::F32Only);
        let mut mixed = Renderer::new(Kernel::Mixed);
        let mut name = None;
        let widths = [
            "1e-37", "1e-38", "6e-39", "3e-39", "1e-39", "1e-41", "1e-42", "1e-45", "2e-49",
        ];
        for width in widths {
            let view = View {
                center_re: "-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502".into(),
                center_im: "0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922".into(),
                width: width.into(),
                rotation: 0.0,
            };
            let (hc, cpu, _, _) = fd_kernel::render_zone(&view, &p, &zone).unwrap();
            let t = Instant::now();
            let got = match f32only.render(&view, &p, &zone) {
                Ok((_, s)) => s,
                Err(e) if e.contains("no wgpu") => {
                    // The zone kernel is Vulkan-only: Windows CI has only DX12 WARP.
                    assert!(
                        std::env::var_os("FD_REQUIRE_GPU").is_none() || cfg!(windows),
                        "no wgpu adapter for f32only, but FD_REQUIRE_GPU is set: {e}"
                    );
                    eprintln!("GPU-28 SKIP: {e}");
                    return;
                }
                Err(e) => panic!("f32only {width}: {e}"),
            };
            let ms = t.elapsed().as_secs_f64() * 1e3;
            name.get_or_insert_with(|| f32only.opened.as_ref().unwrap().name.clone());
            // Mixed for timing, where a binary64 device exists.
            let mixed_ms = {
                let t = Instant::now();
                mixed
                    .render(&view, &p, &zone)
                    .ok()
                    .filter(|_| mixed.mode == Kernel::Mixed)
                    .map(|_| t.elapsed().as_secs_f64() * 1e3)
            };
            let (a, b) = (cpu.nu.as_ref().unwrap(), got.nu.as_ref().unwrap());
            let (ad, bd) = (cpu.de.as_ref().unwrap(), got.de.as_ref().unwrap());
            let (an, bn) = (cpu.normal.as_ref().unwrap(), got.normal.as_ref().unwrap());
            let (mut classes, mut strict, mut worst_pos) = (0, 0, 0.0f64);
            for i in 0..cpu.class.len() {
                if cpu.class[i].kind() != got.class[i].kind() {
                    classes += 1;
                    continue;
                }
                if !cpu.class[i].escaped() {
                    continue;
                }
                let de = f64::from(ad[i]);
                let pos = (a[i] - b[i]).abs() * std::f64::consts::LN_2 * de / 2.0;
                let other = [
                    (de - f64::from(bd[i])).abs(),
                    f64::from(an[i].wrapping_sub(bn[i]).min(bn[i].wrapping_sub(an[i])))
                        * std::f64::consts::TAU
                        / 65536.0
                        * de,
                ];
                worst_pos = worst_pos.max(pos);
                strict += usize::from(pos.max(other[0]).max(other[1]) > 1e-3);
            }
            let (x, y) = (
                studio.shade_with(&hc, &cpu, &film),
                studio.shade_with(&hc, &got, &film),
            );
            let off = x
                .data
                .chunks_exact(3)
                .zip(y.data.chunks_exact(3))
                .filter(|(p, q)| p.iter().zip(q.iter()).any(|(u, v)| u.abs_diff(*v) > 1))
                .count();
            let pixels = x.w * x.h;
            eprintln!(
                "GPU-28 {width} on {}: {off}/{pixels} shaded pixels > 1 level; class {classes}, worst nu position {worst_pos:.2e} px, >1e-3 px (any column) {strict}; {ms:.1} ms/keyframe, mixed {}",
                name.as_deref().unwrap_or("?"),
                mixed_ms.map_or("n/a".into(), |m| format!("{m:.1} ms"))
            );
            assert!(
                off * 1000 <= pixels,
                "{width}: {off} of {pixels} shaded pixels differ by more than one level"
            );
        }
    }

    /// The full v0 zone (with KERN-02's mid-band constants), built by
    /// tools/research/misiurewicz/make_zone.sh: FD_V0_ZONE, else out/zones/v0.zone.
    fn full_v0_zone() -> Option<Zone> {
        let path = std::env::var_os("FD_V0_ZONE").map_or_else(
            || std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../out/zones/v0.zone"),
            std::path::PathBuf::from,
        );
        Some(Zone::parse(&std::fs::read_to_string(path).ok()?).unwrap())
    }

    /// GPU-22: mid-band frames of scripts/kern02_subset.sh's 14 at 960x540, against the
    /// CPU zone: binary64 (mixed) at the strict bar, f32only at DEC-29 film-equivalence.
    #[test]
    fn gpu_zone_mid_parity() {
        use fd_shade::Pass as _;
        let Some(zone) = full_v0_zone() else {
            eprintln!("GPU-22 SKIP: no full v0 zone (FD_V0_ZONE or out/zones/v0.zone)");
            return;
        };
        let rows: Vec<String> = include_str!("../../../bench/path-atlas-v0.txt")
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
            .map(str::to_owned)
            .collect();
        let p = Params {
            nx: 960,
            ny: 540,
            ss: 1,
            max_iter: 100_000,
            escape_radius: 1e10,
            columns: fd_samples::ColumnSet::of(&[Column::Nu, Column::De, Column::Normal]),
            threads: 24,
            tier: None,
        };
        let look = fd_shade::Look::parse(include_str!("../../../looks/master-ice.look")).unwrap();
        let film = fd_shade::Appearance {
            time: 1.0,
            flow: look.flow,
            aa: true,
            dither: true,
            unresolved_interior: true,
            ..fd_shade::Appearance::STILL
        };
        let studio = fd_shade::Studio(look);
        let modes: Vec<Kernel> = match std::env::var("FD_MID_MODES") {
            Ok(m) => m.split(',').map(|k| Kernel::parse(k).unwrap()).collect(),
            Err(_) => vec![Kernel::Mixed, Kernel::F32Only],
        };
        for mode in modes {
            let mut gpu = Renderer::new(mode);
            for index in [
                100, 125, 150, 175, 200, 225, 250, 275, 300, 325, 350, 400, 430, 480,
            ] {
                let f: Vec<&str> = rows[index].split_whitespace().collect();
                let view = View {
                    center_re: f[0].into(),
                    center_im: f[1].into(),
                    width: f[2].into(),
                    rotation: 0.0,
                };
                if zone.gpu_mid_input(&view, &p).is_err() {
                    continue;
                }
                let t = Instant::now();
                let (hc, cpu, _, zs) = fd_kernel::render_zone(&view, &p, &zone).unwrap();
                // Pixels the mid operator declined to the full-perturbation fallback.
                let fallback = 1.0 - zs.jumps as f64 / cpu.class.len() as f64;
                let cpu_ms = t.elapsed().as_secs_f64() * 1e3;
                let t = Instant::now();
                let got = match gpu.render(&view, &p, &zone) {
                    Ok((h, s)) => {
                        assert!(h.kernel.contains("mid"), "{}", h.kernel);
                        s
                    }
                    Err(e) if e.contains("no wgpu") => {
                        eprintln!("GPU-22 SKIP {mode:?}: {e}");
                        break;
                    }
                    Err(e) => panic!("{mode:?} frame {index}: {e}"),
                };
                let ms = t.elapsed().as_secs_f64() * 1e3;
                let (a, b) = (cpu.nu.as_ref().unwrap(), got.nu.as_ref().unwrap());
                let (ad, bd) = (cpu.de.as_ref().unwrap(), got.de.as_ref().unwrap());
                let (an, bn) = (cpu.normal.as_ref().unwrap(), got.normal.as_ref().unwrap());
                let (mut classes, mut strict, mut worst) = (0, 0, [0.0f64; 3]);
                for i in 0..cpu.class.len() {
                    if cpu.class[i].kind() != got.class[i].kind() {
                        classes += 1;
                        continue;
                    }
                    if !cpu.class[i].escaped() {
                        continue;
                    }
                    let de = f64::from(ad[i]);
                    let e = [
                        (a[i] - b[i]).abs() * std::f64::consts::LN_2 * de / 2.0,
                        (de - f64::from(bd[i])).abs(),
                        f64::from(an[i].wrapping_sub(bn[i]).min(bn[i].wrapping_sub(an[i])))
                            * std::f64::consts::TAU
                            / 65536.0
                            * de,
                    ];
                    for c in 0..3 {
                        worst[c] = worst[c].max(e[c]);
                    }
                    strict += usize::from(e.iter().any(|&x| x.is_nan() || x > 1e-3));
                }
                let (x, y) = (
                    studio.shade_with(&hc, &cpu, &film),
                    studio.shade_with(&hc, &got, &film),
                );
                let off = x
                    .data
                    .chunks_exact(3)
                    .zip(y.data.chunks_exact(3))
                    .filter(|(p, q)| p.iter().zip(q.iter()).any(|(u, v)| u.abs_diff(*v) > 1))
                    .count();
                let pixels = x.w * x.h;
                eprintln!(
                    "GPU-22 {mode:?} frame {index} (width {}): class {classes}, >1e-3 px {strict}, worst nu/de/normal px {:.1e} {:.1e} {:.1e}, shaded > 1 level {off}/{pixels}, fallback {:.1}%; GPU {ms:.0} ms, CPU zone {cpu_ms:.0} ms",
                    f[2],
                    worst[0],
                    worst[1],
                    worst[2],
                    fallback * 100.0
                );
                if mode == Kernel::F32Only {
                    assert!(
                        off * 1000 <= pixels,
                        "{mode:?} frame {index}: {off} shaded pixels differ"
                    );
                } else {
                    assert_eq!(classes, 0, "{mode:?} frame {index}: class mismatches");
                    assert_eq!(strict, 0, "{mode:?} frame {index}: samples over 1e-3 px");
                }
            }
        }
    }
}
