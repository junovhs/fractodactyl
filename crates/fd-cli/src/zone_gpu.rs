//! wgpu zone samples (GPU-13), the zone's only implementation (CLEA-04, DEC-28). The CPU
//! preflight owns decimal camera arithmetic and the zone validity gate; a frame the GPU
//! declines renders with the perturbation kernel.
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

/// The zone kernel for a `--zone` frame source (CLEA-06): binary64 unless `--gpu-kernel`
/// names one, or f32only for an extended-range zone (the only GPU kernel for those).
/// Both meet the strict bar on the committed fixtures, which mixed does not (a few
/// samples' DE off by up to 0.3%); on the RX 5700 XT binary64 is as fast as mixed at
/// 1080p (50 vs 52 ms deep, 48 vs 48 ms mid; f32only 93 and 133 ms). Without an
/// f64-capable GPU the renderer opens the f32-only kernel instead, on a software
/// adapter if need be (DEC-28).
pub(crate) fn choose(flag: Option<&str>, zone: &Zone) -> Result<Kernel, String> {
    match flag {
        Some(value) => Kernel::parse(value),
        None if zone.extended() => Ok(Kernel::F32Only),
        None => Ok(Kernel::F64),
    }
}

/// One zone-covered frame on the wgpu zone kernel, or `None` for the caller to render
/// it with the perturbation kernel (the zone's only CPU route since CLEA-04): when
/// `gpu` holds no renderer, when no GPU stage takes the frame, or on a renderer error,
/// which also drops the renderer for later frames. Each fallback prints one line.
pub(crate) fn render_zone_frame(
    gpu: &mut Option<Renderer>,
    view: &View,
    p: &Params,
    zone: &Zone,
) -> Option<(Header, Samples)> {
    let renderer = gpu.as_mut()?;
    match renderer.render_or_decline(view, p, zone) {
        Ok(Ok(frame)) => return Some(frame),
        Ok(Err(why)) => eprintln!(
            "fd: the wgpu zone kernel declines this frame ({why}); using the perturbation kernel"
        ),
        Err(e) => {
            eprintln!("fd: wgpu zone unavailable ({e}); using the perturbation kernel");
            *gpu = None;
        }
    }
    None
}

