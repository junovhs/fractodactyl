//! Bivariate linear approximation (BLA, ACC-01): affine maps `delta_{m+l} = A delta_m +
//! B dc` over one reference orbit that replace `l = 2^j` perturbation steps at once.
//! Each block carries a validity radius `R` and remainder coefficients `alpha`, `beta`:
//! for `|delta_m| < R` and `|dc| <= dc_max`, in exact arithmetic,
//! `|delta_{m+l} - (A delta_m + B dc)| <= alpha |delta_m| + beta |dc|`, and every
//! skipped step `k` had `|delta_k| <= 2 eps |Z_k|` (its dropped `delta_k^2` is at most
//! `eps` times the kept `2 Z_k delta_k`). Spec: docs/spec/ATLAS.md "BLA tables".
use crate::reference::Reference;
use crate::sample::RESOLVABLE;

/// Largest accepted `eps`, `2^-41`. Inside a block every skipped iterate has `|delta_k| <
/// 2 eps |Z_k|` and so `|delta_k| < 2 eps / (1 - 2 eps) |z_k|`; keeping that at most the
/// periodicity threshold `RESOLVABLE |z|` means no block ever covers an iterate the
/// plain kernel would judge for periodicity.
pub const EPS_MAX: f64 = 1.0 / (1u64 << 41) as f64;
const _: () = assert!(2.0 * EPS_MAX <= RESOLVABLE * (1.0 - 2.0 * EPS_MAX));

/// Refuse an `eps` outside `(0, EPS_MAX]` or a bad `dc_max`.
fn check(eps: f64, dc_max: f64) -> Result<(), String> {
    if eps > 0.0 && eps <= EPS_MAX && dc_max > 0.0 && dc_max.is_finite() {
        Ok(())
    } else {
        Err(format!("BLA needs 0 < eps <= 2^-41 ({EPS_MAX:e}, so blocks never cover a periodicity-judged iterate) and a finite dc_max > 0; got eps {eps:e}, dc_max {dc_max:e}"))
    }
}

/// One block: the affine map over `2^level` steps and its contract. All zero (`r == 0`)
/// when the block is never valid.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Block {
    /// `A` (real, imaginary): the product of `2 Z_k` over the block.
    pub a: (f64, f64),
    /// `B` (real, imaginary): the coefficient of `dc`.
    pub b: (f64, f64),
    /// Validity radius: the block applies only while `|delta_m| < r`.
    pub r: f64,
    /// Remainder coefficient on `|delta_m|`.
    pub alpha: f64,
    /// Remainder coefficient on `|dc|`.
    pub beta: f64,
}

impl Block {
    /// Fields in storage order: `a.re, a.im, b.re, b.im, r, alpha, beta`.
    pub fn to_array(&self) -> [f64; 7] {
        [self.a.0, self.a.1, self.b.0, self.b.1, self.r, self.alpha, self.beta]
    }

    /// Inverse of [`Block::to_array`].
    pub fn from_array(v: [f64; 7]) -> Block {
        Block { a: (v[0], v[1]), b: (v[2], v[3]), r: v[4], alpha: v[5], beta: v[6] }
    }

    /// One step at `Z_k`: `A = 2 Z_k`, `B = 1`, `r = alpha = 2 eps |Z_k|`. Never valid
    /// where `|Z_k| > 2` (the reference is escaping; samples step there one by one).
    fn single(zr: f64, zi: f64, eps: f64) -> Block {
        let az = zr.hypot(zi);
        let r = 2.0 * eps * az;
        if !(r > 0.0 && az <= 2.0) {
            return Block::default();
        }
        Block { a: (2.0 * zr + 0.0, 2.0 * zi + 0.0), b: (1.0, 0.0), r, alpha: r, beta: 0.0 }
    }

    /// `x` then `y` for any `|dc| <= c`. With `Ahat = |A_x| + alpha_x` and `Bhat = |B_x| +
    /// beta_x`, the true `|delta|` after `x` is at most `Ahat |delta_m| + Bhat |dc|`, so
    /// `r = min(r_x, (r_y - Bhat c) / Ahat)` keeps it inside `y`'s radius.
    fn merge(x: &Block, y: &Block, c: f64) -> Block {
        if x.r == 0.0 || y.r == 0.0 {
            return Block::default();
        }
        let (ax, ay) = (x.a.0.hypot(x.a.1), y.a.0.hypot(y.a.1));
        let ahat = ax + x.alpha;
        let bhat = x.b.0.hypot(x.b.1) + x.beta;
        let r = x.r.min((y.r - bhat * c) / ahat);
        let z = Block {
            a: (y.a.0 * x.a.0 - y.a.1 * x.a.1 + 0.0, y.a.0 * x.a.1 + y.a.1 * x.a.0 + 0.0),
            b: (y.a.0 * x.b.0 - y.a.1 * x.b.1 + y.b.0 + 0.0, y.a.0 * x.b.1 + y.a.1 * x.b.0 + y.b.1 + 0.0),
            r,
            alpha: ay * x.alpha + y.alpha * ahat,
            beta: ay * x.beta + y.alpha * bhat + y.beta,
        };
        if r > 0.0 && z.to_array().iter().all(|v| v.is_finite()) {
            z
        } else {
            Block::default()
        }
    }
}

