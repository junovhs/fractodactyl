//! Unsigned integers of any size as little-endian u64 limbs, always normalised (no
//! high zero limbs, so zero is empty and equality is structural). Off the hot path.
use std::cmp::Ordering;

/// A non-negative integer of any size (tile indices, exact decimal arithmetic).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Nat(Vec<u64>);

impl Nat {
    /// Zero.
    pub fn zero() -> Nat {
        Nat(Vec::new())
    }

    /// A one-limb value.
    pub fn small(v: u64) -> Nat {
        Nat(vec![v]).norm()
    }

    /// `2^k`.
    pub fn pow2(k: u64) -> Nat {
        Nat::small(1).shl_bits(k)
    }

    fn norm(mut self) -> Nat {
        while self.0.last() == Some(&0) {
            self.0.pop();
        }
        self
    }

    /// True for zero.
    pub fn is_zero(&self) -> bool {
        self.0.is_empty()
    }

    /// Number of significant bits (0 for zero).
    pub fn bits(&self) -> u64 {
        self.0.last().map_or(0, |&top| 64 * self.0.len() as u64 - top.leading_zeros() as u64)
    }

    /// The low `k <= 64` bits.
    pub fn low(&self, k: u32) -> u64 {
        let v = self.0.first().copied().unwrap_or(0);
        if k >= 64 {
            v
        } else {
            v & ((1u64 << k) - 1)
        }
    }

    /// `self * 2^k`.
    pub fn shl_bits(&self, k: u64) -> Nat {
        if self.is_zero() {
            return Nat::zero();
        }
        let (limbs, b) = ((k / 64) as usize, (k % 64) as u32);
        let mut v = vec![0u64; limbs];
        let mut carry = 0u64;
        for &x in &self.0 {
            v.push(if b == 0 { x } else { (x << b) | carry });
            carry = if b == 0 { 0 } else { x >> (64 - b) };
        }
        v.push(carry);
        Nat(v).norm()
    }

    /// `floor(self / 2^k)`.
    pub fn shr_bits(&self, k: u64) -> Nat {
        let (limbs, b) = ((k / 64) as usize, (k % 64) as u32);
        let s = self.0.get(limbs..).unwrap_or(&[]);
        let v = (0..s.len())
            .map(|i| {
                let hi = s.get(i + 1).copied().unwrap_or(0);
                if b == 0 {
                    s[i]
                } else {
                    (s[i] >> b) | (hi << (64 - b))
                }
            })
            .collect();
        Nat(v).norm()
    }

    /// `self | v` for `v` below the lowest set bit position of interest (plain addition
    /// when the low bits of `self` are zero, which is how callers use it).
    pub fn or_small(&self, v: u64) -> Nat {
        let mut r = self.0.clone();
        if r.is_empty() {
            r.push(0);
        }
        r[0] |= v;
        Nat(r).norm()
    }

    /// `self + o`.
    pub fn plus(&self, o: &Nat) -> Nat {
        let n = self.0.len().max(o.0.len());
        let (mut v, mut c) = (Vec::with_capacity(n + 1), false);
        for i in 0..n {
            let (a, b) = (self.0.get(i).copied().unwrap_or(0), o.0.get(i).copied().unwrap_or(0));
            let (s1, o1) = a.overflowing_add(b);
            let (s2, o2) = s1.overflowing_add(c as u64);
            v.push(s2);
            c = o1 || o2;
        }
        v.push(c as u64);
        Nat(v).norm()
    }

    /// `self - o`, or `None` when that would be negative.
    pub fn minus(&self, o: &Nat) -> Option<Nat> {
        if self.cmp(o) == Ordering::Less {
            return None;
        }
        let (mut v, mut br) = (Vec::with_capacity(self.0.len()), false);
        for (i, &a) in self.0.iter().enumerate() {
            let b = o.0.get(i).copied().unwrap_or(0);
            let (d1, o1) = a.overflowing_sub(b);
            let (d2, o2) = d1.overflowing_sub(br as u64);
            v.push(d2);
            br = o1 || o2;
        }
        Some(Nat(v).norm())
    }

    /// `self * m`.
    pub fn mul_small(&self, m: u64) -> Nat {
        let mut v = Vec::with_capacity(self.0.len() + 1);
        let mut c = 0u64;
        for &x in &self.0 {
            let t = x as u128 * m as u128 + c as u128;
            v.push(t as u64);
            c = (t >> 64) as u64;
        }
        v.push(c);
        Nat(v).norm()
    }

    /// `(floor(self / d), self mod d)`.
    pub fn div_small(&self, d: u64) -> (Nat, u64) {
        let mut v = self.0.clone();
        let mut r = 0u128;
        for x in v.iter_mut().rev() {
            let cur = (r << 64) | *x as u128;
            *x = (cur / d as u128) as u64;
            r = cur % d as u128;
        }
        (Nat(v).norm(), r as u64)
    }

