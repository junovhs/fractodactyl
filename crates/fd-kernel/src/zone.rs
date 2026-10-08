//! Minibrot-band fast path (KERN-01, research write-up
//! docs/research/10-8-26/misiurewicz-frame-transfer.md, PROB-07/08/09).
//!
//! A zone is a period-`P` minibrot nucleus `C` parked near a Misiurewicz point whose
//! repelling 2-cycle (point `A`, multiplier `rho`) the orbit circles many times. For a
//! pixel `c` close enough to `C`, its orbit is:
//! 1. returns to `C` every `P` steps: one degree-4 biseries map per return (in
//!    `u = (z - C) / scale` and `v = (c - C) / scale`), while `|z - C| <= guard`;
//! 2. 23 steps against `C`'s critical orbit, landing near `A`;
//! 3. the spiral out along the 2-cycle in one Koenigs jump: `w = phi(z - A)`,
//!    `w rho^j`, back with `psi = phi^-1`;
//! 4. a tail patch (Taylor polynomial of `f_C^m(A + psi(w))` in `log w`) and the last
//!    plain steps at the fixed parameter `C`.
//!
//! Every pixel is ordinary f64; the constants (one zone file, written by
//! tools/research/misiurewicz/koenigs_bench_consts.py and tail_patches.py) are computed
//! once at high precision. `dz/dc` is carried through every stage by the chain rule, so
//! `de` and `normal` columns come out as from the perturbation kernel.
//!
//! Contract (DEC-10): **validity** is frame-level: every sample within `max_dc` of `C`
//! (default the loop guard, 1e-28; whole frames at 1080p were checked against per-frame
//! BLA down to 2e-48: METHOD.md PROB-09). **Error**: measured, not bounded (PROB-13 is
//! the certification). **Fallback**: a frame outside the zone is not rendered here
//! ([`zone_covers`] is false); the caller renders it with the perturbation kernel.
use crate::grid::{header, setup, Params, Stats};
use crate::sample::Outcome;
use crate::store::{Row, Store};
use fd_fixed::Fixed;
use fd_samples::{Column, Header, Samples, View};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Tail patch polynomial length (tail_patches.py `D`).
const PATCH_D: usize = 16;
/// Critical-orbit steps from a return to the Koenigs chart (the zone's `q - 1`).
const APPROACH: usize = 23;

#[derive(Clone, Copy, Default, Debug, PartialEq)]
struct Cx(f64, f64);

impl Cx {
    #[inline(always)]
    fn add(self, o: Cx) -> Cx {
        Cx(self.0 + o.0, self.1 + o.1)
    }
    #[inline(always)]
    fn mul(self, o: Cx) -> Cx {
        Cx(self.0 * o.0 - self.1 * o.1, self.0 * o.1 + self.1 * o.0)
    }
    #[inline(always)]
    fn scale(self, s: f64) -> Cx {
        Cx(self.0 * s, self.1 * s)
    }
    #[inline(always)]
    fn norm2(self) -> f64 {
        self.0 * self.0 + self.1 * self.1
    }
    #[inline(always)]
    fn abs(self) -> f64 {
        self.0.hypot(self.1)
    }
    #[inline(always)]
    fn div(self, o: Cx) -> Cx {
        let d = o.norm2();
        Cx((self.0 * o.0 + self.1 * o.1) / d, (self.1 * o.0 - self.0 * o.1) / d)
    }
}

/// One tail patch: centre and half-width in `s = log w`, the steps `m` it covers (-1:
/// invalid leaf, finish plainly), and the Taylor coefficients in `t = (s - centre) / r`.
#[derive(Clone, Debug)]
struct Leaf {
    centre: Cx,
    r: f64,
    m: i64,
    coef: [Cx; PATCH_D],
}

