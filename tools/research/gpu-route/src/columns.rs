//! Six-depth tiny-grid recurrence and GPU encoding gate. Output remains
//! diagnostic terminal state plus columns; packed-buffer performance is pending.
use cubecl::prelude::*;
use fd_samples::{Column, ColumnSet, Kind, Samples, View};

#[derive(Default)]
struct Gate {
    class_wrong: usize,
    iteration_wrong: usize,
    displacement_wrong: usize,
    de_wrong: usize,
    normal_wrong: usize,
    near_de_wrong: usize,
    near_normal_wrong: usize,
    nonfinite: usize,
    max_px: f64,
    max_de_relative: f64,
    max_normal_units: u16,
}

fn gate(
    reference: &Samples,
    indices: &[usize],
    terminal: &[f64],
    words: &[u32],
    expected_words: &[u32],
) -> Result<Gate, String> {
    if terminal.len() != indices.len() * 7
        || words.len() != indices.len() * 2
        || expected_words.len() != indices.len() * 2
    {
        return Err("finish arm returned the wrong buffer lengths".into());
    }
    let mut g = Gate::default();
    for (i, &index) in indices.iter().enumerate() {
        let n = words[2 * i];
        let class = words[2 * i + 1];
        let escaped = reference.class[index].kind() == Some(Kind::Escaped);
        let want_class = if escaped { 0 } else { 2 };
        g.class_wrong += usize::from(class != want_class);
        g.iteration_wrong += usize::from(n != u32::MAX && n != expected_words[2 * i]);
        if class != 0 || !escaped {
            continue;
        }
        let z = &terminal[7 * i..7 * i + 4];

        let nu = terminal[7 * i + 4];
        let de = terminal[7 * i + 5] as f32;
        let angle = terminal[7 * i + 6];
        let normal = (angle * (65536.0 / std::f64::consts::TAU)).round() as i64 as u16;
        let rn = reference.nu.as_ref().unwrap()[index];
        let rd = reference.de.as_ref().unwrap()[index];
        let rm = reference.normal.as_ref().unwrap()[index];
        if z.iter().any(|v| !v.is_finite())
            || !nu.is_finite()
            || !de.is_finite()
            || !rn.is_finite()
            || !rd.is_finite()
            || rd < 0.0
            || de < 0.0
            || !angle.is_finite()
        {
            g.nonfinite += 1;
            continue;
        }
        let px = (nu - rn).abs() * f64::from(rd) * std::f64::consts::LN_2 / 2.0;
        let relative = if rd == 0.0 && de == 0.0 {
            0.0
        } else {
            (f64::from(de) - f64::from(rd)).abs() / f64::from(rd)
        };
        let angle = normal.wrapping_sub(rm).min(rm.wrapping_sub(normal));
        g.max_px = g.max_px.max(px);
        g.max_de_relative = g.max_de_relative.max(relative);
        g.max_normal_units = g.max_normal_units.max(angle);
        g.displacement_wrong += usize::from(px > 1e-3);
        // Match fd compare (GATE-02): DE/normal within 1e-3 px of the
        // boundary are ill-conditioned and reported separately. Escape-count
        // changes are diagnostic; displacement and class are the contract.
        let de_wrong = usize::from(relative > 0.002);
        let normal_wrong = usize::from(f64::from(angle) * 360.0 / 65536.0 > 0.2);
        if rd <= 1e-3 {
            g.near_de_wrong += de_wrong;
            g.near_normal_wrong += normal_wrong;
        } else {
            g.de_wrong += de_wrong;
            g.normal_wrong += normal_wrong;
        }
    }
    Ok(g)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fd_samples::{Class, Evidence};

    fn fixture(de: f32) -> (Samples, Vec<f64>, Vec<u32>) {
        let mut reference =
            Samples::alloc(1, ColumnSet::of(&[Column::Nu, Column::De, Column::Normal]));
        reference.class[0] = Class::new(Kind::Escaped, Evidence::Heuristic);
        reference.nu.as_mut().unwrap()[0] = 100.0;
        reference.de.as_mut().unwrap()[0] = de;
        reference.normal.as_mut().unwrap()[0] = 0;
        (
            reference,
            vec![3.0, 4.0, 10.0, 0.0, 100.0, f64::from(de), 0.0],
            vec![100, 0],
        )
    }

    #[test]
    fn gate_rejects_displacement_class_and_nonfinite_output() {
        let (r, mut t, mut w) = fixture(1.0);
        let expected = w.clone();
        t[4] += 1.0;
        assert_eq!(
            gate(&r, &[0], &t, &w, &expected)
                .unwrap()
                .displacement_wrong,
            1
        );
        t[4] = f64::NAN;
        assert_eq!(gate(&r, &[0], &t, &w, &expected).unwrap().nonfinite, 1);
        t[4] = 100.0;
        t[6] = f64::INFINITY;
        assert_eq!(gate(&r, &[0], &t, &w, &expected).unwrap().nonfinite, 1);
        w[1] = 2;
        assert_eq!(gate(&r, &[0], &t, &w, &expected).unwrap().class_wrong, 1);
    }

    #[test]
    fn gate_uses_fd_compare_secondary_thresholds_and_reports_boundary_errors() {
        for (de, near) in [(1.0, false), (1e-5, true)] {
            let (r, mut t, mut w) = fixture(de);
            let expected = w.clone();
            t[5] *= 1.01;
            t[6] = 0.01; // ~0.57 degree, above fd compare's 0.2-degree bar.
            w[0] += 1;
            let g = gate(&r, &[0], &t, &w, &expected).unwrap();
            assert_eq!(g.iteration_wrong, 1);
            assert_eq!(g.de_wrong, usize::from(!near));
            assert_eq!(g.normal_wrong, usize::from(!near));
            assert_eq!(g.near_de_wrong, usize::from(near));
            assert_eq!(g.near_normal_wrong, usize::from(near));
        }
    }

    #[test]
    fn gate_rejects_truncated_buffers_and_wraps_normal_angles() {
        let (r, mut t, w) = fixture(1.0);
        assert!(gate(&r, &[0], &t[..6], &w, &w).is_err());
        assert!(gate(&r, &[0], &t, &w[..1], &w).is_err());
        t[6] = -std::f64::consts::TAU / 65536.0;
        let g = gate(&r, &[0], &t, &w, &w).unwrap();
        assert_eq!(g.max_normal_units, 1);
        assert_eq!(g.normal_wrong, 0);
    }

    #[test]
    fn gate_accepts_zero_de_without_masking_negative_de() {
        let (r, mut t, w) = fixture(0.0);
        assert_eq!(gate(&r, &[0], &t, &w, &w).unwrap().nonfinite, 0);
        t[5] = -1.0;
        assert_eq!(gate(&r, &[0], &t, &w, &w).unwrap().nonfinite, 1);
    }
}

