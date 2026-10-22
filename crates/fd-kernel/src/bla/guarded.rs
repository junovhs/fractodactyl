//! GPU-33 affine uncertainty envelopes over the existing BLA validity domain.
//! Exact-reference perturbation is (2Z+delta)delta+dc; its parameter derivative
//! evolves with multiplier 2Z+2delta. These distinct factors, not differentiation
//! of the state remainder bound, give the paired envelopes below (DEC-10/24).
use super::{affine, check, Block};
use crate::Reference;

fn norm(z: (f64, f64)) -> f64 {
    z.0.abs() + z.1.abs()
}
fn up(x: f64) -> f64 {
    x * (1.0 + 64.0 * f64::EPSILON) + f64::MIN_POSITIVE
}
fn remainder(ay: f64, ax: f64, bx: f64, xa: f64, xb: f64, ya: f64, yb: f64) -> (f64, f64) {
    (
        up(ay * xa + ya * (ax + xa)),
        up(ay * xb + ya * (bx + xb) + yb),
    )
}

/// A nominal BLA block plus absolute operator uncertainty on dz/dc.
/// For valid delta, state error <= alpha|delta|+beta|dc|; derivative error
/// <= derivative_alpha|D|+derivative_beta. These are heuristic CPU RN envelopes;
/// GPU coefficient quantization/evaluation rounding must be charged separately.
#[derive(Clone, Copy, Debug, Default)]
pub struct GuardedBlock {
    /// Nominal A/B, common validity radius and state envelope.
    pub block: Block,
    /// Uncertainty in the derivative's multiplicative coefficient A.
    pub derivative_alpha: f64,
    /// Uncertainty in the derivative's additive coefficient B.
    pub derivative_beta: f64,
}
impl GuardedBlock {
    fn single(z: (f64, f64), q: f64, eps: f64) -> Self {
        let mut b = Block::single(z.0, z.1, eps);
        if b.r == 0.0 || !q.is_finite() || q < 0.0 || norm(z) + q > 2.0 {
            return Self::default();
        }
        // |delta|<r: omitted delta^2 <= r|delta|. Reference coefficient
        // 2Z differs by <=2q; derivative multiplier has the extra 2delta.
        let da = up(2.0 * b.r + 2.0 * q);
        b.alpha = up(b.r + 2.0 * q);
        Self {
            block: b,
            derivative_alpha: da,
            derivative_beta: 0.0,
        }
    }
    fn merge(x: &Self, y: &Self, c: f64) -> Self {
        let (x, y, dx, dy) = (x.block, y.block, x, y);
        if x.r == 0.0 || y.r == 0.0 {
            return Self::default();
        }
        let (ax, ay, bx) = (norm(x.a), norm(y.a), norm(x.b));
        let r =
            x.r.min((y.r - up((bx + x.beta) * c)) / up(ax + x.alpha)) * (1.0 - 64.0 * f64::EPSILON);
        let (a, b) = (affine(x.a, y.a, (0.0, 0.0)), affine(x.b, y.a, y.b));
        // Binary64 host composition: four products/two sums per complex field;
        // L1 magnitudes and32u allowance cover RN coefficient rounding, independent
        // of cancellation. This is separate from later CE upload/evaluation loss.
        let ra = up(32.0 * f64::EPSILON * ay * ax);
        let rb = up(32.0 * f64::EPSILON * (ay * bx + norm(y.b)));
        let (alpha, beta) = remainder(ay, ax, bx, x.alpha, x.beta, y.alpha, y.beta);
        let (da, db) = remainder(
            ay,
            ax,
            bx,
            dx.derivative_alpha,
            dx.derivative_beta,
            dy.derivative_alpha,
            dy.derivative_beta,
        );
        let z = Self {
            block: Block {
                a,
                b,
                r,
                alpha: up(alpha + ra),
                beta: up(beta + rb),
            },
            derivative_alpha: up(da + ra),
            derivative_beta: up(db + rb),
        };
        if r > 0.0
            && z.block
                .to_array()
                .iter()
                .chain([&z.derivative_alpha, &z.derivative_beta])
                .all(|x| x.is_finite())
        {
            z
        } else {
            Self::default()
        }
    }
}

