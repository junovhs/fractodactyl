//! Arbitrary-precision binary floating point for zone construction (PROB-30): the
//! arithmetic `fd zone` builds constants with, where mpmath served the Python generators.
//! Zone constants run from ~1e-1000 to ~1e500 at hundreds of digits, beyond [`Fixed`]'s
//! one integer limb (DEC-21). Backed by astro-float (pure Rust, DEC-27); every result
//! is rounded to nearest-even at the wider operand's precision.
//!
//! [`Fixed`]: crate::Fixed
use astro_float::{BigFloat, Consts, Radix, RoundingMode, Sign};
use std::cell::RefCell;

const RM: RoundingMode = RoundingMode::ToEven;

thread_local! {
    /// Constant cache for decimal conversion.
    static CONSTS: RefCell<Consts> = RefCell::new(Consts::new().expect("astro-float constants"));
}

/// Real binary float: a mantissa of `precision` bits and a binary exponent.
#[derive(Clone, Debug)]
pub struct Float(BigFloat);

/// Bits carrying `digits` decimal digits, plus a guard word.
pub fn bits_for_digits(digits: usize) -> usize {
    (digits as f64 * std::f64::consts::LOG2_10).ceil() as usize + 64
}

impl Float {
    /// A decimal string ("-0.74", "1e-1000") rounded to `bits` of mantissa.
    pub fn parse(s: &str, bits: usize) -> Result<Float, String> {
        let x =
            CONSTS.with(|c| BigFloat::parse(s.trim(), Radix::Dec, bits, RM, &mut c.borrow_mut()));
        if x.is_nan() || x.is_inf() {
            return Err(format!("not a finite decimal: {s:?}"));
        }
        Ok(Float(x))
    }

    pub fn from_f64(x: f64, bits: usize) -> Float {
        Float(BigFloat::from_f64(x, bits))
    }

    pub fn from_i64(x: i64, bits: usize) -> Float {
        let m = Float(BigFloat::from_word(x.unsigned_abs(), bits));
        if x < 0 {
            m.neg()
        } else {
            m
        }
    }

    pub fn zero(bits: usize) -> Float {
        Float::from_i64(0, bits)
    }

    /// Mantissa bits.
    pub fn precision(&self) -> usize {
        self.0.precision().unwrap_or(0)
    }

    fn prec(&self, o: &Float) -> usize {
        self.precision().max(o.precision())
    }

    pub fn add(&self, o: &Float) -> Float {
        Float(self.0.add(&o.0, self.prec(o), RM))
    }

    pub fn sub(&self, o: &Float) -> Float {
        Float(self.0.sub(&o.0, self.prec(o), RM))
    }

    pub fn mul(&self, o: &Float) -> Float {
        Float(self.0.mul(&o.0, self.prec(o), RM))
    }

    pub fn div(&self, o: &Float) -> Float {
        Float(self.0.div(&o.0, self.prec(o), RM))
    }

    pub fn neg(&self) -> Float {
        Float(self.0.neg())
    }

    pub fn abs(&self) -> Float {
        if self.0.is_negative() {
            self.neg()
        } else {
            self.clone()
        }
    }

    /// `self * 2^k`, exact.
    pub fn ldexp(&self, k: i64) -> Float {
        let mut x = self.0.clone();
        if let Some(e) = x.exponent() {
            let e = i64::from(e) + k;
            x.set_exponent(i32::try_from(e).expect("binary exponent within i32"));
        }
        Float(x)
    }

    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    pub fn is_negative(&self) -> bool {
        self.0.is_negative()
    }

    /// `self < o`.
    pub fn lt(&self, o: &Float) -> bool {
        self.0.cmp(&o.0).is_some_and(|c| c < 0)
    }

    /// `(m, e)` with `self = m * 2^e` and `0.5 <= |m| < 1` (`(0, 0)` for zero), as C's
    /// frexp: `m` carries the top 53 bits, `e` any depth.
    pub fn frexp(&self) -> (f64, i64) {
        let Some((words, _, sign, e, _)) = self.0.as_raw_parts() else {
            return (0.0, 0);
        };
        if self.0.is_zero() {
            return (0.0, 0);
        }
        // Words are little-endian with the most significant bit of the last word set.
        let top = words.len() - 1;
        let hi = words[top] as f64 / 18446744073709551616.0;
        let lo = if top > 0 {
            words[top - 1] as f64 / 18446744073709551616.0 / 18446744073709551616.0
        } else {
            0.0
        };
        let m = hi + lo;
        (if sign == Sign::Neg { -m } else { m }, i64::from(e))
    }

