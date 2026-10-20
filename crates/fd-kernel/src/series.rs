//! PROB-41 research arm: PROB-40's growing-order parameter series initializer over a
//! general complex reference, admitted frame-wide before rendering (DEC-19). Not used by
//! any fd command; `render_series` exists so a probe can time it on whole frames against
//! the plain and per-frame BLA kernels it continues with.
//!
//! Normalised series (docs/2026-10-20/README.md): with `dc = rho x`, `|x| <= 1` over the
//! frame, the delta after `n` steps is `S_n(x) = sum b[k] x^k`, where
//! `b[1]' = 2 Z b[1] + rho` and `b[k]' = 2 Z b[k] + sum(i+j=k) b[i] b[j]`. Only the first
//! `active` coefficients are kept; `active` doubles (8, 16, 32, 64) once the carried
//! remainder passes `grow_at` of the step's budget, and the remainder is never reset.
//! Exact-arithmetic majorants on the disk (M = sum |b|, D = sum k|b|, T/T' the omitted
//! square's coefficient norms):
//!
//!   E' = 2 (|Z| + M) E + E^2 + T            (state)
//!   F' = 2 (|Z| + M + E) F + 2 E D + T'     (d/dx of the state)
//!
//! Contract (DEC-10). Validity: every intermediate `|Z| + M + E < 2` (no sample can leave
//! the escape disk inside the prefix; a near-2 reference abstains rather than
//! compensates) and `dz/dc` stays bounded away from zero:
//! `Dmin = |b1| - sum(k>=2) k|b[k]| - F > 0`. Error: `E rho / Dmin`, the first-order shift in `c` equivalent to the state
//! error, is at most `shift_px` output pixels, and `F <= derivative_rel Dmin`. Fallback:
//! a frame where no step passes gets `n = 0`, i.e. the ordinary kernel. Rounding of the
//! reference, coefficients and evaluation is not enclosed (research thresholds only).
use crate::reference::Reference;

/// Guard thresholds and the order schedule.
#[derive(Clone, Copy, Debug)]
pub struct SeriesConfig {
    /// First active order (equal to `max_order` for a fixed-order series).
    pub start_order: usize,
    /// Largest order.
    pub max_order: usize,
    /// Double the active order once the carried state bound passes this fraction of the
    /// step's state budget (PROB-40's selected schedule: 1e-4).
    pub grow_at: f64,
    /// Largest admitted equivalent shift, in output pixels.
    pub shift_px: f64,
    /// Largest admitted relative error of `dz/dc`.
    pub derivative_rel: f64,
}

impl Default for SeriesConfig {
    fn default() -> SeriesConfig {
        SeriesConfig {
            start_order: 8,
            max_order: 64,
            grow_at: 1e-4,
            // 1e-4 failed fd compare on 13 of 24 PROB-22 frames: the state error is
            // amplified ~1e4 through the reference's near-zero passage just after the
            // prefix, which the first-order shift does not see (PROB-41).
            shift_px: 1e-8,
            derivative_rel: 1e-6,
        }
    }
}

/// An admitted prefix: every sample with `|dc| <= rho` starts at iterate `n`.
#[derive(Clone, Debug)]
pub struct Series {
    /// Iterates every sample skips (0: the guard admitted none; the plain kernel runs).
    pub n: u64,
    /// Normalising radius (the frame's largest `|dc|`).
    pub rho: f64,
    /// Points of the reference orbit the series was built over.
    pub points: u64,
    /// Coefficients `b[1..]` at `n`, trimmed to the order evaluation needs.
    pub b: Vec<(f64, f64)>,
    /// State and normalised-derivative bounds at `n`, including trimmed coefficients.
    pub state_bound: f64,
    pub derivative_bound: f64,
    /// Equivalent shift of `state_bound` in output pixels.
    pub shift_px: f64,
    /// Steps the build ran (past `n` until a guard stopped it) and why it stopped.
    pub built_steps: u64,
    pub stop: &'static str,
    /// Coefficient-pair products computed (the build's work).
    pub coefficient_pairs: u64,
    /// Steps at which the active order doubled.
    pub growth: Vec<u64>,
}

