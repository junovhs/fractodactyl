//! GPU-32: opt-in escaped-only frame route. Exact camera + reference are the
//! pre-loss checkpoint. Every rejected sample restarts on CPU at n=0 (DEC-25).
use super::{CE_ADAPTER, CF, STEPPER};
use fd_kernel::{EscapedState, Params, Plane, Reference, Stats, Tier};
use fd_samples::{Column, Header, Samples, View};
use fd_shade::gpu::testkit::{bytes, dispatch_batched, words, Binding, DispatchBatch};
use std::time::Instant;

const BLA: &str = include_str!("../../shaders/perturb_bla.wgsl");
const FRAME: &str = include_str!("../../shaders/perturb_frame.wgsl");
const GPU_STEPS: u32 = 10000;
// Measured-work ceiling per completed submission, not a speed/profitability claim.
const GPU_CHUNK_SAMPLES: u32 = 16384;

fn pack(x: (f64, f64), exponent: i64) -> [u32; 4] {
    let big = x.0.abs().max(x.1.abs());
    let k = if big == 0.0 {
        0
    } else {
        big.log2().floor() as i32 + 1
    };
    [
        ((x.0 * fd_fixed::exp2i(-i64::from(k))) as f32).to_bits(),
        ((x.1 * fd_fixed::exp2i(-i64::from(k))) as f32).to_bits(),
        (exponent + i64::from(k)) as u32,
        0,
    ]
}
fn unpack(w: &[u32], exponent: i64) -> (f64, f64) {
    let e = fd_fixed::exp2i(i64::from(w[2] as i32) + exponent);
    (
        f64::from(f32::from_bits(w[0])) * e,
        f64::from(f32::from_bits(w[1])) * e,
    )
}
pub(super) fn pack_ce(x: (f64, f64), exponent: i64) -> [u32; 8] {
    let w = pack(x, exponent);
    let e = i64::from(w[2] as i32) - exponent;
    let a = (x.0 * fd_fixed::exp2i(-e), x.1 * fd_fixed::exp2i(-e));
    [
        w[0],
        w[1],
        ((a.0 - f64::from(f32::from_bits(w[0]))) as f32).to_bits(),
        ((a.1 - f64::from(f32::from_bits(w[1]))) as f32).to_bits(),
        w[2],
        0,
        0,
        0,
    ]
}
pub(super) fn unpack_ce(w: &[u32], exponent: i64) -> (f64, f64) {
    let e = fd_fixed::exp2i(i64::from(w[4] as i32) + exponent);
    (
        (f64::from(f32::from_bits(w[0])) + f64::from(f32::from_bits(w[2]))) * e,
        (f64::from(f32::from_bits(w[1])) + f64::from(f32::from_bits(w[3]))) * e,
    )
}
fn l1(x: (f64, f64)) -> f64 {
    x.0.abs() + x.1.abs()
}

pub(crate) fn enabled(value: Option<&str>) -> Result<bool, String> {
    match value.unwrap_or("off") {
        "off" => Ok(false),
        "on" | "bla" => Ok(true),
        v => Err(format!("--gpu-perturb: expected on, bla or off, got {v:?}")),
    }
}

pub(crate) fn render(
    view: &View,
    p: &Params,
    orbit: Option<(Reference, u32)>,
) -> Result<(Header, Samples, Stats), String> {
    render_inner(view, p, orbit, 0.0, 0, None, false)
}

pub(crate) fn render_bla(
    view: &View,
    p: &Params,
    orbit: Option<(Reference, u32)>,
) -> Result<(Header, Samples, Stats), String> {
    render_inner(view, p, orbit, 0.0, 0, None, true)
}

fn pack_bla(table: &fd_kernel::GuardedBla) -> (Vec<u32>, Vec<u32>, usize) {
    let mut table_words = Vec::new();
    let mut level_offsets = Vec::new();
    let mut table_blocks = 0;
    // Lower dc_max: actual sample bound must fit, including upload uncertainty.
    table_words.extend(pack((table.dc_max * 0.99999, 0.0), 0));
    for level in &table.levels {
        level_offsets.push((table_words.len() / 4) as u32);
        for g in level {
            let b = g.block;
            let a = pack_ce(b.a, 0);
            let bb = pack_ce(b.b, 0);
            let ar = unpack_ce(&a, 0);
            let br = unpack_ce(&bb, 0);
            let aq = l1((ar.0 - b.a.0, ar.1 - b.a.1));
            let bq = l1((br.0 - b.b.0, br.1 - b.b.1));
            table_words.extend(a);
            table_words.extend(bb);
            table_words.extend(pack((b.r * 0.9999, 0.0), 0));
            for bound in [
                b.alpha + aq,
                b.beta + bq,
                g.derivative_alpha + aq,
                g.derivative_beta + bq,
            ] {
                table_words.extend(pack((bound * 1.0001, 0.0), 0));
            }
            table_blocks += 1;
        }
    }
    (table_words, level_offsets, table_blocks)
}

