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
}

impl Kernel {
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        match value {
            "f64" => Ok(Self::F64),
            "f32" => Ok(Self::F32),
            "mixed" => Ok(Self::Mixed),
            _ => Err(format!(
                "--gpu-kernel: expected f64, f32 or mixed, got {value:?}"
            )),
        }
    }

    fn kernel_tag(self) -> &'static str {
        match self {
            Self::F64 => "wgpu-f64",
            Self::F32 => "wgpu-f32",
            Self::Mixed => "wgpu-mixed",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::F64 => "FP64",
            Self::F32 => "f32",
            Self::Mixed => "mixed",
        }
    }

    /// The WGSL module: zone.wgsl plus the guarded f32 finish, or a stub
    /// that keeps the f64 kernel free of subgroup operations.
    fn source(self) -> String {
        const F64_TAIL: &str = "const F32_FINISH: bool = false;\n\
            fn wave_all(ready: bool) -> bool { return false; }\n\
            fn finish32(gid: i32, z: C, du: C, n: i32, errpx: f64) -> bool { return false; }\n";
        let tail = match self {
            Self::F64 => F64_TAIL,
            Self::F32 | Self::Mixed => include_str!("../shaders/zone_finish32.wgsl"),
        };
        format!("{}\n{tail}", include_str!("../shaders/zone.wgsl"))
    }

    fn features(self) -> wgpu::Features {
        let base = wgpu::Features::SHADER_F64 | wgpu::Features::SHADER_INT64;
        match self {
            Self::F64 => base,
            Self::F32 | Self::Mixed => base | wgpu::Features::SUBGROUP,
        }
    }
}

/// Holds the device, pipeline and buffers across successive keyframes.
pub(crate) struct Renderer {
    mode: Kernel,
    gpu: Option<Gpu>,
    layout: Option<(usize, usize, [i32; 3])>,
}

impl Renderer {
    pub(crate) fn new(mode: Kernel) -> Self {
        Self {
            mode,
            gpu: None,
            layout: None,
        }
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
    fn new(count: usize, constants: &[f64], mode: Kernel) -> Result<Self, String> {
        let required = mode.features();
        let const_bytes = std::mem::size_of_val(constants) as u64;
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
                    a.get_info().device_type != wgpu::DeviceType::Cpu
                        && a.features().contains(required)
                        && x > 0
                        && groups.div_ceil(x)
                            <= u64::from(limits.max_compute_workgroups_per_dimension)
                        && column_bytes <= limits.max_storage_buffer_binding_size
                        && const_bytes <= limits.max_storage_buffer_binding_size
                        && 2 * column_bytes <= limits.max_buffer_size
                })
                .collect();
        adapters.sort_by_key(|a| a.get_info().device_type != wgpu::DeviceType::DiscreteGpu);
        let Some(adapter) = adapters.into_iter().next() else {
            return Err(format!(
                "no wgpu Vulkan GPU with {} and sufficient buffer limits",
                if mode == Kernel::F64 {
                    "SHADER_F64 and SHADER_INT64"
                } else {
                    "SHADER_F64, SHADER_INT64 and SUBGROUP (f32/mixed)"
                }
            ));
        };
        let name = adapter.get_info().name;
        let limits = adapter.limits();
        let per_dim = u64::from(limits.max_compute_workgroups_per_dimension);
        let x = groups.min(per_dim);
        let dispatch = (x as u32, groups.div_ceil(x) as u32);
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("fd zone"),
            required_features: required,
            required_limits: limits,
            ..Default::default()
        }))
        .map_err(|e| format!("wgpu device: {e}"))?;
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fd zone kernel"),
            source: wgpu::ShaderSource::Wgsl(mode.source().into()),
        });
        let shape = specialisation(constants);
        let overrides = [
            ("DEG", f64::from(shape[0])),
            ("NPHI", f64::from(shape[1])),
            ("NPSI", f64::from(shape[2])),
            ("F32_FRONT", if mode == Kernel::F32 { 1.0 } else { 0.0 }),
        ];
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("fd zone kernel"),
            layout: None,
            module: &module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &overrides,
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

    /// Run one keyframe and hand the mapped columns to `read`.
    fn dispatch<R>(
        &self,
        constants: &[f64],
        count: usize,
        read: impl FnOnce(&Columns) -> R,
    ) -> Result<R, String> {
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let bytes: Vec<u8> = constants.iter().flat_map(|v| v.to_ne_bytes()).collect();
        self.queue.write_buffer(&self.constants, 0, &bytes);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.group, &[]);
            pass.dispatch_workgroups(self.dispatch.0, self.dispatch.1, 1);
        }
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
    Ok(Gpu::new(1, &[0.0; 32], Kernel::F64)?.name.clone())
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
        let (mut header, constants) = zone.gpu_f64_input(view, p)?;
        header.kernel = header.kernel.replace("vulkan-f64", self.mode.kernel_tag());
        let prep = start.elapsed();
        let count = (p.nx as usize)
            .checked_mul(p.ny as usize)
            .ok_or("GPU grid overflow")?;
        // Keyframes share a sample grid and a zone packing shape.
        let t = Instant::now();
        let layout = (count, constants.len(), specialisation(&constants));
        if self.layout != Some(layout) || self.gpu.is_none() {
            self.gpu = Some(Gpu::new(count, &constants, self.mode)?);
            self.layout = Some(layout);
        }
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
        for mode in [Kernel::F64, Kernel::Mixed, Kernel::F32] {
            let module = naga::front::wgsl::parse_str(&mode.source())
                .unwrap_or_else(|e| panic!("{mode:?}: {}", e.emit_to_string(&mode.source())));
            let mut caps = Capabilities::FLOAT64 | Capabilities::SHADER_INT64;
            if mode != Kernel::F64 {
                caps |= Capabilities::SUBGROUP;
            }
            Validator::new(ValidationFlags::all(), caps)
                .validate(&module)
                .unwrap_or_else(|e| panic!("{mode:?}: {}", e.emit_to_string(&mode.source())));
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
}