fn report(arm: &str, width: &str, rotation: f64, count: usize, g: Gate) -> Result<(), String> {
    let precision = if arm.ends_with("-mixed") {
        "guarded-f32"
    } else {
        "f64"
    };
    let encoding = if arm.starts_with("existing-ash") {
        "DEVICE_PACKED"
    } else if arm == "shared-cpu" {
        "HOST"
    } else {
        "DEVICE_PACKED"
    };
    let iteration_diagnostic = if arm != "shared-cpu" {
        "UNAVAILABLE"
    } else {
        "AVAILABLE"
    };
    println!(
        "finish_columns arm={arm} precision={precision} width={width} rotation={rotation} grid=16x9 inputs={count} class_wrong={} iteration_changed={} displacement_wrong={} de_wrong={} normal_wrong={} near_de_over={} near_normal_over={} nonfinite={} max_px={:e} max_de_relative={:e} max_normal_units={} encoding={encoding} iteration_diagnostic={iteration_diagnostic}",
        g.class_wrong,
        g.iteration_wrong,
        g.displacement_wrong,
        g.de_wrong,
        g.normal_wrong,
        g.near_de_wrong,
        g.near_normal_wrong,
        g.nonfinite,
        g.max_px,
        g.max_de_relative,
        g.max_normal_units
    );
    if g.class_wrong + g.displacement_wrong + g.de_wrong + g.normal_wrong + g.nonfinite != 0 {
        return Err(format!(
            "{arm} finish column gate failed at {width}, rotation {rotation}"
        ));
    }
    Ok(())
}