/// A BLA table for one reference orbit of `points` points, valid for `|dc| <= dc_max`.
/// `levels[i]` holds level `j = i + 1`: `steps >> j` blocks of `2^j` steps (`steps =
/// points - 2`); block `t` starts at reference index `1 + t 2^j` and lands at most on
/// the last point. Single steps (level 0) are only built, never stored or applied: a
/// one-step block saves no step and costs more than the plain step it would replace.
#[derive(Clone, Debug, PartialEq)]
pub struct Bla {
    /// Relative tolerance of each dropped quadratic term.
    pub eps: f64,
    /// Largest `|dc|` the radii were built for.
    pub dc_max: f64,
    /// Length of the reference orbit the table was built over.
    pub points: u64,
    /// Levels 1, 2, ...; ends before the first level with no valid block.
    pub levels: Vec<Vec<Block>>,
    /// Squared radii, per level, for the lookup (contiguous, so the radius tests stay in
    /// cache while the coefficients are read only for the chosen block).
    r2: Vec<Vec<f64>>,
}

impl Bla {
    /// Build the table over `r` for samples with `|dc| <= dc_max`.
    pub fn build(r: &Reference, eps: f64, dc_max: f64) -> Result<Bla, String> {
        check(eps, dc_max)?;
        let steps = r.len().saturating_sub(2);
        let merge = |l: &[Block]| -> Vec<Block> { (0..l.len() / 2).map(|i| Block::merge(&l[2 * i], &l[2 * i + 1], dc_max)).collect() };
        let singles: Vec<Block> = (1..=steps).map(|k| Block::single(r.re[k], r.im[k], eps)).collect();
        let mut levels: Vec<Vec<Block>> = Vec::new();
        let mut next = merge(&singles);
        while next.iter().any(|b| b.r > 0.0) {
            let up = merge(&next);
            levels.push(std::mem::replace(&mut next, up));
        }
        Bla::new(eps, dc_max, r.len() as u64, levels)
    }

    /// Assemble a table (e.g. decoded from the atlas), checking its shape and values.
    pub fn new(eps: f64, dc_max: f64, points: u64, levels: Vec<Vec<Block>>) -> Result<Bla, String> {
        check(eps, dc_max)?;
        let steps = points.saturating_sub(2);
        for (i, l) in levels.iter().enumerate() {
            let j = i + 1;
            if j >= 64 || l.len() as u64 != steps >> j {
                return Err(format!("BLA level {j} has {} blocks; a {points}-point orbit needs {}", l.len(), steps >> j));
            }
            if !l.iter().flat_map(Block::to_array).all(|v| v.is_finite()) || l.iter().any(|b| b.r < 0.0) {
                return Err(format!("BLA level {j} holds a non-finite value or negative radius"));
            }
        }
        let r2 = levels.iter().map(|l| l.iter().map(|b| b.r * b.r).collect()).collect();
        Ok(Bla { eps, dc_max, points, levels, r2 })
    }

    /// Blocks stored, and how many of them are valid somewhere.
    pub fn counts(&self) -> (usize, usize) {
        let all = self.levels.iter().map(Vec::len).sum();
        (all, self.levels.iter().flatten().filter(|b| b.r > 0.0).count())
    }