    /// `self * 10^e`.
    pub fn mul_pow10(&self, e: u64) -> Nat {
        self.mul_pow(10, 19, e)
    }

    /// `self * 5^e`.
    pub fn mul_pow5(&self, e: u64) -> Nat {
        self.mul_pow(5, 27, e)
    }

    /// `floor(self / 5^e)`.
    pub fn div_pow5(&self, mut e: u64) -> Nat {
        let mut n = self.clone();
        while e > 0 && !n.is_zero() {
            let k = e.min(27);
            n = n.div_small(5u64.pow(k as u32)).0;
            e -= k;
        }
        n
    }

    /// `self * b^e`, in chunks of at most `b^chunk` (which must fit a u64).
    fn mul_pow(&self, b: u64, chunk: u64, mut e: u64) -> Nat {
        let mut n = self.clone();
        while e > 0 && !n.is_zero() {
            let k = e.min(chunk);
            n = n.mul_small(b.pow(k as u32));
            e -= k;
        }
        n
    }

    /// Integer from decimal digits (values 0..=9, most significant first).
    pub fn from_digits(digits: &[u8]) -> Nat {
        digits.chunks(19).fold(Nat::zero(), |n, c| {
            let v = c.iter().fold(0u64, |v, &d| v * 10 + d as u64);
            n.mul_small(10u64.pow(c.len() as u32)).plus(&Nat::small(v))
        })
    }

    /// Decimal digits, most significant first ("0" for zero).
    pub fn to_decimal(&self) -> String {
        let (mut n, mut groups) = (self.clone(), Vec::new());
        while !n.is_zero() {
            let (q, r) = n.div_small(10_000_000_000_000_000_000);
            groups.push(r);
            n = q;
        }
        let mut s = groups.pop().map_or("0".to_string(), |g| g.to_string());
        for g in groups.iter().rev() {
            s.push_str(&format!("{g:019}"));
        }
        s
    }

    /// Lowercase hex without leading zeros ("0" for zero).
    pub fn to_hex(&self) -> String {
        let mut s = match self.0.last() {
            None => return "0".into(),
            Some(top) => format!("{top:x}"),
        };
        for x in self.0.iter().rev().skip(1) {
            s.push_str(&format!("{x:016x}"));
        }
        s
    }

    /// Inverse of `to_hex`; rejects anything non-canonical (leading zeros, uppercase).
    pub fn from_hex(s: &str) -> Option<Nat> {
        let ok = !s.is_empty()
            && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            && (s == "0" || !s.starts_with('0'));
        if !ok {
            return None;
        }
        let b = s.as_bytes();
        let v = (0..b.len().div_ceil(16))
            .map(|i| {
                let end = b.len() - 16 * i;
                let start = end.saturating_sub(16);
                u64::from_str_radix(&s[start..end], 16).unwrap()
            })
            .collect();
        Some(Nat(v).norm())
    }
}

impl PartialOrd for Nat {
    fn partial_cmp(&self, o: &Nat) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

impl Ord for Nat {
    fn cmp(&self, o: &Nat) -> Ordering {
        self.0.len().cmp(&o.0.len()).then_with(|| self.0.iter().rev().cmp(o.0.iter().rev()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shifts_and_bits() {
        let x = Nat::small(0b1011);
        assert_eq!(x.shl_bits(130).shr_bits(130), x);
        assert_eq!(x.shl_bits(130).bits(), 134);
        assert_eq!(x.shr_bits(1), Nat::small(0b101));
        assert_eq!(x.shr_bits(4), Nat::zero());
        assert_eq!(Nat::pow2(64).bits(), 65);
        assert_eq!(Nat::pow2(200).shr_bits(199), Nat::small(2));
    }

    #[test]
    fn arithmetic() {
        let a = Nat::pow2(128);
        let b = Nat::small(1);
        let c = a.minus(&b).unwrap();
        assert_eq!(c.to_hex(), "f".repeat(32));
        assert_eq!(c.plus(&b), a);
        assert!(b.minus(&a).is_none());
        assert_eq!(Nat::small(3).mul_pow5(30).div_pow5(30), Nat::small(3));
        assert_eq!(Nat::small(1).mul_pow10(25).to_decimal(), format!("1{}", "0".repeat(25)));
        assert_eq!(Nat::from_digits(&[0, 0, 1, 2]), Nat::small(12));
        assert_eq!(Nat::zero().to_decimal(), "0");
    }

    #[test]
    fn hex_round_trip() {
        for s in ["0", "1", "abc", "1000000000000000000000000000000000ff"] {
            assert_eq!(Nat::from_hex(s).unwrap().to_hex(), s);
        }
        for s in ["", "00", "01", "AB", "g", "-1"] {
            assert!(Nat::from_hex(s).is_none(), "{s}");
        }
    }
}