/// Per-zone constants (a zone file).
#[derive(Clone, Debug)]
pub struct Zone {
    /// The nucleus `C` as exact decimals (`c_exact`).
    c_re: String,
    c_im: String,
    c: Cx,
    /// Minibrot period `P`.
    pub period: u64,
    scale: f64,
    guard: f64,
    /// Largest `|c - C|` over a frame's samples for which the zone renders it.
    pub max_dc: f64,
    r0: f64,
    bias: Cx,
    alpha: Cx,
    rho: Cx,
    z24ma: Cx,
    orbit: Vec<Cx>,
    deg: usize,
    /// `bis[i][j]`: coefficient of `u^i v^j`.
    bis: Vec<Vec<Cx>>,
    /// `phi[k - 1]`: coefficient of `h^k`.
    phi: Vec<Cx>,
    /// `psi[k - 1]`: coefficient of `w^k`, `psi = phi^-1`.
    psi: Vec<Cx>,
    patch_nx: usize,
    /// (centre, first child or -1, leaf index or -1)
    nodes: Vec<(Cx, i64, i64)>,
    leaves: Vec<Leaf>,
}

impl Zone {
    /// Read a zone file.
    pub fn load(path: &str) -> Result<Zone, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        Zone::parse(&text).map_err(|e| format!("{path}: {e}"))
    }

    /// Parse zone-file text: one constant per line (`name values...`), `#` comments.
    pub fn parse(text: &str) -> Result<Zone, String> {
        let mut z = Zone {
            c_re: String::new(),
            c_im: String::new(),
            c: Cx::default(),
            period: 0,
            scale: 0.0,
            guard: 0.0,
            max_dc: 0.0,
            r0: 0.0,
            bias: Cx::default(),
            alpha: Cx::default(),
            rho: Cx::default(),
            z24ma: Cx::default(),
            orbit: Vec::new(),
            deg: 0,
            bis: Vec::new(),
            phi: Vec::new(),
            psi: Vec::new(),
            patch_nx: 0,
            nodes: Vec::new(),
            leaves: Vec::new(),
        };
        let mut raw = vec![];
        for (ln, line) in text.lines().enumerate().filter(|(_, l)| !l.starts_with('#') && !l.trim().is_empty()) {
            let f: Vec<&str> = line.split_whitespace().collect();
            let bad = |what: &str| format!("line {}: {what}", ln + 1);
            let n = |i: usize| -> Result<f64, String> {
                f.get(i).ok_or_else(|| bad("missing value"))?.parse::<f64>().map_err(|_| bad("bad number"))
            };
            let cx = |i: usize| -> Result<Cx, String> { Ok(Cx(n(i)?, n(i + 1)?)) };
            match f[0] {
                "c_exact" => {
                    let (re, im) = (f.get(1).ok_or_else(|| bad("missing value"))?, f.get(2).ok_or_else(|| bad("missing value"))?);
                    (z.c_re, z.c_im) = (re.to_string(), im.to_string());
                }
                "c" => z.c = cx(1)?,
                "period" => z.period = n(1)? as u64,
                "scale" => z.scale = n(1)?,
                "guard" => z.guard = n(1)?,
                "max_dc" => z.max_dc = n(1)?,
                "r0" => z.r0 = n(1)?,
                "bias" => z.bias = cx(1)?,
                "alpha" => z.alpha = cx(1)?,
                "rho" => z.rho = cx(1)?,
                "z24_minus_alpha" => z.z24ma = cx(1)?,
                "orbit" => z.orbit.push(cx(2)?),
                "biseries" => raw.push((n(1)? as usize, n(2)? as usize, cx(3)?)),
                "phi" => z.phi.push(cx(2)?),
                "psi" => z.psi.push(cx(2)?),
                "patch_root" => {
                    z.patch_nx = n(3)? as usize;
                    if n(4)? as usize != PATCH_D {
                        return Err(bad("patch degree must be 16"));
                    }
                }
                "node" => z.nodes.push((cx(1)?, n(3)? as i64, n(4)? as i64)),
                "leaf" => {
                    if f.len() != 5 + 2 * PATCH_D {
                        return Err(bad("leaf needs 16 coefficients"));
                    }
                    let mut coef = [Cx::default(); PATCH_D];
                    for (i, c) in coef.iter_mut().enumerate() {
                        *c = cx(5 + 2 * i)?;
                    }
                    z.leaves.push(Leaf { centre: cx(1)?, r: n(3)?, m: n(4)? as i64, coef });
                }
                // Lean-baseline reference orbit of the research bench: not used here.
                "ref" => {}
                other => return Err(bad(&format!("unknown constant {other:?}"))),
            }
        }
        if z.c_re.is_empty() {
            return Err("no c_exact line (the nucleus as exact decimals)".into());
        }
        if raw.is_empty() || z.period == 0 || z.scale <= 0.0 || z.guard <= 0.0 || z.r0 <= 0.0 {
            return Err("incomplete zone: needs period, scale, guard, r0 and biseries".into());
        }
        if z.orbit.len() != APPROACH {
            return Err(format!("expected {APPROACH} orbit points, got {}", z.orbit.len()));
        }
        if z.phi.is_empty() || z.psi.is_empty() {
            return Err("needs phi and psi series".into());
        }
        for &(_, kid, leaf) in &z.nodes {
            if (kid >= 0 && kid as usize + 4 > z.nodes.len()) || (kid < 0 && (leaf < 0 || leaf as usize >= z.leaves.len())) {
                return Err("malformed patch tree".into());
            }
        }
        if !z.nodes.is_empty() && (z.patch_nx == 0 || z.patch_nx > z.nodes.len()) {
            return Err("malformed patch tree root".into());
        }
        z.deg = raw.iter().map(|&(i, j, _)| i + j).max().unwrap_or(0);
        if z.deg >= 8 {
            return Err("biseries degree must be below 8".into());
        }
        z.bis = vec![vec![Cx::default(); z.deg + 1]; z.deg + 1];
        for (i, j, a) in raw {
            z.bis[i][j] = a;
        }
        if z.max_dc == 0.0 {
            z.max_dc = z.guard;
        }
        Ok(z)
    }
}