    /// The longest block starting at reference index `m` that is valid for
    /// `|delta|^2 = d2` and at most `budget` steps long, with its step count (at least 2).
    ///
    /// A level-`j` block starts where its first half (the level `j-1` block at the same
    /// index) starts and its radius is at most that half's (`merge`), so validity only
    /// shrinks going up: walk up from level 1 and stop at the first failure.
    #[inline]
    pub(crate) fn find(&self, m: usize, d2: f64, budget: u64) -> Option<(&Block, usize)> {
        // Blocks of 2^j steps start at k = m - 1 divisible by 2^j (m = 0 wraps: odd).
        let k = m.wrapping_sub(1);
        let top = (k.trailing_zeros() as usize).min(self.levels.len());
        let ok = |i: usize| (2u64 << i) <= budget && self.r2[i].get(k >> (i + 1)).is_some_and(|&r2| d2 < r2);
        if top == 0 || !ok(0) {
            return None;
        }
        let mut i = 0;
        while i + 1 < top && ok(i + 1) {
            i += 1;
        }
        Some((&self.levels[i][k >> (i + 1)], 2 << i))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Plain perturbation steps from `m`, in f64 (rounding is far below the bounds).
    fn steps(r: &Reference, m: usize, l: usize, d: (f64, f64), dc: (f64, f64)) -> (f64, f64) {
        let (mut x, mut y) = d;
        for k in m..m + l {
            let (zr, zi) = (r.re[k], r.im[k]);
            (x, y) = (2.0 * (zr * x - zi * y) + x * x - y * y + dc.0, 2.0 * (zr * y + zi * x + x * y) + dc.1);
        }
        (x, y)
    }

    #[test]
    fn remainder_bound_covers_true_perturbation() {
        // A long bounded orbit, at the largest eps: dropped terms ~2^-41 per step, far
        // above the f64 rounding (~U per step) the slack below allows for.
        let r = Reference::new(-0.743643887037158, 0.131825904205312, 5000, 1e10);
        let (eps, c) = (EPS_MAX, 1e-17);
        let t = Bla::build(&r, eps, c).unwrap();
        assert!(r.len() == 5001 && t.levels.len() > 3, "points {} levels {}", r.len(), t.levels.len());
        let mut checked = 0;
        for (j, level) in t.levels.iter().enumerate() {
            for (i, b) in level.iter().enumerate().step_by(3).filter(|(_, b)| b.r > 0.0) {
                let (m, l) = (1 + (i << (j + 1)), 2usize << j);
                for (u, v, w) in [(0.9, 0.3, 1.0), (-0.5, 0.8, -0.7), (0.0, -0.99, 0.2)] {
                    let d = (u * b.r, v * b.r * 0.1);
                    let dc = (w * c * 0.7, 0.7 * c * 0.7);
                    let got = steps(&r, m, l, d, dc);
                    let lin = (b.a.0 * d.0 - b.a.1 * d.1 + b.b.0 * dc.0 - b.b.1 * dc.1, b.a.0 * d.1 + b.a.1 * d.0 + b.b.0 * dc.1 + b.b.1 * dc.0);
                    let err = (got.0 - lin.0).hypot(got.1 - lin.1);
                    let bound = b.alpha * d.0.hypot(d.1) + b.beta * dc.0.hypot(dc.1);
                    let round = 8.0 * l as f64 * f64::EPSILON * (b.a.0.hypot(b.a.1) * d.0.hypot(d.1) + b.b.0.hypot(b.b.1) * dc.0.hypot(dc.1));
                    assert!(err <= bound + round, "level {j} block {i}: err {err} > bound {bound}");
                    checked += 1;
                }
            }
        }
        assert!(checked > 1000, "{checked}");
    }

    #[test]
    fn radii_shrink_upward_so_find_takes_the_longest_valid_block() {
        let r = Reference::new(-0.743643887037158, 0.131825904205312, 20_000, 1e10);
        let t = Bla::build(&r, 2f64.powi(-50), 1e-25).unwrap();
        assert!(t.levels.len() > 5, "levels {}", t.levels.len());
        for i in 1..t.levels.len() {
            for (x, b) in t.levels[i].iter().enumerate() {
                assert!(b.r <= t.levels[i - 1][2 * x].r, "level {} block {x}", i + 1);
            }
        }
        // Brute force: the longest block from m with d2 < r^2 and length <= budget.
        for m in (0..t.points as usize).step_by(3) {
            for (d2, budget) in [(1e-40, u64::MAX), (1e-31, u64::MAX), (1e-34, 5), (1e-36, 2), (1e-40, 1)] {
                let k = m.wrapping_sub(1);
                let want = (0..t.levels.len())
                    .take_while(|&i| m > 0 && k % (2 << i) == 0 && (2u64 << i) <= budget)
                    .map(|i| (i, t.levels[i].get(k >> (i + 1))))
                    .take_while(|(_, b)| b.is_some_and(|b| d2 < b.r * b.r))
                    .last()
                    .map(|(i, b)| (*b.unwrap(), 2usize << i));
                assert_eq!(t.find(m, d2, budget).map(|(b, l)| (*b, l)), want, "m {m} d2 {d2} budget {budget}");
            }
        }
    }

    #[test]
    fn shape_is_checked_and_find_prefers_long_blocks() {
        let r = Reference::new(-0.7453, 0.1127, 2000, 1e10);
        let t = Bla::build(&r, EPS_MAX, 1e-20).unwrap();
        let mut bad = t.levels.clone();
        bad[1].pop();
        assert!(Bla::new(t.eps, t.dc_max, t.points, bad).is_err());
        assert!(Bla::new(0.0, 1.0, t.points, vec![]).is_err());
        // eps above the periodicity threshold's cap is refused by build and new alike.
        assert!(Bla::new(2.0 * EPS_MAX, 1.0, t.points, vec![]).is_err());
        assert!(Bla::build(&r, 2.0 * EPS_MAX, 1e-20).is_err());
        assert!(Bla::new(EPS_MAX, 1.0, t.points, vec![]).is_ok());
        // A tiny delta at index 1 takes the longest valid block; index 0 (Z_0 = 0) and
        // even indices (odd k) never skip, nor does a budget under 2 steps.
        let (_, len) = t.find(1, 1e-40, u64::MAX).unwrap();
        assert!(len > 2, "{len}");
        assert!(t.find(0, 1e-40, u64::MAX).is_none());
        assert!(t.find(2, 1e-40, u64::MAX).is_none());
        assert!(t.find(1, 1e-40, 1).is_none());
        assert_eq!(t.find(1, 1e-40, 3).unwrap().1, 2);
        assert!(t.find(1, 1.0, u64::MAX).is_none());
    }
}
