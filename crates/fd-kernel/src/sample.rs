//! One sample: perturbed iteration with rebasing (Zhuoran), Brent periodicity check
//! confirmed by Newton, and the derivative only when a column needs it. With `B`, also a
//! rigorous running error radius on `z` and `dz/dc` (Bounded evidence, docs/spec/LOD.md).
use crate::interior::{attracting, in_main_components};
use crate::reference::Reference;

/// Result of one sample, before quantisation into columns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Outcome {
    /// `z` and `dz/dc = (dr, di) * 2^dexp` at the first iterate outside the escape
    /// radius (`n` iterates), with error radii `ez >= |z - z_exact|` and
    /// `ed >= |(dr, di) - dz/dc_exact * 2^-dexp|` (infinite when not tracked).
    Escaped { n: u64, zr: f64, zi: f64, dr: f64, di: f64, dexp: i64, ez: f64, ed: f64 },
    /// Judged bounded after `n` iterates (0 for the closed-form main-component test).
    Interior { n: u64 },
    Unresolved,
}

impl Outcome {
    /// Perturbation iterates this sample spent (`max_iter` when unresolved).
    #[inline]
    pub(crate) fn iterations(self, max_iter: u64) -> u64 {
        match self {
            Outcome::Escaped { n, .. } | Outcome::Interior { n } => n,
            Outcome::Unresolved => max_iter,
        }
    }
}

/// Unit roundoff of f64.
pub(crate) const U: f64 = f64::EPSILON / 2.0;
/// Covers the rounding of the error-radius arithmetic itself.
const SAFE: f64 = 1.0 + 8.0 * U;

/// Iterate sample `C + (ar, ai)`. `D` selects whether `dz/dc` is tracked. `c` is the
/// absolute sample position when f64 resolves it, enabling the closed-form interior test.
/// `B` (needs `D`) tracks error radii against the exact orbit, using `q` from
/// [`Reference::error_radius`]; `q` is unused otherwise.
#[inline]
#[allow(clippy::too_many_arguments)]
pub(crate) fn sample<const D: bool, const B: bool>(
    r: &Reference,
    q: &[f64],
    c: Option<(f64, f64)>,
    ar: f64,
    ai: f64,
    max_iter: u64,
    r2: f64,
) -> Outcome {
    if c.is_some_and(|c| in_main_components(c.0, c.1)) {
        return Outcome::Interior { n: 0 };
    }
    let (zr, zi, last) = (&r.re, &r.im, r.len() - 1);
    let (mut xr, mut xi) = (0.0f64, 0.0f64); // delta z
    let (mut dr, mut di) = (0.0f64, 0.0f64); // dz/dc
    let (mut m, mut n) = (0usize, 0u64);
    // Brent: compare against the point saved at the last power of two.
    let (mut sr, mut si, mut chk) = (0.0f64, 0.0f64, 16u64);
    let (mut newton_at, mut tries) = (64u64, 0);
    // Error radii (B): e >= |z_exact - (A_m + delta)| with A the exact reference orbit,
    // ed on dz/dc. rc: the f64 offset's distance from the exact sample position.
    let (mut e, mut ed) = (0.0f64, 0.0f64);
    let rc = 16.0 * U * (ar.abs() + ai.abs()) + 1e-300;
    while n < max_iter {
        let (fr, fi) = (zr[m] + xr, zi[m] + xi);
        if B {
            let af = (fr * fr + fi * fi).sqrt() * (1.0 + 4.0 * U) + 1e-150; // >= |f|, f = z as computed
            let ef = e + q[m] + 2.0 * U * af; // >= |z_exact - f|
            let ad = dr.abs() + di.abs();
            ed = (2.0 * (af + ef) * ed + 2.0 * ef * ad + 8.0 * U * (2.0 * af * ad + 1.0)) * SAFE;
            let (aw, ax) = (zr[m].abs() + zi[m].abs(), xr.abs() + xi.abs());
            // Defect of the delta step against 2 A delta + delta^2 + dc_exact.
            let rho = 8.0 * U * (2.0 * aw * ax + ax * ax + ar.abs() + ai.abs()) + 2.0 * q[m] * ax + rc;
            let w = af + q[m] + 2.0 * U * af; // >= |A_m + delta|
            e = (e * (2.0 * w + e) + rho) * SAFE;
        }
        if D {
            let t = 2.0 * (fr * dr - fi * di) + 1.0;
            di = 2.0 * (fr * di + fi * dr);
            dr = t;
        }
        // delta' = 2 Z delta + delta^2 + dc
        let t = 2.0 * (zr[m] * xr - zi[m] * xi) + xr * xr - xi * xi + ar;
        xi = 2.0 * (zr[m] * xi + zi[m] * xr + xr * xi) + ai;
        xr = t;
        m += 1;
        n += 1;
        let (fr, fi) = (zr[m] + xr, zi[m] + xi);
        let f2 = fr * fr + fi * fi;
        if f2 > r2 {
            let (ez, ed) = if B { (e + q[m] + 4.0 * U * f2.sqrt(), ed) } else { (f64::INFINITY, f64::INFINITY) };
            return Outcome::Escaped { n, zr: fr, zi: fi, dr, di, dexp: 0, ez, ed };
        }
        if f2 < xr * xr + xi * xi || m == last {
            if B {
                e += q[m] + 4.0 * U * f2.sqrt() + 1e-300; // delta := f, rounded, against A_m
            }
            (xr, xi, m) = (fr, fi, 0); // rebase onto Z_0 = 0
        }
        // Until the delta is resolvable next to Z, z == Z in f64 and a periodic
        // reference would look like a settled cycle: judge nothing yet.
        if !resolvable(xr, xi, f2) {
            if n == chk {
                (sr, si, chk) = (fr, fi, chk * 2);
            }
            continue;
        }
        let dist = (fr - sr).abs() + (fi - si).abs();
        let size = sr.abs() + si.abs();
        if dist < 1e-13 * size + 1e-300 {
            return Outcome::Interior { n }; // exact return: settled on a cycle
        }
        if tries < 4 && n >= newton_at && dist < 1e-3 * size {
            if attracting(r, m, xr, xi, ar, ai, n - chk / 2) {
                return Outcome::Interior { n };
            }
            tries += 1;
            newton_at = 4 * n;
        }
        if n == chk {
            (sr, si, chk) = (fr, fi, chk * 2);
        }
    }
    Outcome::Unresolved
}

