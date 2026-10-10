//! PROB-14 co-moving two-cycle jump for v0 mid-band pixels.
//! Valid for |dc| <= 1e-6, chart |w| <= 0.031 and an in-budget jump;
//! every declined operator falls back to full reference perturbation.
use super::Cx;
use crate::sample::Outcome;

const N: usize = 18;
const APPROACH: usize = 24;
const RADIUS: f64 = 0.03;
const MAX_DC: f64 = 1e-6;
// PROB-14 was whole-frame checked at 1e-6 and 1e-11..1e-26. The intervening
// annulus failed the KERN-02 derivative/normal gate, so its jumps abstain.
const OUTER_MIN_DC: f64 = 1e-7;
const INNER_MAX_DC: f64 = 7e-12;
// Near the minibrot, returns must be handled by the deep operator. Its
// truncation guard may decline before it can take over (PROB-20).
const RETURN_HANDOVER_DC: f64 = 5e-27;

/// Empirical operator domain, not a certified error bound (DEC-10/17).
fn chart_parameter_ok(dc: f64) -> bool {
    dc.is_finite() && (dc <= INNER_MAX_DC || (OUTER_MIN_DC..=MAX_DC).contains(&dc))
}

#[derive(Clone, Debug, Default)]
pub(super) struct Mid {
    z: Vec<Cx>,
    reference: Vec<Cx>,
    u: Cx,
    s: Cx,
    p: Cx,
    sign: f64,
    lambda: Cx,
    k: Vec<Cx>,
    kp: Vec<Cx>,
    powers: Vec<Cx>,
    enabled: bool,
}

impl Mid {
    pub(super) fn reference(&mut self, index: usize, z: Cx) -> Result<(), String> {
        if index != self.reference.len() {
            return Err("out-of-order ref index".into());
        }
        self.reference.push(z);
        Ok(())
    }

    /// Consume one mid-band constant; indexed vectors must have no gaps or duplicates.
    pub(super) fn add(&mut self, f: &[&str]) -> Result<(), String> {
        self.enabled = true;
        let num = |i: usize| -> Result<f64, String> {
            let v = f
                .get(i)
                .ok_or("missing mid constant")?
                .parse::<f64>()
                .map_err(|_| "bad mid number")?;
            if !v.is_finite() {
                return Err("nonfinite mid constant".into());
            }
            Ok(v)
        };
        let cx = |i: usize| -> Result<Cx, String> { Ok(Cx(num(i)?, num(i + 1)?)) };
        let indexed = |v: &mut Vec<Cx>| -> Result<(), String> {
            let i = num(1)?;
            if i != v.len() as f64 {
                return Err("out-of-order mid index".into());
            }
            v.push(cx(2)?);
            Ok(())
        };
        match f[0] {
            "mid_z" => indexed(&mut self.z)?,
            "mid_u" => self.u = cx(1)?,
            "mid_s" => self.s = cx(1)?,
            "mid_p" => self.p = cx(1)?,
            "mid_sign" => self.sign = num(1)?,
            "mid_lambda" => self.lambda = cx(1)?,
            "mid_k" => indexed(&mut self.k)?,
            "mid_kp" => indexed(&mut self.kp)?,
            "mid_t" => indexed(&mut self.powers)?,
            _ => return Err("unknown mid constant".into()),
        }
        Ok(())
    }

    pub(super) fn finish(self, period: u64) -> Result<Option<Self>, String> {
        if !self.enabled {
            return Ok(None);
        }
        if self.z.len() != APPROACH + 1
            || self.reference.len() as u64 != period
            || self.k.len() != N + 1
            || self.kp.len() != N + 1
            || self.powers.len() != 512
            || self.sign.abs() != 1.0
            || self.s.abs() == 0.0
            || self.lambda.abs() <= 1.0
            || self.powers[0] != Cx(1.0, 0.0)
        {
            return Err(
                "incomplete mid constants (need z[0..24], ref[0..P), k/kp[0..18], t[0..511])"
                    .into(),
            );
        }
        Ok(Some(self))
    }

