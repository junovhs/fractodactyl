//! GPU-12 prerequisite checks. No film render or large dispatch is implicit.
use cubecl::{Device, device::WgpuDeviceKind, prelude::*};
mod ash_finish;
mod columns;
mod encoding;
mod ff;
mod frame;
mod mixed;
mod wgsl;

/// The same scalar recurrence is callable as ordinary Rust and compiled by CubeCL.
/// This is a bounded synthetic smoke check, not a measured zone-frame result.
#[cube]
fn finish<F: Float>(mut zr: F, mut zi: F, mut dr: F, mut di: F, cr: F, ci: F) -> (F, F, F, F) {
    for _ in 0..8 {
        let next_dr = F::new(2.0) * (zr * dr - zi * di) + F::new(1.0);
        let next_di = F::new(2.0) * (zr * di + zi * dr);
        let next_zr = zr * zr - zi * zi + cr;
        zi = F::new(2.0) * zr * zi + ci;
        zr = next_zr;
        dr = next_dr;
        di = next_di;
    }
    (zr, zi, dr, di)
}

#[cube(launch_unchecked)]
fn smoke_kernel<F: Float>(input: &[F], output: &mut [F]) {
    if ABSOLUTE_POS < 8 {
        let at = ABSOLUTE_POS * 6;
        let (zr, zi, dr, di) = finish::<F>(
            input[at],
            input[at + 1],
            input[at + 2],
            input[at + 3],
            input[at + 4],
            input[at + 5],
        );
        let out = ABSOLUTE_POS * 4;
        output[out] = zr;
        output[out + 1] = zi;
        output[out + 2] = dr;
        output[out + 3] = di;
    }
}

/// Actual plain finish recurrence, shared by CPU and GPU.
#[cube]
fn finish_loop<F: Float>(
    mut zr: F,
    mut zi: F,
    mut dr: F,
    mut di: F,
    cr: F,
    ci: F,
    mut n: u32,
    #[comptime] max_iter: u32,
    escape_sq: F,
) -> (F, F, F, F, u32, bool) {
    let mut escaped = false;
    while n < max_iter {
        let next_dr = F::new(2.0) * (zr * dr - zi * di) + F::new(1.0);
        let next_di = F::new(2.0) * (zr * di + zi * dr);
        let next_zr = zr * zr - zi * zi + cr;
        zi = F::new(2.0) * zr * zi + ci;
        zr = next_zr;
        dr = next_dr;
        di = next_di;
        n += 1;
        if zr * zr + zi * zi > escape_sq {
            escaped = true;
            break;
        }
    }
    (zr, zi, dr, di, n, escaped)
}

#[cube(launch_unchecked)]
fn finish_kernel(
    input: &[f64],
    iterations: &[u32],
    output: &mut [f64],
    de_out: &mut [f32],
    words: &mut [u32],
    camera: &[f64],
    #[comptime] mixed: bool,
) {
    if ABSOLUTE_POS < iterations.len() {
        let at = ABSOLUTE_POS * 4;
        let (zr, zi, dr, di, n, escaped) = if mixed {
            mixed::finish(
                input[at],
                input[at + 1],
                input[at + 2],
                input[at + 3],
                camera[3],
                camera[4],
                iterations[ABSOLUTE_POS],
                camera[5],
                camera[0],
                true,
            )
        } else {
            finish_loop::<f64>(
                input[at],
                input[at + 1],
                input[at + 2],
                input[at + 3],
                camera[3],
                camera[4],
                iterations[ABSOLUTE_POS],
                20_000u32,
                camera[5],
            )
        };
        let (nu, de, angle) = if escaped {
            encoding::encode(zr, zi, dr, di, n, camera[0], camera[1], camera[2])
        } else {
            (0.0f64, 0.0f64, 0.0f64)
        };
        output[ABSOLUTE_POS] = nu;
        de_out[ABSOLUTE_POS] = de as f32;
        words[ABSOLUTE_POS] = encoding::packed_word(angle, escaped);
    }
}

