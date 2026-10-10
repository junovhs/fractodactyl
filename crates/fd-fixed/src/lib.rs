//! Arbitrary-precision signed fixed point, built for one job: carrying deep absolute
//! coordinates and reference orbits exactly enough, as cheaply as possible.
//!
//! A `Fixed` has `L` little-endian u64 limbs: one integer limb and `64(L-1)` fraction
//! bits. No exponent, no normalisation, no allocation in the orbit loop. Values handled
//! here (coordinates, orbit points before escape) stay well inside the integer limb.
mod decimal;
mod limbs;
mod orbit;

pub use decimal::Decimal;
pub use orbit::orbit;

/// Sign-magnitude fixed-point number.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fixed {
    /// True for negative values; zero is never negative.
    pub neg: bool,
    /// Magnitude limbs, little-endian; the last limb is the integer part.
    pub mag: Vec<u64>,
}

/// Limbs needed for `bits` fraction bits.
pub fn limbs_for(bits: u64) -> usize {
    1 + bits.div_ceil(64) as usize
}

/// `2^k` as f64: exact where representable, 0 below the subnormals, inf above.
#[inline]
pub fn exp2i(k: i64) -> f64 {
    if k > 1023 {
        f64::INFINITY
    } else if k >= -1022 {
        f64::from_bits(((k + 1023) as u64) << 52)
    } else if k >= -1074 {
        f64::from_bits(1u64 << (k + 1074))
    } else {
        0.0
    }
}

impl Fixed {
    /// Parse a decimal string, truncated toward zero to `limbs` limbs.
    pub fn parse(s: &str, limbs: usize) -> Result<Fixed, String> {
        Decimal::parse(s)?.to_fixed(limbs)
    }

    /// Value as `m * 2^e` with `1 <= |m| < 2`, or `None` for zero. Never under/overflows,
    /// whatever the depth. `m` carries ~53 correct bits (truncated, not rounded).
    pub fn frexp(&self) -> Option<(f64, i64)> {
        frexp(self.neg, &self.mag)
    }

    /// Nearest-ish f64 (0 when too small, inf when too large).
    pub fn to_f64(&self) -> f64 {
        to_f64(self.neg, &self.mag)
    }

    /// `self - o`, exact; both must have the same limb count.
    pub fn sub(&self, o: &Fixed) -> Fixed {
        assert_eq!(
            self.mag.len(),
            o.mag.len(),
            "Fixed::sub: limb counts differ"
        );
        let mut mag = vec![0; self.mag.len()];
        let neg = limbs::add_signed(self.neg, &self.mag, !o.neg, &o.mag, &mut mag);
        Fixed { neg, mag }
    }
}

/// `frexp` on raw sign and limbs (lets hot loops convert without building a `Fixed`).
pub(crate) fn frexp(neg: bool, mag: &[u64]) -> Option<(f64, i64)> {
    let k = mag.iter().rposition(|&x| x != 0)?;
    let below = |d: usize| k.checked_sub(d).map_or(0.0, |i| mag[i] as f64);
    let v = mag[k] as f64 + below(1) * exp2i(-64) + below(2) * exp2i(-128);
    let e = ((v.to_bits() >> 52) & 0x7ff) as i64 - 1023;
    let m = v * exp2i(-e);
    let scale = 64 * (k as i64 - (mag.len() as i64 - 1));
    Some((if neg { -m } else { m }, e + scale))
}

/// `to_f64` on raw sign and limbs.
pub(crate) fn to_f64(neg: bool, mag: &[u64]) -> f64 {
    match frexp(neg, mag) {
        None => 0.0,
        // Split the scaling so subnormal results round once, not twice.
        Some((m, e)) if e < -1000 => m * exp2i(e + 100) * exp2i(-100),
        Some((m, e)) => m * exp2i(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_convert() {
        for (s, v) in [
            ("1.5", 1.5),
            ("-0.75", -0.75),
            ("0", 0.0),
            ("2.5e-3", 2.5e-3),
            ("-1.25E2", -125.0),
        ] {
            assert_eq!(Fixed::parse(s, 4).unwrap().to_f64(), v, "{s}");
        }
    }

    #[test]
    fn deep_values_keep_their_exponent() {
        let x = Fixed::parse("3e-1000", limbs_for(3400)).unwrap();
        let (m, e) = x.frexp().unwrap();
        let log10 = (m.log2() + e as f64) * std::f64::consts::LOG10_2;
        assert!((log10 - (3f64.log10() - 1000.0)).abs() < 1e-12, "{log10}");
        assert_eq!(x.to_f64(), 0.0);
    }

    #[test]
    fn too_large_is_refused() {
        assert!(Fixed::parse("1e20", 3).is_err());
        assert!(Fixed::parse("abc", 3).is_err());
    }

    #[test]
    fn powers_of_two() {
        assert_eq!(exp2i(0), 1.0);
        assert_eq!(exp2i(-1074), f64::from_bits(1));
        assert_eq!(exp2i(-1075), 0.0);
        assert_eq!(exp2i(10), 1024.0);
    }
}