    pub(super) fn covers(&self, radius: f64, escape_radius: f64) -> bool {
        // The largest possible |c-C| must fit one validated operator regime.
        // Above OUTER_MIN_DC, individual pixels in the unvalidated annulus
        // abstain; below INNER_MAX_DC, the whole frame is in the inner regime.
        radius.is_finite()
            && radius >= RETURN_HANDOVER_DC
            && chart_parameter_ok(radius)
            && escape_radius == 1e10
    }
}

#[inline]
fn eval(a: &[Cx], w: Cx) -> (Cx, Cx) {
    let (mut v, mut d) = (Cx::default(), Cx::default());
    for x in a.iter().rev() {
        d = d.mul(w).add(v);
        v = v.mul(w).add(*x);
    }
    (v, d)
}

fn escaped(n: u64, z: Cx, dz: Cx) -> Outcome {
    Outcome::Escaped {
        n,
        zr: z.0,
        zi: z.1,
        dr: dz.0,
        di: dz.1,
        dexp: 0,
        ez: f64::INFINITY,
        ed: f64::INFINITY,
    }
}

/// Unaccelerated orbit at the pixel's own parameter, with periodic rebasing.
fn fallback<const D: bool>(m: &Mid, dc: Cx, max_iter: u64, r2: f64) -> (Outcome, u64, bool) {
    let (mut d, mut dz, mut at) = (Cx::default(), Cx::default(), 0usize);
    for n in 1..=max_iter {
        let z = m.reference[at].add(d);
        if D {
            dz = z.scale(2.0).mul(dz).add(Cx(1.0, 0.0));
        }
        d = m.reference[at].scale(2.0).mul(d).add(d.mul(d)).add(dc);
        at += 1;
        if at == m.reference.len() {
            at = 0;
        }
        let z = m.reference[at].add(d);
        let a = z.norm2();
        if a > r2 {
            if a.is_finite() && (!D || (dz.abs().is_finite() && dz.abs() > 0.0)) {
                return (escaped(n, z, dz), n, false);
            }
            return (Outcome::Unresolved, n, false);
        }
        if a < d.norm2() {
            d = z;
            at = 0;
        }
    }
    (Outcome::Unresolved, max_iter, false)
}

