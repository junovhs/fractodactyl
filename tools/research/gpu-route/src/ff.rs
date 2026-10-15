//! GPU-09/GPU-05 float-float WGSL finish arm. The device sees no f64: the host
//! (binary64) rounds each landing state to float-float and dz/dc to an f32
//! mantissa plus i32 exponent while uploading, inside the timed finish stage.
use cubecl::prelude::*;

pub struct Runner {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    pub adapter: String,
    pub shader_f64: bool,
    two_prod: TwoProd,
    selection: &'static str,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TwoProd {
    /// One fma per TwoProd; needs a fused fma.
    Fma,
    /// Veltkamp/Dekker splitting, no fma; for drivers that split fma.
    Dekker,
}

/// FD_GPU_ROUTE_FF_TWOPROD=fma|dekker forces a TwoProd; otherwise the
/// self-test picks fma when the device fuses it exactly, else Dekker.
fn forced() -> Option<TwoProd> {
    match std::env::var("FD_GPU_ROUTE_FF_TWOPROD").ok().as_deref() {
        Some("fma") => Some(TwoProd::Fma),
        Some("dekker") => Some(TwoProd::Dekker),
        _ => None,
    }
}

fn guarded() -> bool {
    std::env::var_os("FD_GPU_ROUTE_FF_NOGUARD").is_none()
}

/// FD_GPU_ROUTE_FF_NOGUARD=1 strips the reassociation guard (diagnosis only).
fn source(two_prod: TwoProd) -> String {
    let mut source = include_str!("ff.wgsl").to_owned();
    if !guarded() {
        source = source.replace(
            "fn opq(x: f32) -> f32 { return bitcast<f32>(bitcast<u32>(x) ^ KEY); }",
            "fn opq(x: f32) -> f32 { return x; }",
        );
    }
    if two_prod == TwoProd::Dekker {
        source = source
            .replace("fn ff_mul(x: FF", "fn ff_mul_fma(x: FF")
            .replace("fn ff_mul_dekker(x: FF", "fn ff_mul(x: FF");
    }
    source
}

/// Float-float pair: hi = f32(x), lo = f32(x - hi).
fn split(x: f64) -> (f32, f32) {
    let hi = x as f32;
    (hi, (x - f64::from(hi)) as f32)
}

/// frexp exponent of a positive normal binary64 (mantissa in [0.5, 1)).
fn exponent_of(x: f64) -> i32 {
    ((x.to_bits() >> 52) & 2047) as i32 - 1022
}

fn pow2(k: i32) -> f64 {
    f64::from_bits(((1023 + k) as u64) << 52)
}

/// Mantissa/exponent of a complex value: (re 2^-e, im 2^-e, e).
fn scaled(re: f64, im: f64) -> (f32, f32, i32) {
    let big = re.abs().max(im.abs());
    if big == 0.0 || !big.is_normal() {
        return (re as f32, im as f32, 0);
    }
    let e = exponent_of(big);
    ((re * pow2(-e)) as f32, (im * pow2(-e)) as f32, e)
}

impl Runner {
    /// `FD_GPU_ROUTE_ADAPTER` picks a hardware adapter by name substring
    /// (case-insensitive); otherwise a discrete GPU is preferred.
    pub fn new() -> Result<Self, String> {
        let (two_prod, selection) = match forced() {
            Some(t) => (t, "forced"),
            None => {
                let (_, bad, _) = selftest_run(TwoProd::Fma)?;
                if bad[2] == 0 && bad[3] == 0 {
                    (TwoProd::Fma, "auto")
                } else {
                    (TwoProd::Dekker, "auto_fma_not_fused")
                }
            }
        };
        let mut runner = Self::build(source(two_prod), "GPU-09 float-float finish")?;
        runner.two_prod = two_prod;
        runner.selection = selection;
        Ok(runner)
    }

    pub fn variant(&self) -> String {
        format!(
            "reassociation_guard={} two_prod={:?} two_prod_selection={}",
            if guarded() { "ON" } else { "OFF" },
            self.two_prod,
            self.selection
        )
    }

