//! Sample-grid geometry at any depth, and the choice of numeric tier.
use fd_fixed::{exp2i, Decimal, Fixed};
use fd_samples::View;

/// Numeric tier, cheapest first. Each is used only where its contract holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    /// f64 reference at the f64-rounded centre, f64 deltas (`pert-f64/1`).
    F64,
    /// Fixed-point reference at the exact centre, f64 deltas (`pert-fx/1`).
    Fixed,
    /// Fixed-point reference, deltas scaled by exact powers of two (`pert-fx-scaled/1`).
    Scaled,
}

/// Centre and transform from grid indices to complex offsets.
#[derive(Clone, Copy, Debug)]
pub struct Plane {
    /// Centre, real part, rounded to f64.
    pub c_re: f64,
    /// Centre, imaginary part, rounded to f64.
    pub c_im: f64,
    /// Sample spacing `h = h_m * 2^h_e` with `1 <= h_m < 2`; exact range at any depth.
    pub h_m: f64,
    /// Binary exponent of the sample spacing.
    pub h_e: i64,
    /// Cheapest valid tier for this view.
    pub tier: Tier,
    /// Fraction bits for a fixed-point reference at this depth.
    pub bits: u64,
    cos: f64,
    sin: f64,
    half_x: f64,
    half_y: f64,
}

/// Deltas below `2^SCALED_BELOW` go to the scaled tier (f64 keeps ~120 binary orders
/// of headroom above its subnormal range for orbit growth and squaring).
const SCALED_BELOW: i64 = -900;

impl Plane {
    pub fn new(v: &View, nx: u32, ny: u32) -> Result<Plane, String> {
        if nx == 0 || ny == 0 {
            return Err("empty sample grid".into());
        }
        let width = Decimal::parse(&v.width)?;
        let lw = width.log2_abs();
        if width.neg || !lw.is_finite() {
            return Err("view width must be positive".into());
        }
        // Enough fraction bits to hold the width's leading 64 bits exactly.
        let wfix = width.to_fixed(fd_fixed::limbs_for((64.0 - lw).max(64.0).ceil() as u64))?;
        let (wm, we) = wfix.frexp().ok_or("view width underflowed")?;
        let q = wm / nx as f64; // in (2^-32, 2)
        let qe = ((q.to_bits() >> 52) & 0x7ff) as i64 - 1023;
        let (h_m, h_e) = (q * exp2i(-qe), we + qe);

        let num = |s: &str| Decimal::parse(s).map(|_| s.trim().parse::<f64>().unwrap_or(f64::NAN));
        let (c_re, c_im) = (num(&v.center_re)?, num(&v.center_im)?);
        if !(c_re.is_finite() && c_im.is_finite()) {
            return Err("view centre out of range".into());
        }
        // f64 tier: rounding the centre must move it by under 1/1024 of a sample.
        let c_mag = c_re.abs().max(c_im.abs()).max(exp2i(-1022));
        let f64_ok = c_mag.log2() - 52.0 <= h_e as f64 - 10.0 && h_e > -1000;
        let tier = if f64_ok {
            Tier::F64
        } else if h_e >= SCALED_BELOW {
            Tier::Fixed
        } else {
            Tier::Scaled
        };
        let bits = (128 - h_e).max(128) as u64;
        let (sin, cos) = v.rotation.sin_cos();
        Ok(Plane { c_re, c_im, h_m, h_e, tier, bits, cos, sin, half_x: nx as f64 / 2.0, half_y: ny as f64 / 2.0 })
    }

    /// Kernel id written to the header for `tier`.
    pub fn kernel(&self, tier: Tier) -> String {
        match tier {
            Tier::F64 => "pert-f64/1".into(),
            Tier::Fixed => format!("pert-fx/1 bits={}", self.bits),
            Tier::Scaled => format!("pert-fx-scaled/1 bits={}", self.bits),
        }
    }

    /// Whether `tier` is valid for this view (deeper tiers are valid everywhere).
    pub fn allows(&self, tier: Tier) -> bool {
        match tier {
            Tier::F64 => self.tier == Tier::F64,
            Tier::Fixed => self.tier != Tier::Scaled,
            Tier::Scaled => true,
        }
    }

    /// Exact-centre fixed-point coordinates for the reference orbit.
    pub fn center_fixed(v: &View, bits: u64) -> Result<(Fixed, Fixed), String> {
        let l = fd_fixed::limbs_for(bits);
        Ok((Fixed::parse(&v.center_re, l)?, Fixed::parse(&v.center_im, l)?))
    }

    /// Offset of sample `(i, j)` from the centre in units of `h` (multiply by `h`).
    #[inline]
    pub fn unit_offset(&self, i: usize, j: usize) -> (f64, f64) {
        let x = i as f64 + 0.5 - self.half_x;
        let y = j as f64 + 0.5 - self.half_y;
        (self.cos * x + self.sin * y, self.sin * x - self.cos * y)
    }

    /// Sample spacing as f64 (valid in the f64-delta tiers).
    #[inline]
    pub fn h(&self) -> f64 {
        self.h_m * exp2i(self.h_e)
    }

    /// Complex-plane direction to screen direction (x right, y down).
    #[inline]
    pub fn to_screen(&self, re: f64, im: f64) -> (f64, f64) {
        (self.cos * re + self.sin * im, self.sin * re - self.cos * im)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(re: &str, width: &str, rot: f64) -> View {
        View { center_re: re.into(), center_im: "0.25".into(), width: width.into(), rotation: rot }
    }

    #[test]
    fn screen_axes_match_sample_layout() {
        let p = Plane::new(&view("-0.5", "4", 0.7), 8, 8).unwrap();
        let (a, b) = (p.unit_offset(3, 3), p.unit_offset(4, 3));
        let (x, y) = p.to_screen(b.0 - a.0, b.1 - a.1);
        assert!((x - 1.0).abs() < 1e-12 && y.abs() < 1e-12);
        let b = p.unit_offset(3, 4);
        let (x, y) = p.to_screen(b.0 - a.0, b.1 - a.1);
        assert!(x.abs() < 1e-12 && (y - 1.0).abs() < 1e-12);
    }

    #[test]
    fn imaginary_axis_points_up() {
        let p = Plane::new(&view("-0.5", "4", 0.0), 8, 8).unwrap();
        assert!(p.unit_offset(0, 0).1 > 0.0 && p.unit_offset(0, 7).1 < 0.0);
    }

    #[test]
    fn tiers_follow_depth() {
        let tier = |w: &str| Plane::new(&view("-0.5", w, 0.0), 1000, 1000).unwrap().tier;
        assert_eq!(tier("1e-3"), Tier::F64);
        assert_eq!(tier("1e-15"), Tier::Fixed);
        assert_eq!(tier("1e-260"), Tier::Fixed);
        assert_eq!(tier("1e-300"), Tier::Scaled);
        assert_eq!(tier("1e-100000"), Tier::Scaled);
    }

    #[test]
    fn deep_spacing_is_exact_in_range() {
        let p = Plane::new(&view("0", "3e-5000", 0.0), 3, 3).unwrap();
        let log10 = (p.h_m.log2() + p.h_e as f64) * std::f64::consts::LOG10_2;
        assert!((log10 + 5000.0).abs() < 1e-9, "{log10}");
        assert!((1.0..2.0).contains(&p.h_m));
    }
}