/// One mid-band operator: 24 perturbation steps, a 2j-step chart jump carrying
/// dz/dc, then direct iteration at the pixel's c. Work is executed steps, not 2j.
#[inline]
pub(super) fn pixel<const D: bool>(
    m: &Mid,
    c0: Cx,
    dc: Cx,
    max_iter: u64,
    r2: f64,
    h: f64,
) -> (Outcome, u64, bool) {
    let fallback = || fallback::<D>(m, dc, max_iter, r2);
    if !chart_parameter_ok(dc.abs()) || max_iter <= APPROACH as u64 {
        return fallback();
    }
    let (mut d, mut dz) = (Cx::default(), Cx::default());
    for z in &m.z[..APPROACH] {
        let at = z.add(d);
        if D {
            dz = at.scale(2.0).mul(dz).add(Cx(1.0, 0.0));
        }
        d = z.scale(2.0).mul(d).add(d.mul(d)).add(dc);
    }
    // Closed-form shift p(c)-p(C), without subtracting two rounded cycle points.
    let mut ds = dc.scale(-4.0).div(m.s.scale(2.0));
    for _ in 0..2 {
        ds = dc.scale(-4.0).div(m.s.scale(2.0).add(ds));
    }
    let dp = ds.scale(0.5 * m.sign);
    let pc = m.p.add(dp);
    let pprime = Cx(-1.0, 0.0).div(pc.scale(2.0).add(Cx(1.0, 0.0)));
    let u0 = m.u.add(d).sub(dp);
    let lam = m.lambda.add(dc.scale(4.0));
    if !(lam.abs() > 1.0 && u0.abs() < RADIUS / lam.abs() && u0.abs().is_finite()) {
        return fallback();
    }

    let mut k = [Cx::default(); N + 1];
    k.copy_from_slice(&m.k);
    for (a, b) in k.iter_mut().zip(&m.kp) {
        *a = a.add(dc.mul(*b));
    }
    let mut w0 = u0;
    for _ in 0..3 {
        let (v, derivative) = eval(&k, w0);
        if !(derivative.abs() > 0.0 && derivative.abs().is_finite()) {
            return fallback();
        }
        w0 = w0.sub(v.sub(u0).div(derivative));
    }
    let r = w0.abs();
    if !(r > 0.0 && r.is_finite()) {
        return fallback();
    }
    let jf = (RADIUS / r).ln() / lam.abs().ln();
    if !(jf.is_finite() && jf >= 1.0 && jf < m.powers.len() as f64) {
        return fallback();
    }
    let j = jf.floor() as usize;
    let n0 = APPROACH as u64 + 2 * j as u64;
    if n0 >= max_iter {
        return fallback();
    }
    // High-precision tabulated lambda(C)^j, with first-order parameter correction.
    let factor = Cx(1.0, 0.0).add(dc.scale(4.0 * j as f64).div(m.lambda));
    let lj = m.powers[j].mul(factor);
    let q = w0.mul(lj);
    if !(q.abs().is_finite() && q.abs() <= 0.031) {
        return fallback();
    }
    let (_, dkw) = eval(&k, w0);
    let (kq, dkq) = eval(&k, q);
    let (pcw, _) = eval(&m.kp, w0);
    let (pcq, _) = eval(&m.kp, q);
    if !(dkw.abs() > 0.0 && dkw.abs().is_finite()) {
        return fallback();
    }
    if D {
        let bracket = dz
            .sub(pprime)
            .sub(pcw)
            .add(w0.mul(dkw).mul(Cx(4.0 * j as f64, 0.0).div(lam)));
        dz = pprime.add(lj.mul(dkq).div(dkw).mul(bracket)).add(pcq);
    }
    let mut z = pc.add(kq);
    if !z.abs().is_finite() || (D && !dz.abs().is_finite()) {
        return fallback();
    }

    // The finish MUST use the pixel's own c, not the nucleus C (PROB-14).
    let c = c0.add(dc);
    let mut work = APPROACH as u64 + 1;
    for n in n0 + 1..=max_iter {
        if D {
            dz = z.scale(2.0).mul(dz).add(Cx(1.0, 0.0));
        }
        z = z.mul(z).add(c);
        work += 1;
        let a = z.norm2();
        if a > r2 {
            if !a.is_finite() || (D && !(dz.abs().is_finite() && dz.abs() > 0.0)) {
                return fallback();
            }
            // A long chaotic finish near the boundary needs the full orbit.
            if D && dc.abs() > 1e-8 {
                let de_px = 2.0 * a.sqrt() * (0.5 * a.ln()) / (dz.abs() * h);
                if de_px < 1.0 || !de_px.is_finite() {
                    return fallback();
                }
            }
            return (escaped(n, z, dz), work, true);
        }
    }
    (Outcome::Unresolved, work, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horner_transports_value_and_derivative() {
        let mut k = vec![Cx::default(); N + 1];
        k[1] = Cx(1.0, 0.0);
        k[2] = Cx(2.0, 0.0);
        let (v, d) = eval(&k, Cx(0.3, 0.0));
        assert!((v.0 - 0.48).abs() < 1e-14);
        assert!((d.0 - 2.2).abs() < 1e-14);
    }

    #[test]
    fn declined_jump_uses_own_parameter_and_derivative() {
        let m = Mid {
            reference: vec![Cx::default()],
            ..Mid::default()
        };
        let dc = Cx(0.5, 0.5);
        let (o, _, jumped) = pixel::<true>(&m, Cx::default(), dc, 100, 4.0, 1e-3);
        assert!(!jumped);
        let (mut z, mut dz) = (Cx::default(), Cx::default());
        let mut expected = None;
        for n in 1..=100 {
            dz = z.scale(2.0).mul(dz).add(Cx(1.0, 0.0));
            z = z.mul(z).add(dc);
            if z.norm2() > 4.0 {
                expected = Some((n, z, dz));
                break;
            }
        }
        let (n, z, dz) = expected.unwrap();
        match o {
            Outcome::Escaped {
                n: got,
                zr,
                zi,
                dr,
                di,
                ..
            } => {
                assert_eq!(got, n);
                assert!((zr - z.0).abs() < 1e-12 && (zi - z.1).abs() < 1e-12);
                assert!((dr - dz.0).abs() < 1e-12 && (di - dz.1).abs() < 1e-12);
            }
            _ => panic!("expected escape"),
        }
    }

    #[test]
    fn jump_reports_original_iterations_and_transports_derivative() {
        let mut m = Mid {
            z: vec![Cx::default(); APPROACH + 1],
            reference: vec![Cx::default()],
            u: Cx(1e-5, 0.0),
            s: Cx(1.0, 0.0),
            p: Cx::default(),
            sign: 1.0,
            lambda: Cx(2.0, 0.0),
            k: vec![Cx::default(); N + 1],
            kp: vec![Cx::default(); N + 1],
            ..Mid::default()
        };
        m.k[1] = Cx(1.0, 0.0);
        let mut power = Cx(1.0, 0.0);
        for _ in 0..512 {
            m.powers.push(power);
            power = power.scale(2.0);
        }
        let (o, work, jumped) = pixel::<true>(&m, Cx(1000.0, 0.0), Cx::default(), 100, 4.0, 1e-3);
        assert!(jumped);
        let j = 11u64;
        let q = 1e-5 * 2f64.powi(j as i32);
        let d_jump = -1.0 + 2f64.powi(j as i32) * (2.0 + 1e-5 * 2.0 * j as f64);
        match o {
            Outcome::Escaped { n, zr, dr, .. } => {
                assert_eq!(n, APPROACH as u64 + 2 * j + 1);
                assert!((zr - (q * q + 1000.0)).abs() < 1e-10);
                assert!((dr - (2.0 * q * d_jump + 1.0)).abs() < 1e-10);
                assert_eq!(work, APPROACH as u64 + 2);
            }
            _ => panic!("expected escape after the jump"),
        }
    }

    #[test]
    fn frame_150_parameter_annulus_abstains_before_the_bad_jump() {
        // Owner's 960x540 full-frame failure: at width 5.35e-10, the old
        // |dc| <= 1e-6 guard admitted all pixels (20,294 bad de, 14,483
        // bad normals). Every such camera is outside the validated domain.
        let radius = 5.35e-10 * 480.0_f64.hypot(270.0) / 960.0;
        let m = Mid {
            reference: vec![Cx::default()],
            ..Mid::default()
        };
        assert!(!m.covers(radius, 1e10));
        assert!(!chart_parameter_ok(radius));
        let (_, work, jumped) = pixel::<true>(&m, Cx::default(), Cx(radius, 0.0), 100, 1e20, 1e-12);
        assert!(!jumped);
        assert_eq!(work, 100);
        // Both sides of the annulus are admissible; these are not film
        // frame-index gates. Individual pixels in the gap always abstain.
        assert!(chart_parameter_ok(6e-7));
        assert!(chart_parameter_ok(6.94e-12));
        assert!(!chart_parameter_ok(1.36e-8)); // frame 125
        assert!(m.covers(6.94e-12, 1e10)); // frame 175
        assert!(m.covers(1.08e-26, 1e10)); // frame 400
        assert!(!m.covers(1.14e-28, 1e10)); // frame 430
    }

    #[test]
    fn refuses_truncated_constants() {
        let mut m = Mid::default();
        m.add(&["mid_u", "0.01", "0.0"]).unwrap();
        assert!(m.finish(764).is_err());
    }
}
