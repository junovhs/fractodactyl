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
use crate::sample::Outcome;
use crate::store::{Row, Store};
use crate::view::Plane;
use fd_fixed::{exp2i, Fixed};
use fd_samples::{Column, Header, Samples, View};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

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
        Cx(
            (self.0 * o.0 + self.1 * o.1) / d,
            (self.1 * o.0 - self.0 * o.1) / d,
        )
    }
}

/// Complex `m * 2^e` with `1 <= max(|m.re|, |m.im|) < 2` (or `m = 0`): the deep path's
/// number, for zones whose constants, states or derivatives leave f64's range (DEC-21).
#[derive(Clone, Copy, Default, Debug)]
struct Fx {
    m: Cx,
    e: i64,
}

impl Fx {
    const ONE: Fx = Fx {
        m: Cx(1.0, 0.0),
        e: 0,
    };

    #[inline(always)]
    fn new(m: Cx, e: i64) -> Fx {
        Fx { m, e }.norm()
    }
    #[inline(always)]
    fn norm(self) -> Fx {
        let a = self.m.0.abs().max(self.m.1.abs());
        if a == 0.0 || !a.is_finite() {
            return Fx {
                m: self.m,
                e: if a == 0.0 { 0 } else { self.e },
            };
        }
        let k = ((a.to_bits() >> 52) & 0x7ff) as i64 - 1023;
        if k == -1023 {
            return Fx {
                m: self.m.scale(exp2i(64)),
                e: self.e - 64,
            }
            .norm();
        }
        Fx {
            m: self.m.scale(exp2i(-k)),
            e: self.e + k,
        }
    }
    #[inline(always)]
    fn is_zero(self) -> bool {
        self.m.0 == 0.0 && self.m.1 == 0.0
    }
    #[inline(always)]
    fn add(self, o: Fx) -> Fx {
        if o.is_zero() {
            return self;
        }
        if self.is_zero() {
            return o;
        }
        let (a, b) = if self.e >= o.e { (self, o) } else { (o, self) };
        let d = a.e - b.e;
        if d > 110 {
            return a;
        }
        Fx {
            m: a.m.add(b.m.scale(exp2i(-d))),
            e: a.e,
        }
        .norm()
    }
    #[inline(always)]
    fn sub(self, o: Fx) -> Fx {
        self.add(Fx {
            m: o.m.scale(-1.0),
            e: o.e,
        })
    }
    #[inline(always)]
    fn mul(self, o: Fx) -> Fx {
        Fx {
            m: self.m.mul(o.m),
            e: self.e + o.e,
        }
        .norm()
    }
    #[inline(always)]
    fn inv(self) -> Fx {
        Fx {
            m: Cx(1.0, 0.0).div(self.m),
            e: -self.e,
        }
        .norm()
    }
    /// Nearest f64 pair: 0 below f64's range, infinite above it.
    #[inline(always)]
    fn to_cx(self) -> Cx {
        if self.is_zero() {
            return Cx::default();
        }
        if self.e < -1100 {
            return Cx::default();
        }
        self.m.scale(exp2i(self.e))
    }
    #[inline(always)]
    fn ln_abs(self) -> f64 {
        self.m.abs().ln() + self.e as f64 * std::f64::consts::LN_2
    }
    #[inline(always)]
    fn arg(self) -> f64 {
        self.m.1.atan2(self.m.0)
    }
    /// `exp(ln_mag + i angle)`.
    fn polar(ln_mag: f64, angle: f64) -> Fx {
        let e = (ln_mag / std::f64::consts::LN_2).floor();
        let r = (ln_mag - e * std::f64::consts::LN_2).exp();
        Fx::new(Cx(r * angle.cos(), r * angle.sin()), e as i64)
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
        };
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
                // Lean-baseline reference orbit of the research bench: not used here.
                "ref" => {}
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
pub fn zone_covers(view: &View, p: &Params, zone: &Zone) -> Result<bool, String> {
    let (plane, _) = setup(view, p)?;
    let off = centre_offset(view, zone, &plane)?;
    let diagonal = (f64::from(p.nx) / 2.0).hypot(f64::from(p.ny) / 2.0);
    let spacing = ZoneSize {
        mant: plane.h_m,
        exp2: plane.h_e,
    }
    .over(zone.scale);
    let inside = if zone.f64_geometry(&plane) {
        off.absolute.abs() + plane.h() * diagonal <= zone.max_dc.to_f64()
    } else {
        off.scaled.abs() + spacing * diagonal <= zone.max_dc.over(zone.scale)
    };
    let shift = zone.truncation_shift(off.scaled.abs() + spacing * diagonal, spacing);
    Ok(inside && shift.is_none_or(|s| s <= SHIFT_PX))
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
                            if let Some(deep) = &zone.deep {
                                let v = off
                                    .exact
                                    .add(Fx::new(Cx(ux * plane.h_m, uy * plane.h_m), plane.h_e))
                                    .mul(inv_scale);
                                let (o, w) = if deriv {
                                    pixel_deep::<true>(zone, deep, v, p.max_iter, r2, &mut st)
                                } else {
                                    pixel_deep::<false>(zone, deep, v, p.max_iter, r2, &mut st)
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
    let stats = Stats {
        reference_len: 0,
        reference_seconds: 0.0,
        iterations: work.into_inner(),
    };
    Ok((hd, s, stats, total.into_inner().unwrap()))
}

/// One sample at `c = C + v * scale`. With `D`, `dz/dc` is carried by the chain rule.
/// Returns the outcome and the work it took (biseries returns + approach + jump +
/// patch + plain steps, each counted once).
#[inline]
fn pixel<const D: bool>(
    k: &Zone,
    v: Cx,
    max_iter: u64,
    r2: f64,
    st: &mut ZoneStats,
) -> (Outcome, u64) {
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
    let scale = k.scale.to_f64();
    let guard_f64 = k.guard.to_f64();
    let legacy = scale > 0.0 && guard_f64 > 0.0;
    let guard = k.guard.over(k.scale);
    while (legacy && u.abs() * scale <= guard_f64) || (!legacy && u.abs() <= guard) {
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
        if (su.norm2() < 1.0 && step.norm2() <= 1e-24 * u.norm2())
            || (work >= 16 && work.is_power_of_two() && attracted(&b, k.deg, u))
        {
            st.returns += 1;
            return (Outcome::Interior { n }, work + 1);
        }
        u = s;
        n += k.period;
        work += 1;
        st.returns += 1;
    }
    // Stage 2: approach against C's critical orbit (fixed parameter C).
    let mut d = u.scale(scale);
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
        if let Some((zp, dzs, m)) = patch(k, w0.abs().ln(), w0.1.atan2(w0.0), j) {
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

/// [`pixel`] for a deep zone: the same four stages with states, constants and `dz/dc`
/// in [`Fx`]. `v = (c - C) / scale`. Returns `dz/dc` as mantissa and `dexp`.
fn pixel_deep<const D: bool>(
    k: &Zone,
    x: &Deep,
    v: Fx,
    max_iter: u64,
    r2: f64,
    st: &mut ZoneStats,
) -> (Outcome, u64) {
    let mut b = [Fx::default(); 8];
    let mut bv = [Fx::default(); 8];
    let mut pv = [Cx(1.0, 0.0); 8];
    for j in 1..=k.deg {
        pv[j] = pv[j - 1].mul(v.m);
    }
    for i in 0..=k.deg {
        b[i] = poly_fx(&x.bis[i], k.deg - i, &pv, v.e, 0);
        if D {
            bv[i] = poly_fx(&x.bis[i], k.deg - i, &pv, v.e, 1);
        }
    }
    b[0] = b[0].add(Fx::new(k.bias, 0));
    // Stage 1: returns.
    let (mut u, mut du) = (v, Fx::ONE);
    let mut n: u64 = 1;
    let mut work = 0u64;
    let guard = k.guard.over(k.scale);
    while u.to_cx().abs() <= guard {
        if n + k.period > max_iter {
            return (Outcome::Unresolved, work);
        }
        // B(u), B'(u), dB/dv(u) term by term: u^i is u.m^i * 2^(i u.e), so each sum
        // aligns and normalises once instead of after every Horner step.
        let mut pw = [Cx(1.0, 0.0); 8];
        for i in 1..=k.deg {
            pw[i] = pw[i - 1].mul(u.m);
        }
        let s = poly_fx(&b, k.deg, &pw, u.e, 0);
        let su = poly_fx(&b, k.deg, &pw, u.e, 1);
        let sv = if D {
            poly_fx(&bv, k.deg, &pw, u.e, 0)
        } else {
            Fx::default()
        };
        let step = s.sub(u);
        if D {
            du = su.mul(du).add(sv);
        }
        // As in `pixel`; `attracted` runs on the map conjugated by 2^u.e, which is f64.
        let settled = su.to_cx().norm2() < 1.0
            && step.m.norm2() * exp2i(2 * (step.e - u.e)) <= 1e-24 * u.m.norm2();
        if settled || (work >= 16 && work.is_power_of_two() && attracted_deep(&b, k.deg, u)) {
            st.returns += 1;
            return (Outcome::Interior { n }, work + 1);
        }
        u = s;
        n += k.period;
        work += 1;
        st.returns += 1;
    }
    // Stage 2: approach against C's critical orbit. Next to the O(1) orbit, d and dz/dc
    // need no per-step normalisation: each is a mantissa with an exponent that is
    // rescaled only when the mantissa drifts out of [2^-32, 2^32] (as in scaled.rs).
    let mut d = Lazy::from(u.mul(Fx::new(Cx(k.scale.mant, 0.0), k.scale.exp2)));
    let mut g = Lazy::from(du);
    for z in &k.orbit {
        if D {
            g = g.step(z.add(d.to_cx()));
        }
        d = d.square_plus_2z(*z);
    }
    let d = d.fx();
    du = g.fx();
    n += APPROACH as u64;
    work += APPROACH as u64;
    // Stage 3: Koenigs jump from an entry offset h0 that may be far below f64's range.
    let h0 = x.z24ma.add(d);
    let mut z = k.alpha.add(h0.to_cx());
    if h0.to_cx().abs() < k.r0 {
        let (w0, dphi) = series_d_fx(&x.phi, h0);
        let (lw, aw) = (w0.ln_abs(), w0.arg());
        let (lr, ar) = (k.rho.abs().ln(), k.rho.1.atan2(k.rho.0));
        let j = ((k.r0.ln() - lw) / lr).floor();
        if let Some((zp, dzs, m)) = patch(k, lw, aw, j) {
            if D {
                du = Fx::new(dzs, 0).mul(dphi).mul(du).mul(w0.inv());
            }
            z = zp;
            n += 2 * j as u64 + m;
            st.jumps += 1;
            st.patches += 1;
        } else if j > 0.0 {
            let w = Fx::polar(lw + j * lr, aw + j * ar).to_cx();
            let (hw, dpsi) = series_d(&k.psi, w);
            if D {
                du = Fx::new(dpsi, 0)
                    .mul(Fx::polar(j * lr, j * ar))
                    .mul(dphi)
                    .mul(du);
            }
            z = k.alpha.add(hw);
            n += 2 * j as u64;
            st.jumps += 1;
        }
        work += 1;
    }
    // Stage 4: plain steps at C.
    let mut g = Lazy::from(du);
    loop {
        if n >= max_iter {
            return (Outcome::Unresolved, work);
        }
        if D {
            g = g.step(z);
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
                dr: g.m.0,
                di: g.m.1,
                dexp: g.e,
                ez: f64::INFINITY,
                ed: f64::INFINITY,
            };
            return (o, work);
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

/// `sum_i c_i u^i` (`d = 0`) or its derivative `sum_i i c_i u^(i-1)` (`d = 1`), with
/// `pw[i] = u.m^i` and `u = u.m 2^ue`.
#[inline(always)]
fn poly_fx(c: &[Fx; 8], deg: usize, pw: &[Cx; 8], ue: i64, d: usize) -> Fx {
    let terms = || {
        c[..=deg]
            .iter()
            .enumerate()
            .skip(d)
            .filter(|(_, a)| !a.is_zero())
    };
    let Some(top) = terms().map(|(i, a)| a.e + (i - d) as i64 * ue).max() else {
        return Fx::default();
    };
    let mut sum = Cx::default();
    for (i, a) in terms() {
        let k = if d == 1 { i as f64 } else { 1.0 };
        sum = sum.add(
            a.m.mul(pw[i - d])
                .scale(k * exp2i(a.e + (i - d) as i64 * ue - top)),
        );
    }
    Fx::new(sum, top)
}

/// [`attracted`] for an [`Fx`] state: the return map conjugated by `x = u / 2^u.e`
/// (coefficients `b_i 2^(u.e (i - 1))`), which keeps the cycle test in f64.
fn attracted_deep(b: &[Fx; 8], deg: usize, u: Fx) -> bool {
    let mut bt = [Cx::default(); 8];
    for (i, (t, a)) in bt.iter_mut().zip(b).enumerate().take(deg + 1) {
        *t = Fx {
            m: a.m,
            e: a.e + u.e * (i as i64 - 1),
        }
        .to_cx();
    }
    attracted(&bt, deg, u.m)
}

/// [`series_d`] in [`Fx`].
fn series_d_fx(c: &[Fx], x: Fx) -> (Fx, Fx) {
    let (mut s, mut d) = (Fx::default(), Fx::default());
    for (i, a) in c.iter().enumerate().rev() {
        s = s.add(*a).mul(x);
        d = d.mul(x).add(a.mul(Fx::new(Cx((i + 1) as f64, 0.0), 0)));
    }
    (s, d)
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
