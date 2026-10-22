//! GPU-32: opt-in escaped-only frame route. Exact camera + reference are the
//! pre-loss checkpoint. Every rejected sample restarts on CPU at n=0 (DEC-25).
use super::{CE_ADAPTER, CF, STEPPER};
use fd_kernel::{EscapedState, Params, Plane, Reference, Stats, Tier};
use fd_samples::{Column, Header, Samples, View};
use fd_shade::gpu::testkit::{bytes, dispatch_batched, words, Binding, DispatchBatch};
use std::time::Instant;

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
        "on" => Ok(true),
        v => Err(format!("--gpu-perturb: expected on or off, got {v:?}")),
    }
}

pub(crate) fn render(
    view: &View,
    p: &Params,
    orbit: Option<(Reference, u32)>,
) -> Result<(Header, Samples, Stats), String> {
    render_inner(view, p, orbit, 0.0, 0, None)
}

fn render_inner(
    view: &View,
    p: &Params,
    orbit: Option<(Reference, u32)>,
    injected_reference_error: f64,
    capability_test_mode: u32,
    admission: Option<&mut usize>,
) -> Result<(Header, Samples, Stats), String> {
    let started = Instant::now();
    let plane = Plane::new(view, p.nx, p.ny)?;
    // DEC-19: valid fixed scope before dispatch. Keep Bound evidence on CPU;
    // mantissa/exponent transport is never replaced by an absolute rounded centre.
    if p.columns.has(Column::Bound)
        || plane.tier == Tier::Scaled
        || plane.h_e < -800
        || plane.h_e > 100
        || p.nx as u64 * p.ny as u64 > 65535 * 64
    {
        eprintln!(
            "fd gpu-perturb: frame declined (bound/scaled/range/grid); CPU restart from zero"
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
    let radius = pack_ce((p.escape_radius, 0.0), 0);
    let mut args = vec![
        count as u32,
        rq.len() as u32,
        GPU_STEPS.min(p.max_iter.min(u64::from(u32::MAX)) as u32),
        capability_test_mode,
    ];
    args.extend(radius);
    args.push(0); // Exact original sample base, updated only between completed chunks.
    let source = format!(
        "{}\n{CF}\n{CE_ADAPTER}\n{STEPPER}\n{FRAME}",
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
    for (k, row) in output.chunks_exact(32).enumerate() {
        let (m, n, status) = (row[24] as usize, row[25], row[26]);
        iterations += u64::from(n);
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
    h.kernel.push_str(" gpu-perturb-ce/1");
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
        };
        let (expected_h, expected) = fd_kernel::render(&view, &p).unwrap();
        let mut accepted = usize::MAX;
        // Corrupt GPU reference values by +100 while carrying their full transport
        // radius. No lossy state may enter the final frame (DEC-25).
        let (mut h, s, _) = render_inner(&view, &p, None, 100.0, 0, Some(&mut accepted)).unwrap();
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
        let (mut h, s, _) = render_inner(&view, &p, None, 0.0, 1, Some(&mut accepted)).unwrap();
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
        let (mut h, s, _) = render_inner(&view, &p, None, 0.0, 2, Some(&mut accepted)).unwrap();
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
