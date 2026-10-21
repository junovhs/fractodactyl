//! The zone's tail patch atlas (PROB-09, PROB-34): after the Koenigs jump every sample
//! sits at z = A + psi(w), w on the fundamental ring R0/|rho| <= |w| < R0, and the rest
//! of its orbit is F_n(w) = f_C^n(A + psi(w)), entire in w. A quadtree over s = log w
//! keeps, per leaf, a degree-D Taylor polynomial of F_n in t = (s - centre)/r for the
//! largest n at which it matches F_n to TOL. This is tail_patches.py ported operation
//! for operation in binary64 (the Python build is numpy doubles), so the same inputs
//! give the same tree.
use std::f64::consts::{PI, SQRT_2};

/// Taylor degree + 1 (tail_patches.py `D`).
pub(crate) const D: usize = 16;
/// Fit tolerance: an s-shift, |dz| <= TOL |dF/ds| ("rel" mode).
const TOL: f64 = 1e-11;
/// Split a square while its pixels' escape exceeds its n by more than GAIN steps.
const GAIN: i64 = 6;
/// Outer radius of the fundamental ring.
const R0: f64 = 0.03;
/// Cauchy circle samples.
const M: usize = 64;
const ESC2: f64 = 4.0;
const MAXN: usize = 6000;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct C64(pub(crate) f64, pub(crate) f64);

impl C64 {
    fn add(self, o: C64) -> C64 {
        C64(self.0 + o.0, self.1 + o.1)
    }
    fn sub(self, o: C64) -> C64 {
        C64(self.0 - o.0, self.1 - o.1)
    }
    fn mul(self, o: C64) -> C64 {
        C64(self.0 * o.0 - self.1 * o.1, self.0 * o.1 + self.1 * o.0)
    }
    fn scale(self, s: f64) -> C64 {
        C64(self.0 * s, self.1 * s)
    }
    fn abs(self) -> f64 {
        self.0.hypot(self.1)
    }
    fn norm2(self) -> f64 {
        self.0 * self.0 + self.1 * self.1
    }
    fn exp(self) -> C64 {
        let e = self.0.exp();
        C64(e * self.1.cos(), e * self.1.sin())
    }
    /// `exp(i theta)`.
    fn cis(theta: f64) -> C64 {
        C64(theta.cos(), theta.sin())
    }
}

/// One leaf: centre and half-width in s, the steps n it covers (-1: no fit, finish
/// plainly) and the Taylor coefficients in t.
pub(crate) struct Leaf {
    pub(crate) centre: C64,
    pub(crate) r: f64,
    pub(crate) n: i64,
    pub(crate) coef: [C64; D],
}

/// `(centre, first child or -1, leaf index or -1)`, as tail_patches.py's node rows.
pub(crate) struct Node {
    pub(crate) centre: C64,
    pub(crate) kid: i64,
    pub(crate) leaf: i64,
}

pub(crate) struct Atlas {
    pub(crate) s0: f64,
    pub(crate) sl: f64,
    pub(crate) nx: usize,
    pub(crate) nodes: Vec<Node>,
    pub(crate) leaves: Vec<Leaf>,
}

struct Chart {
    a: C64,
    c: C64,
    /// psi_1..psi_k.
    psi: Vec<C64>,
}

impl Chart {
    /// z = A + psi(e^s), psi by Horner from the top term.
    fn start(&self, s: C64) -> C64 {
        let w = s.exp();
        let mut h = C64::default();
        for c in self.psi.iter().rev() {
            h = h.add(*c).mul(w);
        }
        self.a.add(h)
    }

    /// Steps until |z|^2 > ESC2, at most `cap`.
    fn escape_steps(&self, mut z: C64, cap: usize) -> i64 {
        for n in 0..cap {
            z = z.mul(z).add(self.c);
            if z.norm2() > ESC2 {
                return n as i64 + 1;
            }
        }
        cap as i64
    }
}