    fn build(source: String, label: &'static str) -> Result<Self, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let mut adapters =
            cubecl::future::block_on(instance.enumerate_adapters(wgpu::Backends::VULKAN))
                .into_iter()
                .filter(|a| a.get_info().device_type != wgpu::DeviceType::Cpu)
                .collect::<Vec<_>>();
        let wanted = std::env::var("FD_GPU_ROUTE_ADAPTER").ok();
        let adapter = if let Some(want) = &wanted {
            let want = want.to_lowercase();
            adapters
                .into_iter()
                .find(|a| a.get_info().name.to_lowercase().contains(&want))
                .ok_or(format!("no hardware Vulkan adapter matches FD_GPU_ROUTE_ADAPTER={want}"))?
        } else {
            adapters.sort_by_key(|a| a.get_info().device_type != wgpu::DeviceType::DiscreteGpu);
            adapters
                .into_iter()
                .next()
                .ok_or("no hardware Vulkan adapter; software adapters are excluded")?
        };
        let name = adapter.get_info().name;
        let shader_f64 = adapter.features().contains(wgpu::Features::SHADER_F64);
        // Deliberately no SHADER_F64/SHADER_INT64/SUBGROUP: the arm must run without them.
        let required = wgpu::Features::TIMESTAMP_QUERY;
        if !adapter.features().contains(required) {
            return Err(format!("WGSL ff arm needs GPU timestamps; {name} has none"));
        }
        let (device, queue) =
            cubecl::future::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                required_features: required,
                ..Default::default()
            }))
            .map_err(|e| format!("WGSL ff device: {e}"))?;
        let errors = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(label),
            layout: None,
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        if let Some(e) = cubecl::future::block_on(errors.pop()) {
            return Err(format!("WGSL ff pipeline rejected: {e}"));
        }
        Ok(Self {
            device,
            queue,
            pipeline,
            adapter: name,
            shader_f64,
            two_prod: TwoProd::Fma,
            selection: "build",
        })
    }

    fn input(&self, bytes: &[u8]) -> Result<wgpu::Buffer, String> {
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GPU-09 ff inputs"),
            size: bytes.len() as u64,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: true,
        });
        buffer
            .get_mapped_range_mut(..)
            .map_err(|e| format!("WGSL ff mapped input: {e}"))?
            .copy_from_slice(bytes);
        buffer.unmap();
        Ok(buffer)
    }

    /// Same packed checkpoints and camera as the f64 arm; conversion is timed.
    pub fn run_measured(
        &self,
        input: &[f64],
        iterations: &[u32],
        camera: &[f64],
        timing: bool,
    ) -> Result<super::columns::Measured, String> {
        let started = std::time::Instant::now();
        let count = iterations.len();
        if camera.len() != 6 {
            return Err("expected pixel/cos/sin camera".into());
        }
        if count == 0 || input.len() != count * 4 || count > 1920 * 1080 {
            return Err("WGSL ff arm needs 1..1080p packed checkpoints".into());
        }
        let errors = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        // Convert straight into the mapped upload buffer (no staging Vec/copy).
        let inputs = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GPU-09 ff inputs"),
            size: (count * 32) as u64,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: true,
        });
        let allocated_ms = started.elapsed().as_secs_f64() * 1000.0;
        {
            let mut mapped = inputs
                .get_mapped_range_mut(..)
                .map_err(|e| format!("WGSL ff mapped input: {e}"))?;
            // Mapped views are write-only: convert in cache-sized chunks.
            const CHUNK: usize = 65536;
            let mut chunk = Vec::<u32>::with_capacity(CHUNK * 8);
            for (at, states) in input.chunks(CHUNK * 4).enumerate() {
                chunk.clear();
                let first = at * CHUNK;
                for (state, &n) in states.chunks_exact(4).zip(&iterations[first..]) {
                    let (zrh, zrl) = split(state[0]);
                    let (zih, zil) = split(state[1]);
                    let (dr, di, kd) = scaled(state[2], state[3]);
                    chunk.extend_from_slice(&[
                        zrh.to_bits(),
                        zrl.to_bits(),
                        zih.to_bits(),
                        zil.to_bits(),
                        dr.to_bits(),
                        di.to_bits(),
                        kd as u32,
                        n,
                    ]);
                }
                mapped
                    .slice(first * 32..first * 32 + chunk.len() * 4)
                    .copy_from_slice(u32::as_bytes(&chunk));
            }
        }
        inputs.unmap();
        let converted_ms = started.elapsed().as_secs_f64() * 1000.0;
        let (crh, crl) = split(camera[3]);
        let (cih, cil) = split(camera[4]);
        let (hm, _, kh) = scaled(camera[0], 0.0);
        let camera32 = [
            crh.to_bits(),
            crl.to_bits(),
            cih.to_bits(),
            cil.to_bits(),
            (camera[1] as f32).to_bits(),
            (camera[2] as f32).to_bits(),
            (camera[5] as f32).to_bits(),
            hm.to_bits(),
            kh as u32,
            0, // opaque zero for the reassociation guard
        ];
        let camera_buffer = self.input(u32::as_bytes(&camera32))?;
        let nu_bytes = count * 8;
        let de_bytes = count * 4;
        let word_bytes = count * 4;
        let output = |label, size: usize| {
            self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: size as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            })
        };
        let nu = output("GPU-09 nu", nu_bytes);
        let de = output("GPU-09 DE", de_bytes);
        let words = output("GPU-09 words", word_bytes);
        let query = self.device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("GPU-09 timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: 2,
        });
        let query_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GPU-09 timestamp resolve"),
            size: 16,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GPU-09 readback"),
            size: (nu_bytes + de_bytes + word_bytes + if timing { 16 } else { 0 }) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let entries = [&inputs, &camera_buffer, &nu, &de, &words]
            .iter()
            .enumerate()
            .map(|(binding, buffer)| wgpu::BindGroupEntry {
                binding: binding as u32,
                resource: buffer.as_entire_binding(),
            })
            .collect::<Vec<_>>();
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GPU-09 ff finish"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let timestamps = timing.then_some(wgpu::ComputePassTimestampWrites {
                query_set: &query,
                beginning_of_pass_write_index: Some(0),
                end_of_pass_write_index: Some(1),
            });
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: timestamps,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups((count as u32).div_ceil(64), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&nu, 0, &staging, 0, nu_bytes as u64);
        encoder.copy_buffer_to_buffer(&de, 0, &staging, nu_bytes as u64, de_bytes as u64);
        encoder.copy_buffer_to_buffer(
            &words,
            0,
            &staging,
            (nu_bytes + de_bytes) as u64,
            word_bytes as u64,
        );
        if timing {
            encoder.resolve_query_set(&query, 0..2, &query_buffer, 0);
            encoder.copy_buffer_to_buffer(
                &query_buffer,
                0,
                &staging,
                (nu_bytes + de_bytes + word_bytes) as u64,
                16,
            );
        }
        let submit_ms = started.elapsed().as_secs_f64() * 1000.0;
        self.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        staging.map_async(wgpu::MapMode::Read, .., move |result| {
            let _ = tx.send(result);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| format!("WGSL ff poll: {e}"))?;
        rx.recv()
            .map_err(|e| format!("WGSL ff callback: {e}"))?
            .map_err(|e| format!("WGSL ff map: {e}"))?;
        if let Some(e) = cubecl::future::block_on(errors.pop()) {
            return Err(format!("WGSL ff dispatch rejected: {e}"));
        }
        let mapped_ms = started.elapsed().as_secs_f64() * 1000.0;
        let bytes = staging
            .get_mapped_range(..)
            .map_err(|e| format!("WGSL ff mapped output: {e}"))?;
        // The f64 arm's stage ends at mapped columns; decoding is excluded there
        // too, so the clock stops before the diagnostic nu-pair sum.
        let frame_ms = started.elapsed().as_secs_f64() * 1000.0;
        // Host sums the exact (integer, fraction) nu pair into the f64 column.
        let mut nu64 = Vec::with_capacity(nu_bytes);
        for pair in bytes[..nu_bytes].chunks_exact(8) {
            let whole = f32::from_le_bytes(pair[..4].try_into().unwrap());
            let fraction = f32::from_le_bytes(pair[4..].try_into().unwrap());
            nu64.extend((f64::from(whole) + f64::from(fraction)).to_le_bytes());
        }
        if std::env::var_os("FD_GPU_ROUTE_FF_STAGES").is_some() {
            eprintln!(
                "ff_stages alloc_ms={allocated_ms:.3} convert_upload_ms={:.3} setup_ms={:.3} gpu_wait_readback_ms={:.3} map_ms={:.3} frame_ms={frame_ms:.3} nu_pair_decode_ms_excluded={:.3}",
                converted_ms - allocated_ms,
                submit_ms - converted_ms,
                mapped_ms - submit_ms,
                frame_ms - mapped_ms,
                started.elapsed().as_secs_f64() * 1000.0 - frame_ms
            );
        }
        let decoded = super::columns::decoded(
            &nu64,
            &bytes[nu_bytes..nu_bytes + de_bytes],
            &bytes[nu_bytes + de_bytes..nu_bytes + de_bytes + word_bytes],
        );
        let gpu_ms = if timing {
            let at = nu_bytes + de_bytes + word_bytes;
            let start = u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap());
            let end = u64::from_le_bytes(bytes[at + 8..at + 16].try_into().unwrap());
            Some(
                end.wrapping_sub(start) as f64 * f64::from(self.queue.get_timestamp_period()) / 1e6,
            )
        } else {
            None
        };
        drop(bytes);
        staging.unmap();
        let (terminal, words) = decoded?;
        Ok(super::columns::Measured {
            terminal,
            words,
            gpu_ms,
            frame_ms,
            stages: None,
        })
    }
}