/// Work counters of one zone render.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ZoneStats {
    /// Biseries returns applied, summed over samples (each replaces `P` steps).
    pub returns: u64,
    /// Samples whose spiral was skipped by the Koenigs jump.
    pub jumps: u64,
    /// Samples that used a tail patch.
    pub patches: u64,
    /// Plain `f_C` steps after the patch, summed over samples.
    pub plain: u64,
}

/// Offset `centre - C` of `view` as f64 (exact decimals subtracted in fixed point).
fn centre_offset(view: &View, zone: &Zone) -> Result<Cx, String> {
    let l = fd_fixed::limbs_for(512);
    let d = |a: &str, b: &str| -> Result<f64, String> { Ok(Fixed::parse(a, l)?.sub(&Fixed::parse(b, l)?).to_f64()) };
    Ok(Cx(d(&view.center_re, &zone.c_re)?, d(&view.center_im, &zone.c_im)?))
}

/// Whether `zone` may render `view` (DEC-10 validity): every sample within `max_dc` of
/// the nucleus.
pub fn zone_covers(view: &View, p: &Params, zone: &Zone) -> Result<bool, String> {
    let (plane, _) = setup(view, p)?;
    let off = centre_offset(view, zone)?;
    let reach = plane.h() * (f64::from(p.nx) / 2.0).hypot(f64::from(p.ny) / 2.0);
    Ok(off.abs() + reach <= zone.max_dc)
}

