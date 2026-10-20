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
        [
            self.a.0, self.a.1, self.b.0, self.b.1, self.r, self.alpha, self.beta,
        ]
    }

    /// Inverse of [`Block::to_array`].
    pub fn from_array(v: [f64; 7]) -> Block {
        Block {
            a: (v[0], v[1]),
            b: (v[2], v[3]),
            r: v[4],
            alpha: v[5],
            beta: v[6],
        }
    }

    /// One step at `Z_k`: `A = 2 Z_k`, `B = 1`, `r = alpha = 2 eps |Z_k|`. Never valid
    /// where `|Z_k| > 2` (the reference is escaping; samples step there one by one).
    fn single(zr: f64, zi: f64, eps: f64) -> Block {
        let az = zr.hypot(zi);
        let r = 2.0 * eps * az;
        if !(r > 0.0 && az <= 2.0) {
            return Block::default();
        }
        Block {
            a: (2.0 * zr + 0.0, 2.0 * zi + 0.0),
            b: (1.0, 0.0),
            r,
            alpha: r,
            beta: 0.0,
        }
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
            a: (
                y.a.0 * x.a.0 - y.a.1 * x.a.1 + 0.0,
                y.a.0 * x.a.1 + y.a.1 * x.a.0 + 0.0,
            ),
            b: (
                y.a.0 * x.b.0 - y.a.1 * x.b.1 + y.b.0 + 0.0,
                y.a.0 * x.b.1 + y.a.1 * x.b.0 + y.b.1 + 0.0,
            ),
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
    min_level: usize,
    #[cfg(test)]
    pub(crate) quadratic: Vec<Vec<Quadratic>>,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Quadratic {
    p: [(f64, f64); 15],
    order: usize,
}

#[cfg(test)]
impl Quadratic {
    fn mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
        (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
    }
    fn sum(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> (f64, f64) {
        (a.0 + b.0 + c.0, a.1 + b.1 + c.1)
    }
    fn index(i: usize, j: usize) -> usize {
        (i + j) * (i + j + 1) / 2 + j
    }
    fn single(order: usize) -> Self {
        let mut q = Self {
            order,
            ..Self::default()
        };
        q.p[Self::index(2, 0)] = (1.0, 0.0);
        q
    }
    fn product(a: Self, b: Self) -> Self {
        let mut c = Self {
            order: a.order,
            ..Self::default()
        };
        for n in 0..=a.order {
            for j in 0..=n {
                let x = a.p[Self::index(n - j, j)];
                if x == (0.0, 0.0) {
                    continue;
                }
                for m in 0..=a.order - n {
                    for k in 0..=m {
                        let y = b.p[Self::index(m - k, k)];
                        if y == (0.0, 0.0) {
                            continue;
                        }
                        let at = Self::index(n - j + m - k, j + k);
                        c.p[at] = Self::sum(c.p[at], Self::mul(x, y), (0.0, 0.0));
                    }
                }
            }
        }
        c
    }
    fn merge(x: &Block, y: &Block, mut p: Self, mut q: Self) -> Self {
        p.p[1] = x.a;
        p.p[2] = x.b;
        q.p[1] = y.a;
        q.p[2] = y.b;
        let mut powers = [Self {
            order: p.order,
            ..Self::default()
        }; 5];
        powers[0].p[0] = (1.0, 0.0);
        powers[1] = p;
        for i in 2..=p.order {
            powers[i] = Self::product(powers[i - 1], p);
        }
        let mut result = Self {
            order: p.order,
            ..Self::default()
        };
        for n in 1..=p.order {
            for j in 0..=n {
                let coeff = q.p[Self::index(n - j, j)];
                if coeff == (0.0, 0.0) {
                    continue;
                }
                for degree in 0..=p.order - j {
                    for k in 0..=degree {
                        let at = Self::index(degree - k, k + j);
                        result.p[at] = Self::sum(
                            result.p[at],
                            Self::mul(coeff, powers[n - j].p[Self::index(degree - k, k)]),
                            (0.0, 0.0),
                        );
                    }
                }
            }
        }
        result.p[1] = (0.0, 0.0);
        result.p[2] = (0.0, 0.0);
        result
    }
    /// Positive-coefficient majorant of the terms discarded by composition. A
    /// scalar t counts total degree; evaluating the tail at t=1 bounds every
    /// degree > order term over |delta| <= radius and |dc| <= dc_max.
    fn tail(mut p: Self, mut q: Self, x: &Block, y: &Block, radius: f64, dc_max: f64) -> f64 {
        p.p[1] = x.a;
        p.p[2] = x.b;
        q.p[1] = y.a;
        q.p[2] = y.b;
        let mut weights = [0.0; 5];
        for n in 1..=p.order {
            for j in 0..=n {
                let a = p.p[Self::index(n - j, j)];
                weights[n] += a.0.hypot(a.1) * radius.powi((n - j) as i32) * dc_max.powi(j as i32);
            }
        }
        let mut powers = [[0.0; 17]; 5];
        powers[0][0] = 1.0;
        for i in 1..=p.order {
            for n in 0..=p.order * (i - 1) {
                for j in 1..=p.order {
                    powers[i][n + j] += powers[i - 1][n] * weights[j];
                }
            }
        }
        let mut tail = 0.0;
        for n in 1..=q.order {
            for j in 0..=n {
                let a = q.p[Self::index(n - j, j)];
                let w = a.0.hypot(a.1) * dc_max.powi(j as i32);
                for k in 0..=p.order * (n - j) {
                    if k + j > p.order {
                        tail += w * powers[n - j][k];
                    }
                }
            }
        }
        tail
    }
    fn norms(mut p: Self, x: &Block, radius: f64, dc_max: f64) -> (f64, f64) {
        p.p[1] = x.a;
        p.p[2] = x.b;
        let (mut state, mut parameter) = (0.0, 0.0);
        for n in 1..=p.order {
            for j in 0..=n {
                let a = p.p[Self::index(n - j, j)];
                let w = a.0.hypot(a.1) * radius.powi((n - j) as i32) * dc_max.powi(j as i32);
                if n > j {
                    state += w;
                } else {
                    parameter += w;
                }
            }
        }
        (state, parameter)
    }
    fn slope(mut q: Self, y: &Block, radius: f64, dc_max: f64) -> f64 {
        q.p[1] = y.a;
        q.p[2] = y.b;
        let mut slope = 0.0;
        for n in 1..=q.order {
            for j in 0..n {
                let a = q.p[Self::index(n - j, j)];
                slope += (n - j) as f64
                    * a.0.hypot(a.1)
                    * radius.powi((n - j - 1) as i32)
                    * dc_max.powi(j as i32);
            }
        }
        slope
    }
    /// Research-only alternative to the linear radius: exact quadratic leaves,
    /// composed Taylor tail bounds, and explicit decline when the error budget or
    /// the second half's input domain cannot be satisfied.
    fn guarded(
        x: &Block,
        y: &Block,
        p: Self,
        q: Self,
        dc: f64,
        eps: f64,
        len: usize,
    ) -> (Block, Self) {
        if x.r == 0.0 || y.r == 0.0 {
            return (
                Block::default(),
                Self {
                    order: p.order,
                    ..Self::default()
                },
            );
        }
        let poly = Self::merge(x, y, p, q);
        let mut b = Block {
            a: Self::mul(y.a, x.a),
            b: Self::sum(Self::mul(y.a, x.b), y.b, (0.0, 0.0)),
            ..Block::default()
        };
        let pure_tail = Self::tail(p, q, x, y, 0.0, dc);
        let mut radius = x.r;
        for _ in 0..64 {
            let (nd, nc) = Self::norms(p, x, radius, dc);
            let ex = x.alpha * radius + x.beta * dc;
            if nd + nc + ex > y.r {
                radius *= 0.5;
                continue;
            }
            let slope = Self::slope(q, y, nd + nc + ex, dc);
            let tail = Self::tail(p, q, x, y, radius, dc);
            b.alpha = (slope + y.alpha) * x.alpha + y.alpha * nd / radius + tail / radius;
            b.beta = (slope + y.alpha) * x.beta + y.alpha * nc / dc + y.beta + pure_tail / dc;
            let error = b.alpha * radius + b.beta * dc;
            let budget = eps * len as f64 * (b.a.0.hypot(b.a.1) * radius + b.b.0.hypot(b.b.1) * dc);
            if error.is_finite() && budget.is_finite() && error <= budget {
                b.r = radius;
                return (b, poly);
            }
            if pure_tail > budget || !tail.is_finite() {
                radius *= 0.1;
            } else {
                radius *= (budget / error).powf(0.2).clamp(0.1, 0.5);
            }
        }
        (
            Block::default(),
            Self {
                order: p.order,
                ..Self::default()
            },
        )
    }
    pub(crate) fn eval(
        self,
        x: (f64, f64),
        dc: (f64, f64),
    ) -> ((f64, f64), (f64, f64), (f64, f64)) {
        let (mut value, mut dx, mut dy) = ((0.0, 0.0), (0.0, 0.0), (0.0, 0.0));
        for i in (0..=self.order).rev() {
            let (mut row, mut drow) = ((0.0, 0.0), (0.0, 0.0));
            for j in (0..=self.order - i).rev() {
                drow = Self::sum(Self::mul(drow, dc), row, (0.0, 0.0));
                row = Self::sum(Self::mul(row, dc), self.p[Self::index(i, j)], (0.0, 0.0));
            }
            dx = Self::sum(Self::mul(dx, x), value, (0.0, 0.0));
            dy = Self::sum(Self::mul(dy, x), drow, (0.0, 0.0));
            value = Self::sum(Self::mul(value, x), row, (0.0, 0.0));
        }
        (value, dx, dy)
    }
}

impl Bla {
    /// Build the table over `r` for samples with `|dc| <= dc_max`.
    pub fn build(r: &Reference, eps: f64, dc_max: f64) -> Result<Bla, String> {
        check(eps, dc_max)?;
        let steps = r.len().saturating_sub(2);
        let merge = |l: &[Block]| -> Vec<Block> {
            (0..l.len() / 2)
                .map(|i| Block::merge(&l[2 * i], &l[2 * i + 1], dc_max))
                .collect()
        };
        let singles: Vec<Block> = (1..=steps)
            .map(|k| Block::single(r.re[k], r.im[k], eps))
            .collect();
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
                return Err(format!(
                    "BLA level {j} has {} blocks; a {points}-point orbit needs {}",
                    l.len(),
                    steps >> j
                ));
            }
            if !l.iter().flat_map(Block::to_array).all(|v| v.is_finite())
                || l.iter().any(|b| b.r < 0.0)
            {
                return Err(format!(
                    "BLA level {j} holds a non-finite value or negative radius"
                ));
            }
        }
        let r2 = levels
            .iter()
            .map(|l| l.iter().map(|b| b.r * b.r).collect())
            .collect();
        Ok(Bla {
            eps,
            dc_max,
            points,
            levels,
            r2,
            min_level: 0,
            #[cfg(test)]
            quadratic: Vec::new(),
        })
    }

    /// Blocks stored, and how many of them are valid somewhere.
    pub fn counts(&self) -> (usize, usize) {
        let all = self.levels.iter().map(Vec::len).sum();
        (
            all,
            self.levels.iter().flatten().filter(|b| b.r > 0.0).count(),
        )
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
        let ok = |i: usize| {
            (2u64 << i) <= budget && self.r2[i].get(k >> (i + 1)).is_some_and(|&r2| d2 < r2)
        };
        let minimum = self.min_level;
        if top <= minimum || !ok(minimum) {
            return None;
        }
        let mut i = minimum;
        while i + 1 < top && ok(i + 1) {
            i += 1;
        }
        Some((&self.levels[i][k >> (i + 1)], 2 << i))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FIX-09 seconds-scale experiment. Deliberately test-only: relaxed radii do not
    /// satisfy the production periodicity contract and must earn whole-frame gates.
    #[test]
    #[ignore]
    fn la_tolerance_probe() {
        use crate::grid::{bla_dc_max, reference, render_bla, render_series, render_with, Params};
        use crate::series::{Series, SeriesConfig};
        use fd_samples::{Column, ColumnSet, View};
        use std::time::Instant;
        let frames = include_str!("../../../docs/2026-10-20/series-frames/results/frames.txt");
        let size: u32 = std::env::var("FD_PROBE_WIDTH")
            .unwrap_or("256".into())
            .parse()
            .unwrap();
        let mut p = Params {
            nx: size,
            ny: size * 9 / 16,
            ss: 1,
            max_iter: 100_000,
            escape_radius: 1e10,
            columns: ColumnSet::of(&[Column::Class, Column::Nu, Column::De, Column::Normal]),
            threads: 8,
            tier: None,
        };
        p.threads = std::env::var("FD_PROBE_THREADS")
            .unwrap_or("8".into())
            .parse()
            .unwrap();
        let report = std::env::var_os("FD_PROBE_REPORT_DIR").map(std::path::PathBuf::from);
        if let Some(dir) = &report {
            for arm in ["plain", "global", "tiled4"] {
                std::fs::create_dir_all(dir.join(arm)).unwrap();
            }
        }
        for (index, line) in frames
            .lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .enumerate()
        {
            if (size < 480 || std::env::var_os("FD_PROBE_SAMPLED").is_some()) && index % 5 != 0 {
                continue;
            }
            let v: Vec<_> = line.split_whitespace().collect();
            let view = View {
                center_re: v[0].into(),
                center_im: v[1].into(),
                width: v[2].into(),
                rotation: 0.0,
            };
            let t = Instant::now();
            let orbit = reference(&view, &p).unwrap();
            let reference_s = t.elapsed().as_secs_f64();
            let t = Instant::now();
            let (plain_header, plain, _) = render_with(&view, &p, Some(orbit.clone())).unwrap();
            let plain_s = t.elapsed().as_secs_f64() + reference_s;
            if let Some(dir) = &report {
                fd_samples::write(
                    std::fs::File::create(dir.join("plain").join(format!("frame-{index:05}.fds")))
                        .unwrap(),
                    &plain_header,
                    &plain,
                )
                .unwrap();
            }
            let dc = bla_dc_max(&view, &p).unwrap();
            // Increasing only max_order cannot recover coefficients discarded
            // before growth. Compare early/fixed orders against the shipped
            // growing-order initializer, charging all setup to each candidate.
            for (eps, minimum, order, prefix, series_order, start_order) in [
                (EPS_MAX, 0, 0, false, 64, 8),
                (EPS_MAX, 0, 0, true, 64, 8),
                (EPS_MAX, 0, 0, true, 2, 8),
                (EPS_MAX, 0, 0, true, 4, 8),
                (EPS_MAX, 1, 0, true, 4, 8),
                (EPS_MAX, 1, 0, true, 8, 8),
            ] {
                if std::env::var_os("FD_PROBE_PROMOTE").is_some()
                    && !(prefix && minimum == 0 && (series_order == 64 || series_order == 4))
                {
                    continue;
                }
                let t = Instant::now();
                let series = prefix.then(|| {
                    if series_order <= 8 {
                        return Series::tiled(
                            &orbit.0,
                            view.width.parse().unwrap(),
                            p.nx,
                            p.ny,
                            p.max_iter,
                            series_order as u32,
                        );
                    }
                    let s = Series::build(
                        &orbit.0,
                        dc,
                        view.width.parse::<f64>().unwrap() / f64::from(p.nx),
                        p.max_iter,
                        SeriesConfig {
                            start_order,
                            max_order: 64,
                            ..SeriesConfig::default()
                        },
                    );
                    s
                });
                let no_table = prefix && minimum == 0;
                let singles: Vec<_> = (1..if no_table { 1 } else { orbit.0.len() - 1 })
                    .map(|k| {
                        if order == 0 {
                            Block::single(orbit.0.re[k], orbit.0.im[k], eps)
                        } else if orbit.0.re[k].hypot(orbit.0.im[k]) <= 2.0 {
                            Block {
                                a: (2.0 * orbit.0.re[k], 2.0 * orbit.0.im[k]),
                                b: (1.0, 0.0),
                                r: 0.125,
                                ..Block::default()
                            }
                        } else {
                            Block::default()
                        }
                    })
                    .collect();
                let merge = |l: &[Block]| -> Vec<Block> {
                    l.chunks_exact(2)
                        .map(|b| Block::merge(&b[0], &b[1], dc))
                        .collect()
                };
                let mut next = merge(&singles);
                let mut levels = Vec::new();
                let mut q_levels = Vec::new();
                let mut prev = singles;
                let mut q_prev = if order > 0 {
                    vec![Quadratic::single(order); prev.len()]
                } else {
                    Vec::new()
                };
                let mut block_len = 2;
                while next.iter().any(|b| b.r > 0.0) {
                    if order > 0 {
                        let pairs: Vec<_> = (0..prev.len() / 2)
                            .map(|i| {
                                Quadratic::guarded(
                                    &prev[2 * i],
                                    &prev[2 * i + 1],
                                    q_prev[2 * i],
                                    q_prev[2 * i + 1],
                                    dc,
                                    eps,
                                    block_len,
                                )
                            })
                            .collect();
                        next = pairs.iter().map(|p| p.0).collect();
                        let q: Vec<_> = pairs.into_iter().map(|p| p.1).collect();
                        q_prev = q.clone();
                        q_levels.push(q);
                    }
                    let up = merge(&next);
                    prev = next.clone();
                    levels.push(std::mem::replace(&mut next, up));
                    block_len *= 2;
                }
                let table = Bla {
                    min_level: minimum,
                    quadratic: q_levels,
                    eps,
                    dc_max: dc,
                    points: orbit.0.len() as u64,
                    r2: levels
                        .iter()
                        .map(|l| l.iter().map(|b| b.r * b.r).collect())
                        .collect(),
                    levels,
                };
                let prepared_s = t.elapsed().as_secs_f64();
                let (header, accelerated, stats, bla) = match series {
                    Some(ref s) => render_series(
                        &view,
                        &p,
                        orbit.clone(),
                        s,
                        if no_table { None } else { Some(&table) },
                    ),
                    None => render_bla(&view, &p, orbit.clone(), &table),
                }
                .unwrap();
                let la_s = t.elapsed().as_secs_f64() + reference_s;
                let render_s = la_s - reference_s - prepared_s;
                if let Some(dir) = &report {
                    let arm = match (prefix, minimum, series_order) {
                        (true, 0, 64) => Some("global"),
                        (true, 0, 4) => Some("tiled4"),
                        _ => None,
                    };
                    if let Some(arm) = arm {
                        fd_samples::write(
                            std::fs::File::create(
                                dir.join(arm).join(format!("frame-{index:05}.fds")),
                            )
                            .unwrap(),
                            &header,
                            &accelerated,
                        )
                        .unwrap();
                    }
                }
                let mismatches = plain
                    .class
                    .iter()
                    .zip(&accelerated.class)
                    .filter(|(a, b)| a != b)
                    .count();
                let mut nu_max = 0.0f64;
                let mut de_max = 0.0f64;
                let mut non_finite = 0;
                for i in 0..plain.class.len() {
                    if !plain.class[i].escaped() || !accelerated.class[i].escaped() {
                        continue;
                    }
                    let a = plain.nu.as_ref().unwrap()[i];
                    let b = accelerated.nu.as_ref().unwrap()[i];
                    if !a.is_finite()
                        || !b.is_finite()
                        || !accelerated.de.as_ref().unwrap()[i].is_finite()
                    {
                        non_finite += 1;
                    }
                    if a.is_finite() && b.is_finite() {
                        nu_max = nu_max.max(
                            (a - b).abs()
                                * f64::from(plain.de.as_ref().unwrap()[i])
                                * std::f64::consts::LN_2
                                / 2.0,
                        );
                    }
                    let a = plain.de.as_ref().unwrap()[i] as f64;
                    let b = accelerated.de.as_ref().unwrap()[i] as f64;
                    if a > 1e-3 && a.is_finite() && b.is_finite() {
                        de_max = de_max.max((a - b).abs() / a);
                    }
                }
                println!("frame={index} width={} eps={eps:e} min={} order={order} prefix={prefix} series_order={series_order} start_order={start_order} series_n={} plain={plain_s:.6} la={la_s:.6} prepare={prepared_s:.6} render={render_s:.6} speed={:.3} class={mismatches} nonfinite={non_finite} nu_px={nu_max:e} de={de_max:e} blocks={} iterations={}", view.width, 2 << minimum, series.as_ref().map_or(0,|s| s.n), plain_s/la_s, bla.blocks, stats.iterations);
            }
        }
    }

    /// Plain perturbation steps from `m`, in f64 (rounding is far below the bounds).
    fn steps(r: &Reference, m: usize, l: usize, d: (f64, f64), dc: (f64, f64)) -> (f64, f64) {
        let (mut x, mut y) = d;
        for k in m..m + l {
            let (zr, zi) = (r.re[k], r.im[k]);
            (x, y) = (
                2.0 * (zr * x - zi * y) + x * x - y * y + dc.0,
                2.0 * (zr * y + zi * x + x * y) + dc.1,
            );
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
        assert!(
            r.len() == 5001 && t.levels.len() > 3,
            "points {} levels {}",
            r.len(),
            t.levels.len()
        );
        let mut checked = 0;
        for (j, level) in t.levels.iter().enumerate() {
            for (i, b) in level
                .iter()
                .enumerate()
                .step_by(3)
                .filter(|(_, b)| b.r > 0.0)
            {
                let (m, l) = (1 + (i << (j + 1)), 2usize << j);
                for (u, v, w) in [(0.9, 0.3, 1.0), (-0.5, 0.8, -0.7), (0.0, -0.99, 0.2)] {
                    let d = (u * b.r, v * b.r * 0.1);
                    let dc = (w * c * 0.7, 0.7 * c * 0.7);
                    let got = steps(&r, m, l, d, dc);
                    let lin = (
                        b.a.0 * d.0 - b.a.1 * d.1 + b.b.0 * dc.0 - b.b.1 * dc.1,
                        b.a.0 * d.1 + b.a.1 * d.0 + b.b.0 * dc.1 + b.b.1 * dc.0,
                    );
                    let err = (got.0 - lin.0).hypot(got.1 - lin.1);
                    let bound = b.alpha * d.0.hypot(d.1) + b.beta * dc.0.hypot(dc.1);
                    let round = 8.0
                        * l as f64
                        * f64::EPSILON
                        * (b.a.0.hypot(b.a.1) * d.0.hypot(d.1)
                            + b.b.0.hypot(b.b.1) * dc.0.hypot(dc.1));
                    assert!(
                        err <= bound + round,
                        "level {j} block {i}: err {err} > bound {bound}"
                    );
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
            for (d2, budget) in [
                (1e-40, u64::MAX),
                (1e-31, u64::MAX),
                (1e-34, 5),
                (1e-36, 2),
                (1e-40, 1),
            ] {
                let k = m.wrapping_sub(1);
                let want = (0..t.levels.len())
                    .take_while(|&i| m > 0 && k % (2 << i) == 0 && (2u64 << i) <= budget)
                    .map(|i| (i, t.levels[i].get(k >> (i + 1))))
                    .take_while(|(_, b)| b.is_some_and(|b| d2 < b.r * b.r))
                    .last()
                    .map(|(i, b)| (*b.unwrap(), 2usize << i));
                assert_eq!(
                    t.find(m, d2, budget).map(|(b, l)| (*b, l)),
                    want,
                    "m {m} d2 {d2} budget {budget}"
                );
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