/// The ff helpers (TwoSum, TwoProd, float-float add/mul) without the finish
/// kernel's bindings, followed by the self-test entry point.
fn selftest_source(two_prod: TwoProd) -> String {
    let source = source(two_prod);
    let start = source.find("var<private> KEY").unwrap();
    let end = source.find("@compute").unwrap();
    format!("{}\n{}", &source[start..end], include_str!("ff_selftest.wgsl"))
}

/// `ff-selftest`: checks on the device that TwoSum, FastTwoSum, the fma error
/// term and TwoProd are exact, and one float-float z^2 + C step is within
/// 2^-44, scored on the host in binary64. Compiler rewrites that break the
/// error-free transformations show up here directly.
pub fn selftest() -> Result<(), String> {
    let variants = match forced() {
        Some(t) => vec![t],
        None => vec![TwoProd::Fma, TwoProd::Dekker],
    };
    for two_prod in variants {
        let (adapter, bad, step_worst) = selftest_run(two_prod)?;
        // The raw fma error term only matters when TwoProd uses fma.
        let used = if two_prod == TwoProd::Fma { 0 } else { 2 };
        let pass = bad.iter().enumerate().all(|(i, &b)| b == 0 || (used == 2 && i == 2));
        println!(
            "ff_selftest adapter={adapter:?} reassociation_guard={} two_prod={two_prod:?} cases={} two_sum_wrong={} fast_two_sum_wrong={} fma_error_term_wrong={} two_prod_wrong={} ff_step_over_2^-44={} ff_step_worst_relative={step_worst:e} verdict={}",
            if guarded() { "ON" } else { "OFF" },
            SELFTEST_CASES,
            bad[0],
            bad[1],
            bad[2],
            bad[3],
            bad[4],
            if pass { "PASS" } else { "FAIL" }
        );
    }
    Ok(())
}

