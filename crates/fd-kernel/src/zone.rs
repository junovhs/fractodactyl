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
//! A zone with `diag` lines also refuses a frame whose first-return truncation shift
//! could exceed [`SHIFT_PX`] (PROB-20): decided before rendering (DEC-19).
use crate::grid::{header, setup, Params, Stats};
use crate::number::{Complex, Cx, Fx};
use crate::sample::Outcome;
use crate::store::{Row, Store};
use crate::view::Plane;
use fd_fixed::{exp2i, Fixed};
use fd_samples::{Column, Header, Samples, View};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

mod mid;

/// Tail patch polynomial length (tail_patches.py `D`).
const PATCH_D: usize = 16;
/// Critical-orbit steps from a return to the Koenigs chart (the zone's `q - 1`).
const APPROACH: usize = 23;
/// Largest first-return truncation shift, in pixels, a covered frame may carry. A shift
/// of `s` px moves `de` by about `s` (|grad de| ~ 1), so near the scored floor
/// (`de` = 1e-3 px, `fd compare` tolerance 0.2%) the relative `de` error is ~ s / 1e-3.
/// Allowed: 2e-3 * 1e-3 / 4. The factor 4 covers the measured excess of up to 2.2x on
/// the v0 band-top frames (PROB-20).
pub const SHIFT_PX: f64 = 5e-7;

/// Representation operations used by the one zone pixel pipeline. Range handling
/// and established summation order belong to the number, not a second kernel.
trait ZoneNumber: Complex {
    const EXTENDED: bool;
    type Orbit: OrbitNumber<Self>;
    fn one() -> Self;
    fn from_parts(m: Cx, e: i64) -> Self;
    fn parts(self) -> (Cx, i64);
    fn physical(self) -> Cx;
    fn sub(self, other: Self) -> Self;
    fn div(self, other: Self) -> Self;
    fn coefficients(k: &Zone, i: usize) -> &[Self];
    fn phi(k: &Zone) -> &[Self];
    fn entry(k: &Zone) -> Self;
    fn initial_delta(self, scale: ZoneSize) -> Self;
    fn inside_guard(self, k: &Zone) -> bool;
    fn settled(self, step: Self, u: Self) -> bool;
    fn log_arg(self) -> (f64, f64);
    fn jump(self, radius: f64, lr: f64) -> f64;
    fn advance(self, j: f64, lr: f64, arg: f64) -> (Cx, Self);
}

trait OrbitNumber<N: Complex>: Copy {
    fn from_number(value: N) -> Self;
    fn number(self) -> N;
    fn physical(self) -> Cx;
    fn derivative_step(self, z: Cx) -> Self;
    fn delta_step(self, z: Cx) -> Self;
    fn parts(self) -> (Cx, i64);
}

impl ZoneNumber for Cx {
    const EXTENDED: bool = false;
    type Orbit = Cx;
    fn one() -> Self {
        Cx(1.0, 0.0)
    }
    fn from_parts(m: Cx, e: i64) -> Self {
        m.scale(exp2i(e))
    }
    fn parts(self) -> (Cx, i64) {
        (self, 0)
    }
    fn physical(self) -> Cx {
        self
    }
    fn sub(self, other: Self) -> Self {
        Cx::sub(self, other)
    }
    fn div(self, other: Self) -> Self {
        Cx::div(self, other)
    }
    fn coefficients(k: &Zone, i: usize) -> &[Self] {
        &k.bis[i]
    }
    fn phi(k: &Zone) -> &[Self] {
        &k.phi
    }
    fn entry(k: &Zone) -> Self {
        k.z24ma
    }
    fn initial_delta(self, scale: ZoneSize) -> Self {
        self.scale(scale.to_f64())
    }
    fn inside_guard(self, k: &Zone) -> bool {
        let (scale, guard) = (k.scale.to_f64(), k.guard.to_f64());
        if scale > 0.0 && guard > 0.0 {
            self.abs() * scale <= guard
        } else {
            self.abs() <= k.guard.over(k.scale)
        }
    }
    fn settled(self, step: Self, u: Self) -> bool {
        self.norm2() < 1.0 && step.norm2() <= 1e-24 * u.norm2()
    }
    fn log_arg(self) -> (f64, f64) {
        (self.abs().ln(), self.1.atan2(self.0))
    }
    fn jump(self, radius: f64, lr: f64) -> f64 {
        ((radius / self.abs()).ln() / lr).floor()
    }
    fn advance(self, j: f64, lr: f64, arg: f64) -> (Cx, Self) {
        let (mg, t) = ((j * lr).exp(), j * arg);
        let factor = Cx(mg * t.cos(), mg * t.sin());
        (self.mul(factor), factor)
    }
}

impl OrbitNumber<Cx> for Cx {
    fn from_number(value: Cx) -> Self {
        value
    }
    fn number(self) -> Cx {
        self
    }
    fn physical(self) -> Cx {
        self
    }
    #[inline(always)]
    fn derivative_step(self, z: Cx) -> Self {
        z.mul(self).scale(2.0).add(Cx(1.0, 0.0))
    }
    #[inline(always)]
    fn delta_step(self, z: Cx) -> Self {
        z.scale(2.0).mul(self).add(self.mul(self))
    }
    fn parts(self) -> (Cx, i64) {
        (self, 0)
    }
}

impl ZoneNumber for Fx {
    const EXTENDED: bool = true;
    type Orbit = Lazy;
    fn one() -> Self {
        Fx::ONE
    }
    fn from_parts(m: Cx, e: i64) -> Self {
        Fx::new(m, e)
    }
    fn parts(self) -> (Cx, i64) {
        (self.m, self.e)
    }
    fn physical(self) -> Cx {
        self.to_cx()
    }
    fn sub(self, other: Self) -> Self {
        Fx::sub(self, other)
    }
    fn div(self, other: Self) -> Self {
        self.mul(other.inv())
    }
    fn coefficients(k: &Zone, i: usize) -> &[Self] {
        &k.deep.as_ref().unwrap().bis[i]
    }
    fn phi(k: &Zone) -> &[Self] {
        &k.deep.as_ref().unwrap().phi
    }
    fn entry(k: &Zone) -> Self {
        k.deep.as_ref().unwrap().z24ma
    }
    fn initial_delta(self, scale: ZoneSize) -> Self {
        self.mul(Fx::new(Cx(scale.mant, 0.0), scale.exp2))
    }
    fn inside_guard(self, k: &Zone) -> bool {
        self.to_cx().abs() <= k.guard.over(k.scale)
    }
    fn settled(self, step: Self, u: Self) -> bool {
        self.to_cx().norm2() < 1.0
            && step.m.norm2() * exp2i(2 * (step.e - u.e)) <= 1e-24 * u.m.norm2()
    }
    fn log_arg(self) -> (f64, f64) {
        (self.ln_abs(), self.arg())
    }
    fn jump(self, radius: f64, lr: f64) -> f64 {
        ((radius.ln() - self.ln_abs()) / lr).floor()
    }
    fn advance(self, j: f64, lr: f64, arg: f64) -> (Cx, Self) {
        let (lw, aw) = self.log_arg();
        (
            Fx::polar(lw + j * lr, aw + j * arg).to_cx(),
            Fx::polar(j * lr, j * arg),
        )
    }
}