fn render_inner(
    view: &View,
    p: &Params,
    orbit: Option<(Reference, u32)>,
    injected_reference_error: f64,
    capability_test_mode: u32,
    admission: Option<&mut usize>,
    use_bla: bool,
) -> Result<(Header, Samples, Stats), String> {
    let started = Instant::now();
    let plane = Plane::new(view, p.nx, p.ny)?;
    // DEC-19: valid fixed scope before dispatch. Keep Bound evidence on CPU;
    // mantissa/exponent transport is never replaced by an absolute rounded centre.
    // The stripe column (STYL-09) is a CPU kernel column.
    if p.columns.has(Column::Bound)
        || p.columns.has(Column::Stripe)
        || plane.tier == Tier::Scaled
        || plane.h_e < -800
        || plane.h_e > 100
        || p.nx as u64 * p.ny as u64 > 65535 * 64
    {
        eprintln!(
            "fd gpu-perturb: frame declined (bound/stripe/scaled/range/grid); CPU restart from zero"
        );
        return fd_kernel::render_with(view, p, orbit);
    }
    let reference_started = Instant::now();
    let (r, bits) = match orbit {
        Some(r) => r,
        None if plane.tier == Tier::F64 => fd_kernel::reference(view, p)?,
        None => {
            let bits = fd_kernel::reference_bits(view, p)?;
            match Reference::from_decimal_full_escape(
                &view.center_re,
                &view.center_im,
                bits,
                p.max_iter,
                p.escape_radius,
            ) {
                Ok(r) => (r, bits),
                Err(why) => {
                    eprintln!("fd gpu-perturb: {why}; CPU restart from zero");
                    return fd_kernel::render_with(view, p, None);
                }
            }
        }
    };
    let reference_seconds = reference_started.elapsed().as_secs_f64();
    let q = r.error_radius(
        (bits != 53).then_some(u64::from(bits)),
        plane.c_re,
        plane.c_im,
    );
    let mut refs = Vec::new();
    let mut rq = Vec::new();
    for ((&re, &im), &error) in r.re.iter().zip(&r.im).zip(&q).take(GPU_STEPS as usize + 1) {
        let z = pack_ce((re + injected_reference_error, im), 0);
        let rounded = unpack_ce(&z, 0);
        let error = error + l1((re - rounded.0, im - rounded.1)) + injected_reference_error;
        if !error.is_finite() {
            break;
        }
        refs.extend(z);
        refs.extend(pack((error * 1.0001, 0.0), 0));
        rq.push(error);
    }
    if rq.len() < 2 {
        eprintln!("fd gpu-perturb: reference uncertainty declined; CPU restart from zero");
        return fd_kernel::render_with(view, p, Some((r, bits)));
    }
    let count = p.nx as usize * p.ny as usize;
    let mut offsets = Vec::with_capacity(count * 8);
    for j in 0..p.ny as usize {
        for i in 0..p.nx as usize {
            let u = plane.unit_offset(i, j);
            let exact = (u.0 * plane.h(), u.1 * plane.h());
            let dc = pack_ce(exact, 0);
            let rounded = unpack_ce(&dc, 0);
            let rc =
                l1((exact.0 - rounded.0, exact.1 - rounded.1)) + 16.0 * f64::EPSILON * l1(exact);
            offsets.extend(dc);
            offsets.extend(pack((rc * 1.0001, 0.0), 0));
        }
    }
    let table_started = Instant::now();
    let mut table_words = Vec::new();
    let mut level_offsets = Vec::new();
    let mut table_blocks = 0usize;
    if use_bla && p.escape_radius >= 4.0 {
        let dc_max = fd_kernel::bla_dc_max(view, p)? * 1.0001;
        let table = match fd_kernel::GuardedBla::build(&r, &q, 2f64.powi(-50), dc_max) {
            Ok(table) => table,
            Err(reason) => {
                return restart_after_attempt(
                    view,
                    p,
                    started,
                    &format!("BLA table declined: {reason}"),
                )
            }
        };
        (table_words, level_offsets, table_blocks) = pack_bla(&table);
    }
    if table_words.is_empty() {
        table_words.extend([0u32; 4]);
    }
    let table_seconds = table_started.elapsed().as_secs_f64();
    let radius = pack_ce((p.escape_radius, 0.0), 0);
    let mut args = vec![
        count as u32,
        rq.len() as u32,
        GPU_STEPS.min(p.max_iter.min(u64::from(u32::MAX)) as u32),
        capability_test_mode,
    ];
    args.extend(radius);
    args.push(0); // Exact original sample base, updated only between completed chunks.
    args.push(level_offsets.len() as u32);
    args.extend(level_offsets);
    let source = format!(
        "{}\n{CF}\n{CE_ADAPTER}\n{STEPPER}\n{BLA}\n{FRAME}",
        crate::zone_ff::FF
    );
    let gpu_started = Instant::now();
    let result = dispatch_batched(
        "fd generic perturbation",
        &source,
        "main",
        &[
            Binding::Read(&bytes(&refs)),
            Binding::Read(&bytes(&offsets)),
            Binding::Read(&bytes(&args)),
            Binding::Out((count * 128) as u64),
            Binding::Read(&bytes(&table_words)),
        ],
        ((count as u32).div_ceil(64), 1),
        DispatchBatch {
            groups_per_batch: GPU_CHUNK_SAMPLES / 64,
            invocations_per_group: 64,
            offset_binding: 2,
            offset_word: 12,
        },
    );
    let (adapter, output) = match result {
        Ok(r) => r,
        Err(e) => {
            if let Some(count) = admission {
                *count = 0;
            }
            return restart_after_attempt(view, p, started, &e);
        }
    };
    let gpu_seconds = gpu_started.elapsed().as_secs_f64();
    let output = words(&output);
    let entered = output
        .chunks_exact(32)
        .filter(|row| row[28] == 0x47505501)
        .count();
    let completed = output
        .chunks_exact(32)
        .enumerate()
        .filter(|(i, row)| row[28] == 0x47505502 && row[29] == *i as u32)
        .count();
    if output.len() != count * 32 || completed != count {
        if let Some(count) = admission {
            *count = 0;
        }
        return restart_after_attempt(view,p,started,&format!("incomplete GPU dispatch: completed={completed}/{count} entered_only={entered}; discard entire attempt"));
    }
    let mut candidates = vec![None; count];
    let mut guard = 0;
    let mut unfinished = 0;
    let mut invalid = 0;
    let mut iterations = 0u64;
    let mut la_blocks = 0u64;
    let mut la_skipped = 0u64;
    for (k, row) in output.chunks_exact(32).enumerate() {
        let (m, n, status) = (row[24] as usize, row[25], row[26]);
        iterations += u64::from(n);
        la_blocks += u64::from(row[30]);
        la_skipped += u64::from(row[31]);
        if status != 1 {
            if status >= 2 {
                invalid += 1;
            } else {
                unfinished += 1;
            }
            continue;
        }
        if m >= rq.len() {
            invalid += 1;
            continue;
        }
        let delta = unpack_ce(row, 0);
        let derivative = (
            f64::from(f32::from_bits(row[8])) + f64::from(f32::from_bits(row[10])),
            f64::from(f32::from_bits(row[9])) + f64::from(f32::from_bits(row[11])),
        );
        let dexp = i64::from(row[12] as i32);
        let z = (r.re[m] + delta.0, r.im[m] + delta.1);
        let ez = unpack(&row[16..], 0).0 + q[m] + 8.0 * f64::EPSILON * l1(z);
        let ed = unpack(&row[20..], -dexp).0 + 8.0 * f64::EPSILON * l1(derivative);
        let s = EscapedState {
            n: u64::from(n),
            z,
            derivative,
            derivative_exponent: dexp,
        };
        if z.0.hypot(z.1) - ez > p.escape_radius
            && fd_kernel::external_escape_ok(&s, ez, ed, (plane.h_m * f64::from(p.ss), plane.h_e))
        {
            candidates[k] = Some(s);
        } else {
            guard += 1;
        }
    }
    let admitted = candidates.iter().filter(|s| s.is_some()).count();
    if let Some(count) = admission {
        *count = admitted;
    }
    let mask: Vec<_> = candidates.iter().map(Option::is_none).collect();
    let fallback_started = Instant::now();
    let (mut h, mut s, mut st) = fd_kernel::render_with_masked(view, p, None, &mask)?;
    fd_kernel::overlay_escaped(view, p, &mut s, &candidates)?;
    st.reference_seconds = reference_seconds;
    st.iterations += iterations;
    let fallback_seconds = fallback_started.elapsed().as_secs_f64();
    h.kernel.push_str(if use_bla {
        " gpu-perturb-ce-bla/1"
    } else {
        " gpu-perturb-ce/1"
    });
    if use_bla {
        eprintln!("fd gpu-bla: blocks={la_blocks} skipped={la_skipped} table_blocks={table_blocks} table_bytes={} table_seconds={table_seconds:.6}",table_words.len()*4);
    }
    eprintln!("fd gpu-perturb: adapter={adapter:?} accepted={admitted} fallback={} guard={guard} unfinished={unfinished} invalid={invalid} gpu_seconds={gpu_seconds:.6} cpu_fallback_seconds={fallback_seconds:.6} total_seconds={:.6} restart=zero max_gpu_steps={GPU_STEPS} chunk_samples={GPU_CHUNK_SAMPLES} completed={completed}",count-admitted,started.elapsed().as_secs_f64());
    Ok((h, s, st))
}

