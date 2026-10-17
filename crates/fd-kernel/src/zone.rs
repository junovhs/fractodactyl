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
//! The constants (one zone file, written by
//! tools/research/misiurewicz/koenigs_bench_consts.py and tail_patches.py) are computed
//! once at high precision. `dz/dc` is carried through every stage by the chain rule, so
//! `de` and `normal` columns come out as from the perturbation kernel.
//!
//! The per-pixel stages exist only as WGSL (crates/fd-cli/shaders/zone*.wgsl, DEC-28;
//! CLEA-04 deleted the CPU copy). This module keeps the CPU side: zone file parsing,
//! the coverage gate ([`zone_covers`]) and the exact offset and constant packing.
//!
//! Contract (DEC-10): **validity** is frame-level: every sample within `max_dc` of `C`
//! (default the loop guard, 1e-28; whole frames at 1080p were checked against per-frame
//! BLA down to 2e-48: METHOD.md PROB-09). **Error**: measured, not bounded (PROB-13 is
//! the certification). **Fallback**: a frame outside the zone, or one the GPU declines,
//! is rendered by the caller with the perturbation kernel.
//! A zone with `diag` lines also refuses a frame whose first-return truncation shift
//! could exceed [`SHIFT_PX`] (PROB-20): decided before rendering (DEC-19).
use crate::grid::{header, setup, Params};
use crate::number::{Cx, Fx};
use crate::view::Plane;
use fd_fixed::{exp2i, Fixed};
use fd_samples::{Column, Header, View};

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

/// A deep zone's biseries and Koenigs entry offset as mantissa + exponent (`biseries_x`
/// and `z24_minus_alpha_x` lines): at the k = 7676 rung the coefficients reach ~1e500
/// and the offset ~1e-500, so the GPU reads them with exponents (GPU-23).
#[derive(Clone, Debug)]
struct Deep {
    /// `bis[i][j]`: coefficient of `u^i v^j`.
    bis: Vec<[Fx; 8]>,
    z24ma: Fx,
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
            z.deep = Some(Deep { bis, z24ma });
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
    /// It declines mid-band and extended-range frames, which have their own inputs.
    pub fn gpu_f64_input(&self, view: &View, p: &Params) -> Result<(Header, Vec<f64>), String> {
        if self.deep.is_some() {
            return Err("FP64 Vulkan zone needs a covered, f64-range deep frame".into());
        }
        self.gpu_deep_input(view, p).map(|(h, v, _)| (h, v))
    }

    /// [`Zone::gpu_f64_input`]'s layout for an extended-range zone (GPU-23): the same
    /// slots, each a binary64 mantissa times `2^exps[slot]`. Scale, guard, the output
    /// pixel, the biseries and the Koenigs entry offset leave f64's range (k = 7676:
    /// 2^-3322..2^1666); every other slot has exponent 0. Only kernels that read the
    /// exponents (f32only) take it.
    pub fn gpu_extended_input(
        &self,
        view: &View,
        p: &Params,
    ) -> Result<(Header, Vec<f64>, Vec<i32>), String> {
        if self.deep.is_none() {
            return Err("not an extended-range zone".into());
        }
        self.gpu_deep_input(view, p)
    }

    /// Whether the zone's constants leave f64's range (the deep, mantissa + exponent
    /// tier; GPU-23).
    pub fn extended(&self) -> bool {
        self.deep.is_some()
    }