/// For each square (centre, half-width r): the largest n whose degree-D Taylor of F_n
/// in t fits to TOL on the covering disk |t| <= sqrt 2, its coefficients, and the
/// earliest escape among the check points.
fn fit(ch: &Chart, centres: &[C64], r: f64) -> Vec<(i64, [C64; D], i64)> {
    let th: Vec<C64> = (0..M)
        .map(|k| C64::cis(2.0 * PI * k as f64 / M as f64))
        .collect();
    let mut chk_t: Vec<C64> = (0..12)
        .map(|k| C64::cis(2.0 * PI * (k as f64 + 0.5) / 12.0).scale(SQRT_2))
        .collect();
    chk_t.extend(th.iter().step_by(8).map(|t| t.scale(0.7)));
    chk_t.push(C64::default());
    // tpow[k][j] = chk_t[k]^j.
    let tpow: Vec<[C64; D]> = chk_t
        .iter()
        .map(|t| {
            let mut p = [C64(1.0, 0.0); D];
            for j in 1..D {
                p[j] = p[j - 1].mul(*t);
            }
            p
        })
        .collect();
    // DFT twiddles: coefficient j of (t/1.5)^j is (1/M) sum_m zr_m e^{-2 pi i m j / M}.
    let tw: Vec<C64> = (0..M)
        .map(|m| C64::cis(-2.0 * PI * m as f64 / M as f64))
        .collect();
    let pow15: Vec<f64> = (0..D).map(|j| 1.5f64.powi(j as i32)).collect();
    centres
        .iter()
        .map(|&cen| {
            let mut zr: Vec<C64> = th
                .iter()
                .map(|t| ch.start(cen.add(t.scale(1.5 * r))))
                .collect();
            let mut zc: Vec<C64> = chk_t
                .iter()
                .map(|t| ch.start(cen.add(t.scale(r))))
                .collect();
            let (mut best_n, mut best_c) = (-1i64, [C64::default(); D]);
            for n in 0..MAXN {
                let mut cf = [C64::default(); D];
                for (j, c) in cf.iter_mut().enumerate() {
                    let mut acc = C64::default();
                    for (m, z) in zr.iter().enumerate() {
                        acc = acc.add(z.mul(tw[(m * j) % M]));
                    }
                    let c1 = acc.scale(1.0 / M as f64);
                    *c = C64(c1.0 / pow15[j], c1.1 / pow15[j]);
                }
                let mut err = 0.0f64;
                for (k, p) in tpow.iter().enumerate() {
                    let mut approx = C64::default();
                    for j in 0..D {
                        approx = approx.add(cf[j].mul(p[j]));
                    }
                    err = err.max(approx.sub(zc[k]).abs());
                }
                let big = zr.iter().any(|z| z.abs() > 2.0);
                let scale = cf[1].abs() / r;
                // As numpy's `err <= TOL*scale`: a NaN error fails the fit.
                let fits = err <= TOL * scale;
                if !fits || big {
                    break;
                }
                best_n = n as i64;
                best_c = cf;
                for z in zr.iter_mut().chain(zc.iter_mut()) {
                    *z = z.mul(*z).add(ch.c);
                }
            }
            let min_t = chk_t
                .iter()
                .map(|t| ch.escape_steps(ch.start(cen.add(t.scale(r))), MAXN))
                .min()
                .unwrap_or(0);
            (best_n, best_c, min_t)
        })
        .collect()
}

/// The atlas for the cycle point `a`, multiplier `rho`, `psi` (psi_1..) and the
/// parameter `c`, to `max_depth` levels (frozen research defaults: 6).
pub(crate) fn build(a: C64, rho: C64, psi: &[C64], c: C64, max_depth: usize) -> Atlas {
    let ch = Chart {
        a,
        c,
        psi: psi.to_vec(),
    };
    let s0 = (R0 / rho.abs()).ln();
    let sl = rho.abs().ln();
    let nx = ((2.0 * PI / sl).round_ties_even() as usize).max(1);
    let r0 = sl / 2.0;
    let rr = r0.max(PI / nx as f64);
    let mut nodes: Vec<Node> = (0..nx)
        .map(|k| Node {
            centre: C64(s0 + r0, (k as f64 + 0.5) * (2.0 * PI / nx as f64)),
            kid: -1,
            leaf: -1,
        })
        .collect();
    let mut leaves = Vec::new();
    let mut level: Option<(Vec<usize>, Vec<C64>, f64, usize)> = Some((
        (0..nx).collect(),
        nodes.iter().map(|n| n.centre).collect(),
        rr,
        0,
    ));
    while let Some((ids, cen, r, d)) = level.take() {
        let fits = fit(&ch, &cen, r);
        let split: Vec<bool> = fits
            .iter()
            .map(|&(n, _, min_t)| min_t - n > GAIN && d < max_depth)
            .collect();
        for (i, &(n, coef, _)) in fits.iter().enumerate() {
            if !split[i] {
                nodes[ids[i]].leaf = leaves.len() as i64;
                leaves.push(Leaf {
                    centre: cen[i],
                    r,
                    n,
                    coef,
                });
            }
        }
        if split.iter().any(|&s| s) {
            let q = r / 2.0;
            let (mut kid_ids, mut kid_cen) = (Vec::new(), Vec::new());
            for i in (0..ids.len()).filter(|&i| split[i]) {
                nodes[ids[i]].kid = nodes.len() as i64;
                for sx in [-1.0, 1.0] {
                    for sy in [-1.0, 1.0] {
                        let k = cen[i].add(C64(q * sx, q * sy));
                        kid_ids.push(nodes.len());
                        kid_cen.push(k);
                        nodes.push(Node {
                            centre: k,
                            kid: -1,
                            leaf: -1,
                        });
                    }
                }
            }
            level = Some((kid_ids, kid_cen, q, d + 1));
        }
    }
    Atlas {
        s0,
        sl,
        nx,
        nodes,
        leaves,
    }
}

impl Atlas {
    /// The `patch_root`, `node` and `leaf` lines (tail_patches.py `export`).
    pub(crate) fn lines(&self) -> String {
        use std::fmt::Write as _;
        let mut o = String::new();
        let _ = writeln!(o, "patch_root {:?} {:?} {} {D}", self.s0, self.sl, self.nx);
        for n in &self.nodes {
            let _ = writeln!(
                o,
                "node {:?} {:?} {} {}",
                n.centre.0, n.centre.1, n.kid, n.leaf
            );
        }
        for l in &self.leaves {
            let _ = write!(
                o,
                "leaf {:?} {:?} {:?} {}",
                l.centre.0, l.centre.1, l.r, l.n
            );
            for c in &l.coef {
                let _ = write!(o, " {:?} {:?}", c.0, c.1);
            }
            o.push('\n');
        }
        o
    }
}