/// Render `view` with the zone pipeline. Refused when the zone does not cover the view
/// (see [`zone_covers`]) or `Bound` is requested (the pipeline carries no error radii).
/// The header's kernel id is `zone-koenigs/1 P=<period>`.
pub fn render_zone(view: &View, p: &Params, zone: &Zone) -> Result<(Header, Samples, Stats, ZoneStats), String> {
    if p.columns.has(Column::Bound) {
        return Err("--columns bound: the zone pipeline carries no error radii (PROB-13)".into());
    }
    if !zone_covers(view, p, zone)? {
        return Err(format!("the zone (nucleus within {:e}) does not cover this view", zone.max_dc));
    }
    let (plane, tier) = setup(view, p)?;
    let off = centre_offset(view, zone)?;
    let h = plane.h();
    let cols = p.columns.with(Column::Class);
    let deriv = cols.needs_derivative();
    let store = Store::new(&plane, p.ss);
    let (nx, n) = (p.nx as usize, p.nx as usize * p.ny as usize);
    let r2 = p.escape_radius * p.escape_radius;
    let mut s = Samples::alloc(n, cols);
    let work = AtomicU64::new(0);
    let total = Mutex::new(ZoneStats::default());
    {
        let queue = Mutex::new(Row::split(&mut s, nx).into_iter());
        std::thread::scope(|sc| {
            for _ in 0..p.threads.clamp(1, p.ny as usize) {
                sc.spawn(|| {
                    let mut st = ZoneStats::default();
                    let mut its = 0u64;
                    loop {
                        let Some(mut row) = queue.lock().unwrap().next() else { break };
                        for i in 0..row.class.len() {
                            let (ux, uy) = plane.unit_offset(i, row.j);
                            let v = Cx(off.0 + ux * h, off.1 + uy * h).scale(1.0 / zone.scale);
                            let (o, w) = if deriv {
                                pixel::<true>(zone, v, p.max_iter, r2, &mut st)
                            } else {
                                pixel::<false>(zone, v, p.max_iter, r2, &mut st)
                            };
                            its += w;
                            store.put(&mut row, i, o);
                        }
                    }
                    work.fetch_add(its, Ordering::Relaxed);
                    let mut t = total.lock().unwrap();
                    t.returns += st.returns;
                    t.jumps += st.jumps;
                    t.patches += st.patches;
                    t.plain += st.plain;
                });
            }
        });
    }
    let mut hd = header(view, p, cols, &plane, tier);
    hd.kernel = format!("zone-koenigs/1 P={}", zone.period);
    let stats = Stats { reference_len: 0, reference_seconds: 0.0, iterations: work.into_inner() };
    Ok((hd, s, stats, total.into_inner().unwrap()))
}

