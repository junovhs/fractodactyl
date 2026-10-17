//! Constants and frame gate of the PROB-14 co-moving two-cycle jump for v0 mid-band
//! pixels. The per-pixel operator exists only as WGSL (zone_mid.wgsl, zone_mid32.wgsl;
//! DEC-28). Valid for |dc| <= 1e-6, chart |w| <= 0.031 and an in-budget jump; every
//! declined operator falls back to plain iteration at the pixel's own parameter.
use super::Cx;

const N: usize = 18;
const APPROACH: usize = 24;
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

    /// The operator constants for the wgpu mid kernel (GPU-22), in the order
    /// zone_mid.wgsl reads them: u, s, p, sign, lambda, then from slot `at` the 25
    /// approach points, k, k', the 512 powers lambda^j and the reference orbit.
    pub(super) fn pack(&self, v: &mut Vec<f64>, at: usize) {
        v.extend([
            self.u.0, self.u.1, self.s.0, self.s.1, self.p.0, self.p.1, self.sign,
        ]);
        v.extend([self.lambda.0, self.lambda.1]);
        v.resize(at, 0.0);
        for a in self
            .z
            .iter()
            .chain(&self.k)
            .chain(&self.kp)
            .chain(&self.powers)
            .chain(&self.reference)
        {
            v.extend([a.0, a.1]);
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_150_parameter_annulus_is_not_covered() {
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