const SELFTEST_CASES: usize = 1 << 16;

/// Returns the adapter, the failure counts (TwoSum, FastTwoSum, fma error
/// term, TwoProd, z^2 + C step over 2^-44) and the worst step error.
fn selftest_run(two_prod: TwoProd) -> Result<(String, [usize; 5], f64), String> {
    let runner = Runner::build(selftest_source(two_prod), "GPU-09 ff self-test")?;
    const N: usize = SELFTEST_CASES;
    const CR: f64 = -0.743_291_890_852_430_2;
    const CI: f64 = 0.131_240_552_308_797_6;
    // Deterministic xorshift cases: a, b within 2^20 of each other (so a + b
    // is exact in binary64), and float-float states with |z| < 2.
    let mut state = 0x2545_f491_4f6c_dd1du64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 11) as f64 / (1u64 << 53) as f64
    };
    let mut cases = Vec::<f32>::with_capacity(N * 8);
    let mut wide = Vec::<f64>::with_capacity(N * 2);
    for _ in 0..N {
        let a = ((next() * 2.0 - 1.0) * 2f64.powi((next() * 40.0) as i32 - 20)) as f32;
        let b = ((next() * 2.0 - 1.0) * f64::from(a.abs()) * 2f64.powi(-((next() * 20.0) as i32)))
            as f32;
        let (zrh, zrl) = split(next() * 4.0 - 2.0);
        let (zih, zil) = split(next() * 4.0 - 2.0);
        cases.extend([a, b, zrh, zrl, zih, zil, 0.0, 0.0]);
        wide.extend([f64::from(zrh) + f64::from(zrl), f64::from(zih) + f64::from(zil)]);
    }
    let (crh, crl) = split(CR);
    let (cih, cil) = split(CI);
    let key = [0u32, crh.to_bits(), crl.to_bits(), cih.to_bits(), cil.to_bits()];
    let device = &runner.device;
    let inputs = runner.input(f32::as_bytes(&cases))?;
    let key = runner.input(u32::as_bytes(&key))?;
    let out_bytes = (N * 12 * 4) as u64;
    let out = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("GPU-09 self-test out"),
        size: out_bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("GPU-09 self-test readback"),
        size: out_bytes,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let entries = [&inputs, &key, &out]
        .iter()
        .enumerate()
        .map(|(binding, buffer)| wgpu::BindGroupEntry {
            binding: binding as u32,
            resource: buffer.as_entire_binding(),
        })
        .collect::<Vec<_>>();
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &runner.pipeline.get_bind_group_layout(0),
        entries: &entries,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&runner.pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups((N as u32).div_ceil(64), 1, 1);
    }
    encoder.copy_buffer_to_buffer(&out, 0, &staging, 0, out_bytes);
    runner.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    staging.map_async(wgpu::MapMode::Read, .., move |r| {
        let _ = tx.send(r);
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| format!("self-test poll: {e}"))?;
    rx.recv()
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("self-test map: {e}"))?;
    let bytes = staging.get_mapped_range(..).map_err(|e| e.to_string())?;
    let o = bytes
        .chunks_exact(4)
        .map(|b| f64::from(f32::from_le_bytes(b.try_into().unwrap())))
        .collect::<Vec<_>>();
    drop(bytes);
    let mut bad = [0usize; 5];
    let mut step_worst = 0f64;
    for i in 0..N {
        let (a, b) = (f64::from(cases[8 * i]), f64::from(cases[8 * i + 1]));
        let r = &o[12 * i..12 * i + 12];
        // Every f32 sum here and every f32 product is exact in binary64.
        bad[0] += usize::from(r[0] + r[1] != a + b || r[0] != f64::from((a + b) as f32));
        bad[1] += usize::from(r[2] + r[3] != a + b);
        bad[2] += usize::from(r[4] != a * b - f64::from((a * b) as f32));
        bad[3] += usize::from(r[5] + r[6] != a * b);
        let (zr, zi) = (wide[2 * i], wide[2 * i + 1]);
        let (er, ei) = (zr * zr - zi * zi + CR, 2.0 * zr * zi + CI);
        let scale = zr * zr + zi * zi + CR.abs();
        let err = (r[7] + r[8] - er).abs().max((r[9] + r[10] - ei).abs()) / scale;
        step_worst = step_worst.max(err);
        bad[4] += usize::from(err > 2f64.powi(-44));
    }
    Ok((runner.adapter, bad, step_worst))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ff_selftest_wgsl_validates_without_f64() {
        use naga::valid::{Capabilities, ValidationFlags, Validator};
        let source = selftest_source(TwoProd::Fma);
        let module = naga::front::wgsl::parse_str(&source)
            .unwrap_or_else(|e| panic!("{}", e.emit_to_string(&source)));
        Validator::new(ValidationFlags::all(), Capabilities::empty())
            .validate(&module)
            .unwrap_or_else(|e| panic!("{}", e.emit_to_string(&source)));
    }

    /// CPU-only: naga validates the kernel with no capabilities at all, which
    /// proves it uses no f64, int64 or subgroups.
    #[test]
    fn ff_wgsl_validates_without_f64() {
        use naga::valid::{Capabilities, ValidationFlags, Validator};
        for two_prod in [TwoProd::Fma, TwoProd::Dekker] {
            let source = source(two_prod);
            let module = naga::front::wgsl::parse_str(&source)
                .unwrap_or_else(|e| panic!("{}", e.emit_to_string(&source)));
            Validator::new(ValidationFlags::all(), Capabilities::empty())
                .validate(&module)
                .unwrap_or_else(|e| panic!("{}", e.emit_to_string(&source)));
        }
    }

    #[test]
    fn host_split_is_exact_float_float() {
        let x = -0.7432918908524302_f64;
        let (hi, lo) = split(x);
        assert!((f64::from(hi) + f64::from(lo) - x).abs() <= x.abs() * 2f64.powi(-48));
        let (re, im, e) = scaled(3.0e52, -1.0e52);
        assert!((f64::from(re) * 2f64.powi(e) / 3.0e52 - 1.0).abs() < 1e-7);
        assert!((0.5..1.0).contains(&re.abs().max(im.abs())));
    }
}
