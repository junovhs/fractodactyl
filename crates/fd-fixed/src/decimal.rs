//! Exact decimal strings (`-1.25e-300`) to fixed point. Off the hot path.
use crate::limbs::{add_small, div_small, mul_small};
use crate::Fixed;

const TEN19: u64 = 10_000_000_000_000_000_000;

/// A decimal number `±digits * 10^exp10`, held exactly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decimal {
    /// Sign.
    pub neg: bool,
    /// Decimal digits, most significant first, values 0..=9.
    pub digits: Vec<u8>,
    /// Power of ten applied to the digit string.
    pub exp10: i64,
}

impl Decimal {
    /// Accepts `[+-]digits[.digits][(e|E)[+-]digits]`.
    pub fn parse(s: &str) -> Result<Decimal, String> {
        let bad = || format!("not a decimal number: {s:?}");
        let t = s.trim();
        let (neg, t) = match t.as_bytes().first() {
            Some(b'-') => (true, &t[1..]),
            Some(b'+') => (false, &t[1..]),
            _ => (false, t),
        };
        let (mant, exp) = match t.find(['e', 'E']) {
            Some(i) => (&t[..i], t[i + 1..].parse::<i64>().map_err(|_| bad())?),
            None => (t, 0),
        };
        let (int, frac) = mant.split_once('.').unwrap_or((mant, ""));
        let digits: Vec<u8> = int
            .bytes()
            .chain(frac.bytes())
            .map(|b| b.wrapping_sub(b'0'))
            .collect();
        if digits.is_empty() || digits.iter().any(|&d| d > 9) {
            return Err(bad());
        }
        let exp10 = exp.checked_sub(frac.len() as i64).ok_or_else(bad)?;
        let zero = digits.iter().all(|&d| d == 0);
        Ok(Decimal {
            neg: neg && !zero,
            digits,
            exp10,
        })
    }

    /// log2 of the absolute value (−inf for zero), accurate to ~1e-15 relative.
    pub fn log2_abs(&self) -> f64 {
        let Some(first) = self.digits.iter().position(|&d| d != 0) else {
            return f64::NEG_INFINITY;
        };
        let sig = &self.digits[first..];
        let head = sig.iter().take(17).fold(0f64, |v, &d| v * 10.0 + d as f64);
        let rest = sig.len().saturating_sub(17) as f64;
        (head.log10() + rest + self.exp10 as f64) * std::f64::consts::LOG2_10
    }

    /// Exact value truncated toward zero to `limbs` limbs (64(limbs-1) fraction bits).
    pub fn to_fixed(&self, limbs: usize) -> Result<Fixed, String> {
        let mut n = vec![0u64];
        for chunk in self.digits.chunks(19) {
            mul_small(&mut n, 10u64.pow(chunk.len() as u32));
            add_small(&mut n, chunk.iter().fold(0u64, |v, &d| v * 10 + d as u64));
        }
        let mut e = self.exp10;
        while e > 0 {
            let k = e.min(19) as u32;
            mul_small(&mut n, 10u64.pow(k));
            e -= k as i64;
            if n.len() > 1 {
                return Err("value too large for fixed point".into());
            }
        }
        // Shift into place: limbs-1 fraction limbs below the integer limb.
        let mut v = vec![0u64; limbs - 1];
        v.extend_from_slice(&n);
        while e < 0 {
            let k = (-e).min(19) as u32;
            div_small(&mut v, if k == 19 { TEN19 } else { 10u64.pow(k) });
            e += k as i64;
            if v.iter().all(|&x| x == 0) {
                break; // underflowed to zero: further division changes nothing
            }
        }
        if v[limbs..].iter().any(|&x| x != 0) {
            return Err("value too large for fixed point".into());
        }
        v.truncate(limbs);
        let neg = self.neg && v.iter().any(|&x| x != 0);
        Ok(Fixed { neg, mag: v })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_forms() {
        let d = Decimal::parse("-012.50e-3").unwrap();
        assert_eq!((d.neg, d.exp10), (true, -5));
        assert!(!Decimal::parse("-0.0").unwrap().neg);
        for bad in ["", ".", "1e", "--1", "1.2.3", "0x10"] {
            assert!(Decimal::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn log2_estimate() {
        let d = Decimal::parse("1e-1000").unwrap();
        assert!((d.log2_abs() + 1000.0 * std::f64::consts::LOG2_10).abs() < 1e-9);
        assert_eq!(
            Decimal::parse("0.000").unwrap().log2_abs(),
            f64::NEG_INFINITY
        );
    }

    #[test]
    fn exact_binary_fractions() {
        // 2^-100 is exact in 2 fraction limbs.
        let s = "7.888609052210118054117285652827862296732064351090230047702789306640625E-31";
        let f = Decimal::parse(s).unwrap().to_fixed(3).unwrap();
        assert_eq!(f.mag, vec![1 << 28, 0, 0]);
    }
}