/// One sample at `c = C + v * scale`. With `D`, `dz/dc` is carried by the chain rule.
/// Returns the outcome and the work it took (biseries returns + approach + jump +
/// patch + plain steps, each counted once).
#[inline]
fn pixel<const D: bool>(k: &Zone, v: Cx, max_iter: u64, r2: f64, st: &mut ZoneStats) -> (Outcome, u64) {
    // Biseries coefficients in u, with the v powers folded in: b_i(v) and db_i/dv.
    let mut b = [Cx::default(); 8];
    let mut bv = [Cx::default(); 8];
    for i in 0..=k.deg {
        let (mut s, mut ds) = (Cx::default(), Cx::default());
        for j in (0..=k.deg - i).rev() {
            if D {
                ds = ds.mul(v).add(s);
            }
            s = s.mul(v).add(k.bis[i][j]);
        }
        b[i] = s;
        bv[i] = ds;
    }
    b[0] = b[0].add(k.bias);
    // Stage 1: returns. u = (z_n - C) / scale, du/dv = dz/dc.
    let (mut u, mut du) = (v, Cx(1.0, 0.0));
    let mut n: u64 = 1;
    let mut work = 0u64;
    while u.abs() * k.scale <= k.guard {
        if n + k.period > max_iter {
            return (Outcome::Unresolved, work);
        }
        let (mut s, mut su, mut sv) = (b[k.deg], Cx::default(), bv[k.deg]);
        for i in (0..k.deg).rev() {
            su = su.mul(u).add(s);
            s = s.mul(u).add(b[i]);
            sv = sv.mul(u).add(bv[i]);
        }
        let step = Cx(s.0 - u.0, s.1 - u.1);
        if D {
            du = su.mul(du).add(sv);
        }
        // Interior: the return map contracts here and has settled, or (checked after 16,
        // 32, 64, ... returns) u provably lies in the basin of an attracting cycle of
        // the return map of period <= 4 (an attracting q-cycle of the minibrot's
        // return map is an attracting qP-cycle of f_c: the minibrot or one of its bulbs).
        if (su.norm2() < 1.0 && step.norm2() <= 1e-24 * u.norm2()) || (work >= 16 && work.is_power_of_two() && attracted(&b, k.deg, u)) {
            st.returns += 1;
            return (Outcome::Interior { n }, work + 1);
        }
        u = s;
        n += k.period;
        work += 1;
        st.returns += 1;
    }
    // Stage 2: approach against C's critical orbit (fixed parameter C).
    let mut d = u.scale(k.scale);
    for z in &k.orbit {
        if D {
            du = z.add(d).mul(du).scale(2.0).add(Cx(1.0, 0.0));
        }
        d = z.scale(2.0).mul(d).add(d.mul(d));
    }
    n += APPROACH as u64;
    work += APPROACH as u64;
    // Stage 3: Koenigs jump along the 2-cycle, then a tail patch.
    let h0 = k.z24ma.add(d);
    let mut z = k.alpha.add(h0);
    if h0.abs() < k.r0 {
        let (w0, dphi) = series_d(&k.phi, h0);
        let lr = k.rho.abs().ln();
        let j = ((k.r0 / w0.abs()).ln() / lr).floor();
        if let Some((zp, dzs, m)) = patch(k, w0, j) {
            // z = P(t), t = (log w - centre) / r: dz/dc = P'(t) / r * phi'(h0) / w0 * dh0/dc.
            if D {
                du = dzs.mul(dphi).mul(du).div(w0);
            }
            z = zp;
            n += 2 * j as u64 + m;
            st.jumps += 1;
            st.patches += 1;
        } else if j > 0.0 {
            let (mg, t) = ((j * lr).exp(), j * k.rho.1.atan2(k.rho.0));
            let rj = Cx(mg * t.cos(), mg * t.sin());
            let w = w0.mul(rj);
            let (hw, dpsi) = series_d(&k.psi, w);
            if D {
                du = dpsi.mul(rj).mul(dphi).mul(du);
            }
            z = k.alpha.add(hw);
            n += 2 * j as u64;
            st.jumps += 1;
        }
        work += 1;
    }
    // Stage 4: plain steps at C.
    loop {
        if n >= max_iter {
            return (Outcome::Unresolved, work);
        }
        if D {
            du = z.mul(du).scale(2.0).add(Cx(1.0, 0.0));
        }
        z = z.mul(z).add(k.c);
        n += 1;
        work += 1;
        st.plain += 1;
        if z.norm2() > r2 {
            let o = Outcome::Escaped {
                n,
                zr: z.0,
                zi: z.1,
                dr: du.0,
                di: du.1,
                dexp: 0,
                ez: f64::INFINITY,
                ed: f64::INFINITY,
            };
            return (o, work);
        }
    }
}

/// `B(u)`, `B'(u)` and `B''(u)` for `B(u) = sum b[i] u^i`, `i <= deg`.
#[inline]
fn eval2(b: &[Cx; 8], deg: usize, u: Cx) -> (Cx, Cx, Cx) {
    let (mut s, mut d, mut dd) = (b[deg], Cx::default(), Cx::default());
    for i in (0..deg).rev() {
        dd = dd.mul(u).add(d);
        d = d.mul(u).add(s);
        s = s.mul(u).add(b[i]);
    }
    (s, d, dd.scale(2.0))
}

/// `B^q(x)` with its first and second derivatives.
#[inline]
fn compose(b: &[Cx; 8], deg: usize, x: Cx, q: usize) -> (Cx, Cx, Cx) {
    let (mut y, mut d1, mut d2) = (x, Cx(1.0, 0.0), Cx::default());
    for _ in 0..q {
        let (s, d, dd) = eval2(b, deg, y);
        d2 = dd.mul(d1).mul(d1).add(d.mul(d2));
        d1 = d.mul(d1);
        y = s;
    }
    (y, d1, d2)
}

