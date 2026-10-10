//! Scaled perturbation for deltas below f64 range (`pert-fx-scaled/1`).
//!
//! The delta is `delta = 2^e * w` and the derivative `dz/dc = 2^g * v`, with `w`, `v`
//! plain f64 near unit size and `e`, `g` integers. Rescaling multiplies by exact powers
//! of two, so it adds no rounding. Per step this costs one real-by-complex product more
//! than the f64 kernel. With `S = 2^e` and `dc = 2^ed * a`:
//! `w' = 2 Z w + S w^2 + a 2^(ed - e)`, `v' = 2 z v + 2^-g`.
//! A close cycle return that contracts ends the sample as `Unresolved`, not interior;
//! one that expands (a shadowed repelling cycle) keeps iterating.
use crate::interior::contracting;
use crate::reference::Reference;
use crate::sample::{resolvable, Outcome};
use fd_fixed::exp2i;

const HI: f64 = 4294967296.0; // 2^32
const LO: f64 = 1.0 / HI;

/// Iterate the sample at `C + 2^ed * (ar, ai)`.
pub(crate) fn scaled<const D: bool>(
    r: &Reference,
    ar: f64,
    ai: f64,
    ed: i64,
    max_iter: u64,
    r2: f64,
) -> Outcome {
    let (zr, zi, last) = (&r.re, &r.im, r.len() - 1);
    let (mut wr, mut wi, mut e) = (0.0f64, 0.0f64, ed);
    let mut s = exp2i(e); // 2^e as f64 (0 while below range)
    let (mut dr, mut di) = (ar, ai); // dc / 2^e
    let (mut vr, mut vi, mut g, mut inv) = (0.0f64, 0.0f64, 0i64, 1.0f64);
    let (mut m, mut n) = (0usize, 0u64);
    let (mut sr, mut si, mut chk, mut sn, mut returns) = (0.0f64, 0.0f64, 16u64, 0u64, 0u32);
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
            return Outcome::Escaped {
                n,
                zr: fr,
                zi: fi,
                dr: vr,
                di: vi,
                dexp: g,
                ez: f64::INFINITY,
                ed: f64::INFINITY,
            };
        }
        if f2 <= xr * xr + xi * xi || m == last {
            // Rebase onto Z_0 = 0: the delta becomes z itself. Near a deep nucleus z is
            // far below f64's range, so form it as Z_m + 2^e w without leaving the
            // scaled representation and keep dc at the new exponent (FIX-40).
            (wr, wi, e) = if f2 > SQUARE_OK {
                (fr, fi, 0)
            } else {
                sum2(&[(zr[m], zi[m], 0), (wr, wi, e)], e)
            };
            s = exp2i(e);
            m = 0;
            if s < TINY {
                // The step from Z_0 = 0 is z' = z^2 + dc: with z below f64's range the
                // fixed-exponent step below rounds both terms to 0, so take it here in
                // exponent form. A reference point that is exactly 0 rebases (f2 == x2).
                if D {
                    (vr, vi, g) = zero_step_dz((wr, wi, e), (vr, vi, g));
                    inv = exp2i(-g);
                }
                (wr, wi, e) = zero_step((wr, wi, e), (ar, ai, ed));
                s = exp2i(e);
                (m, n) = (1, n + 1);
            }
            let q = exp2i(ed - e);
            (dr, di) = (ar * q, ai * q);
        } else {
            let wm = wr.abs().max(wi.abs());
            let k = if wm > HI && e < 0 {
                32.min(-e)
            } else if wm < LO && wm != 0.0 {
                -32
            } else {
                0
            };
            if k != 0 {
                let f = exp2i(-k);
                (wr, wi, e) = (wr * f, wi * f, e + k);
                s = exp2i(e);
                let q = exp2i(ed - e);
                (dr, di) = (ar * q, ai * q);
            }
        }
        if returns < 4 && resolvable(xr, xi, f2) {
            let dist = (fr - sr).abs() + (fi - si).abs();
            if dist < 1e-13 * (sr.abs() + si.abs()) + 1e-300 {
                // An orbit shadowing a weakly repelling cycle (the M(24,2) 2-cycle at
                // the k = 7676 rung) returns as closely but expands over the return
                // (FIX-41). Only a contracting return ends the sample; it is still not
                // an attraction certificate, so the answer is Unresolved.
                let dc = exp2i(ed);
                if contracting(r, m, xr, xi, ar * dc, ai * dc, n - sn) {
                    return Outcome::Unresolved;
                }
                returns += 1;
            }
        }
        if n >= chk {
            (sr, si, sn, chk) = (fr, fi, n, chk * 2);
        }
    }
    Outcome::Unresolved
}