fn restart_after_attempt(
    view: &View,
    p: &Params,
    started: Instant,
    reason: &str,
) -> Result<(Header, Samples, Stats), String> {
    let attempt = started.elapsed().as_secs_f64();
    let fallback = Instant::now();
    let result = fd_kernel::render_with(view, p, None);
    eprintln!("fd gpu-perturb: declined reason={reason:?} accepted=0 fallback={} gpu_attempt_seconds={attempt:.6} cpu_fallback_seconds={:.6} total_seconds={:.6} restart=zero",u64::from(p.nx)*u64::from(p.ny),fallback.elapsed().as_secs_f64(),started.elapsed().as_secs_f64());
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use fd_samples::{Column, ColumnSet};
    #[test]
    #[ignore = "actual GPU affine block transport/selection/derivative windows"]
    fn gpu_perturb_bla_windows() {
        for (cr, ci) in [(-0.743643887037158, 0.131825904205312), (1.0, 0.0)] {
            let r = Reference::new(cr, ci, 10000, 1e10);
            let c = 1e-24;
            let table =
                fd_kernel::GuardedBla::build(&r, &vec![0.0; r.len()], 2f64.powi(-50), c).unwrap();
            let (table_words, level_offsets, _) = pack_bla(&table);
            assert!(unpack(&table_words, 0).0 <= table.dc_max);
            for (level, offset) in table.levels.iter().zip(&level_offsets) {
                for (i, g) in level.iter().enumerate() {
                    let base = *offset as usize * 4 + i * 36;
                    assert!(
                        unpack(&table_words[base + 16..], 0).0 <= g.block.r,
                        "validity must round inward"
                    );
                    for (field, value) in [
                        (20, g.block.alpha),
                        (24, g.block.beta),
                        (28, g.derivative_alpha),
                        (32, g.derivative_beta),
                    ] {
                        assert!(
                            unpack(&table_words[base + field..], 0).0 >= value,
                            "error must round outward"
                        );
                    }
                }
            }
            let mut args = vec![0u32; 14];
            args[1] = r.len() as u32;
            args[2] = 10000;
            args[4..12].copy_from_slice(&pack_ce((4.0, 0.0), 0));
            args[13] = level_offsets.len() as u32;
            args.extend(level_offsets);
            let mut refwords = Vec::new();
            for (&re, &im) in r.re.iter().zip(&r.im) {
                refwords.extend(pack_ce((re, im), 0));
                refwords.extend([0u32; 4]);
            }
            let mut input = Vec::new();
            let mut cases = Vec::new();
            for m in [0usize, 1, 2, 3, 5, 9, 17, 33, 9997, 9999] {
                for mode in 0..4 {
                    let radius = if m > 0 {
                        table.levels[0].get((m - 1) / 2).map_or(0.0, |g| g.block.r)
                    } else {
                        0.0
                    };
                    let d = (if mode == 1 { 2.0 } else { 0.2 } * radius, 0.0);
                    let dc = (if mode == 2 { 2.0 } else { 0.3 } * c, -0.2 * c);
                    let derivative = (0.7, -0.3);
                    input.extend(pack_ce(d, 0));
                    input.extend(pack_ce(derivative, 0));
                    input.extend(pack_ce(dc, 0));
                    input.extend([m as u32, if mode == 3 { 9999 } else { 0 }, 0, 0]);
                    cases.push((m, mode, d, dc, derivative));
                }
            }
            args[0] = cases.len() as u32;
            let entry = r#"
@group(0) @binding(5) var<storage,read> window_inputs:array<vec4<u32>>;
@compute @workgroup_size(64)
fn window_main(@builtin(global_invocation_id) id:vec3<u32>){
 let i=id.x;if(i>=args[0]){return;}if(offset_words[0].x!=0u){return;}ff_init(args[3]);let k=7u*i;let metadata=window_inputs[k+6u];
 let delta=unpack_ce(window_inputs[k],window_inputs[k+1u]);let derivative=unpack_ce(window_inputs[k+2u],window_inputs[k+3u]);let dc=unpack_ce(window_inputs[k+4u],window_inputs[k+5u]);
 let out=bla_advance(PerturbState(delta,metadata.x,metadata.y,0u,0u),dc,cf_zero(),derivative,cf_zero(),cf_zero());let b=8u*i;
 result[b]=vec4<u32>(bitcast<u32>(out.s.delta.re.hi),bitcast<u32>(out.s.delta.im.hi),bitcast<u32>(out.s.delta.re.lo),bitcast<u32>(out.s.delta.im.lo));result[b+1u]=vec4<u32>(bitcast<u32>(out.s.delta.e),0u,0u,0u);
 result[b+2u]=vec4<u32>(bitcast<u32>(out.derivative.re.hi),bitcast<u32>(out.derivative.im.hi),bitcast<u32>(out.derivative.re.lo),bitcast<u32>(out.derivative.im.lo));result[b+3u]=vec4<u32>(bitcast<u32>(out.derivative.e),0u,0u,0u);
 result[b+4u]=packed_cf(out.e);result[b+5u]=packed_cf(out.ed);result[b+6u]=vec4<u32>(out.s.m,out.s.n,out.length,u32(pn_capability_ok()));result[b+7u]=vec4<u32>(out.s.status,0u,0u,0u);
}
"#;
            let source = format!(
                "{}\n{CF}\n{CE_ADAPTER}\n{STEPPER}\n{BLA}\n{FRAME}\n{entry}",
                crate::zone_ff::FF
            );
            let (_, out) = fd_shade::gpu::testkit::dispatch(
                "GPU33 independent block windows",
                &source,
                "window_main",
                &[
                    Binding::Read(&bytes(&refwords)),
                    Binding::Read(&bytes(&[0u32; 4])),
                    Binding::Read(&bytes(&args)),
                    Binding::Out((cases.len() * 128) as u64),
                    Binding::Read(&bytes(&table_words)),
                    Binding::Read(&bytes(&input)),
                ],
                (1, 1),
            )
            .unwrap();
            let words = words(&out);
            let mut used = 0;
            let mut skipped = 0;
            for ((m, mode, delta, dc, initial_d), row) in cases.iter().zip(words.chunks_exact(32)) {
                assert_eq!(row[27], 1, "runtime capability");
                let len = row[26] as usize;
                if *m == 0
                    || (*m - 1) % 2 != 0
                    || *mode == 1
                    || *mode == 2
                    || *mode == 3
                    || *m + 2 >= r.len()
                {
                    assert_eq!(len, 0, "m{m} mode{mode}");
                }
                if len == 0 {
                    continue;
                }
                used += 1;
                skipped += len;
                if cr == 1.0 && *m == 1 && *mode == 0 {
                    assert_eq!(row[28], 1, "block endpoint must escape immediately");
                    assert_eq!(len, 2);
                }
                assert_eq!((*m - 1) % len, 0);
                assert_eq!(row[24] as usize, *m + len);
                assert!(*m + len < r.len());
                let (mut d, mut derivative) = (*delta, *initial_d);
                for k in *m..*m + len {
                    let z = (r.re[k] + d.0, r.im[k] + d.1);
                    derivative = (
                        2.0 * (z.0 * derivative.0 - z.1 * derivative.1) + 1.0,
                        2.0 * (z.0 * derivative.1 + z.1 * derivative.0),
                    );
                    d = (
                        2.0 * (r.re[k] * d.0 - r.im[k] * d.1) + d.0 * d.0 - d.1 * d.1 + dc.0,
                        2.0 * (r.re[k] * d.1 + r.im[k] * d.0 + d.0 * d.1) + dc.1,
                    );
                }
                let got = unpack_ce(row, 0);
                let gd = unpack_ce(&row[8..], 0);
                let e = unpack(&row[16..], 0).0;
                let ed = unpack(&row[20..], 0).0;
                let serr = (got.0 - d.0).hypot(got.1 - d.1);
                let derr = (gd.0 - derivative.0).hypot(gd.1 - derivative.1);
                let round = 64.0 * f64::EPSILON * len as f64;
                if serr > e + round * (l1(d) + l1(got)) {
                    let level = len.trailing_zeros() as usize - 1;
                    let g = table.levels[level][(*m - 1) / len];
                    let a = g.block.a;
                    let b = g.block.b;
                    let nominal = (
                        a.0 * delta.0 - a.1 * delta.1 + b.0 * dc.0 - b.1 * dc.1,
                        a.0 * delta.1 + a.1 * delta.0 + b.0 * dc.1 + b.1 * dc.0,
                    );
                    eprintln!("GPU33_DIAG m{m} len{len} input={delta:?} dc={dc:?} got={got:?} direct={d:?} nominal={nominal:?} block={g:?}");
                }
                assert!(
                    serr <= e + round * (l1(d) + l1(got)),
                    "state m{m} len{len} {serr:e}>{e:e}"
                );
                assert!(
                    derr <= ed + round * (l1(derivative) + l1(gd)),
                    "derivative m{m} len{len} {derr:e}>{ed:e}"
                );
            }
            assert!(used >= if cr == 1.0 { 1 } else { 4 }, "{used}");
            println!(
                "GPU33_WINDOWS reference={cr} cases={} used={used} skipped={skipped}",
                cases.len()
            );
        }
    }
    #[test]
    #[ignore = "actual GPU corruption/uncertainty rejection and complete CPU restart"]
    fn gpu_perturb_guard_restarts_corrupted_reference() {
        let view = View {
            center_re: "1".into(),
            center_im: "0.5".into(),
            width: "0.25".into(),
            rotation: 0.0,
        };
        let p = Params {
            nx: 8,
            ny: 8,
            ss: 1,
            max_iter: 256,
            escape_radius: 1e10,
            columns: ColumnSet::of(&[Column::Nu, Column::De, Column::Normal]),
            threads: 1,
            tier: None,
            stripe: Default::default(),
        };
        let (expected_h, expected) = fd_kernel::render(&view, &p).unwrap();
        for use_bla in [false, true] {
            let mut accepted = usize::MAX;
            // Corrupt GPU reference values by +100 while carrying their full transport
            // radius. No lossy state may enter the final frame (DEC-25).
            let (mut h, s, _) =
                render_inner(&view, &p, None, 100.0, 0, Some(&mut accepted), use_bla).unwrap();
            assert_eq!(
                accepted, 0,
                "must dispatch and reject every corrupted candidate"
            );
            h.kernel = expected_h.kernel.clone();
            let (mut a, mut b) = (Vec::new(), Vec::new());
            fd_samples::write(&mut a, &h, &s).unwrap();
            fd_samples::write(&mut b, &expected_h, &expected).unwrap();
            assert_eq!(
                a, b,
                "complete samples must come from exact-camera zero restart"
            );
            let (mut h, s, _) =
                render_inner(&view, &p, None, 0.0, 1, Some(&mut accepted), use_bla).unwrap();
            assert_eq!(
                accepted, 0,
                "runtime capability decline must admit no pixel"
            );
            h.kernel = expected_h.kernel.clone();
            let mut declined = Vec::new();
            fd_samples::write(&mut declined, &h, &s).unwrap();
            assert_eq!(
                a, declined,
                "failed capability must restart exact original CPU route"
            );
            let (mut h, s, _) =
                render_inner(&view, &p, None, 0.0, 2, Some(&mut accepted), use_bla).unwrap();
            assert_eq!(
                accepted, 0,
                "incomplete GPU writes must invalidate the attempt"
            );
            h.kernel = expected_h.kernel.clone();
            let mut incomplete = Vec::new();
            fd_samples::write(&mut incomplete, &h, &s).unwrap();
            assert_eq!(
                a, incomplete,
                "incomplete GPU output must restart original CPU route"
            );
        }
    }
}