pub fn smoke() -> Result<(), String> {
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
    let client = super::f64_client()?;
    let wgsl = super::wgsl::Runner::new(&client.properties().identity.name, false)?;
    let wgsl_mixed = super::wgsl::Runner::new(&client.properties().identity.name, true)?;
    let ash = super::ash_finish::Runner::new(&client.properties().identity.name, false)?;
    let ash_mixed = super::ash_finish::Runner::new(&client.properties().identity.name, true)?;
    for (width, rotation) in [
        ("1e-37", 0.0),
        ("1e-40", 0.0),
        ("1e-43", 0.0),
        ("1e-46", 0.0),
        ("2e-48", 0.0),
        ("2e-49", 0.0),
        ("1e-40", 0.4),
    ] {
        let view = View {
            center_re: "-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502".into(),
            center_im: "0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922".into(),
            width: width.into(), rotation,
        };
        let (_, reference, states, c) = zone.probe_finish_inputs(&view, &p)?;
        let mut indices = Vec::new();
        let mut input = Vec::new();
        let mut iterations = Vec::new();
        let mut cpu_terminal = Vec::new();
        let mut cpu_words = Vec::new();
        for (index, state) in states.into_iter().enumerate() {
            let Some(state) = state else {
                continue;
            };
            let n = u32::try_from(state.iteration).map_err(|_| "checkpoint iteration overflow")?;
            indices.push(index);
            iterations.push(n);
            input.extend([
                state.z[0],
                state.z[1],
                state.derivative[0],
                state.derivative[1],
            ]);
            let (zr, zi, dr, di, n, escaped) = super::finish_loop::<f64>(
                state.z[0],
                state.z[1],
                state.derivative[0],
                state.derivative[1],
                c[0],
                c[1],
                n,
                20_000,
                1e20,
            );
            let (sn, cs) = rotation.sin_cos();
            let pixel = width.parse::<f64>().map_err(|e| e.to_string())? / f64::from(p.nx);
            let (nu, de, angle) = if escaped {
                super::encoding::encode(zr, zi, dr, di, n, pixel, cs, sn)
            } else {
                (0.0, 0.0, 0.0)
            };
            cpu_terminal.extend([zr, zi, dr, di, nu, de, angle]);
            cpu_words.extend([n, if escaped { 0 } else { 2 }]);
        }
        let count = indices.len();
        if count == 0 {
            return Err(format!("no finish inputs at {width}"));
        }
        let pixel = width.parse::<f64>().map_err(|e| e.to_string())? / f64::from(p.nx);
        report(
            "shared-cpu",
            width,
            rotation,
            count,
            gate(&reference, &indices, &cpu_terminal, &cpu_words, &cpu_words)?,
        )?;
        let (sn, cs) = rotation.sin_cos();
        let camera = [pixel, cs, sn, c[0], c[1], 1e20];
        for mixed in [false, true] {
            let (terminal, words) = shared(&client, &input, &iterations, &camera, mixed)?;
            report(
                if mixed {
                    "shared-wgpu-spirv-mixed"
                } else {
                    "shared-wgpu-spirv"
                },
                width,
                rotation,
                count,
                gate(&reference, &indices, &terminal, &words, &cpu_words)?,
            )?;
        }
        let (terminal, words) = wgsl.run(&input, &iterations, &camera)?;
        report(
            "hand-written-wgsl",
            width,
            rotation,
            count,
            gate(&reference, &indices, &terminal, &words, &cpu_words)?,
        )?;
        let (terminal, words) = wgsl_mixed.run(&input, &iterations, &camera)?;
        report(
            "hand-written-wgsl-mixed",
            width,
            rotation,
            count,
            gate(&reference, &indices, &terminal, &words, &cpu_words)?,
        )?;
        let (terminal, words) = ash.run(&input, &iterations, &camera)?;
        report(
            "existing-ash-glsl",
            width,
            rotation,
            count,
            gate(&reference, &indices, &terminal, &words, &cpu_words)?,
        )?;
        let (terminal, words) = ash_mixed.run(&input, &iterations, &camera)?;
        report(
            "existing-ash-glsl-mixed",
            width,
            rotation,
            count,
            gate(&reference, &indices, &terminal, &words, &cpu_words)?,
        )?;
    }
    println!(
        "tiny_grid_recurrence_columns=PASS gpu_output_encoding=PASS_PACKED_16_BYTES ash_arm=PASS guarded_f32=PASS whole_frame_1080p=UNMEASURED timing=UNMEASURED route_verdict=PENDING"
    );
    Ok(())
}

