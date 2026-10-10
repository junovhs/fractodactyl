//! Scaled perturbation for deltas below f64 range (`pert-fx-scaled/1`).
//!
//! The delta is `delta = 2^e * w` and the derivative `dz/dc = 2^g * v`, with `w`, `v`
//! plain f64 near unit size and `e`, `g` integers. Rescaling multiplies by exact powers
//! of two, so it adds no rounding. Per step this costs one real-by-complex product more
//! than the f64 kernel. With `S = 2^e` and `dc = 2^ed * a`:
//! `w' = 2 Z w + S w^2 + a 2^(ed - e)`, `v' = 2 z v + 2^-g`.
//! A close cycle return without an attraction check is `Unresolved`, not interior.
use crate::reference::Reference;
use crate::sample::{resolvable, Outcome};
use fd_fixed::exp2i;

const HI: f64 = 4294967296.0; // 2^32
const LO: f64 = 1.0 / HI;

/// Iterate the sample at `C + 2^ed * (ar, ai)`.
pub(crate) fn scaled<const D: bool>(r: &Reference, ar: f64, ai: f64, ed: i64, max_iter: u64, r2: f64) -> Outcome {
    let (zr, zi, last) = (&r.re, &r.im, r.len() - 1);
    let (mut wr, mut wi, mut e) = (0.0f64, 0.0f64, ed);
    let mut s = exp2i(e); // 2^e as f64 (0 while below range)
    let (mut dr, mut di) = (ar, ai); // dc / 2^e
    let (mut vr, mut vi, mut g, mut inv) = (0.0f64, 0.0f64, 0i64, 1.0f64);
    let (mut m, mut n) = (0usize, 0u64);
    let (mut sr, mut si, mut chk) = (0.0f64, 0.0f64, 16u64);
    while n < max_iter {
        if D {
            let (fr, fi) = (zr[m] + s * wr, zi[m] + s * wi);
            let t = 2.0 * (fr * vr - fi * vi) + inv;
            vi = 2.0 * (fr * vi + fi * vr);
            vr = t;
            let vm = vr.abs().max(vi.abs());
            if vm > HI {
                (vr, vi, g) = (vr * LO, vi * LO, g + 32);
                inv = exp2i(-g);
            }
        }
        let t = 2.0 * (zr[m] * wr - zi[m] * wi) + s * (wr * wr - wi * wi) + dr;
        wi = 2.0 * (zr[m] * wi + zi[m] * wr) + s * (2.0 * wr * wi) + di;
        wr = t;
        m += 1;
        n += 1;
        let (xr, xi) = (s * wr, s * wi); // delta as f64, 0 while below range
        let (fr, fi) = (zr[m] + xr, zi[m] + xi);
        let f2 = fr * fr + fi * fi;
        if f2 > r2 {
            return Outcome::Escaped { n, zr: fr, zi: fi, dr: vr, di: vi, dexp: g, ez: f64::INFINITY, ed: f64::INFINITY };
        }
        if f2 < xr * xr + xi * xi || m == last {
            // Rebase onto Z_0 = 0: the delta becomes z itself, order one.
            (wr, wi, e, s, m) = (fr, fi, 0, 1.0, 0);
            (dr, di) = (ar * exp2i(ed), ai * exp2i(ed));
        } else {
            let wm = wr.abs().max(wi.abs());
            let k = if wm > HI && e < 0 { 32.min(-e) } else if wm < LO && wm != 0.0 { -32 } else { 0 };
            if k != 0 {
                let f = exp2i(-k);
                (wr, wi, e) = (wr * f, wi * f, e + k);
                s = exp2i(e);
                let q = exp2i(ed - e);
                (dr, di) = (ar * q, ai * q);
            }
        }
        if resolvable(xr, xi, f2) {
            let dist = (fr - sr).abs() + (fi - si).abs();
            if dist < 1e-13 * (sr.abs() + si.abs()) + 1e-300 {
                // Repelling cycles also return closely; attraction is not established.
                return Outcome::Unresolved;
            }
        }
        if n == chk {
            (sr, si, chk) = (fr, fi, chk * 2);
        }
    }
    Outcome::Unresolved
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample::sample;

    /// The scaled kernel at an f64-representable offset must agree with the f64 kernel.
    #[test]
    fn agrees_with_f64_kernel_where_both_apply() {
        let r = Reference::new(-0.7487, 0.0789, 10_000, 1e10); // escapes after 52 steps
        for (ar, ai) in [(1e-5, 2e-5), (-3e-5, 1e-5), (5e-5, -5e-5)] {
            // Represent the same offset as 2^-20 * (ar * 2^20, ai * 2^20).
            let k = 1048576.0;
            let a = scaled::<true>(&r, ar * k, ai * k, -20, 10_000, 1e20);
            let b = sample::<true, false>(&r, &[], None, ar, ai, 10_000, 1e20);
            match (a, b) {
                (
                    Outcome::Escaped { n: n1, zr: z1, dr: d1, di: e1, dexp, .. },
                    Outcome::Escaped { n: n2, zr: z2, dr: d2, di: e2, .. },
                ) => {
                    assert_eq!(n1, n2);
                    assert!((z1 - z2).abs() < 1e-6 * z2.abs());
                    let m1 = (d1 * d1 + e1 * e1).sqrt() * exp2i(dexp);
                    let m2 = (d2 * d2 + e2 * e2).sqrt();
                    assert!((m1 - m2).abs() < 1e-6 * m2, "{m1} {m2}");
                }
                o => panic!("{o:?}"),
            }
        }
    }

    /// Below f64 range the delta still grows and the sample escapes.
    #[test]
    fn escapes_from_far_below_f64_range() {
        // c = i is Misiurewicz: its 2-cycle multiplier is 4*sqrt(2), so a delta grows by
        // ~1.25 bits per step and a 2^-3000 offset escapes after roughly 2400 steps.
        let r = Reference::new(0.0, 1.0, 100_000, 1e10);
        match scaled::<true>(&r, 1.0, 0.5, -3000, 100_000, 1e20) {
            Outcome::Escaped { n, dexp, .. } => assert!(n > 1500 && n < 4000 && dexp > 2900, "{n} {dexp}"),
            o => panic!("{o:?}"),
        }
    }

    /// At sub-f64 widths, a near-repelling or near-parabolic exterior is not interior.
    #[test]
    fn deep_repelling_and_near_parabolic_exteriors() {
        // Direct mpmath at 1200 bits: c + 2^-1100 * (1 + 0.5i).
        for (cr, ci, oracle_n) in [(0.0, 1.0, 887), (0.250001, 0.0, 3145), (-0.75, 0.01, 320)] {
            let r = Reference::new(cr, ci, 20_000, 65536.0);
            match scaled::<true>(&r, 1.0, 0.5, -1100, 20_000, 1e20) {
                Outcome::Escaped { n, .. } => assert_eq!(n, oracle_n, "{cr}, {ci}"),
                Outcome::Unresolved => {} // no attraction certificate
                o => panic!("{cr}, {ci}: {o:?}"),
            }
        }
    }

    /// Samples inside the v0 ladder's 1e-100 minibrot may remain unresolved.
    #[test]
    fn deep_ladder_interior_stays_non_escaping() {
        // k = 409 nucleus from tools/research/misiurewicz/ladder_rungs.txt.
        let limbs = fd_fixed::limbs_for(1280);
        let cr = fd_fixed::Fixed::parse(
            "-0.743291890852430202931624325972510757176348558120774356370590210260840951728744990155474936011582556204309239952297545796655290288288531259645658145590048845370914634545066454",
            limbs,
        )
        .unwrap();
        let ci = fd_fixed::Fixed::parse(
            "0.131240552308797604770845906581478143077114151158591754835109820677855616894736385331957444256069258907400140765662344877761310383645126000894706155294661737082962412611433299571783",
            limbs,
        )
        .unwrap();
        let r = Reference::from_fixed(&cr, &ci, 6000);
        for (ar, ai) in [(1.0, 0.0), (0.0, 1.0)] {
            let result = scaled::<true>(&r, ar, ai, -1100, 6000, 1e20);
            assert!(matches!(result, Outcome::Interior { .. } | Outcome::Unresolved), "{result:?}");
        }
    }
}