    fn gpu_deep_input(
        &self,
        view: &View,
        p: &Params,
    ) -> Result<(Header, Vec<f64>, Vec<i32>), String> {
        let (plane, tier) = setup(view, p)?;
        let off = centre_offset(view, self, &plane)?;
        if !deep_covers(&plane, p, self, off) {
            return Err("FP64 Vulkan zone needs a covered, f64-range deep frame".into());
        }
        if p.columns.has(Column::Bound) {
            return Err("FP64 Vulkan zone has no certified bounds".into());
        }
        if p.max_iter > i32::MAX as u64 || self.period > i32::MAX as u64 {
            return Err("FP64 Vulkan zone iteration count exceeds i32".into());
        }
        // Values that may leave f64's range, as (mantissa, binary exponent).
        let deep = self.deep.as_ref();
        let wide = |m: f64, e: i64| match deep {
            Some(_) => (m, e),
            None => (m * exp2i(e), 0),
        };
        let (scale, scale_e) = wide(self.scale.mant, self.scale.exp2);
        let (guard, guard_e) = wide(self.guard.mant, self.guard.exp2);
        let (h, h_e) = wide(plane.h_m / self.scale.mant, plane.h_e - self.scale.exp2);
        let (px, px_e) = wide(plane.h_m, plane.h_e);
        // The centre offset in v units, from the exact offset where it leaves f64.
        let (offset, offset_e) = match deep {
            Some(_) => (
                off.exact.m.scale(1.0 / self.scale.mant),
                off.exact.e - self.scale.exp2,
            ),
            None => (off.scaled, 0),
        };
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
        let z24ma = deep.map_or(Fx::new(self.z24ma, 0), |d| d.z24ma);
        let mut v = vec![
            scale,
            guard,
            self.r0,
            self.period as f64,
            self.deg as f64,
            p.max_iter as f64,
            p.nx as f64,
            p.ny as f64,
            offset.0,
            offset.1,
            h,
            self.c.0,
            self.c.1,
            self.alpha.0,
            self.alpha.1,
            self.rho.abs().ln(),
            self.rho.1.atan2(self.rho.0),
            z24ma.m.0,
            z24ma.m.1,
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
        let mut exps = vec![0i64; v.len()];
        exps[0] = scale_e;
        exps[1] = guard_e;
        exps[8] = offset_e;
        exps[9] = offset_e;
        exps[10] = h_e;
        exps[17] = z24ma.e;
        exps[18] = z24ma.e;
        exps[30] = px_e;
        if deep.is_none() {
            // The f64 kernels read z24ma as one binary64 per component.
            v[17] = self.z24ma.0;
            v[18] = self.z24ma.1;
            exps[17] = 0;
            exps[18] = 0;
        }
        // A fixed 8x8 complex biseries and the 23 orbit points precede the variable data.
        for i in 0..8 {
            for j in 0..8 {
                let a = match deep {
                    _ if i > self.deg || j > self.deg => Fx::default(),
                    Some(d) => d.bis[i][j],
                    None => Fx {
                        m: self.bis[i][j],
                        e: 0,
                    },
                };
                v.extend([a.m.0, a.m.1]);
                exps.extend([a.e, a.e]);
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
        exps.resize(v.len(), 0);
        // Leave headroom for products of two extended values in i32 exponents.
        let Ok(exps) = exps
            .into_iter()
            .map(|e| {
                i32::try_from(e)
                    .ok()
                    .filter(|e| e.abs() < 1 << 28)
                    .ok_or(())
            })
            .collect::<Result<Vec<i32>, ()>>()
        else {
            return Err("zone constant exponents exceed the GPU's i32 range".into());
        };
        let cols = p.columns.with(Column::Class);
        let mut hd = header(view, p, cols, &plane, tier);
        hd.kernel = format!("zone-vulkan-f64/1 P={}", self.period);
        Ok((hd, v, exps))
    }

    /// Constants for the wgpu mid-band kernel (GPU-22) when the coverage gate gives
    /// `view` the mid operator; `Err` otherwise. Every pixel's parameter offset
    /// dc = off + (ux, uy) h is formed on the device from the frame's centre offset to
    /// the nucleus.
    pub fn gpu_mid_input(&self, view: &View, p: &Params) -> Result<(Header, Vec<f64>), String> {
        let (plane, tier) = setup(view, p)?;
        let off = centre_offset(view, self, &plane)?;
        if deep_covers(&plane, p, self, off) || !mid_covers(&plane, p, self, off) {
            return Err("not a mid-band zone frame".into());
        }
        if !p.columns.needs_derivative() || p.columns.has(Column::Bound) {
            return Err("the wgpu mid kernel computes nu, de and normal only".into());
        }
        if p.max_iter > i32::MAX as u64 || self.period > i32::MAX as u64 {
            return Err("wgpu mid zone iteration count exceeds i32".into());
        }
        let m = self.mid.as_ref().expect("mid_covers");
        let mut v = vec![
            p.nx as f64,
            p.ny as f64,
            p.max_iter as f64,
            p.escape_radius * p.escape_radius,
            plane.h(),
            plane.h() * p.ss as f64,
            off.absolute.0,
            off.absolute.1,
            self.c.0,
            self.c.1,
            view.rotation.cos(),
            view.rotation.sin(),
            self.period as f64,
        ];
        m.pack(&mut v, 32);
        if !v.iter().all(|x| x.is_finite()) {
            return Err("wgpu mid zone constants are non-finite".into());
        }
        let cols = p.columns.with(Column::Class);
        let mut hd = header(view, p, cols, &plane, tier);
        hd.kernel = format!("zone-mid-vulkan-f64/1 P={}", self.period);
        Ok((hd, v))
    }
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
    /// biseries degree, for samples out to `r` with spacing `h` (both in `v` units, given
    /// as `log2 r` and `log2 h`: at the k = 7676 rung they are ~2^-1650, below f64).
    /// `None` if the zone has no `diag` lines; infinite where the return map's
    /// derivative cannot be bounded away from 0.
    ///
    /// Dropped terms: `sum_{n > deg} |S_n| r^n`. Derivative `dM/dv = sum n S_n v^(n-1)`,
    /// bounded below at `|v| = r` by its largest term minus the others. The ratio grows
    /// like `r^4`, so the frame's outer radius is where it is largest. Not certified
    /// (PROB-13); later returns are not covered (their inputs do not depend on the frame).
    fn truncation_shift(&self, lr: f64, lh: f64) -> Option<f64> {
        if self.diag.is_empty() {
            return None;
        }
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
        Some((rsum - top - (1.0 - others).log2() - lh).exp2())
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
    // log2 of the spacing and outer radius in v units, exact where they underflow f64.
    let lh = (plane.h_m / zone.scale.mant).log2() + (plane.h_e - zone.scale.exp2) as f64;
    let (lo, ls) = (off.scaled.abs().log2(), lh + diagonal.log2());
    let lr = lo.max(ls) + (lo.min(ls) - lo.max(ls)).exp2().ln_1p() / std::f64::consts::LN_2;
    let shift = zone.truncation_shift(lr, lh);
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

#[cfg(test)]
mod tests {
    use super::*;

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
            assert!(z.gpu_mid_input(&v, &p).is_err(), "{width}");
        }
        for width in ["1.05e-6", "1.21e-11", "1.88e-26", "1.02e-31"] {
            assert!(zone_covers(&view(width), &p, &z).unwrap(), "{width}");
        }
    }

    #[test]
    fn zone_refuses_views_it_does_not_cover() {
        let (z, p) = (v0(), params(&[Column::Nu]));
        assert!(!zone_covers(&view("1e-20"), &p, &z).unwrap());
        assert!(z.gpu_f64_input(&view("1e-20"), &p).is_err());
        assert!(z.gpu_mid_input(&view("1e-20"), &p).is_err());
        let mut far = view("1e-40");
        far.center_re = "-0.7432918908".into();
        assert!(!zone_covers(&far, &p, &z).unwrap());
        let bound = params(&[Column::Nu, Column::Bound]);
        assert!(z.gpu_f64_input(&view("1e-40"), &bound).is_err());
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
            1.70834e-28f64 / 1280.0 / 1e-25,
            1.70834e-28f64 / 1280.0 * 640f64.hypot(360.0) / 1e-25,
        );
        let s = z.truncation_shift(r.log2(), h.log2()).unwrap();
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
    fn v0_binary_scales_keep_the_same_constants() {
        let old = v0();
        let text = include_str!("../../../bench/zones/v0-core.zone")
            .replace("scale 1e-25", "scale 1.9342813113834068 -84")
            .replace("guard 1e-28", "guard 1.9807040628566084 -94");
        let new = Zone::parse(&text).unwrap();
        let (v, p) = (
            view("1e-36"),
            params(&[Column::Nu, Column::De, Column::Normal]),
        );
        // The kernel reads nothing else, so equal constants mean equal samples.
        let (_, a) = old.gpu_f64_input(&v, &p).unwrap();
        let (_, b) = new.gpu_f64_input(&v, &p).unwrap();
        assert_eq!(
            a.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
            b.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
        );
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
