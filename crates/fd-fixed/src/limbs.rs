//! Unsigned magnitudes as little-endian u64 limbs, all of one length `L`.
//! The top limb is the integer part; the rest are 64(L-1) fraction bits.
use std::cmp::Ordering;

/// Compare two magnitudes of equal length.
#[inline]
pub fn cmp(a: &[u64], b: &[u64]) -> Ordering {
    for i in (0..a.len()).rev() {
        match a[i].cmp(&b[i]) {
            Ordering::Equal => continue,
            o => return o,
        }
    }
    Ordering::Equal
}

/// `out = a + b`. The integer limb never overflows for orbit-sized values.
#[inline]
pub fn add(a: &[u64], b: &[u64], out: &mut [u64]) {
    let mut c = false;
    for i in 0..a.len() {
        let (s, c1) = a[i].overflowing_add(b[i]);
        let (s, c2) = s.overflowing_add(c as u64);
        out[i] = s;
        c = c1 | c2;
    }
    debug_assert!(!c, "fixed-point overflow");
}

/// `out = a - b`, requires `a >= b`.
#[inline]
pub fn sub(a: &[u64], b: &[u64], out: &mut [u64]) {
    let mut br = false;
    for i in 0..a.len() {
        let (d, b1) = a[i].overflowing_sub(b[i]);
        let (d, b2) = d.overflowing_sub(br as u64);
        out[i] = d;
        br = b1 | b2;
    }
    debug_assert!(!br, "fixed-point underflow");
}

/// Signed sum `(an, a) + (bn, b)` into `out`; returns the sign (`true` = negative).
#[inline]
pub fn add_signed(an: bool, a: &[u64], bn: bool, b: &[u64], out: &mut [u64]) -> bool {
    if an == bn {
        add(a, b, out);
        return an && out.iter().any(|&x| x != 0);
    }
    match cmp(a, b) {
        Ordering::Less => {
            sub(b, a, out);
            bn
        }
        _ => {
            sub(a, b, out);
            an && out.iter().any(|&x| x != 0)
        }
    }
}

/// `out = a^2`, truncated to `L` limbs. `prod` is scratch of length `2L`.
/// Off-diagonal products are summed once and doubled, halving the multiplications.
pub fn sqr(a: &[u64], prod: &mut [u64], out: &mut [u64]) {
    let l = a.len();
    prod.fill(0);
    for i in 0..l {
        let ai = a[i] as u128;
        if ai == 0 {
            continue;
        }
        let mut c = 0u64;
        for j in i + 1..l {
            let t = ai * a[j] as u128 + prod[i + j] as u128 + c as u128;
            prod[i + j] = t as u64;
            c = (t >> 64) as u64;
        }
        prod[i + l] = c;
    }
    let mut top = 0u64;
    for p in prod.iter_mut() {
        let next = *p >> 63;
        *p = (*p << 1) | top;
        top = next;
    }
    let mut c = 0u64;
    for i in 0..l {
        let sq = a[i] as u128 * a[i] as u128;
        let t = prod[2 * i] as u128 + (sq as u64) as u128 + c as u128;
        prod[2 * i] = t as u64;
        let t = prod[2 * i + 1] as u128 + (sq >> 64) + (t >> 64);
        prod[2 * i + 1] = t as u64;
        c = (t >> 64) as u64;
    }
    debug_assert!(
        c == 0 && prod[2 * l - 1] == 0,
        "fixed-point square overflow"
    );
    out.copy_from_slice(&prod[l - 1..2 * l - 1]);
}

/// Variable-length helpers for decimal parsing (not on any hot path).
pub fn mul_small(v: &mut Vec<u64>, m: u64) {
    let mut c = 0u64;
    for x in v.iter_mut() {
        let t = *x as u128 * m as u128 + c as u128;
        *x = t as u64;
        c = (t >> 64) as u64;
    }
    if c != 0 {
        v.push(c);
    }
}

pub fn add_small(v: &mut Vec<u64>, a: u64) {
    let mut c = a;
    for x in v.iter_mut() {
        let (s, o) = x.overflowing_add(c);
        *x = s;
        c = o as u64;
        if c == 0 {
            return;
        }
    }
    if c != 0 {
        v.push(c);
    }
}

/// `v = floor(v / d)`.
pub fn div_small(v: &mut [u64], d: u64) {
    let mut r = 0u128;
    for x in v.iter_mut().rev() {
        let cur = (r << 64) | *x as u128;
        *x = (cur / d as u128) as u64;
        r = cur % d as u128;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_matches_u128() {
        // L = 2: one integer limb, one fraction limb. (1.5)^2 = 2.25.
        let a = [1u64 << 63, 1];
        let (mut prod, mut out) = ([0u64; 4], [0u64; 2]);
        sqr(&a, &mut prod, &mut out);
        assert_eq!(out, [1u64 << 62, 2]);
    }

    #[test]
    fn square_with_carries() {
        let a = [u64::MAX, u64::MAX, 3];
        let (mut prod, mut out) = ([0u64; 6], [0u64; 3]);
        sqr(&a, &mut prod, &mut out);
        // (4 - 2^-128)^2 = 16 - 2^-125 + 2^-256 -> truncated to 128 fraction bits.
        assert_eq!(out, [u64::MAX - 7, u64::MAX, 15]);
    }

    #[test]
    fn signed_add() {
        let (a, b) = ([5u64, 1], [7u64, 0]);
        let mut out = [0u64; 2];
        assert!(!add_signed(false, &a, true, &b, &mut out));
        assert_eq!(out, [u64::MAX - 1, 0]);
        assert!(!add_signed(true, &b, false, &a, &mut out));
        assert!(!add_signed(true, &a, false, &a, &mut out)); // -x + x = +0
    }
}