/// Whether `u` lies in the basin of an attracting cycle of the return map `B` of
/// period `q <= 4`: Newton finds `x*` with `B^q(x*) = x*` and `|(B^q)'(x*)| = l < 1`,
/// and `u` is within the disc around `x*` on which `|(B^q)'| < 1` holds to first order
/// (`|u - x*| |(B^q)''(x*)| <= (1 - l) / 2`), so iterating `B^q` from `u` converges to
/// `x*`. Heuristic evidence, like the perturbation kernel's interior tests.
fn attracted(b: &[Cx; 8], deg: usize, u: Cx) -> bool {
    for q in 1..=4 {
        let mut x = u;
        for _ in 0..16 {
            let (f, d, _) = compose(b, deg, x, q);
            let st = Cx(f.0 - x.0, f.1 - x.1).div(Cx(d.0 - 1.0, d.1));
            x = Cx(x.0 - st.0, x.1 - st.1);
            if !(x.0.is_finite() && x.1.is_finite()) {
                break;
            }
            if st.norm2() <= 1e-28 * x.norm2() {
                let (_, d, dd) = compose(b, deg, x, q);
                let l = d.abs();
                if l < 1.0 && Cx(u.0 - x.0, u.1 - x.1).abs() * dd.abs() <= 0.5 * (1.0 - l) {
                    return true;
                }
                break;
            }
        }
    }
    false
}

/// `sum c[k-1] x^k` and its derivative.
#[inline]
fn series_d(c: &[Cx], x: Cx) -> (Cx, Cx) {
    let (mut s, mut d) = (Cx::default(), Cx::default());
    for (i, a) in c.iter().enumerate().rev() {
        s = s.add(*a).mul(x);
        d = d.mul(x).add(a.scale((i + 1) as f64));
    }
    (s, d)
}