pub fn decoded(nu: &[u8], de: &[u8], packed: &[u8]) -> Result<(Vec<f64>, Vec<u32>), String> {
    if nu.len() % 8 != 0 || nu.len() / 8 * 4 != de.len() || de.len() != packed.len() {
        return Err("packed column length mismatch".into());
    }
    let count = nu.len() / 8;
    let mut terminal = Vec::with_capacity(count * 7);
    let mut words = Vec::with_capacity(count * 2);
    for i in 0..count {
        let n = f64::from_le_bytes(nu[i * 8..i * 8 + 8].try_into().unwrap());
        let d = f32::from_le_bytes(de[i * 4..i * 4 + 4].try_into().unwrap());
        let w = u32::from_le_bytes(packed[i * 4..i * 4 + 4].try_into().unwrap());
        terminal.extend([
            0.0,
            0.0,
            0.0,
            0.0,
            n,
            f64::from(d),
            f64::from(w & 65535) * std::f64::consts::TAU / 65536.0,
        ]);
        words.extend([u32::MAX, w >> 16]);
    }
    Ok((terminal, words))
}

fn shared_measured(
    client: &cubecl::client::Client,
    input: &[f64],
    iterations: &[u32],
    camera: &[f64],
    mixed: bool,
    timing: bool,
) -> Result<Measured, String> {
    let started = std::time::Instant::now();
    let count = iterations.len();
    let buffer = client.create_from_slice(f64::as_bytes(input));
    let n = client.create_from_slice(u32::as_bytes(iterations));
    let output = client.empty(count * 8);
    let de = client.empty(count * 4);
    let words = client.empty(count * 4);
    let camera_buffer = client.create_from_slice(f64::as_bytes(camera));
    let window = if timing {
        Some(
            client
                .profile_start()
                .map_err(|e| format!("CubeCL profile: {e:?}"))?,
        )
    } else {
        None
    };
    unsafe {
        super::finish_kernel::launch_unchecked(
            client,
            CubeCount::Static((count as u32).div_ceil(64), 1, 1),
            CubeDim::new_1d(64),
            BufferArg::from_raw_parts(buffer, count * 4),
            BufferArg::from_raw_parts(n, count),
            BufferArg::from_raw_parts(output.clone(), count),
            BufferArg::from_raw_parts(de.clone(), count),
            BufferArg::from_raw_parts(words.clone(), count),
            BufferArg::from_raw_parts(camera_buffer, 6),
            mixed,
        );
    }
    let duration = window
        .map(|w| client.profile_end(w))
        .transpose()
        .map_err(|e| format!("CubeCL profile: {e:?}"))?;
    let result = client
        .read(vec![output, de, words])
        .map_err(|e| format!("CubeCL batched read: {e:?}"))?;
    if result.len() != 3 {
        return Err("CubeCL read returned wrong column count".into());
    }
    let nu = &result[0];
    let de = &result[1];
    let words = &result[2];
    let frame_ms = started.elapsed().as_secs_f64() * 1000.0;
    let gpu_ms = if let Some(duration) = duration {
        if duration.timing_method().to_string() != "device" {
            None
        } else {
            cubecl::future::block_on(duration.resolve())
                .map(|ticks| ticks.duration().as_secs_f64() * 1000.0)
        }
    } else {
        None
    };
    let (terminal, words) = decoded(&nu, &de, &words)?;
    Ok(Measured {
        terminal,
        words,
        gpu_ms,
        frame_ms,
    })
}

fn shared(
    client: &cubecl::client::Client,
    input: &[f64],
    iterations: &[u32],
    camera: &[f64],
    mixed: bool,
) -> Result<(Vec<f64>, Vec<u32>), String> {
    shared_measured(client, input, iterations, camera, mixed, false).map(|r| (r.terminal, r.words))
}

pub struct Measured {
    pub terminal: Vec<f64>,
    pub words: Vec<u32>,
    pub gpu_ms: Option<f64>,
    pub frame_ms: f64,
}