fn capabilities() -> Result<(), String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let f64_shader = "@group(0) @binding(0) var<storage, read_write> values: array<f64>; @compute @workgroup_size(1) fn main() { values[0] = values[1] * 2.0; }";
    let mut found = false;
    for adapter in cubecl::future::block_on(instance.enumerate_adapters(wgpu::Backends::VULKAN)) {
        let info = adapter.get_info();
        if info.device_type == wgpu::DeviceType::Cpu {
            continue;
        }
        found = true;
        let f = adapter.features();
        println!(
            "adapter={:?} backend={:?} shader_f64={} subgroup={}",
            info.name,
            info.backend,
            f.contains(wgpu::Features::SHADER_F64),
            f.contains(wgpu::Features::SUBGROUP)
        );
        match naga::front::wgsl::parse_str(f64_shader) {
            Err(e) => println!("wgsl_f64_parser=REJECTED\n{}", e.emit_to_string(f64_shader)),
            Ok(_) => {
                println!("wgsl_f64_parser=ACCEPTED (parser acceptance alone is insufficient)");
                if !f.contains(wgpu::Features::SHADER_F64) {
                    println!("wgsl_f64_pipeline=INELIGIBLE missing shader_f64");
                    continue;
                }
                let (device, _) =
                    cubecl::future::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                        required_features: wgpu::Features::SHADER_F64,
                        ..Default::default()
                    }))
                    .map_err(|e| format!("wgpu device: {e}"))?;
                let errors = device.push_error_scope(wgpu::ErrorFilter::Validation);
                let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("GPU-12 native f64 WGSL eligibility"),
                    source: wgpu::ShaderSource::Wgsl(f64_shader.into()),
                });
                let _pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("GPU-12 eligibility only; no dispatch"),
                    layout: None,
                    module: &module,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    cache: None,
                });
                match cubecl::future::block_on(errors.pop()) {
                    Some(e) => println!("wgsl_f64_pipeline=REJECTED\n{e}"),
                    None => println!(
                        "wgsl_f64_pipeline=ACCEPTED native Vulkan extension; not portable WebGPU"
                    ),
                }
            }
        }
    }
    if !found {
        return Err("no hardware Vulkan adapter; software adapters are excluded".into());
    }
    println!("route_verdict=PENDING; capabilities are not parity or performance evidence");
    Ok(())
}

fn f64_client() -> Result<cubecl::client::Client, String> {
    let device = Device::vulkan(WgpuDeviceKind::DiscreteGpu(0))
        .map_err(|e| format!("CubeCL device: {e:?}"))?;
    let client = device.client();
    println!(
        "cubecl_device={} adapter={}",
        client.name(),
        client.properties().identity.name
    );
    let usage = client
        .features()
        .type_usage(cubecl::ir::ElemType::Float(cubecl::ir::FloatKind::F64));
    if !usage.contains(cubecl::features::TypeUsage::Arithmetic)
        || !usage.contains(cubecl::features::TypeUsage::Buffer)
    {
        return Err(
            "selected hardware does not expose f64 arithmetic and storage through CubeCL SPIR-V"
                .into(),
        );
    }
    Ok(client)
}

fn smoke() -> Result<(), String> {
    let client = f64_client()?;
    let mut input = Vec::<f64>::new();
    let mut expected = Vec::<f64>::new();
    for i in 0..8 {
        let zr = 0.1 + i as f64 * 0.001;
        let zi = 0.05;
        let (a, b, c, d) = finish::<f64>(zr, zi, 1.0, 0.0, -0.75, 0.1);
        input.extend([zr, zi, 1.0, 0.0, -0.75, 0.1]);
        expected.extend([a, b, c, d]);
    }
    let output = client.empty(32 * size_of::<f64>());
    let input = client.create_from_slice(f64::as_bytes(&input));
    unsafe {
        smoke_kernel::launch_unchecked::<f64>(
            &client,
            CubeCount::Static(1, 1, 1),
            CubeDim::new_1d(8),
            BufferArg::from_raw_parts(input, 48),
            BufferArg::from_raw_parts(output.clone(), 32),
        );
    }
    let bytes = client
        .read_one(output)
        .map_err(|e| format!("GPU read: {e:?}"))?;
    let actual = f64::from_bytes(&bytes);
    let error = actual
        .iter()
        .zip(&expected)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f64, f64::max);
    if actual.len() != expected.len() || actual.iter().any(|v| !v.is_finite()) || error > 1e-12 {
        return Err(format!(
            "shared scalar smoke failed: max absolute difference={error}"
        ));
    }
    println!("shared_source_smoke=PASS samples=8 iterations=8 max_absolute_difference={error:e}");
    println!("zone_frame_parity=UNMEASURED route_verdict=PENDING");
    Ok(())
}