/// `sum t.0 2^t.2 + i t.1 2^t.2` as `(re, im, e)` with the larger part near unit size;
/// parts more than ~2^1000 below the largest are dropped. `(0, 0, zero_e)` if all vanish.
#[cold]
#[inline(never)]
fn sum2(terms: &[(f64, f64, i64)], zero_e: i64) -> (f64, f64, i64) {
    let lg = |t: &(f64, f64, i64)| {
        let a = t.0.abs().max(t.1.abs());
        (a != 0.0).then(|| a.log2().floor() as i64 + t.2)
    };
    let Some(top) = terms.iter().filter_map(lg).max() else {
        return (0.0, 0.0, zero_e);
    };
    // 2^k in two factors, so |k| up to 2046 stays finite.
    let sc = |x: f64, k: i64| {
        if k < -2046 {
            0.0
        } else {
            x * exp2i(k / 2) * exp2i(k - k / 2)
        }
    };
    let (re, im) = terms.iter().fold((0.0, 0.0), |(r, i), t| {
        (r + sc(t.0, t.2 - top), i + sc(t.1, t.2 - top))
    });
    (re, im, top)
}

/// `w' 2^e'` for the step from `Z_0 = 0`: `z' - Z_1 = z^2 + dc`, `z = 2^e w`, `dc = 2^ed a`.
#[cold]
#[inline(never)]
fn zero_step(w: (f64, f64, i64), a: (f64, f64, i64)) -> (f64, f64, i64) {
    let (wr, wi, e) = w;
    sum2(&[(wr * wr - wi * wi, 2.0 * wr * wi, 2 * e), a], e)
}

/// `v' 2^g'` for `dz/dc' = 2 z dz/dc + 1` with `z = 2^e w`, `dz/dc = 2^g v`.
#[cold]
#[inline(never)]
fn zero_step_dz(w: (f64, f64, i64), v: (f64, f64, i64)) -> (f64, f64, i64) {
    let q = (
        2.0 * (w.0 * v.0 - w.1 * v.1),
        2.0 * (w.0 * v.1 + w.1 * v.0),
        w.2 + v.2,
    );
    sum2(&[q, (1.0, 0.0, 0)], v.2)
}

/// A rebased `z` with `|z|^2` above this keeps exponent 0: its square and `dc` still
/// fit the fixed-exponent step. Smaller ones are formed exactly ([`sum2`]).
const SQUARE_OK: f64 = 1e-290;