pub fn benchmark(
    width: &str,
    nx: u32,
    ny: u32,
    reference: &Samples,
    indices: &[usize],
    input: &[f64],
    iterations: &[u32],
    camera: &[f64],
    only_arm: Option<&str>,
    only_precision: Option<&str>,
) -> Result<(), String> {
    if only_arm.is_some_and(|a| !["shared", "wgsl", "ash"].contains(&a))
        || only_precision.is_some_and(|p| !["f64", "mixed"].contains(&p))
    {
        return Err("arm: shared|wgsl|ash; precision: f64|mixed".into());
    }
    let client = super::f64_client()?;
    let expected_words = vec![u32::MAX; indices.len() * 2];
    for arm in ["shared", "wgsl", "ash"] {
        if only_arm.is_some_and(|a| a != arm) {
            continue;
        }
        for mixed in [false, true] {
            let precision = if mixed { "mixed" } else { "f64" };
            if only_precision.is_some_and(|p| p != precision) {
                continue;
            }
            let wgsl = if arm == "wgsl" {
                Some(super::wgsl::Runner::new(
                    &client.properties().identity.name,
                    mixed,
                )?)
            } else {
                None
            };
            let ash = if arm == "ash" {
                Some(super::ash_finish::Runner::new(
                    &client.properties().identity.name,
                    mixed,
                )?)
            } else {
                None
            };
            let run = |timing| match arm {
                "shared" => shared_measured(&client, input, iterations, camera, mixed, timing),
                "wgsl" => wgsl
                    .as_ref()
                    .unwrap()
                    .run_measured(input, iterations, camera, timing),
                _ => ash
                    .as_ref()
                    .unwrap()
                    .run_measured(input, iterations, camera, timing),
            };
            let warm = run(false)?;
            let mut verdict = gate(
                reference,
                indices,
                &warm.terminal,
                &warm.words,
                &expected_words,
            )?;
            drop(warm);
            let mut frames = Vec::new();
            let mut kernels = Vec::new();
            for timing in [false, true] {
                for _ in 0..3 {
                    let measured = run(timing)?;
                    let g = gate(
                        reference,
                        indices,
                        &measured.terminal,
                        &measured.words,
                        &expected_words,
                    )?;
                    // Preserve the worst run; never let later passing outputs mask failures.
                    verdict.class_wrong = verdict.class_wrong.max(g.class_wrong);
                    verdict.displacement_wrong =
                        verdict.displacement_wrong.max(g.displacement_wrong);
                    verdict.de_wrong = verdict.de_wrong.max(g.de_wrong);
                    verdict.normal_wrong = verdict.normal_wrong.max(g.normal_wrong);
                    verdict.nonfinite = verdict.nonfinite.max(g.nonfinite);
                    verdict.max_px = verdict.max_px.max(g.max_px);
                    verdict.near_de_wrong = verdict.near_de_wrong.max(g.near_de_wrong);
                    verdict.near_normal_wrong = verdict.near_normal_wrong.max(g.near_normal_wrong);
                    if timing {
                        kernels.push(measured.gpu_ms.ok_or(
                            "GPU timestamp measurement unavailable; no system-time substitution",
                        )?);
                    } else {
                        frames.push(measured.frame_ms);
                    }
                }
            }
            frames.sort_by(f64::total_cmp);
            kernels.sort_by(f64::total_cmp);
            if frames
                .iter()
                .chain(&kernels)
                .any(|v| !v.is_finite() || *v <= 0.0)
            {
                return Err("invalid timing sample".into());
            }
            let pass = verdict.class_wrong
                + verdict.displacement_wrong
                + verdict.de_wrong
                + verdict.normal_wrong
                + verdict.nonfinite
                == 0;
            println!(
                "frame_result arm={arm} precision={precision} width={width} grid={nx}x{ny} total_pixels={} finish_inputs={} cpu_bypassed={} parity={} class_wrong={} displacement_wrong={} de_wrong={} normal_wrong={} nonfinite={} max_px={:e} near_de_over={} near_normal_over={} dispatch_ms={:.6} finish_frame_ms={:.6} repeats=3 timing=GPU_TIMESTAMP_AND_SEPARATE_UNPROFILED_WALL output_bytes_per_input=16",
                nx as usize * ny as usize,
                indices.len(),
                nx as usize * ny as usize - indices.len(),
                if pass { "PASS" } else { "FAIL" },
                verdict.class_wrong,
                verdict.displacement_wrong,
                verdict.de_wrong,
                verdict.normal_wrong,
                verdict.nonfinite,
                verdict.max_px,
                verdict.near_de_wrong,
                verdict.near_normal_wrong,
                kernels[1],
                frames[1]
            );
        }
    }
    println!(
        "frame_probe=COMPLETE route_verdict=PENDING; parity failures are recorded results, never eligible timings"
    );
    Ok(())
}