fn capture() -> Result<(), String> {
    use fd_samples::{Column, ColumnSet, View};
    let zone = fd_kernel::Zone::parse(include_str!("../../../../bench/zones/v0-core.zone"))?;
    let p = fd_kernel::Params {
        nx: 16,
        ny: 9,
        ss: 1,
        max_iter: 20_000,
        escape_radius: 1e10,
        columns: ColumnSet::of(&[Column::Nu, Column::De, Column::Normal]),
        threads: 1,
        tier: None,
    };
    // Exact camera strings, not rounded absolute f64 coordinates.
    let re = "-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502";
    let im = "0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922";
    for width in ["1e-37", "1e-40", "1e-43", "1e-46", "2e-48", "2e-49"] {
        let view = View {
            center_re: re.into(),
            center_im: im.into(),
            width: width.into(),
            rotation: 0.0,
        };
        let (h, samples, states, _) = zone.probe_finish_inputs(&view, &p)?;
        let (reference_h, reference, _, _) = fd_kernel::render_zone(&view, &p, &zone)?;
        if h != reference_h
            || samples.class != reference.class
            || samples.nu != reference.nu
            || samples.de != reference.de
            || samples.normal != reference.normal
        {
            return Err(format!("CPU capture changes reference samples at {width}"));
        }
        let count = states.iter().flatten().count();
        if count == 0 {
            return Err(format!("no finish-stage inputs at {width}"));
        }
        println!(
            "capture width={width} grid=16x9 finish_inputs={count} reference_columns=IDENTICAL"
        );
    }
    println!("full_frame_route_comparison=PENDING");
    Ok(())
}

fn main() {
    // The runtime smoke is deliberately tiny; cut off even driver setup hangs.
    let (done, finished) = std::sync::mpsc::channel::<()>();
    std::thread::spawn(move || {
        if finished
            .recv_timeout(std::time::Duration::from_secs(30))
            .is_err()
        {
            eprintln!("GPU-12 probe exceeded its 30-second wall limit");
            std::process::exit(124);
        }
    });
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let result = match args.first().map(String::as_str) {
        Some("capabilities") => capabilities(),
        Some("smoke") => smoke(),
        Some("capture") => capture(),
        Some("zone-smoke") => columns::smoke(),
        Some("capture-frame") | Some("frame") | Some("capture-grid") | Some("grid") => {
            (|| {
                let width = args.get(1).ok_or("specify a listed width")?;
                let small = args[0].ends_with("grid");
                let nx = if small {args.get(2).ok_or("specify nx")?.parse::<u32>().map_err(|e| e.to_string())?} else {1920};
                let ny = if small {args.get(3).ok_or("specify ny")?.parse::<u32>().map_err(|e| e.to_string())?} else {1080};
                if args[0].starts_with("capture") { frame::capture(width, nx, ny) }
                else {
                    let at = if small {4} else {2};
                    frame::run(width, nx, ny, args.get(at).map(String::as_str), args.get(at+1).map(String::as_str))
                }
            })()
        }
        _ => Err("usage: gpu-route-probe capabilities|smoke|capture|zone-smoke (smoke: 8x8; zone-smoke: six 16x9 grids plus rotation, <=20000 iterations each)".into()),
    };
    let _ = done.send(());
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