/// Below this `2^e` is too small for the fixed-exponent step from `Z_0 = 0`.
const TINY: f64 = 1.0 / 1e270;

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
                    Outcome::Escaped {
                        n: n1,
                        zr: z1,
                        dr: d1,
                        di: e1,
                        dexp,
                        ..
                    },
                    Outcome::Escaped {
                        n: n2,
                        zr: z2,
                        dr: d2,
                        di: e2,
                        ..
                    },
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
            Outcome::Escaped { n, dexp, .. } => {
                assert!(n > 1500 && n < 4000 && dexp > 2900, "{n} {dexp}")
            }
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
            assert!(
                matches!(result, Outcome::Interior { .. } | Outcome::Unresolved),
                "{result:?}"
            );
        }
    }

    /// A 1e-1000 pixel next to the k = 7676 ladder nucleus escapes: the steps where the
    /// reference sits on 0 must keep `2^e w^2`, `dc` and `dz/dc` below f64 range (FIX-40).
    #[test]
    fn deep_nucleus_neighbour_escapes_at_the_oracle_count() {
        let (_, _, r) = rung_7676(140_000);
        // Pixel (5, 5) of a 96x54 frame 5000 minibrot sizes wide; mpmath at 1200
        // digits: escape (|z| > 1e10) at n = 133115, z = -2.7519423631080235e10 - ...,
        // log2 |dz/dc| = 3359.4391791173814.
        match scaled::<true>(
            &r,
            -0.5737130423454617,
            0.290231303774763,
            -3310,
            140_000,
            1e20,
        ) {
            Outcome::Escaped {
                n,
                zr,
                dr,
                di,
                dexp,
                ..
            } => {
                assert_eq!(n, 133_115);
                assert!((zr / -27519423631.080235 - 1.0).abs() < 1e-6, "{zr}");
                let l = dr.hypot(di).log2() + dexp as f64;
                assert!((l - 3359.4391791173814).abs() < 1e-6, "{l}");
            }
            o => panic!("{o:?}"),
        }
        // Pixel (48, 19), near the nucleus: after its eighth return |z| ~ 2^-557 is above
        // the old 1e-270 cut but its square is not, so a rebase to exponent 0 rounded
        // z^2 to 0 and the pixel never escaped. mpmath: escape at n = 139850.
        match scaled::<true>(
            &r,
            0.006749565204064256,
            0.10124347806096384,
            -3310,
            140_000,
            1e20,
        ) {
            Outcome::Escaped { n, .. } => assert_eq!(n, 139_850),
            o => panic!("{o:?}"),
        }
    }

    /// The k = 7676 ladder nucleus (~1e-1000 minibrot) as decimal strings and a
    /// reference of length `len`.
    fn rung_7676(len: u64) -> (String, String, Reference) {
        let rung = include_str!("../../../tools/research/misiurewicz/ladder_rungs.txt")
            .lines()
            .find(|line| line.starts_with("7676 "))
            .unwrap();
        let f: Vec<&str> = rung.split_whitespace().collect();
        let limbs = fd_fixed::limbs_for(3456);
        let cr = fd_fixed::Fixed::parse(f[2], limbs).unwrap();
        let ci = fd_fixed::Fixed::parse(f[3], limbs).unwrap();
        (
            f[2].to_string(),
            f[3].to_string(),
            Reference::from_fixed(&cr, &ci, len),
        )
    }

    /// PROB-19's k = 7676 frame 500 minibrot sizes wide (480x270), pixel (184, 201):
    /// the orbit shadows the weakly repelling M(24,2) 2-cycle (|rho| ~ 1.15) to within
    /// 1e-13 for many laps, which the close-return check took for a settled cycle and
    /// returned Unresolved (FIX-41). mpmath at 1200 digits: escape at n = 145079.
    #[test]
    fn repelling_cycle_shadow_at_the_7676_rung_escapes() {
        let (re, im, r) = rung_7676(150_000);
        let v = fd_samples::View {
            center_re: re,
            center_im: im,
            width: "5.050000000000E-998".into(),
            rotation: 0.0,
        };
        let pl = crate::Plane::new(&v, 480, 270).unwrap();
        let (ux, uy) = pl.unit_offset(184, 201);
        match scaled::<true>(&r, ux * pl.h_m, uy * pl.h_m, pl.h_e, 644_640, 1e20) {
            Outcome::Escaped { n, .. } => assert_eq!(n, 145_079),
            o => panic!("{o:?}"),
        }
    }
}