impl OrbitNumber<Fx> for Lazy {
    fn from_number(value: Fx) -> Self {
        Lazy::from(value)
    }
    fn number(self) -> Fx {
        self.fx()
    }
    fn physical(self) -> Cx {
        self.to_cx()
    }
    #[inline(always)]
    fn derivative_step(self, z: Cx) -> Self {
        self.step(z)
    }
    #[inline(always)]
    fn delta_step(self, z: Cx) -> Self {
        self.square_plus_2z(z)
    }
    fn parts(self) -> (Cx, i64) {
        (self.m, self.e)
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

/// Positive scale as a mantissa times a power of two.
#[derive(Clone, Copy, Debug)]
pub struct ZoneSize {
    mant: f64,
    exp2: i64,
}

impl ZoneSize {
    fn parse(f: &[&str]) -> Result<Self, String> {
        if !(2..=3).contains(&f.len()) {
            return Err("scale needs a mantissa and optional binary exponent".into());
        }
        let mant = f[1].parse::<f64>().map_err(|_| "bad scale mantissa")?;
        let exp2 = if f.len() == 3 {
            f[2].parse::<i64>().map_err(|_| "bad scale exponent")?
        } else {
            0
        };
        if !mant.is_finite()
            || mant < 0.0
            || (f.len() == 3 && mant != 0.0 && !(1.0..2.0).contains(&mant))
        {
            return Err("scale must be finite and positive (new mantissas in [1, 2))".into());
        }
        Ok(Self { mant, exp2 })
    }

    fn to_f64(self) -> f64 {
        self.mant * exp2i(self.exp2)
    }

    fn over(self, other: Self) -> f64 {
        (self.mant / other.mant) * exp2i(self.exp2.saturating_sub(other.exp2))
    }

    fn bits(self) -> u64 {
        (128.0 - self.mant.log2() - self.exp2 as f64)
            .max(128.0)
            .ceil() as u64
    }
}

impl std::fmt::LowerExp for ZoneSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = self.to_f64();
        if value > 0.0 && value.is_finite() {
            std::fmt::LowerExp::fmt(&value, f)
        } else {
            write!(f, "{:e} * 2^{}", self.mant, self.exp2)
        }
    }
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
    scale: ZoneSize,
    guard: ZoneSize,
    /// Largest `|c - C|` over a frame's samples for which the zone renders it.
    pub max_dc: ZoneSize,
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
    /// `log2 |S_n|` (index `n`, `-inf` if absent) of the first return from `u = v`,
    /// `M(v, v) = sum S_n v^n`, to beyond the biseries degree (`diag` lines).
    diag: Vec<f64>,
    patch_nx: usize,
    /// (centre, first child or -1, leaf index or -1)
    nodes: Vec<(Cx, i64, i64)>,
    leaves: Vec<Leaf>,
    /// Exact-range constants, set when the f64 ones are unusable (a deep ladder rung).
    deep: Option<Deep>,
    mid: Option<mid::Mid>,
}

/// A deep zone's biseries, Koenigs entry offset and `phi` as mantissa + exponent
/// (`biseries_x` and `z24_minus_alpha_x` lines): at the k = 7676 rung the coefficients
/// reach ~1e500 and the offset ~1e-500, so the whole pixel runs on [`Fx`].
#[derive(Clone, Debug)]
struct Deep {
    /// `bis[i][j]`: coefficient of `u^i v^j`.
    bis: Vec<[Fx; 8]>,
    z24ma: Fx,
    phi: Vec<Fx>,
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
            scale: ZoneSize { mant: 0.0, exp2: 0 },
            guard: ZoneSize { mant: 0.0, exp2: 0 },
            max_dc: ZoneSize { mant: 0.0, exp2: 0 },
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
            diag: Vec::new(),
            patch_nx: 0,
            nodes: Vec::new(),
            leaves: Vec::new(),
            deep: None,
            mid: None,
        };
        let mut mid = mid::Mid::default();
        let mut raw = vec![];
        let (mut raw_x, mut z24ma_x) = (vec![], None);
        for (ln, line) in text
            .lines()
            .enumerate()
            .filter(|(_, l)| !l.starts_with('#') && !l.trim().is_empty())
        {
            let f: Vec<&str> = line.split_whitespace().collect();
            let bad = |what: &str| format!("line {}: {what}", ln + 1);
            let n = |i: usize| -> Result<f64, String> {
                f.get(i)
                    .ok_or_else(|| bad("missing value"))?
                    .parse::<f64>()
                    .map_err(|_| bad("bad number"))
            };
            let cx = |i: usize| -> Result<Cx, String> { Ok(Cx(n(i)?, n(i + 1)?)) };
            let fx = |i: usize| -> Result<Fx, String> {
                let e = f
                    .get(i + 2)
                    .ok_or_else(|| bad("missing value"))?
                    .parse::<i64>()
                    .map_err(|_| bad("bad exponent"))?;
                let m = cx(i)?;
                if !(m.0.is_finite() && m.1.is_finite()) {
                    return Err(bad("non-finite mantissa"));
                }
                Ok(Fx::new(m, e))
            };
            match f[0] {
                "c_exact" => {
                    let (re, im) = (
                        f.get(1).ok_or_else(|| bad("missing value"))?,
                        f.get(2).ok_or_else(|| bad("missing value"))?,
                    );
                    (z.c_re, z.c_im) = (re.to_string(), im.to_string());
                }
                "c" => z.c = cx(1)?,
                "period" => z.period = n(1)? as u64,
                "scale" | "size" => z.scale = ZoneSize::parse(&f).map_err(|e| bad(&e))?,
                "guard" => z.guard = ZoneSize::parse(&f).map_err(|e| bad(&e))?,
                "max_dc" => z.max_dc = ZoneSize::parse(&f).map_err(|e| bad(&e))?,
                "r0" => z.r0 = n(1)?,
                "bias" => z.bias = cx(1)?,
                "alpha" => z.alpha = cx(1)?,
                "rho" => z.rho = cx(1)?,
                "z24_minus_alpha" => z.z24ma = cx(1)?,
                "z24_minus_alpha_x" => z24ma_x = Some(fx(1)?),
                "orbit" => z.orbit.push(cx(2)?),
                "biseries" => raw.push((n(1)? as usize, n(2)? as usize, cx(3)?)),
                "biseries_x" => raw_x.push((n(1)? as usize, n(2)? as usize, fx(3)?)),
                "phi" => z.phi.push(cx(2)?),
                "psi" => z.psi.push(cx(2)?),
                "diag" => {
                    let k = n(1)? as usize;
                    if k == 0 || k > 16 {
                        return Err(bad("diag degree must be 1..=16"));
                    }
                    let a = fx(2)?;
                    if z.diag.len() <= k {
                        z.diag.resize(k + 1, f64::NEG_INFINITY);
                    }
                    z.diag[k] = if a.is_zero() {
                        f64::NEG_INFINITY
                    } else {
                        a.m.abs().log2() + a.e as f64
                    };
                }
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
                    z.leaves.push(Leaf {
                        centre: cx(1)?,
                        r: n(3)?,
                        m: n(4)? as i64,
                        coef,
                    });
                }
                "ref" => mid.reference(n(1)? as usize, cx(2)?).map_err(|e| bad(&e))?,
                other if other.starts_with("mid_") => mid.add(&f).map_err(|e| bad(&e))?,
                other => return Err(bad(&format!("unknown constant {other:?}"))),
            }
        }
        if z.c_re.is_empty() {
            return Err("no c_exact line (the nucleus as exact decimals)".into());
        }
        if raw.is_empty()
            || z.period == 0
            || z.scale.mant <= 0.0
            || z.guard.mant <= 0.0
            || z.r0 <= 0.0
        {
            return Err("incomplete zone: needs period, scale, guard, r0 and biseries".into());
        }
        if z.orbit.len() != APPROACH {
            return Err(format!(
                "expected {APPROACH} orbit points, got {}",
                z.orbit.len()
            ));
        }
        if z.phi.is_empty() || z.psi.is_empty() {
            return Err("needs phi and psi series".into());
        }
        for &(_, kid, leaf) in &z.nodes {
            if (kid >= 0 && kid as usize + 4 > z.nodes.len())
                || (kid < 0 && (leaf < 0 || leaf as usize >= z.leaves.len()))
            {
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
        for &(i, j, a) in &raw {
            z.bis[i][j] = a;
        }
        // Deep ladder rungs have a Koenigs entry offset below f64's range and
        // biseries coefficients above it. Rounded constants would invent a
        // result, so such a zone renders only from its exact (`_x`) lines.
        let f64_ok = z.scale.to_f64() >= f64::MIN_POSITIVE
            && z.z24ma.abs() != 0.0
            && raw
                .iter()
                .all(|&(_, _, a)| a.0.is_finite() && a.1.is_finite());
        if !f64_ok {
            let Some(z24ma) = z24ma_x else {
                return Err("Koenigs entry offset or biseries leaves f64's range; z24_minus_alpha_x and biseries_x lines required".into());
            };
            if raw_x.len() != raw.len() {
                return Err("need one biseries_x line per biseries line".into());
            }
            let mut bis = vec![[Fx::default(); 8]; z.deg + 1];
            for (i, j, a) in raw_x {
                if i + j > z.deg {
                    return Err("biseries_x term above the biseries degree".into());
                }
                bis[i][j] = a;
            }
            z.deep = Some(Deep {
                bis,
                z24ma,
                phi: z.phi.iter().map(|&a| Fx::new(a, 0)).collect(),
            });
        }
        if !z.diag.is_empty() && z.diag.len() <= z.deg + 1 {
            return Err("diag lines must go beyond the biseries degree".into());
        }
        if z.max_dc.mant == 0.0 {
            z.max_dc = z.guard;
        }
        z.mid = mid.finish(z.period)?;
        Ok(z)
    }
}