impl Kernel {
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        match value {
            "f64" => Ok(Self::F64),
            "f32" => Ok(Self::F32),
            "mixed" => Ok(Self::Mixed),
            "f32only" => Ok(Self::F32Only),
            "cpu" => Err(
                "--gpu-kernel cpu: the CPU zone was deleted (CLEA-04); without \
                          --zone, frames render with the perturbation kernel"
                    .into(),
            ),
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

/// Which zone operator a frame uses, as the zone's coverage gate decides it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    /// Return map, approach, Koenigs jump or patch, finish (zone.wgsl / zone32.wgsl).
    Deep,
    /// KERN-02's co-moving two-cycle jump (zone_mid.wgsl / zone_mid32.wgsl, GPU-22).
    Mid,
    /// [`Stage::Deep`] on an extended-range zone (GPU-23, f32only only): the binary
    /// exponent table starts at this constant slot.
    Extended(usize),
}

impl Stage {
    /// Slot of r^2 in the stage's constants.
    fn escape_slot(self) -> usize {
        match self {
            Self::Deep | Self::Extended(_) => 21,
            Self::Mid => 3,
        }
    }
}

/// Append slot exponents two per 8-byte word (low word first), as zone32_lib.wgsl's
/// `fe_at` reads them; returns the first exponent slot (`EXP_BASE`).
fn pack_exponents(v: &mut Vec<f64>, exps: &[i32]) -> usize {
    let base = v.len();
    v.extend(exps.chunks(2).map(|e| {
        let hi = e.get(1).copied().unwrap_or(0) as u32;
        f64::from_bits(u64::from(e[0] as u32) | u64::from(hi) << 32)
    }));
    base
}

/// The frame's constants for whichever operator the coverage gate picks for it.
fn inputs(
    view: &View,
    p: &Params,
    zone: &Zone,
    mode: Kernel,
) -> Result<(Stage, Header, Vec<f64>), String> {
    let deep = match zone.gpu_f64_input(view, p) {
        Ok((h, v)) => return Ok((Stage::Deep, h, v)),
        Err(e) => e,
    };
    if zone.extended() {
        if mode != Kernel::F32Only {
            return Err("extended-range zones run on the GPU with --gpu-kernel f32only".into());
        }
        let (h, mut v, exps) = zone.gpu_extended_input(view, p)?;
        let base = pack_exponents(&mut v, &exps);
        return Ok((Stage::Extended(base), h, v));
    }
    zone.gpu_mid_input(view, p)
        .map(|(h, v)| (Stage::Mid, h, v))
        .map_err(|_| deep)
}

impl Renderer {
    /// Why no GPU stage takes this frame (the perturbation kernel renders it), or None.
    pub(crate) fn declines(&self, view: &View, p: &Params, zone: &Zone) -> Option<String> {
        inputs(view, p, zone, self.mode).err()
    }

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
        // The binary64 kernels are proven on Vulkan only (GPU-12/19/20). The f32-only
        // kernel runs on any adapter (CLEA-05): Vulkan or DX12, the latter through the
        // statically linked DXC, as fd-shade's passes (FIX-59).
        let choice = fd_shade::gpu::adapters::choice()?;
        let inventory = if choice.is_some() {
            fd_shade::gpu::adapters::enumerate()
        } else {
            fd_shade::gpu::adapters::native()
        };
        if let Some(index) = choice {
            fd_shade::gpu::adapters::selected(&inventory, index)?;
        }
        let mut adapters: Vec<_> = inventory
            .into_iter()
            .enumerate()
            .filter(|(index, _)| choice.is_none_or(|selected| selected == *index))
            .map(|(_, a)| a)
            .filter(|a| {
                let limits = a.limits();
                let x = groups.min(u64::from(limits.max_compute_workgroups_per_dimension));
                (mode == Kernel::F32Only || a.get_info().backend == wgpu::Backend::Vulkan)
                    && (mode == Kernel::F32Only
                        || a.get_info().device_type != wgpu::DeviceType::Cpu)
                    && a.features().contains(required)
                    && x > 0
                    && groups.div_ceil(x) <= u64::from(limits.max_compute_workgroups_per_dimension)
                    && column_bytes <= limits.max_storage_buffer_binding_size
                    && 2 * column_bytes <= limits.max_buffer_size
            })
            .collect();
        // Hardware first, software (lavapipe, WARP) last; Vulkan before DX12.
        adapters.sort_by_key(|a| {
            let info = a.get_info();
            let kind = match info.device_type {
                wgpu::DeviceType::DiscreteGpu => 0,
                wgpu::DeviceType::IntegratedGpu => 1,
                wgpu::DeviceType::Cpu => 3,
                _ => 2,
            };
            (kind, info.backend != wgpu::Backend::Vulkan)
        });
        let Some(adapter) = adapters.into_iter().next() else {
            return Err(format!(
                "no wgpu adapter with {} and sufficient buffer limits",
                if mode == Kernel::F32Only {
                    "no f64 features (f32only)"
                } else if mode == Kernel::F64 {
                    "SHADER_F64 and SHADER_INT64"
                } else {
                    "SHADER_F64, SHADER_INT64 and SUBGROUP (f32/mixed)"
                }
            ));
        };
        let info = adapter.get_info();
        let name = info.name;
        if let Some(index) = choice {
            eprintln!("fd: zone adapter {index}: {name} ({:?})", info.backend);
        }
        if info.device_type == wgpu::DeviceType::Cpu && choice.is_none() {
            eprintln!("fd: no hardware GPU for the zone kernel; running it on the software adapter {name} ({:?})", info.backend);
        }
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

/// The f32-only kernels' second entry point and the bytes of per-sample state it reads
/// (zone32.wgsl and zone_mid32.wgsl STATE_WORDS): each kernel is two entry points so
/// that neither is too large for DX12 WARP (CLEA-05, CLEA-06).
fn second_entry(stage: Stage) -> (&'static str, u64) {
    match stage {
        Stage::Deep | Stage::Extended(_) => ("main_front", 9 * 4),
        Stage::Mid => ("main_finish", 8 * 4),
    }
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    group: wgpu::BindGroup,
    /// The f32-only kernel's second entry point (second_entry) and its group;
    /// the per-sample state between the two lives in a device-only buffer (CLEA-05).
    front: Option<(wgpu::ComputePipeline, wgpu::BindGroup)>,
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
        let split = mode == Kernel::F32Only;
        let (second, state_per_sample) = second_entry(stage);
        let state_bytes = column_bytes / 8 * state_per_sample;
        if split
            && (state_bytes > limits.max_storage_buffer_binding_size
                || state_bytes > limits.max_buffer_size)
        {
            return Err("zone keyframe state exceeds the wgpu device's limits".into());
        }
        let (name, device, queue) = (
            opened.name.clone(),
            opened.device.clone(),
            opened.queue.clone(),
        );
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let source = match stage {
            Stage::Deep | Stage::Extended(_) => mode.source(),
            Stage::Mid => mode.mid_source(),
        };
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fd zone kernel"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let shape = match stage {
            Stage::Deep | Stage::Extended(_) => specialisation(constants),
            Stage::Mid => [0; 3],
        };
        let exp_base = match stage {
            Stage::Extended(base) => base as f64,
            _ => -1.0,
        };
        let all = [
            ("DEG", f64::from(shape[0])),
            ("NPHI", f64::from(shape[1])),
            ("NPSI", f64::from(shape[2])),
            ("F32_FRONT", if mode == Kernel::F32 { 1.0 } else { 0.0 }),
        ];
        let extended = [
            ("DEG", f64::from(shape[0])),
            ("NPHI", f64::from(shape[1])),
            ("NPSI", f64::from(shape[2])),
            ("EXP_BASE", exp_base),
        ];
        let overrides = match (stage, mode) {
            (Stage::Mid, _) => &all[..0],
            (Stage::Extended(_), _) => &extended[..],
            (Stage::Deep, Kernel::F32Only) => &all[..3],
            (Stage::Deep, _) => &all[..],
        };
        // A split kernel's entries share one bind group, so its layout is explicit: an
        // entry's automatic layout lists only the bindings it touches.
        let layout = split.then(|| {
            let entries: Vec<_> = (0..5)
                .map(|binding| wgpu::BindGroupLayoutEntry {
                    binding,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: binding == 0,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                })
                .collect();
            let group = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("fd zone"),
                entries: &entries,
            });
            let pipeline = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("fd zone"),
                bind_group_layouts: &[Some(&group)],
                immediate_size: 0,
            });
            (group, pipeline)
        });
        let entry = |name| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("fd zone kernel"),
                layout: layout.as_ref().map(|(_, pipeline)| pipeline),
                module: &module,
                entry_point: Some(name),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: overrides,
                    ..Default::default()
                },
                cache: None,
            })
        };
        let pipeline = entry("main");
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
        let state = split.then(|| storage("zone state", state_bytes, wgpu::BufferUsages::STORAGE));
        let entries: Vec<_> = std::iter::once(&constants_buffer)
            .chain(&outputs)
            .chain(&state)
            .enumerate()
            .map(|(binding, buffer)| wgpu::BindGroupEntry {
                binding: binding as u32,
                resource: buffer.as_entire_binding(),
            })
            .collect();
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fd zone"),
            layout: &match &layout {
                Some((group, _)) => group.clone(),
                None => pipeline.get_bind_group_layout(0),
            },
            entries: &entries,
        });
        let front = split.then(|| (entry(second), group.clone()));
        if let Some(e) = pollster::block_on(scope.pop()) {
            return Err(format!("wgpu zone pipeline rejected: {e}"));
        }
        Ok(Self {
            device,
            queue,
            pipeline,
            group,
            front,
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
            if let Some((pipeline, group)) = &self.front {
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, group, &[]);
                pass.dispatch_workgroups(self.dispatch.0, self.dispatch.1, 1);
            }
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

impl Renderer {
    pub(crate) fn render(
        &mut self,
        view: &View,
        p: &Params,
        zone: &Zone,
    ) -> Result<(Header, Samples), String> {
        self.render_or_decline(view, p, zone)?
    }

    /// [`Renderer::render`], with a frame no GPU stage takes as the inner error and a
    /// device or pipeline failure as the outer one.
    pub(crate) fn render_or_decline(
        &mut self,
        view: &View,
        p: &Params,
        zone: &Zone,
    ) -> Result<Result<(Header, Samples), String>, String> {
        let start = Instant::now();
        let (stage, mut header, constants) = match inputs(view, p, zone, self.mode) {
            Ok(frame) => frame,
            Err(why) => return Ok(Err(why)),
        };
        let prep = start.elapsed();
        let count = (p.nx as usize)
            .checked_mul(p.ny as usize)
            .ok_or("GPU grid overflow")?;
        // Keyframes share a sample grid and a zone packing shape.
        let t = Instant::now();
        self.prepare(stage, count, &constants)?;
        let init = t.elapsed();
        // After prepare: opening the device may have switched to the f32-only kernel.
        header.kernel = header.kernel.replace("vulkan-f64", self.mode.kernel_tag());
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
        Ok(Ok((header, samples)))
    }
}

impl Renderer {
    /// Build (or keep) the pipeline and buffers for this keyframe's grid and zone shape.
    fn prepare(&mut self, stage: Stage, count: usize, constants: &[f64]) -> Result<(), String> {
        let shape = match stage {
            Stage::Deep | Stage::Extended(_) => specialisation(constants),
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
        let (stage, mut header, constants) = inputs(view, p, zone, self.mode)?;
        let count = (p.nx as usize)
            .checked_mul(p.ny as usize)
            .ok_or("GPU grid overflow")?;
        self.prepare(stage, count, &constants)?;
        header.kernel = header.kernel.replace("vulkan-f64", self.mode.kernel_tag());
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

    /// GPU-23: slot exponents at -3000 and +3000 survive the packing and zone32_lib's
    /// `fe_at`/`ce_at` on the device, including a zero mantissa that carries a huge
    /// exponent next to a tiny partner (a zero must not decide CE alignment).
    #[test]
    fn zone_extended_packing_roundtrip() {
        use fd_shade::gpu::testkit::{dispatch, or_skip, Binding};
        // (mantissa, exponent) per slot; slots 6..7 and 8..9 are complex pairs.
        let slots: [(f64, i32); 10] = [
            (1.5, -3000),
            (-1.25, 3000),
            (0.75, 0),
            (1.0000000001, -3000),
            (-1.9999999, 2999),
            (0.0, 3000),
            (1.5, -3000),
            (0.0, 0),
            (0.0, 3000),
            (1.25, -3000),
        ];
        let mut v: Vec<f64> = slots.iter().map(|s| s.0).collect();
        let exps: Vec<i32> = slots.iter().map(|s| s.1).collect();
        let base = pack_exponents(&mut v, &exps);
        let words: Vec<u8> = v.iter().flat_map(|x| x.to_ne_bytes()).collect();
        let entry = r#"
@compute @workgroup_size(1)
fn probe() {
    ff_init(input_data[2].x);
    for (var i = 0; i < 6; i += 1) {
        let x = fe_at(i);
        nu_out[2 * i] = vec2<u32>(bitcast<u32>(x.m.hi), bitcast<u32>(x.m.lo));
        nu_out[2 * i + 1] = vec2<u32>(bitcast<u32>(x.e), 0u);
    }
    for (var k = 0; k < 2; k += 1) {
        let z = ce_at(6 + 2 * k);
        nu_out[12 + 3 * k] = vec2<u32>(bitcast<u32>(z.re.hi), bitcast<u32>(z.re.lo));
        nu_out[13 + 3 * k] = vec2<u32>(bitcast<u32>(z.im.hi), bitcast<u32>(z.im.lo));
        nu_out[14 + 3 * k] = vec2<u32>(bitcast<u32>(z.e), 0u);
    }
}
"#;
        let source = format!(
            "{}\n{}\n{}\n{entry}",
            crate::zone_ff::FF,
            include_str!("../../fd-shade/shaders/soft_f64.wgsl"),
            include_str!("../shaders/zone32_lib.wgsl").replace(
                "override EXP_BASE: i32 = -1;",
                &format!("const EXP_BASE: i32 = {base};")
            )
        );
        let Some((name, out)) = or_skip(
            dispatch(
                "fd zone extended packing",
                &source,
                "probe",
                &[Binding::Read(&words), Binding::Out(18 * 8)],
                (1, 1),
            ),
            "GPU-23 packing",
        ) else {
            return;
        };
        let w: Vec<u32> = out
            .chunks_exact(4)
            .map(|b| u32::from_ne_bytes(b.try_into().unwrap()))
            .collect();
        let ff =
            |at: usize| f64::from(f32::from_bits(w[at])) + f64::from(f32::from_bits(w[at + 1]));
        // m 2^e with m in [0.5, 1), as fe_norm leaves it.
        let norm = |m: f64, e: i32| {
            if m == 0.0 {
                return (0.0, 0);
            }
            let k = m.abs().log2().floor() as i32 + 1;
            (m / 2f64.powi(k), e + k)
        };
        for (i, &(m, e)) in slots.iter().take(6).enumerate() {
            let (want, we) = norm(m, e);
            let (got, ge) = (ff(4 * i), w[4 * i + 2] as i32);
            assert!(
                (got - want).abs() <= 2f64.powi(-44) && (want == 0.0 || ge == we),
                "{name}: slot {i}: got {got:e} * 2^{ge}, want {want:e} * 2^{we}"
            );
        }
        for k in 0..2 {
            let at = 24 + 6 * k;
            let (re, im, e) = (ff(at), ff(at + 2), w[at + 4] as i32);
            let (want_re, want_im) = (slots[6 + 2 * k], slots[7 + 2 * k]);
            // The pair's one exponent belongs to its non-zero part.
            let (m, we) = if want_re.0 != 0.0 {
                norm(want_re.0, want_re.1)
            } else {
                norm(want_im.0, want_im.1)
            };
            let (gr, gi) = if want_re.0 != 0.0 { (re, im) } else { (im, re) };
            assert!(
                (gr - m).abs() <= 2f64.powi(-44) && gi == 0.0 && e == we,
                "{name}: pair {k}: got ({re:e}, {im:e}) * 2^{e}, want {m:e} * 2^{we}"
            );
        }
        eprintln!("GPU-23 packing on {name}: exponents -3000..3000 round-trip");
    }
}