/// Tail patch for the jumped point `w0 rho^j`: `f_C^m(A + psi(w))`, its derivative in
/// `s = log w`, and `m`.
#[inline]
fn patch(k: &Zone, w0: Cx, j: f64) -> Option<(Cx, Cx, u64)> {
    if k.nodes.is_empty() || j < 0.0 {
        return None;
    }
    let tau = std::f64::consts::TAU;
    let re = w0.abs().ln() + j * k.rho.abs().ln();
    let im = (w0.1.atan2(w0.0) + j * k.rho.1.atan2(k.rho.0)).rem_euclid(tau);
    let mut node = ((im / (tau / k.patch_nx as f64)) as usize).min(k.patch_nx - 1);
    while k.nodes[node].1 >= 0 {
        let c = k.nodes[node].0;
        node = k.nodes[node].1 as usize + 2 * usize::from(re >= c.0) + usize::from(im >= c.1);
    }
    let leaf = &k.leaves[k.nodes[node].2 as usize];
    if leaf.m < 0 {
        return None;
    }
    let t = Cx((re - leaf.centre.0) / leaf.r, (im - leaf.centre.1) / leaf.r);
    let (mut z, mut dz) = (Cx::default(), Cx::default());
    for a in leaf.coef.iter().rev() {
        dz = dz.mul(t).add(z);
        z = z.mul(t).add(*a);
    }
    Some((z, dz.scale(1.0 / leaf.r), leaf.m as u64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn series_derivative_matches_finite_difference() {
        let c = [Cx(1.0, 0.0), Cx(0.3, -0.2), Cx(-0.1, 0.05)];
        let x = Cx(0.02, 0.01);
        let (_, d) = series_d(&c, x);
        let e = 1e-7;
        let (a, _) = series_d(&c, x.add(Cx(e, 0.0)));
        let (b, _) = series_d(&c, x);
        let fd = Cx((a.0 - b.0) / e, (a.1 - b.1) / e);
        assert!((fd.0 - d.0).abs() < 1e-6 && (fd.1 - d.1).abs() < 1e-6, "{fd:?} {d:?}");
    }

    /// The v0 zone without its tail patch atlas (bench/zones/v0-core.zone; the full
    /// zone file is generated by tools/research/misiurewicz/make_zone.sh).
    fn v0() -> Zone {
        Zone::parse(include_str!("../../../bench/zones/v0-core.zone")).unwrap()
    }

    fn view(width: &str) -> View {
        View {
            center_re: "-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502".into(),
            center_im: "0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922".into(),
            width: width.into(),
            rotation: 0.3,
        }
    }

    fn params(columns: &[Column]) -> Params {
        Params {
            nx: 96,
            ny: 54,
            ss: 1,
            max_iter: 20_000,
            escape_radius: 1e10,
            columns: fd_samples::ColumnSet::of(columns),
            threads: 4,
            tier: None,
        }
    }

    #[test]
    fn zone_matches_the_perturbation_kernel_on_a_deep_frame() {
        // Every sample escaped in both or in neither; nu within 1e-3 px wherever the
        // sample is not within 1e-3 px of the boundary (fd compare's metric); de within
        // 1% and the normal within 1 degree there.
        let (z, p) = (v0(), params(&[Column::Nu, Column::De, Column::Normal]));
        for w in ["1e-36", "3e-41", "2e-48"] {
            let v = view(w);
            assert!(zone_covers(&v, &p, &z).unwrap(), "{w}");
            let (_, a, _) = crate::render_with(&v, &p, None).unwrap();
            let (hb, b, _, st) = render_zone(&v, &p, &z).unwrap();
            assert!(hb.kernel.starts_with("zone-koenigs/1"), "{}", hb.kernel);
            assert!(st.jumps > 0 && st.returns > 0, "{st:?}");
            let esc = |s: &Samples, i: usize| s.class[i].kind() == Some(fd_samples::Kind::Escaped);
            let (an, bn) = (a.nu.as_ref().unwrap(), b.nu.as_ref().unwrap());
            let (ad, bd) = (a.de.as_ref().unwrap(), b.de.as_ref().unwrap());
            let (am, bm) = (a.normal.as_ref().unwrap(), b.normal.as_ref().unwrap());
            let mut checked = 0;
            for i in 0..a.class.len() {
                assert_eq!(esc(&a, i), esc(&b, i), "{w} sample {i}");
                if !esc(&a, i) || ad[i] < 1e-3 {
                    continue;
                }
                let px = (an[i] - bn[i]).abs() * f64::from(ad[i]) * std::f64::consts::LN_2 / 2.0;
                assert!(px <= 1e-3, "{w} sample {i}: {px} px");
                assert!((bd[i] / ad[i] - 1.0).abs() < 1e-2, "{w} sample {i}: de {} vs {}", bd[i], ad[i]);
                let da = am[i].wrapping_sub(bm[i]).min(bm[i].wrapping_sub(am[i]));
                assert!(f64::from(da) * 360.0 / 65536.0 < 1.0, "{w} sample {i}: normal {} vs {}", am[i], bm[i]);
                checked += 1;
            }
            assert!(checked > a.class.len() / 2, "{w}: only {checked} samples checked");
        }
    }

    #[test]
    fn zone_refuses_views_it_does_not_cover() {
        let (z, p) = (v0(), params(&[Column::Nu]));
        assert!(!zone_covers(&view("1e-20"), &p, &z).unwrap());
        assert!(render_zone(&view("1e-20"), &p, &z).is_err());
        let mut far = view("1e-40");
        far.center_re = "-0.7432918908".into();
        assert!(!zone_covers(&far, &p, &z).unwrap());
        assert!(render_zone(&view("1e-40"), &params(&[Column::Nu, Column::Bound]), &z).is_err());
    }

    #[test]
    fn parse_rejects_incomplete_zones() {
        assert!(Zone::parse("period 764\n").is_err());
        assert!(Zone::parse("c_exact -0.75 0.1\nbogus 1\n").is_err());
    }
}