impl Series {
    /// Build over `r` for samples with `|dc| <= rho`, output pixel size `pixel` (in `c`),
    /// at most `max_iter` iterates.
    pub fn build(r: &Reference, rho: f64, pixel: f64, max_iter: u64, cfg: SeriesConfig) -> Series {
        let k_max = cfg.max_order.max(1);
        let mut active = cfg.start_order.clamp(1, k_max);
        // Index 0 unused: b[k] for k = 1..=k_max.
        let (mut br, mut bi) = (vec![0.0f64; k_max + 1], vec![0.0f64; k_max + 1]);
        let (mut sr, mut si) = (vec![0.0f64; 2 * k_max + 1], vec![0.0f64; 2 * k_max + 1]);
        let (mut e, mut f) = (0.0f64, 0.0f64);
        let mut out = Series {
            n: 0,
            rho,
            points: r.len() as u64,
            b: Vec::new(),
            state_bound: 0.0,
            derivative_bound: 0.0,
            shift_px: 0.0,
            built_steps: 0,
            stop: "reference",
            coefficient_pairs: 0,
            growth: Vec::new(),
        };
        let limit = (r.len() as u64)
            .saturating_sub(2)
            .min(max_iter.saturating_sub(1));
        // (n, b, E, F, state budget, derivative budget, Dmin) at the last admitted step.
        type Admitted = (u64, Vec<(f64, f64)>, f64, f64, f64, f64, f64);
        let mut best: Option<Admitted> = None;
        if !(rho > 0.0 && rho.is_finite() && pixel > 0.0) {
            out.stop = "radius";
            return out;
        }
        let (cap_e, cap_f) = (cfg.shift_px * pixel * 2.0 / rho, cfg.derivative_rel * 2.0);
        for n in 0..limit {
            let (zr, zi) = (r.re[n as usize], r.im[n as usize]);
            let az = zr.hypot(zi);
            // Square of the active polynomial, with the omitted (degree > active) part.
            sr[..=2 * active].fill(0.0);
            si[..=2 * active].fill(0.0);
            for i in 1..=active {
                let (ar, ai) = (br[i], bi[i]);
                if ar == 0.0 && ai == 0.0 {
                    continue;
                }
                sr[2 * i] += ar * ar - ai * ai;
                si[2 * i] += 2.0 * ar * ai;
                let (ar, ai) = (2.0 * ar, 2.0 * ai);
                for j in i + 1..=active {
                    sr[i + j] += ar * br[j] - ai * bi[j];
                    si[i + j] += ar * bi[j] + ai * br[j];
                }
            }
            out.coefficient_pairs += (active * (active + 1) / 2) as u64;
            let (mut m, mut d, mut t, mut td) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
            for k in 1..=active {
                let a = br[k].hypot(bi[k]);
                m += a;
                d += k as f64 * a;
            }
            for k in active + 1..=2 * active {
                let a = sr[k].hypot(si[k]);
                t += a;
                td += k as f64 * a;
            }
            let e_next = 2.0 * (az + m) * e + e * e + t;
            f = 2.0 * (az + m + e) * f + 2.0 * e * d + td;
            e = e_next;
            for k in 1..=active {
                let (xr, xi) = (br[k], bi[k]);
                br[k] = 2.0 * (zr * xr - zi * xi) + sr[k];
                bi[k] = 2.0 * (zr * xi + zi * xr) + si[k];
            }
            br[1] += rho;
            // State after step n + 1.
            let step = n + 1;
            out.built_steps = step;
            let (zr, zi) = (r.re[step as usize], r.im[step as usize]);
            let (mut m, mut tail_d) = (0.0f64, 0.0f64);
            for k in 1..=active {
                let a = br[k].hypot(bi[k]);
                m += a;
                if k >= 2 {
                    tail_d += k as f64 * a;
                }
            }
            if !(e.is_finite() && f.is_finite()) {
                out.stop = "non-finite";
                break;
            }
            if zr.hypot(zi) + m + e >= 2.0 {
                out.stop = "escape";
                break;
            }
            let dmin = br[1].hypot(bi[1]) - tail_d - f;
            // Absolute state error allowed now: shift_px output pixels through dz/dc.
            let budget = if dmin > 0.0 {
                cfg.shift_px * pixel * dmin / rho
            } else {
                0.0
            };
            if dmin > 0.0 && e <= budget && f <= cfg.derivative_rel * dmin {
                best = Some((
                    step,
                    (1..=active).map(|k| (br[k], bi[k])).collect(),
                    e,
                    f,
                    budget,
                    cfg.derivative_rel * dmin,
                    dmin,
                ));
            } else if e > cap_e || f > cap_f {
                // |b1| <= M < 2 caps every later budget: nothing past here can pass.
                out.stop = "budget";
                break;
            }
            if active < k_max && e > cfg.grow_at * budget {
                active = (2 * active).min(k_max);
                out.growth.push(step);
            }
        }
        if let Some((n, mut b, e, f, budget_e, budget_f, dmin)) = best {
            // Trim high orders whose norms fit in half the remaining slack of each bound.
            let (slack_e, slack_f) = (budget_e - e, budget_f - f);
            let (mut te, mut tf) = (0.0f64, 0.0f64);
            while b.len() > 1 {
                let k = b.len();
                let a = b[k - 1].0.hypot(b[k - 1].1);
                if te + a > 0.5 * slack_e || tf + k as f64 * a > 0.5 * slack_f {
                    break;
                }
                te += a;
                tf += k as f64 * a;
                b.pop();
            }
            out.n = n;
            out.b = b;
            out.state_bound = e + te;
            out.derivative_bound = f + tf;
            out.shift_px = out.state_bound * rho / (dmin * pixel);
        }
        out
    }

