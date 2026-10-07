//! Interior tests. Both are heuristic in f64 (results carry `Evidence::Heuristic`).
use crate::reference::Reference;

/// Main cardioid or period-2 bulb: closed-form, no iteration at all.
#[inline]
pub(crate) fn in_main_components(cr: f64, ci: f64) -> bool {
    let y2 = ci * ci;
    let q = (cr - 0.25) * (cr - 0.25) + y2;
    q * (q + (cr - 0.25)) <= 0.25 * y2 || (cr + 1.0) * (cr + 1.0) + y2 <= 0.0625
}

/// `period` steps of the orbit from reference phase `m0` with delta `(x0r, x0i)`:
/// the landing phase and delta, and `d z_P / d z_0`. `None` if the orbit leaves the
/// disc of radius 2 (no cycle point there).
#[allow(clippy::too_many_arguments)]
fn map(r: &Reference, m0: usize, x0r: f64, x0i: f64, ar: f64, ai: f64, period: u64) -> Option<(usize, f64, f64, f64, f64)> {
    let (zr, zi, last) = (&r.re, &r.im, r.len() - 1);
    let (mut m, mut xr, mut xi) = (m0, x0r, x0i);
    let (mut dr, mut di) = (1.0f64, 0.0f64); // d z_P / d z_0
    for _ in 0..period {
        let (fr, fi) = (zr[m] + xr, zi[m] + xi);
        let t = 2.0 * (fr * dr - fi * di);
        di = 2.0 * (fr * di + fi * dr);
        dr = t;
        let t = 2.0 * (zr[m] * xr - zi[m] * xi) + xr * xr - xi * xi + ar;
        xi = 2.0 * (zr[m] * xi + zi[m] * xr + xr * xi) + ai;
        xr = t;
        m += 1;
        let (fr, fi) = (zr[m] + xr, zi[m] + xi);
        let f2 = fr * fr + fi * fi;
        if f2 > 4.0 {
            return None;
        }
        if f2 < xr * xr + xi * xi || m == last {
            (xr, xi, m) = (fr, fi, 0); // rebase
        }
    }
    Some((m, xr, xi, dr, di))
}

/// Whether the `period` steps from the orbit point at reference phase `m0` with delta
/// `(x0r, x0i)` contract: `|d z_P / d z_0| < 1`. Confirms a near-return of the orbit to
/// itself after `period` steps as an attracting cycle. A near-return alone is not
/// evidence: an exterior orbit shadowing a repelling cycle (near a minibrot, or a
/// near-periodic reference whose delta is still small) also returns closely, but
/// expands along the way (FIX-02).
pub(crate) fn contracting(r: &Reference, m0: usize, x0r: f64, x0i: f64, ar: f64, ai: f64, period: u64) -> bool {
    map(r, m0, x0r, x0i, ar, ai, period).is_some_and(|(_, _, _, dr, di)| dr * dr + di * di < 1.0)
}

/// Newton for a period-`period` cycle point starting from the orbit point at reference
/// phase `m0` with delta `(x0r, x0i)`. True iff it converges to a cycle whose
/// multiplier has modulus < 1: an attracting cycle exists only for interior `c`.
/// Converges in a few steps even where the orbit would take thousands of periods to settle.
#[allow(clippy::too_many_arguments)]
pub(crate) fn attracting(r: &Reference, m0: usize, mut x0r: f64, mut x0i: f64, ar: f64, ai: f64, period: u64) -> bool {
    let (zr, zi) = (&r.re, &r.im);
    let (mut first, mut prev) = (0.0, 0.0);
    for it in 0..12 {
        let Some((m, xr, xi, dr, di)) = map(r, m0, x0r, x0i, ar, ai, period) else {
            return false; // left the disc: not a cycle point
        };
        // Newton on g(z) = f^P(z) - z: step = g / (D - 1).
        let rr = (zr[m] + xr) - (zr[m0] + x0r);
        let ri = (zi[m] + xi) - (zi[m0] + x0i);
        let (er, ei) = (dr - 1.0, di);
        let den = er * er + ei * ei;
        if den == 0.0 || !den.is_finite() {
            return false;
        }
        let sr = (rr * er + ri * ei) / den;
        let si = (ri * er - rr * ei) / den;
        x0r -= sr;
        x0i -= si;
        let st = sr.abs() + si.abs();
        if it == 0 {
            first = st;
        } else if st <= 1e-7 * first || st < 1e-300 {
            return dr * dr + di * di < 1.0;
        } else if it >= 2 && st > 0.5 * prev {
            return false; // not converging quadratically: wrong period
        }
        prev = st;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_components() {
        assert!(in_main_components(0.0, 0.0));
        assert!(in_main_components(-1.0, 0.1));
        assert!(!in_main_components(0.3, 0.0));
        assert!(!in_main_components(-0.75, 0.2));
    }
}
