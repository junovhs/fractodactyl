//! Shared ordinary and exponent-carrying complex arithmetic for CPU kernel stages.
//! DEC-24: precision tiers reuse these numbers rather than owning kernel copies.
use fd_fixed::exp2i;
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub(crate) struct Cx(pub(crate) f64, pub(crate) f64);

impl Cx {
    #[inline(always)]
    pub(crate) fn add(self, o: Cx) -> Cx {
        Cx(self.0 + o.0, self.1 + o.1)
    }
    #[inline(always)]
    pub(crate) fn scale(self, s: f64) -> Cx {
        Cx(self.0 * s, self.1 * s)
    }
    #[inline(always)]
    pub(crate) fn abs(self) -> f64 {
        self.0.hypot(self.1)
    }
}

/// Complex `m * 2^e` with `1 <= max(|m.re|, |m.im|) < 2` (or `m = 0`): the deep path's
/// number, for zones whose constants, states or derivatives leave f64's range (DEC-21).
#[derive(Clone, Copy, Default, Debug)]
pub(crate) struct Fx {
    pub(crate) m: Cx,
    pub(crate) e: i64,
}

impl Fx {
    #[inline(always)]
    pub(crate) fn new(m: Cx, e: i64) -> Fx {
        Fx { m, e }.norm()
    }
    #[inline(always)]
    pub(crate) fn norm(self) -> Fx {
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
    pub(crate) fn is_zero(self) -> bool {
        self.m.0 == 0.0 && self.m.1 == 0.0
    }
    #[inline(always)]
    pub(crate) fn add(self, o: Fx) -> Fx {
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
}

/// Numeric backends keep each representation's established rounding order. The
/// iteration, derivative, escape, rebase and validity decisions have one source.
pub(crate) trait Arithmetic {
    const EXTENDED: bool;
    fn delta(z: (f64, f64), w: (f64, f64), s: f64, dc: (f64, f64)) -> (f64, f64);
}

pub(crate) type Binary64 = Cx;
pub(crate) type Binary64Exponent = Fx;

impl Arithmetic for Binary64 {
    const EXTENDED: bool = false;
    #[inline(always)]
    fn delta(
        (zr, zi): (f64, f64),
        (wr, wi): (f64, f64),
        _: f64,
        (ar, ai): (f64, f64),
    ) -> (f64, f64) {
        // Preserve the unscaled kernel's left-to-right square subtraction.
        (
            2.0 * (zr * wr - zi * wi) + wr * wr - wi * wi + ar,
            2.0 * (zr * wi + zi * wr + wr * wi) + ai,
        )
    }
}

impl Arithmetic for Binary64Exponent {
    const EXTENDED: bool = true;
    #[inline(always)]
    fn delta(
        (zr, zi): (f64, f64),
        (wr, wi): (f64, f64),
        s: f64,
        (ar, ai): (f64, f64),
    ) -> (f64, f64) {
        // Exact power-of-two scaling stays outside each quadratic term.
        (
            2.0 * (zr * wr - zi * wi) + s * (wr * wr - wi * wi) + ar,
            2.0 * (zr * wi + zi * wr) + s * (2.0 * wr * wi) + ai,
        )
    }
}
