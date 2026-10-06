//! One sample: perturbed iteration with rebasing (Zhuoran), Brent periodicity check
//! confirmed by Newton, and the derivative only when a column needs it.
use crate::interior::{attracting, in_main_components};
use crate::reference::Reference;

/// Result of one sample, before quantisation into columns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Outcome {
    /// `z` and `dz/dc` at the first iterate outside the escape radius (`n` iterates).
    Escaped { n: u64, zr: f64, zi: f64, dr: f64, di: f64 },
    Interior,
    Unresolved,
}

/// Iterate sample `c = C + (ar, ai)`. `D` selects whether `dz/dc` is tracked.
#[inline]
pub(crate) fn sample<const D: bool>(r: &Reference, c: (f64, f64), ar: f64, ai: f64, max_iter: u64, r2: f64) -> Outcome {
    if in_main_components(c.0, c.1) {
        return Outcome::Interior;
    }
    let (zr, zi, last) = (&r.re, &r.im, r.len() - 1);
    let (mut xr, mut xi) = (0.0f64, 0.0f64); // delta z
    let (mut dr, mut di) = (0.0f64, 0.0f64); // dz/dc
    let (mut m, mut n) = (0usize, 0u64);
    // Brent: compare against the point saved at the last power of two.
    let (mut sr, mut si, mut chk) = (0.0f64, 0.0f64, 16u64);
    let (mut newton_at, mut tries) = (64u64, 0);
    while n < max_iter {
        if D {
            let (fr, fi) = (zr[m] + xr, zi[m] + xi);
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
            return Outcome::Escaped { n, zr: fr, zi: fi, dr, di };
        }
        if f2 < xr * xr + xi * xi || m == last {
            (xr, xi, m) = (fr, fi, 0); // rebase onto Z_0 = 0
        }
        let dist = (fr - sr).abs() + (fi - si).abs();
        let size = sr.abs() + si.abs();
        if dist < 1e-13 * size + 1e-300 {
            return Outcome::Interior; // exact return: settled on a cycle
        }
        if tries < 4 && n >= newton_at && dist < 1e-3 * size {
            if attracting(r, m, xr, xi, ar, ai, n - chk / 2) {
                return Outcome::Interior;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn run(cr: f64, ci: f64, max_iter: u64) -> Outcome {
        // Reference at a nearby point exercises the perturbation path.
        let r = Reference::new(cr - 1e-3, ci + 1e-3, max_iter, 1e10);
        sample::<true>(&r, (cr, ci), 1e-3, -1e-3, max_iter, 1e20)
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
            assert_eq!(run(cr, ci, 1_000_000), Outcome::Interior, "{cr},{ci}");
        }
    }

    #[test]
    fn derivative_is_skipped_without_changing_escape() {
        let r = Reference::new(0.3, 0.6, 1000, 1e10);
        let a = sample::<true>(&r, (0.31, 0.6), 0.01, 0.0, 1000, 1e20);
        let b = sample::<false>(&r, (0.31, 0.6), 0.01, 0.0, 1000, 1e20);
        match (a, b) {
            (Outcome::Escaped { n: x, .. }, Outcome::Escaped { n: y, .. }) => assert_eq!(x, y),
            o => panic!("{o:?}"),
        }
    }
}