/// Delta large enough relative to `|z|^2 = f2` that `z = Z + delta` carries it.
#[inline]
pub(crate) fn resolvable(xr: f64, xi: f64, f2: f64) -> bool {
    xr * xr + xi * xi > 1e-24 * f2
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(cr: f64, ci: f64, max_iter: u64) -> Outcome {
        // Reference at a nearby point exercises the perturbation path.
        let r = Reference::new(cr - 1e-3, ci + 1e-3, max_iter, 1e10);
        sample::<true, false>(&r, &[], Some((cr, ci)), 1e-3, -1e-3, max_iter, 1e20)
    }

    fn direct(cr: f64, ci: f64, max_iter: u64) -> Option<u64> {
        let (mut x, mut y) = (0.0f64, 0.0f64);
        for n in 1..=max_iter {
            (x, y) = (x * x - y * y + cr, 2.0 * x * y + ci);
            if x * x + y * y > 1e20 {
                return Some(n);
            }
        }
        None
    }

    #[test]
    fn escape_count_matches_direct_iteration() {
        for (cr, ci) in [(0.4, 0.3), (-0.75, 0.1), (-1.8, 0.01), (0.2501, 0.0), (-0.1, 0.9), (-0.7487, 0.0789)] {
            match run(cr, ci, 100_000) {
                Outcome::Escaped { n, .. } => assert_eq!(Some(n), direct(cr, ci, 100_000), "{cr},{ci}"),
                o => panic!("{cr},{ci}: {o:?}"),
            }
        }
    }

    #[test]
    fn interior_outside_main_components_is_found() {
        // Period-3 bulb centre region (real axis) and a period-4 bulb.
        for (cr, ci) in [(-1.7548, 0.0), (-0.1565, 1.0322), (-1.3107, 0.0)] {
            assert!(matches!(run(cr, ci, 1_000_000), Outcome::Interior { .. }), "{cr},{ci}");
        }
    }

    #[test]
    fn derivative_is_skipped_without_changing_escape() {
        let r = Reference::new(0.3, 0.6, 1000, 1e10);
        let a = sample::<true, false>(&r, &[], Some((0.31, 0.6)), 0.01, 0.0, 1000, 1e20);
        let b = sample::<false, false>(&r, &[], Some((0.31, 0.6)), 0.01, 0.0, 1000, 1e20);
        match (a, b) {
            (Outcome::Escaped { n: x, .. }, Outcome::Escaped { n: y, .. }) => assert_eq!(x, y),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn error_radius_covers_the_exact_orbit() {
        // Dyadic reference + offset, so the sample point is exact; compare against a
        // 256-bit fixed-point orbit at that point (f64 and fixed references).
        let l = fd_fixed::limbs_for(256);
        let fx = |s: &str| fd_fixed::Fixed::parse(s, l).unwrap();
        let cases = [
            ((0.375, 0.28125), (0.03125, 0.015625), ("0.40625", "0.296875")),
            ((-0.7421875, 0.0546875), (-0.0078125, 0.0078125), ("-0.75", "0.0625")),
            ((0.25, 0.0), (0.03125, 0.0078125), ("0.28125", "0.0078125")),
        ];
        let r2 = 4294967296.0f64;
        for ((c0r, c0i), (ar, ai), (er, ei)) in cases {
            let f64_ref = Reference::new(c0r, c0i, 10_000, r2.sqrt());
            let fx_ref = Reference::from_fixed(&fx(&c0r.to_string()), &fx(&c0i.to_string()), 10_000);
            let mut exact = vec![(0.0, 0.0)];
            fd_fixed::orbit(&fx(er), &fx(ei), 10_000, r2, |a, b| exact.push((a, b)));
            for (r, bits) in [(&f64_ref, None), (&fx_ref, Some(256))] {
                let q = r.error_radius(bits, c0r, c0i);
                let Outcome::Escaped { n, zr, zi, ed, ez, .. } = sample::<true, true>(r, &q, Some((c0r + ar, c0i + ai)), ar, ai, 10_000, r2)
                else {
                    panic!("{er},{ei} did not escape")
                };
                let (xr, xi) = exact[n as usize];
                let (d, az) = ((zr - xr).hypot(zi - xi), zr.hypot(zi));
                assert!(d <= ez + 4.0 * U * az, "{er},{ei} {bits:?}: |dz| {d} > ez {ez}");
                assert!(ez < 1e-9 * az && ed.is_finite(), "{er},{ei} {bits:?}: ez {ez} ed {ed} useless");
            }
        }
    }
}