/// A shared CPU-built affine table carrying state/derivative envelopes for GPU use.
/// Build once over the exact same reference; `q[k]` encloses its mathematical
/// reference error, not uploaded coefficient error. No atlas format is changed.
#[derive(Clone, Debug)]
pub struct GuardedBla {
    /// Largest allowed absolute parameter offset, including its input error.
    pub dc_max: f64,
    /// Blocks at levels1,2,..., with the ordinary BLA alignment/layout.
    pub levels: Vec<Vec<GuardedBlock>>,
}
impl GuardedBla {
    /// Build guarded blocks; unsupported/malformed reference uncertainties refuse.
    pub fn build(r: &Reference, q: &[f64], eps: f64, dc_max: f64) -> Result<Self, String> {
        check(eps, dc_max)?;
        if q.len() != r.len() || q.iter().any(|q| !q.is_finite() || *q < 0.0) {
            return Err(
                "guarded BLA needs one finite nonnegative reference radius per point".into(),
            );
        }
        let mut next: Vec<_> = (1..r.len().saturating_sub(1))
            .map(|k| GuardedBlock::single((r.re[k], r.im[k]), q[k], eps))
            .collect();
        let mut levels = Vec::new();
        loop {
            next = next
                .chunks_exact(2)
                .map(|b| GuardedBlock::merge(&b[0], &b[1], dc_max))
                .collect();
            if !next.iter().any(|b| b.block.r > 0.0) {
                break;
            }
            levels.push(next.clone());
        }
        Ok(Self { dc_max, levels })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guarded_bla_direct_derivative_windows() {
        let mut checked = 0;
        let mut naive_misses = 0;
        let mut worst = 0.0f64;
        for (cr, ci, c) in [
            (-0.743643887037158, 0.131825904205312, 1e-28),
            (-0.7453, 0.1127, 1e-24),
            (0.25, 0.0, 1e-24),
        ] {
            let r = Reference::new(cr, ci, 10000, 1e10);
            let q = vec![0.0; r.len()];
            for eps in [super::super::EPS_MAX, 2f64.powi(-50)] {
                let t = GuardedBla::build(&r, &q, eps, c).unwrap();
                for (j, level) in t.levels.iter().enumerate() {
                    for (i, g) in level
                        .iter()
                        .enumerate()
                        .step_by(17)
                        .filter(|(_, g)| g.block.r > 0.0)
                    {
                        let b = g.block;
                        let m = 1 + (i << (j + 1));
                        let len = 2 << j;
                        let delta = (0.9 * b.r, 0.05 * b.r);
                        let dc = (0.3 * c, -0.2 * c);
                        let initial_d = (0.7, -0.3);
                        let (mut d, mut derivative) = (delta, initial_d);
                        for k in m..m + len {
                            let z = (r.re[k] + d.0, r.im[k] + d.1);
                            derivative = affine(derivative, (2.0 * z.0, 2.0 * z.1), (1.0, 0.0));
                            d = affine(d, (2.0 * r.re[k] + d.0, 2.0 * r.im[k] + d.1), dc);
                        }
                        let approximate = affine(initial_d, b.a, b.b);
                        let err =
                            (derivative.0 - approximate.0).hypot(derivative.1 - approximate.1);
                        let bound = g.derivative_alpha * norm(initial_d) + g.derivative_beta;
                        // Independent direct binary64 recurrence itself rounds per step.
                        let round = 64.0
                            * f64::EPSILON
                            * len as f64
                            * (norm(derivative) + norm(approximate));
                        assert!(
                            err <= bound + round,
                            "m{m} len{len} error{err:e} > bound{bound:e}+direct-round{round:e}"
                        );
                        let state = affine(
                            delta,
                            b.a,
                            (b.b.0 * dc.0 - b.b.1 * dc.1, b.b.0 * dc.1 + b.b.1 * dc.0),
                        );
                        let serr = (d.0 - state.0).hypot(d.1 - state.1);
                        let sb = b.alpha * norm(delta) + b.beta * norm(dc);
                        assert!(
                            serr <= sb + 64.0 * f64::EPSILON * len as f64 * (norm(d) + norm(state))
                        );
                        naive_misses += usize::from(err > b.alpha * norm(initial_d) + b.beta);
                        worst = worst.max(err / (bound + round));
                        checked += 1;
                    }
                }
            }
        }
        assert!(checked > 500, "{checked}");
        assert!(
            naive_misses > 0,
            "state-only envelope must demonstrably miss derivative cases"
        );
        println!("GPU33_CPU_DIRECT checked={checked} naive_state_as_derivative_misses={naive_misses} worst_error_envelope_ratio={worst}");
        let r = Reference::new(0.25, 0.0, 20, 1e10);
        assert!(GuardedBla::build(&r, &[], 2f64.powi(-50), 1e-20).is_err());
        assert!(GuardedBla::build(&r, &vec![f64::NAN; r.len()], 2f64.powi(-50), 1e-20).is_err());
    }
}