    /// Nearest f64: 0 below the subnormals, infinite above f64's range.
    pub fn to_f64(&self) -> f64 {
        let (m, e) = self.frexp();
        // Two factors keep each power of two inside f64 for every result f64 can hold.
        let e = e.clamp(-2200, 2200);
        m * crate::exp2i(e / 2) * crate::exp2i(e - e / 2)
    }

    /// Decimal scientific notation with `digits` significant digits (truncated).
    pub fn to_decimal(&self, digits: usize) -> String {
        if self.0.is_zero() {
            return "0".into();
        }
        let s = CONSTS.with(|c| self.0.format(Radix::Dec, RM, &mut c.borrow_mut()));
        let s = s.expect("finite float formats");
        // astro-float prints `[-]d.ddd...e[+-]x`; keep `digits` mantissa digits.
        let (mant, exp) = s.split_once('e').unwrap_or((&s, "0"));
        let (sign, mant) = mant.strip_prefix('-').map_or(("", mant), |m| ("-", m));
        let mut all: String = mant.chars().filter(|c| c.is_ascii_digit()).collect();
        while all.len() < digits.max(1) {
            all.push('0');
        }
        let keep = &all[..digits.max(1)];
        let exp: i64 = exp
            .trim_start_matches('+')
            .parse()
            .expect("decimal exponent");
        // The mantissa printed as d.ddd: the point sits after the first digit.
        let lead = mant.find('.').unwrap_or(mant.len()) as i64;
        let (first, rest) = keep.split_at(1);
        format!("{sign}{first}.{rest}e{}", exp + lead - 1)
    }
}

/// Complex number over [`Float`].
#[derive(Clone, Debug)]
pub struct Complex {
    pub re: Float,
    pub im: Float,
}

impl Complex {
    pub fn new(re: Float, im: Float) -> Complex {
        Complex { re, im }
    }

    pub fn parse(re: &str, im: &str, bits: usize) -> Result<Complex, String> {
        Ok(Complex::new(
            Float::parse(re, bits)?,
            Float::parse(im, bits)?,
        ))
    }

    pub fn zero(bits: usize) -> Complex {
        Complex::new(Float::zero(bits), Float::zero(bits))
    }

    pub fn from_i64(x: i64, bits: usize) -> Complex {
        Complex::new(Float::from_i64(x, bits), Float::zero(bits))
    }

    pub fn add(&self, o: &Complex) -> Complex {
        Complex::new(self.re.add(&o.re), self.im.add(&o.im))
    }

    pub fn sub(&self, o: &Complex) -> Complex {
        Complex::new(self.re.sub(&o.re), self.im.sub(&o.im))
    }

    pub fn mul(&self, o: &Complex) -> Complex {
        Complex::new(
            self.re.mul(&o.re).sub(&self.im.mul(&o.im)),
            self.re.mul(&o.im).add(&self.im.mul(&o.re)),
        )
    }

    pub fn sqr(&self) -> Complex {
        let x = self.re.mul(&self.im);
        Complex::new(self.re.mul(&self.re).sub(&self.im.mul(&self.im)), x.add(&x))
    }

    /// `self * x` for a real `x`.
    pub fn scale(&self, x: &Float) -> Complex {
        Complex::new(self.re.mul(x), self.im.mul(x))
    }

    /// `self * 2^k`, exact.
    pub fn ldexp(&self, k: i64) -> Complex {
        Complex::new(self.re.ldexp(k), self.im.ldexp(k))
    }

    pub fn norm2(&self) -> Float {
        self.re.mul(&self.re).add(&self.im.mul(&self.im))
    }

    pub fn div(&self, o: &Complex) -> Complex {
        let d = o.norm2();
        Complex::new(
            self.re.mul(&o.re).add(&self.im.mul(&o.im)).div(&d),
            self.im.mul(&o.re).sub(&self.re.mul(&o.im)).div(&d),
        )
    }

    pub fn neg(&self) -> Complex {
        Complex::new(self.re.neg(), self.im.neg())
    }

    pub fn is_zero(&self) -> bool {
        self.re.is_zero() && self.im.is_zero()
    }
}