    /// The state after [`Series::n`] iterates for sample offset `dc = (ar, ai)`: the delta
    /// and `dz/dc` (zero when `D` is off).
    #[inline]
    pub(crate) fn eval<const D: bool>(&self, ar: f64, ai: f64) -> (f64, f64, f64, f64) {
        let s = 1.0 / self.rho;
        let (xr, xi) = (ar * s, ai * s);
        let (mut pr, mut pi, mut qr, mut qi) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
        // Horner on x (b[0] = 0): p = sum b[k] x^(k-1), q = p'.
        for &(cr, ci) in self.b.iter().rev() {
            if D {
                (qr, qi) = (qr * xr - qi * xi + pr, qr * xi + qi * xr + pi);
            }
            (pr, pi) = (pr * xr - pi * xi + cr, pr * xi + pi * xr + ci);
        }
        // S = x p, S' = p + x q; dz/dc = S' / rho.
        let (delta_r, delta_i) = (pr * xr - pi * xi, pr * xi + pi * xr);
        if !D {
            return (delta_r, delta_i, 0.0, 0.0);
        }
        let (dr, di) = (pr + qr * xr - qi * xi, pi + qr * xi + qi * xr);
        (delta_r, delta_i, dr * s, di * s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::{reference, render_series, render_with, Params};
    use fd_samples::{Column, ColumnSet, View};

    /// The evaluated start stays within its carried bounds of direct f64 perturbation.
    #[test]
    fn start_is_within_its_bounds_of_direct_iteration() {
        let r = Reference::new(-0.743643887037158, 0.131825904205312, 20_000, 1e10);
        let (rho, pixel) = (1e-11, 1e-13);
        let s = Series::build(&r, rho, pixel, 20_000, SeriesConfig::default());
        assert!(s.n > 100, "admitted prefix {}", s.n);
        assert!(s.shift_px <= SeriesConfig::default().shift_px);
        for (ar, ai) in [
            (rho, 0.0),
            (0.0, -rho),
            (0.6 * rho, 0.7 * rho),
            (1e-3 * rho, 0.0),
        ] {
            let (mut xr, mut xi, mut dr, mut di) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
            for m in 0..s.n as usize {
                let (zr, zi) = (r.re[m] + xr, r.im[m] + xi);
                (dr, di) = (2.0 * (zr * dr - zi * di) + 1.0, 2.0 * (zr * di + zi * dr));
                let (wr, wi) = (r.re[m], r.im[m]);
                (xr, xi) = (
                    2.0 * (wr * xr - wi * xi) + xr * xr - xi * xi + ar,
                    2.0 * (wr * xi + wi * xr) + 2.0 * xr * xi + ai,
                );
            }
            let (sr, si, sdr, sdi) = s.eval::<true>(ar, ai);
            let slack = 1e-9 * xr.hypot(xi);
            assert!(
                (sr - xr).hypot(si - xi) <= s.state_bound + slack,
                "delta at {ar:e},{ai:e}"
            );
            let slack = 1e-9 * dr.hypot(di);
            assert!(
                (sdr - dr).hypot(sdi - di) <= s.derivative_bound / rho + slack,
                "dz/dc at {ar:e},{ai:e}"
            );
        }
    }

    /// A whole frame started from the series keeps every class and moves nu within fd
    /// compare's 1e-3 px; with no admitted prefix it is the plain render.
    #[test]
    fn series_frame_keeps_classes() {
        let view = View {
            center_re: "-0.743643887037158".into(),
            center_im: "0.131825904205312".into(),
            width: "1e-10".into(),
            rotation: 0.0,
        };
        let p = Params {
            nx: 48,
            ny: 27,
            ss: 1,
            max_iter: 20_000,
            escape_radius: 1e10,
            columns: ColumnSet::of(&[Column::Nu, Column::De]),
            threads: 2,
            tier: None,
        };
        let (_, sp, _) = render_with(&view, &p, None).unwrap();
        let orbit = reference(&view, &p).unwrap();
        let rho = crate::grid::bla_dc_max(&view, &p).unwrap();
        let s = Series::build(
            &orbit.0,
            rho,
            1e-10 / 48.0,
            p.max_iter,
            SeriesConfig::default(),
        );
        assert!(s.n > 0);
        let (h, ss, st, _) = render_series(&view, &p, orbit.clone(), &s, None).unwrap();
        assert!(h.kernel.ends_with(" series/0"), "{}", h.kernel);
        assert_eq!(sp.class, ss.class);
        // fd compare's displacement: |dnu| / |grad nu|, |grad nu| = 2 / (de ln 2) per
        // pixel, scored where the reference de exceeds 1e-3 px.
        let (a, b, de) = (sp.nu.unwrap(), ss.nu.unwrap(), sp.de.unwrap());
        let worst = (0..a.len())
            .filter(|&i| de[i] > 1e-3)
            .map(|i| (a[i] - b[i]).abs() * f64::from(de[i]) * std::f64::consts::LN_2 / 2.0)
            .fold(0.0, f64::max);
        assert!(worst < 1e-3, "nu moved {worst} px");
        assert!(st.iterations < 48 * 27 * 20_000);
        let none = Series { n: 0, ..s };
        let (_, sn, _, _) = render_series(&view, &p, orbit, &none, None).unwrap();
        assert_eq!(sn.class, sp.class);
        assert_eq!(sn.nu, Some(a));
    }
}