impl Zone {
    /// FP64 Vulkan constants, with camera offsets obtained from exact decimal arithmetic.
    ///
    /// The shader takes scaled offsets, never a rounded deep centre or width (DEC-21).
    /// It declines mid/deep-range operators and retains the CPU fallback (DEC-10).
    pub fn gpu_f64_input(&self, view: &View, p: &Params) -> Result<(Header, Vec<f64>), String> {
        let (plane, tier) = setup(view, p)?;
        let off = centre_offset(view, self, &plane)?;
        if !deep_covers(&plane, p, self, off) || self.deep.is_some() {
            return Err("FP64 Vulkan zone needs a covered, f64-range deep frame".into());
        }
        if p.columns.has(Column::Bound) {
            return Err("FP64 Vulkan zone has no certified bounds".into());
        }
        if p.max_iter > i32::MAX as u64 || self.period > i32::MAX as u64 {
            return Err("FP64 Vulkan zone iteration count exceeds i32".into());
        }
        let scale = self.scale.to_f64();
        let guard = self.guard.to_f64();
        let h = ZoneSize {
            mant: plane.h_m,
            exp2: plane.h_e,
        }
        .over(self.scale);
        let px = plane.h();
        if !(scale > 0.0
            && guard > 0.0
            && h > 0.0
            && px > 0.0
            && self.rho.abs() > 1.0
            && self.deg < 8
            && self.phi.len() <= i32::MAX as usize
            && self.psi.len() <= i32::MAX as usize
            && self.nodes.len() <= i32::MAX as usize
            && self.leaves.len() <= i32::MAX as usize)
        {
            return Err("FP64 Vulkan zone constants outside supported range".into());
        }
        let mut v = vec![
            scale,
            guard,
            self.r0,
            self.period as f64,
            self.deg as f64,
            p.max_iter as f64,
            p.nx as f64,
            p.ny as f64,
            off.scaled.0,
            off.scaled.1,
            h,
            self.c.0,
            self.c.1,
            self.alpha.0,
            self.alpha.1,
            self.rho.abs().ln(),
            self.rho.1.atan2(self.rho.0),
            self.z24ma.0,
            self.z24ma.1,
            self.bias.0,
            self.bias.1,
            p.escape_radius * p.escape_radius,
            // Jump count j = floor(log(r0)/log|rho| - log|w0|/log|rho|).
            self.r0.ln() / self.rho.abs().ln(),
            self.patch_nx as f64,
            self.phi.len() as f64,
            self.psi.len() as f64,
            self.nodes.len() as f64,
            self.leaves.len() as f64,
            view.rotation.cos(),
            view.rotation.sin(),
            px * p.ss as f64,
            1.0 / self.rho.abs().ln(),
        ];
        // A fixed 8x8 complex biseries and the 23 orbit points precede the variable data.
        for i in 0..8 {
            for j in 0..8 {
                let a = if i <= self.deg && j <= self.deg {
                    self.bis[i][j]
                } else {
                    Cx::default()
                };
                v.extend([a.0, a.1]);
            }
        }
        for a in &self.orbit {
            v.extend([a.0, a.1]);
        }
        for a in self.phi.iter().chain(&self.psi) {
            v.extend([a.0, a.1]);
        }
        for &(centre, kid, leaf) in &self.nodes {
            v.extend([centre.0, centre.1, kid as f64, leaf as f64]);
        }
        for leaf in &self.leaves {
            v.extend([leaf.centre.0, leaf.centre.1, leaf.r, leaf.m as f64]);
            for a in &leaf.coef {
                v.extend([a.0, a.1]);
            }
        }
        if !v.iter().all(|x| x.is_finite()) {
            return Err("FP64 Vulkan zone constants are non-finite".into());
        }
        let cols = p.columns.with(Column::Class);
        let mut hd = header(view, p, cols, &plane, tier);
        hd.kernel = format!("zone-vulkan-f64/1 P={}", self.period);
        Ok((hd, v))
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

/// Centre difference in absolute and zone-scale units.
#[derive(Clone, Copy)]
struct CentreOffset {
    absolute: Cx,
    scaled: Cx,
    /// The absolute difference at any depth.
    exact: Fx,
}

/// Subtract exact decimal centres at the greater of the view and zone precisions.
fn centre_offset(view: &View, zone: &Zone, plane: &Plane) -> Result<CentreOffset, String> {
    let bits = plane
        .bits
        .max(zone.scale.bits())
        .max(zone.guard.bits())
        .max(zone.max_dc.bits());
    let limbs = fd_fixed::limbs_for(bits);
    let d = |a: &str, b: &str| -> Result<(f64, f64, (f64, i64)), String> {
        let diff = Fixed::parse(a, limbs)?.sub(&Fixed::parse(b, limbs)?);
        let me = diff.frexp().unwrap_or((0.0, 0));
        let scaled = diff
            .frexp()
            .map_or(0.0, |(m, e)| ZoneSize { mant: m, exp2: e }.over(zone.scale));
        Ok((diff.to_f64(), scaled, me))
    };
    let (re, sr, (rm, rexp)) = d(&view.center_re, &zone.c_re)?;
    let (im, si, (im_m, iexp)) = d(&view.center_im, &zone.c_im)?;
    let exact = Fx::new(Cx(rm, 0.0), rexp).add(Fx::new(Cx(0.0, im_m), iexp));
    Ok(CentreOffset {
        absolute: Cx(re, im),
        scaled: Cx(sr, si),
        exact,
    })
}

impl Zone {
    fn f64_geometry(&self, plane: &Plane) -> bool {
        plane.h() > 0.0
            && [self.scale, self.guard, self.max_dc]
                .iter()
                .all(|size| size.to_f64() > 0.0 && size.to_f64().is_finite())
    }

    /// Bound on the pixel shift from truncating the first return (`u = v`) at the
    /// biseries degree, for samples out to `r` with spacing `h` (both in `v` units).
    /// `None` if the zone has no `diag` lines; infinite where the return map's
    /// derivative cannot be bounded away from 0.
    ///
    /// Dropped terms: `sum_{n > deg} |S_n| r^n`. Derivative `dM/dv = sum n S_n v^(n-1)`,
    /// bounded below at `|v| = r` by its largest term minus the others. The ratio grows
    /// like `r^4`, so the frame's outer radius is where it is largest. Not certified
    /// (PROB-13); later returns are not covered (their inputs do not depend on the frame).
    fn truncation_shift(&self, r: f64, h: f64) -> Option<f64> {
        if self.diag.is_empty() {
            return None;
        }
        let lr = r.log2();
        // log2 of each term's modulus: |S_n| r^n, and n |S_n| r^(n - 1) for dM/dv.
        let rem: Vec<f64> = (self.deg + 1..self.diag.len())
            .map(|n| self.diag[n] + n as f64 * lr)
            .collect();
        let der: Vec<f64> = (1..=self.deg)
            .map(|n| (n as f64).log2() + self.diag[n] + (n - 1) as f64 * lr)
            .collect();
        let top = der.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let others: f64 = der.iter().map(|&t| (t - top).exp2()).sum::<f64>() - 1.0;
        if !top.is_finite() || others >= 1.0 {
            return Some(f64::INFINITY);
        }
        let rtop = rem.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let rsum = rtop + rem.iter().map(|&t| (t - rtop).exp2()).sum::<f64>().log2();
        Some((rsum - top - (1.0 - others).log2() - h.log2()).exp2())
    }
}

/// Whether `zone` may render `view` (DEC-10 validity), decided before rendering
/// (DEC-19): every sample within `max_dc` of the nucleus and, for a zone with `diag`
/// lines, a first-return truncation shift of at most [`SHIFT_PX`].
fn deep_covers(plane: &Plane, p: &Params, zone: &Zone, off: CentreOffset) -> bool {
    let diagonal = (f64::from(p.nx) / 2.0).hypot(f64::from(p.ny) / 2.0);
    let spacing = ZoneSize {
        mant: plane.h_m,
        exp2: plane.h_e,
    }
    .over(zone.scale);
    let inside = if zone.f64_geometry(plane) {
        off.absolute.abs() + plane.h() * diagonal <= zone.max_dc.to_f64()
    } else {
        off.scaled.abs() + spacing * diagonal <= zone.max_dc.over(zone.scale)
    };
    let shift = zone.truncation_shift(off.scaled.abs() + spacing * diagonal, spacing);
    inside && shift.is_none_or(|s| s <= SHIFT_PX)
}

fn mid_covers(plane: &Plane, p: &Params, zone: &Zone, off: CentreOffset) -> bool {
    let diagonal = (f64::from(p.nx) / 2.0).hypot(f64::from(p.ny) / 2.0);
    zone.mid.as_ref().is_some_and(|m| {
        plane.h() > 0.0 && m.covers(off.absolute.abs() + plane.h() * diagonal, p.escape_radius)
    })
}

pub fn zone_covers(view: &View, p: &Params, zone: &Zone) -> Result<bool, String> {
    let (plane, _) = setup(view, p)?;
    let off = centre_offset(view, zone, &plane)?;
    Ok(deep_covers(&plane, p, zone, off) || mid_covers(&plane, p, zone, off))
}

/// Render `view` with the zone pipeline. Refused when the zone does not cover the view
/// (see [`zone_covers`]) or `Bound` is requested (the pipeline carries no error radii).
/// The header's kernel id is `zone-koenigs/1 P=<period>`.
pub fn render_zone(
    view: &View,
    p: &Params,
    zone: &Zone,
) -> Result<(Header, Samples, Stats, ZoneStats), String> {
    if p.columns.has(Column::Bound) {
        return Err("--columns bound: the zone pipeline carries no error radii (PROB-13)".into());
    }
    if !zone_covers(view, p, zone)? {
        return Err(format!(
            "the zone (nucleus within {:e} * 2^{}) does not cover this view",
            zone.max_dc.mant, zone.max_dc.exp2
        ));
    }
    let (plane, tier) = setup(view, p)?;
    let off = centre_offset(view, zone, &plane)?;
    let use_mid = !deep_covers(&plane, p, zone, off) && mid_covers(&plane, p, zone, off);
    let legacy = zone.f64_geometry(&plane);
    // Deep zones: v = (centre offset + pixel offset) / scale, all in Fx.
    let inv_scale = Fx::new(Cx(1.0 / zone.scale.mant, 0.0), -zone.scale.exp2);
    let h = if legacy {
        plane.h()
    } else {
        ZoneSize {
            mant: plane.h_m,
            exp2: plane.h_e,
        }
        .over(zone.scale)
    };
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
                        let Some(mut row) = queue.lock().unwrap().next() else {
                            break;
                        };
                        for i in 0..row.class.len() {
                            let (ux, uy) = plane.unit_offset(i, row.j);
                            if use_mid {
                                let dc = Cx(
                                    off.absolute.0 + ux * plane.h(),
                                    off.absolute.1 + uy * plane.h(),
                                );
                                let m = zone.mid.as_ref().unwrap();
                                let (o, w, jumped) = if deriv {
                                    mid::pixel::<true>(m, zone.c, dc, p.max_iter, r2, plane.h())
                                } else {
                                    mid::pixel::<false>(m, zone.c, dc, p.max_iter, r2, plane.h())
                                };
                                if jumped {
                                    st.jumps += 1;
                                    st.plain += w.saturating_sub(25);
                                } else {
                                    st.plain += w;
                                }
                                its += w;
                                store.put(&mut row, i, o);
                                continue;
                            }
                            if zone.deep.is_some() {
                                let v = off
                                    .exact
                                    .add(Fx::new(Cx(ux * plane.h_m, uy * plane.h_m), plane.h_e))
                                    .mul(inv_scale);
                                let (o, w) = if deriv {
                                    pixel::<Fx, true>(zone, v, p.max_iter, r2, &mut st)
                                } else {
                                    pixel::<Fx, false>(zone, v, p.max_iter, r2, &mut st)
                                };
                                its += w;
                                store.put(&mut row, i, o);
                                continue;
                            }
                            let v = if legacy {
                                Cx(off.absolute.0 + ux * h, off.absolute.1 + uy * h)
                                    .scale(1.0 / zone.scale.to_f64())
                            } else {
                                Cx(off.scaled.0 + ux * h, off.scaled.1 + uy * h)
                            };
                            let (o, w) = if deriv {
                                pixel::<Cx, true>(zone, v, p.max_iter, r2, &mut st)
                            } else {
                                pixel::<Cx, false>(zone, v, p.max_iter, r2, &mut st)
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
    hd.kernel = if use_mid {
        format!("zone-mid-koenigs/1 P={}", zone.period)
    } else {
        format!("zone-koenigs/1 P={}", zone.period)
    };
    let stats = Stats {
        reference_len: 0,
        reference_seconds: 0.0,
        iterations: work.into_inner(),
    };
    Ok((hd, s, stats, total.into_inner().unwrap()))
}

/// One zone sample: the return, approach, jump and finish stages are instantiated
/// over the number representation. Summation and range policies preserve the
/// original rounding and fallback contracts.
// DEC-24: each zone stage is written once for ordinary and deep numbers.
#[inline]
fn pixel<N: ZoneNumber, const D: bool>(
    k: &Zone,
    v: N,
    max_iter: u64,
    r2: f64,
    st: &mut ZoneStats,
) -> (Outcome, u64) {
    let mut b = [N::default(); 8];
    let mut bv = [N::default(); 8];
    let (pv, ve) = powers(v, k.deg);
    for i in 0..=k.deg {
        (b[i], bv[i]) = polynomial::<N, D, false>(N::coefficients(k, i), k.deg - i, v, &pv, ve);
    }
    b[0] = b[0].add(N::from_parts(k.bias, 0));
    let (mut u, mut du) = (v, N::one());
    let mut n = 1u64;
    let mut work = 0u64;
    // Stage 1: returns.
    while u.inside_guard(k) {
        if n + k.period > max_iter {
            return (Outcome::Unresolved, work);
        }
        let (pw, ue) = powers(u, k.deg);
        let (s, su) = polynomial::<N, true, true>(&b, k.deg, u, &pw, ue);
        let sv = if D {
            polynomial::<N, false, true>(&bv, k.deg, u, &pw, ue).0
        } else {
            N::default()
        };
        let step = s.sub(u);
        if D {
            du = su.mul(du).add(sv);
        }
        if su.settled(step, u)
            || (work >= 16 && work.is_power_of_two() && cycle_basin(&b, k.deg, u))
        {
            st.returns += 1;
            return (Outcome::Interior { n }, work + 1);
        }
        u = s;
        n += k.period;
        work += 1;
        st.returns += 1;
    }
    // Stage 2: approach against C's critical orbit.
    let mut d = N::Orbit::from_number(u.initial_delta(k.scale));
    let mut g = N::Orbit::from_number(du);
    for z in &k.orbit {
        if D {
            g = g.derivative_step(z.add(d.physical()));
        }
        d = d.delta_step(*z);
    }
    let d = d.number();
    du = g.number();
    n += APPROACH as u64;
    work += APPROACH as u64;
    // Stage 3: Koenigs jump and optional tail patch.
    let h0 = N::entry(k).add(d);
    let mut z = k.alpha.add(h0.physical());
    if h0.physical().abs() < k.r0 {
        let (w0, dphi) = series_d(N::phi(k), h0);
        let lr = k.rho.abs().ln();
        let j = w0.jump(k.r0, lr);
        let (lw, aw) = w0.log_arg();
        if let Some((zp, dzs, m)) = patch(k, lw, aw, j) {
            if D {
                du = N::from_parts(dzs, 0).mul(dphi).mul(du).div(w0);
            }
            z = zp;
            n += 2 * j as u64 + m;
            st.jumps += 1;
            st.patches += 1;
        } else if j > 0.0 {
            let (w, factor) = w0.advance(j, lr, k.rho.1.atan2(k.rho.0));
            let (hw, dpsi) = series_d(&k.psi, w);
            if D {
                du = N::from_parts(dpsi, 0).mul(factor).mul(dphi).mul(du);
            }
            z = k.alpha.add(hw);
            n += 2 * j as u64;
            st.jumps += 1;
        }
        work += 1;
    }
    // Stage 4: plain steps at C.
    let mut g = N::Orbit::from_number(du);
    loop {
        if n >= max_iter {
            return (Outcome::Unresolved, work);
        }
        if D {
            g = g.derivative_step(z);
        }
        z = z.mul(z).add(k.c);
        n += 1;
        work += 1;
        st.plain += 1;
        if z.norm2() > r2 {
            let (derivative, exponent) = g.parts();
            return (
                Outcome::Escaped {
                    n,
                    zr: z.0,
                    zi: z.1,
                    dr: derivative.0,
                    di: derivative.1,
                    dexp: exponent,
                    ez: f64::INFINITY,
                    ed: f64::INFINITY,
                },
                work,
            );
        }
    }
}
/// `m 2^e` normalised only when `m` leaves `[2^-32, 2^32]`, with `2^e` and `2^-e` cached:
/// the deep path's numbers next to an O(1) orbit, where per-step [`Fx`] normalisation
/// would be the main cost.
#[derive(Clone, Copy)]
struct Lazy {
    m: Cx,
    e: i64,
    up: f64,
    down: f64,
}

impl Lazy {
    #[inline(always)]
    fn from(x: Fx) -> Lazy {
        Lazy {
            m: x.m,
            e: x.e,
            up: exp2i(x.e),
            down: exp2i((-x.e).min(1023)),
        }
    }
    #[inline(always)]
    fn fx(self) -> Fx {
        Fx::new(self.m, self.e)
    }
    #[inline(always)]
    fn to_cx(self) -> Cx {
        self.m.scale(self.up)
    }
    #[inline(always)]
    fn renorm(self) -> Lazy {
        let a = self.m.0.abs().max(self.m.1.abs());
        if a > 4294967296.0 || (a < 1.0 / 4294967296.0 && a != 0.0) {
            Lazy::from(self.fx())
        } else {
            self
        }
    }
    /// `dz/dc <- 2 z dz/dc + 1`.
    #[inline(always)]
    fn step(self, z: Cx) -> Lazy {
        Lazy {
            m: z.mul(self.m).scale(2.0).add(Cx(self.down, 0.0)),
            ..self
        }
        .renorm()
    }
    /// `d <- 2 z d + d^2` (`d^2` is negligible next to `2 z d` whenever `2^e` underflows).
    #[inline(always)]
    fn square_plus_2z(self, z: Cx) -> Lazy {
        Lazy {
            m: z.scale(2.0)
                .mul(self.m)
                .add(self.m.mul(self.m).scale(self.up)),
            ..self
        }
        .renorm()
    }
}

/// Mantissa powers shared by all coefficient evaluations of one state.
#[inline(always)]
fn powers<N: ZoneNumber>(u: N, deg: usize) -> ([Cx; 8], i64) {
    let mut pw = [Cx(1.0, 0.0); 8];
    let (mantissa, exponent) = u.parts();
    if N::EXTENDED {
        for i in 1..=deg {
            pw[i] = pw[i - 1].mul(mantissa);
        }
    }
    (pw, exponent)
}

/// Polynomial value and derivative. Horner order is retained for ordinary
/// numbers; exponent-aligned term sums retain the deep representation's order.
#[inline(always)]
fn polynomial<N: ZoneNumber, const D: bool, const TOP: bool>(
    c: &[N],
    deg: usize,
    u: N,
    pw: &[Cx; 8],
    ue: i64,
) -> (N, N) {
    if N::EXTENDED {
        (
            term_sum(c, deg, pw, ue, 0),
            if D {
                term_sum(c, deg, pw, ue, 1)
            } else {
                N::default()
            },
        )
    } else {
        let (mut s, mut d, end) = if TOP {
            (c[deg], N::default(), deg)
        } else {
            (N::default(), N::default(), deg + 1)
        };
        for i in (0..end).rev() {
            if D {
                d = d.mul(u).add(s);
            }
            s = s.mul(u).add(c[i]);
        }
        (s, d)
    }
}

/// Exponent-aligned polynomial sum, normalised once after summation.
#[inline(always)]
fn term_sum<N: ZoneNumber>(c: &[N], deg: usize, pw: &[Cx; 8], ue: i64, d: usize) -> N {
    let terms = || {
        c[..=deg].iter().enumerate().skip(d).filter(|(_, a)| {
            let (m, _) = a.parts();
            m.0 != 0.0 || m.1 != 0.0
        })
    };
    let Some(top) = terms()
        .map(|(i, a)| a.parts().1 + (i - d) as i64 * ue)
        .max()
    else {
        return N::default();
    };
    let mut sum = Cx::default();
    for (i, a) in terms() {
        let (m, e) = a.parts();
        let k = if d == 1 { i as f64 } else { 1.0 };
        sum = sum.add(
            m.mul(pw[i - d])
                .scale(k * exp2i(e + (i - d) as i64 * ue - top)),
        );
    }
    N::from_parts(sum, top)
}

/// Conjugate a deep return map into its mantissa chart for the shared cycle test.
fn cycle_basin<N: ZoneNumber>(b: &[N; 8], deg: usize, u: N) -> bool {
    let mut bt = [Cx::default(); 8];
    let (um, ue) = u.parts();
    for (i, (t, a)) in bt.iter_mut().zip(b).enumerate().take(deg + 1) {
        let (m, e) = a.parts();
        *t = if N::EXTENDED {
            N::from_parts(m, e + ue * (i as i64 - 1)).physical()
        } else {
            m
        };
    }
    attracted(&bt, deg, um)
}
/// `B(u)`, `B'(u)` and `B''(u)` for `B(u) = sum b[i] u^i`, `i <= deg`.
#[inline]
fn eval2<N: Complex>(b: &[N; 8], deg: usize, u: N) -> (N, N, N) {
    let (mut s, mut d, mut dd) = (b[deg], N::default(), N::default());
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
fn series_d<N: Complex>(c: &[N], x: N) -> (N, N) {
    let (mut s, mut d) = (N::default(), N::default());
    for (i, a) in c.iter().enumerate().rev() {
        s = s.add(*a).mul(x);
        d = d.mul(x).add(a.scale((i + 1) as f64));
    }
    (s, d)
}

/// Tail patch for the jumped point `w0 rho^j` (`w0` given as `ln|w0|` and `arg w0`):
/// `f_C^m(A + psi(w))`, its derivative in `s = log w`, and `m`.
#[inline]
fn patch(k: &Zone, ln_w0: f64, arg_w0: f64, j: f64) -> Option<(Cx, Cx, u64)> {
    if k.nodes.is_empty() || j < 0.0 {
        return None;
    }
    let tau = std::f64::consts::TAU;
    let re = ln_w0 + j * k.rho.abs().ln();
    let im = (arg_w0 + j * k.rho.1.atan2(k.rho.0)).rem_euclid(tau);
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
    fn zone_pixel_render_bytes_are_unchanged() {
        // Frozen from 97c532c on each platform: old and shared kernels emit
        // identical bytes locally, but Windows and Linux have different hashes.
        let expected = if cfg!(target_os = "linux") {
            [
                [0x850c901dfbe62468, 0x988c23c5fc7ef3e3],
                [0xb433623f669f6b7b, 0x242f075f9b9f3e99],
            ]
        } else {
            [
                [0x30499ad7f43278cd, 0xf17526a7d999103d],
                [0x243149996becf305, 0x23b2b21cf0acb4e7],
            ]
        };
        for (deep, zone, expected) in [
            (false, v0(), expected[0]),
            (true, as_deep(v0()), expected[1]),
        ] {
            for (i, width) in ["1e-36", "2e-48"].into_iter().enumerate() {
                let mut p = params(&[Column::Nu, Column::De, Column::Normal]);
                p.nx = 8;
                p.ny = 4;
                let (header, samples, _, _) = render_zone(&view(width), &p, &zone).unwrap();
                let mut bytes = Vec::new();
                fd_samples::write(&mut bytes, &header, &samples).unwrap();
                let hash = bytes.iter().fold(0xcbf29ce484222325u64, |h, b| {
                    (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
                });
                assert_eq!(
                    hash, expected[i],
                    "deep={deep} width={width}: .fds bytes changed"
                );
            }
        }
    }

    fn assert_cx_bits(actual: Cx, expected: Cx) {
        assert_eq!(
            actual.0.to_bits(),
            expected.0.to_bits(),
            "{actual:?} {expected:?}"
        );
        assert_eq!(
            actual.1.to_bits(),
            expected.1.to_bits(),
            "{actual:?} {expected:?}"
        );
    }

    fn assert_fx_bits(actual: Fx, expected: Fx) {
        assert_eq!(actual.e, expected.e, "{actual:?} {expected:?}");
        assert_cx_bits(actual.m, expected.m);
    }

    #[test]
    fn shared_series_preserves_complex_values_and_deep_exponents() {
        // Dyadic inputs make this independent expanded cubic exact in binary64.
        let c = [Cx(1.0, -2.0), Cx(-0.5, 0.25), Cx(0.125, -0.125)];
        let x = Cx(0.5, 0.25);
        let x2 = x.mul(x);
        let expected_s = c[0].mul(x).add(c[1].mul(x2)).add(c[2].mul(x2.mul(x)));
        let expected_d = c[0]
            .add(c[1].scale(2.0).mul(x))
            .add(c[2].scale(3.0).mul(x2));
        let (s, d) = series_d(&c, x);
        assert_cx_bits(s, expected_s);
        assert_cx_bits(d, expected_d);
        for e in [-3000, 0, 3000] {
            // c[k] * x^(k+1) stays finite while x and the derivative leave f64 range.
            let coefficients =
                std::array::from_fn::<_, 3, _>(|k| Fx::new(c[k], -(k as i64 + 1) * e));
            let (s, d) = series_d(&coefficients, Fx::new(x, e));
            assert_fx_bits(s, Fx::new(expected_s, 0));
            assert_fx_bits(d, Fx::new(expected_d, -e));
        }
        let (s, d) = series_d::<Cx>(&[], x);
        assert_eq!((s, d), (Cx::default(), Cx::default()));
        let (s, d) = series_d::<Fx>(&[], Fx::new(x, -3000));
        assert_fx_bits(s, Fx::default());
        assert_fx_bits(d, Fx::default());
    }

    #[test]
    fn shared_polynomial_preserves_both_derivatives_in_each_range() {
        // B(x) = 1 + (2-i)x + (3+i)x^2, including degree-zero and linear cases.
        let mut b = [Cx::default(); 8];
        b[0] = Cx(1.0, 0.0);
        b[1] = Cx(2.0, -1.0);
        b[2] = Cx(3.0, 1.0);
        let x = Cx(0.5, 0.25);
        for degree in 0..=2 {
            let (s, d, dd) = eval2(&b, degree, x);
            let expected = match degree {
                0 => (b[0], Cx::default(), Cx::default()),
                1 => (b[0].add(b[1].mul(x)), b[1], Cx::default()),
                _ => (
                    b[0].add(b[1].mul(x)).add(b[2].mul(x.mul(x))),
                    b[1].add(b[2].scale(2.0).mul(x)),
                    b[2].scale(2.0),
                ),
            };
            assert_cx_bits(s, expected.0);
            assert_cx_bits(d, expected.1);
            assert_cx_bits(dd, expected.2);
            for e in [-3000, 0, 3000] {
                let coefficients = std::array::from_fn(|k| Fx::new(b[k], -(k as i64) * e));
                let (s, d, dd) = eval2(&coefficients, degree, Fx::new(x, e));
                assert_fx_bits(s, Fx::new(expected.0, 0));
                assert_fx_bits(d, Fx::new(expected.1, -e));
                assert_fx_bits(dd, Fx::new(expected.2, -2 * e));
            }
        }
    }

    #[test]
    fn gpu_f64_constants_use_scaled_decimal_offsets() {
        let z = v0();
        let p = params(&[Column::Nu, Column::De, Column::Normal]);
        let view = view("1e-40");
        let (header, packed) = z.gpu_f64_input(&view, &p).unwrap();
        assert!(!header.kernel.is_empty());
        assert_eq!(packed[0], 1e-25);
        assert!(packed[8].is_finite() && packed[9].is_finite());
        assert!(packed[10] > 0.0 && packed[10] < 1e-10);
        assert_eq!(packed[160], z.orbit[0].0);
        assert_eq!(packed[161], z.orbit[0].1);
        assert_eq!(
            packed.len(),
            206 + 2 * (z.phi.len() + z.psi.len()) + 4 * z.nodes.len() + 36 * z.leaves.len()
        );
        let mut outside = view;
        outside.width = "1e-12".into();
        assert!(z.gpu_f64_input(&outside, &p).is_err());
    }

    #[test]
    fn series_derivative_matches_finite_difference() {
        let c = [Cx(1.0, 0.0), Cx(0.3, -0.2), Cx(-0.1, 0.05)];
        let x = Cx(0.02, 0.01);
        let (_, d) = series_d(&c, x);
        let e = 1e-7;
        let (a, _) = series_d(&c, x.add(Cx(e, 0.0)));
        let (b, _) = series_d(&c, x);
        let fd = Cx((a.0 - b.0) / e, (a.1 - b.1) / e);
        assert!(
            (fd.0 - d.0).abs() < 1e-6 && (fd.1 - d.1).abs() < 1e-6,
            "{fd:?} {d:?}"
        );
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
            threads: 1,
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
                assert!(
                    (bd[i] / ad[i] - 1.0).abs() < 1e-2,
                    "{w} sample {i}: de {} vs {}",
                    bd[i],
                    ad[i]
                );
                let da = am[i].wrapping_sub(bm[i]).min(bm[i].wrapping_sub(am[i]));
                assert!(
                    f64::from(da) * 360.0 / 65536.0 < 1.0,
                    "{w} sample {i}: normal {} vs {}",
                    am[i],
                    bm[i]
                );
                checked += 1;
            }
            assert!(
                checked > a.class.len() / 2,
                "{w}: only {checked} samples checked"
            );
        }
    }

    #[test]
    fn mid_frame_gate_rejects_failing_views_without_losing_passing_ones() {
        // Fixture uses the checked v0 nucleus/scale; Mid::covers only needs
        // the operator's parameter domain, not the generated chart contents.
        let mut z = v0();
        z.mid = Some(mid::Mid::default());
        let mut p = params(&[Column::Nu, Column::De, Column::Normal]);
        p.nx = 960;
        p.ny = 540;
        p.max_iter = 100_000;
        for width in ["2.37e-8", "5.35e-10", "1.99e-28"] {
            let v = view(width);
            assert!(!zone_covers(&v, &p, &z).unwrap(), "{width}");
            assert!(render_zone(&v, &p, &z).is_err(), "{width}");
        }
        for width in ["1.05e-6", "1.21e-11", "1.88e-26", "1.02e-31"] {
            assert!(zone_covers(&view(width), &p, &z).unwrap(), "{width}");
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

    /// PROB-20: the v0 band-top frames at 1280x720. Frames 431 and 432 (widths
    /// 1.70834e-28, 1.46809e-28) failed `fd compare` on de; frame 435 (9.3172e-29) is
    /// the first whose bounded first-return truncation shift is within [`SHIFT_PX`].
    #[test]
    fn band_top_frames_wait_for_the_truncation_guard() {
        let z = v0();
        let p = Params {
            nx: 1280,
            ny: 720,
            ..params(&[Column::Nu])
        };
        let at = |w: &str| zone_covers(&view(w), &p, &z).unwrap();
        assert!(!at("1.70834e-28") && !at("1.46809e-28") && !at("1.0842e-28"));
        assert!(at("9.3172e-29") && at("2e-49"));
        // Bound at frame 431's outer radius: 2.622e-6 px from the mpmath series.
        let (h, r) = (
            1.70834e-28 / 1280.0 / 1e-25,
            1.70834e-28 / 1280.0 * 640f64.hypot(360.0) / 1e-25,
        );
        let s = z.truncation_shift(r, h).unwrap();
        assert!((s / 2.622e-6 - 1.0).abs() < 0.01, "{s}");
        // A zone without diag lines keeps the radius-only rule.
        let old: String = include_str!("../../../bench/zones/v0-core.zone")
            .lines()
            .filter(|l| !l.starts_with("diag"))
            .map(|l| format!("{l}\n"))
            .collect();
        let old = Zone::parse(&old).unwrap();
        assert!(zone_covers(&view("1.70834e-28"), &p, &old).unwrap());
    }

    #[test]
    fn parse_rejects_incomplete_zones() {
        assert!(Zone::parse("period 764\n").is_err());
        assert!(Zone::parse("c_exact -0.75 0.1\nbogus 1\n").is_err());
    }

    #[test]
    fn v0_binary_scales_keep_the_same_samples() {
        let old = v0();
        let text = include_str!("../../../bench/zones/v0-core.zone")
            .replace("scale 1e-25", "scale 1.9342813113834068 -84")
            .replace("guard 1e-28", "guard 1.9807040628566084 -94");
        let new = Zone::parse(&text).unwrap();
        let (v, p) = (
            view("1e-36"),
            params(&[Column::Nu, Column::De, Column::Normal]),
        );
        let (_, a, _, _) = render_zone(&v, &p, &old).unwrap();
        let (_, b, _, _) = render_zone(&v, &p, &new).unwrap();
        assert_eq!(a.class, b.class);
        assert_eq!(a.nu, b.nu);
        assert_eq!(a.de, b.de);
        assert_eq!(a.normal, b.normal);
    }

    #[test]
    fn rung_7676_zone_covers_five_sizes_but_not_ten_away() {
        let rung = include_str!("../../../tools/research/misiurewicz/ladder_rungs.txt")
            .lines()
            .find(|line| line.starts_with("7676 "))
            .unwrap();
        let parts: Vec<_> = rung.split_whitespace().collect();
        assert_eq!(parts[1], "16116");
        // v0's diag lines describe v0's map, not this one: dropped.
        let mut lines: Vec<String> = include_str!("../../../bench/zones/v0-core.zone")
            .lines()
            .filter(|line| !line.starts_with("diag"))
            .map(|line| match line.split_whitespace().next().unwrap_or("") {
                "c_exact" => format!("c_exact {} {}", parts[2], parts[3]),
                "period" => "period 16116".into(),
                "scale" => "scale 1.0511037747648835 -3322".into(), // 1e-1000
                "guard" => "guard 1.0763302653592406 -3332".into(), // 1e-1003
                _ => line.to_string(),
            })
            .collect();
        lines.push("max_dc 1.0511037747648835 -3320".into()); // 4e-1000
        let z = Zone::parse(&with_exact_lines(&lines.join("\n"))).unwrap();
        let mut v = View {
            center_re: parts[2].into(),
            center_im: parts[3].into(),
            width: "5e-1000".into(),
            rotation: 0.3,
        };
        let p = params(&[Column::Nu]);
        assert!(zone_covers(&v, &p, &z).unwrap());
        // Increase the magnitude of the negative real centre by 1e-999 (ten sizes).
        let (prefix, fraction) = v.center_re.split_once('.').unwrap();
        let mut digits = fraction.as_bytes().to_vec();
        let mut i = 998;
        loop {
            if digits[i] != b'9' {
                digits[i] += 1;
                break;
            }
            digits[i] = b'0';
            i -= 1;
        }
        v.center_re = format!("{prefix}.{}", String::from_utf8(digits).unwrap());
        let (plane, _) = setup(&v, &p).unwrap();
        let off = centre_offset(&v, &z, &plane).unwrap();
        assert!((off.scaled.0.abs() - 10.0).abs() < 1e-9);
        assert!(!zone_covers(&v, &p, &z).unwrap());
    }

    #[test]
    fn fx_keeps_values_far_outside_f64() {
        let (a, b) = (Fx::new(Cx(0.3, -1.7), -3000), Fx::new(Cx(-2.5, 0.25), 3000));
        let p = a.mul(b).to_cx();
        let q = Cx(0.3, -1.7).mul(Cx(-2.5, 0.25));
        assert!(
            (p.0 - q.0).abs() < 1e-15 && (p.1 - q.1).abs() < 1e-15,
            "{p:?} {q:?}"
        );
        let s = a
            .add(Fx::new(Cx(0.1, 0.0), -3000))
            .sub(a)
            .mul(Fx::new(Cx(1.0, 0.0), 3000))
            .to_cx();
        assert!((s.0 - 0.1).abs() < 1e-15 && s.1.abs() < 1e-15, "{s:?}");
        let r = a.mul(a.inv()).to_cx();
        assert!((r.0 - 1.0).abs() < 1e-15 && r.1.abs() < 1e-15, "{r:?}");
        assert_eq!(a.to_cx(), Cx::default());
        let w = Fx::polar(-1500.0, 0.7);
        assert!((w.ln_abs() + 1500.0).abs() < 1e-12 && (w.arg() - 0.7).abs() < 1e-12);
    }

    /// `z` with its f64 constants copied into the deep (Fx) representation.
    fn as_deep(mut z: Zone) -> Zone {
        let fx = |a: &Cx| Fx::new(*a, 0);
        z.deep = Some(Deep {
            bis: z
                .bis
                .iter()
                .map(|r| std::array::from_fn(|j| r.get(j).map_or(Fx::default(), fx)))
                .collect(),
            z24ma: fx(&z.z24ma),
            phi: z.phi.iter().map(fx).collect(),
        });
        z
    }

    #[test]
    fn deep_path_matches_the_f64_path_on_v0() {
        let (old, new) = (v0(), as_deep(v0()));
        let p = params(&[Column::Nu, Column::De, Column::Normal]);
        for w in ["1e-36", "2e-48"] {
            let v = view(w);
            let (_, a, _, sa) = render_zone(&v, &p, &old).unwrap();
            let (_, b, _, sb) = render_zone(&v, &p, &new).unwrap();
            assert_eq!(a.class, b.class, "{w}");
            assert_eq!(
                (sa.returns, sa.jumps, sa.patches),
                (sb.returns, sb.jumps, sb.patches),
                "{w}"
            );
            let (an, bn) = (a.nu.as_ref().unwrap(), b.nu.as_ref().unwrap());
            let (ad, bd) = (a.de.as_ref().unwrap(), b.de.as_ref().unwrap());
            let (am, bm) = (a.normal.as_ref().unwrap(), b.normal.as_ref().unwrap());
            for i in 0..a.class.len() {
                if a.class[i].kind() != Some(fd_samples::Kind::Escaped) || ad[i] < 1e-3 {
                    continue;
                }
                // fd compare's displacement, 1000x tighter than its 1e-3 px gate.
                let px = (an[i] - bn[i]).abs() * f64::from(ad[i]) * std::f64::consts::LN_2 / 2.0;
                assert!(px <= 1e-6, "{w} {i}: {px} px");
                assert!(
                    (bd[i] / ad[i] - 1.0).abs() < 1e-5,
                    "{w} {i}: de {} {}",
                    ad[i],
                    bd[i]
                );
                let da = am[i].wrapping_sub(bm[i]).min(bm[i].wrapping_sub(am[i]));
                assert!(da <= 1, "{w} {i}: normal {} {}", am[i], bm[i]);
            }
        }
    }

    /// Zone text plus `_x` lines copied from the v0 core zone's f64 constants.
    fn with_exact_lines(text: &str) -> String {
        let core = include_str!("../../../bench/zones/v0-core.zone");
        let mut out = text.to_string();
        for l in core.lines().filter(|l| l.starts_with("biseries ")) {
            let f: Vec<&str> = l.split_whitespace().collect();
            out.push_str(&format!(
                "\nbiseries_x {} {} {} {} 0",
                f[1], f[2], f[3], f[4]
            ));
        }
        let z24 = core
            .lines()
            .find(|l| l.starts_with("z24_minus_alpha "))
            .unwrap();
        out.push_str(&format!(
            "\nz24_minus_alpha_x {} 0",
            &z24["z24_minus_alpha ".len()..]
        ));
        out
    }

    #[test]
    fn deep_zone_needs_exact_lines() {
        let text = include_str!("../../../bench/zones/v0-core.zone");
        let broken: String = text
            .lines()
            .map(|l| {
                if l.starts_with("biseries 2 0 ") {
                    "biseries 2 0 inf -inf".to_string()
                } else {
                    l.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(Zone::parse(&broken).unwrap_err().contains("_x"));
        let z = Zone::parse(&with_exact_lines(&broken)).unwrap();
        assert!(z.deep.is_some());
        assert!(v0().deep.is_none());
    }
}
