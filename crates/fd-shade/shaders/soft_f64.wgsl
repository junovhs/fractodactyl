// Exact IEEE binary64 arithmetic on 32-bit integers (SHAD-04/05) and backend-safe f32
// helpers, shared by the film WGSL passes; Rust prepends this file to each pass's source. Values are
// sign * m * 2^e with m a 64-bit integer held in vec2<u32> (x = low, y = high).

// ---- 64-bit unsigned integers ----

fn add64(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    let lo = a.x + b.x;
    return vec2<u32>(lo, a.y + b.y + select(0u, 1u, lo < a.x));
}

// a - b for a >= b.
fn sub64(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    return vec2<u32>(a.x - b.x, a.y - b.y - select(0u, 1u, a.x < b.x));
}

fn lt64(a: vec2<u32>, b: vec2<u32>) -> bool {
    return a.y < b.y || (a.y == b.y && a.x < b.x);
}

fn eq64(a: vec2<u32>, b: vec2<u32>) -> bool {
    return a.x == b.x && a.y == b.y;
}

fn zero64(a: vec2<u32>) -> bool {
    return a.x == 0u && a.y == 0u;
}

fn bitlen64(a: vec2<u32>) -> u32 {
    if (a.y != 0u) {
        return 64u - countLeadingZeros(a.y);
    }
    return 32u - countLeadingZeros(a.x);
}

// Shifts by 0..63; WGSL takes shift counts modulo 32, so the edges are explicit.
fn shl64(a: vec2<u32>, s: u32) -> vec2<u32> {
    if (s == 0u) {
        return a;
    }
    if (s >= 32u) {
        return vec2<u32>(0u, a.x << (s - 32u));
    }
    return vec2<u32>(a.x << s, (a.y << s) | (a.x >> (32u - s)));
}

fn shr64(a: vec2<u32>, s: u32) -> vec2<u32> {
    if (s == 0u) {
        return a;
    }
    if (s >= 32u) {
        return vec2<u32>(a.y >> (s - 32u), 0u);
    }
    return vec2<u32>((a.x >> s) | (a.y << (32u - s)), a.y >> s);
}

// m * n for m < 2^58 and n <= 64, exact.
fn mul64_small(m: vec2<u32>, n: u32) -> vec2<u32> {
    let a = (m.x & 0xffffu) * n;
    let b = (m.x >> 16u) * n;
    let lo = a + (b << 16u);
    return vec2<u32>(lo, m.y * n + (b >> 16u) + select(0u, 1u, lo < a));
}

// ---- binary floating point as sign * m * 2^e ----

struct Soft {
    neg: bool,
    e: i32,
    m: vec2<u32>,
}

// Round m * 2^e (plus a nonzero fraction below m's last bit when sticky) to `bits`
// significant bits, ties to even.
fn round_bits(neg: bool, e: i32, m: vec2<u32>, sticky: bool, bits: u32) -> Soft {
    let len = bitlen64(m);
    if (len <= bits) {
        return Soft(neg, e, m);
    }
    let sh = len - bits;
    var keep = shr64(m, sh);
    let rem = sub64(m, shl64(keep, sh));
    let half = shl64(vec2<u32>(1u, 0u), sh - 1u);
    var up = lt64(half, rem);
    if (eq64(rem, half)) {
        up = sticky || (keep.x & 1u) == 1u;
    }
    if (up) {
        keep = add64(keep, vec2<u32>(1u, 0u));
    }
    var ee = e + i32(sh);
    if (bitlen64(keep) > bits) {
        keep = shr64(keep, 1u);
        ee += 1;
    }
    return Soft(neg, ee, keep);
}

// Left-align a <= 53-bit significand to 62 bits; shifts by 9+ bits stay exact.
fn align62(a: Soft) -> Soft {
    let len = bitlen64(a.m);
    if (len == 0u) {
        return a;
    }
    let s = 62u - len;
    return Soft(a.neg, a.e - i32(s), shl64(a.m, s));
}

// f64 a + b, both with <= 53-bit significands, rounded to nearest even.
fn add_f64(a: Soft, b: Soft) -> Soft {
    if (zero64(a.m)) {
        return b;
    }
    if (zero64(b.m)) {
        return a;
    }
    var x = align62(a);
    var y = align62(b);
    if (x.e < y.e) {
        let t = x;
        x = y;
        y = t;
    }
    let d = u32(x.e - y.e);
    var sticky = false;
    var ym = y.m;
    if (d >= 64u) {
        sticky = true;
        ym = vec2<u32>(0u, 0u);
    } else if (d > 0u) {
        let shifted = shr64(ym, d);
        sticky = !eq64(shl64(shifted, d), ym);
        ym = shifted;
    }
    if (x.neg == y.neg) {
        return round_bits(x.neg, x.e, add64(x.m, ym), sticky, 53u);
    }
    if (d == 0u) {
        if (eq64(x.m, ym)) {
            return Soft(false, 0, vec2<u32>(0u, 0u));
        }
        if (lt64(x.m, ym)) {
            return round_bits(y.neg, x.e, sub64(ym, x.m), false, 53u);
        }
        return round_bits(x.neg, x.e, sub64(x.m, ym), false, 53u);
    }
    // Sticky only arises for d > 9, where |x| - |y| keeps 60+ bits: borrow one unit
    // and let the sticky bit stand for the remaining fraction.
    var diff = sub64(x.m, ym);
    if (sticky) {
        diff = sub64(diff, vec2<u32>(1u, 0u));
    }
    return round_bits(x.neg, x.e, diff, sticky, 53u);
}

// Rust's f64 rem_euclid(1.0): an exact fmod, then + 1.0 (rounded) when negative.
fn rem_euclid_1(s: Soft) -> Soft {
    var r = s;
    if (zero64(s.m)) {
        return s;
    }
    if (s.e >= 0) {
        r = Soft(s.neg, 0, vec2<u32>(0u, 0u));
    } else if (u32(-s.e) < 64u) {
        let f = u32(-s.e);
        r = Soft(s.neg, s.e, sub64(s.m, shl64(shr64(s.m, f), f)));
    }
    if (r.neg && !zero64(r.m)) {
        r = add_f64(r, Soft(false, 0, vec2<u32>(1u, 0u)));
    }
    return r;
}

fn from_f64(lo: u32, hi: u32) -> Soft {
    let exp = (hi >> 20u) & 0x7ffu;
    let frac = hi & 0xfffffu;
    let neg = (hi >> 31u) == 1u;
    if (exp == 0u) {
        return Soft(neg, -1074, vec2<u32>(lo, frac));
    }
    return Soft(neg, i32(exp) - 1075, vec2<u32>(lo, frac | 0x100000u));
}

fn from_f32(bits: u32) -> Soft {
    let exp = (bits >> 23u) & 0xffu;
    let frac = bits & 0x7fffffu;
    let neg = (bits >> 31u) == 1u;
    if (exp == 0u) {
        return Soft(neg, -149, vec2<u32>(frac, 0u));
    }
    return Soft(neg, i32(exp) - 150, vec2<u32>(frac | 0x800000u, 0u));
}

// f64 -> f32 bits, nearest even. Results below the f32 normal range flush to zero,
// as GPU f32 arithmetic does with them anyway.
fn to_f32_bits(y: Soft) -> u32 {
    let sign = select(0u, 0x80000000u, y.neg);
    if (zero64(y.m)) {
        return sign;
    }
    let z = round_bits(y.neg, y.e, y.m, false, 24u);
    let s = 24u - bitlen64(z.m);
    let m = z.m.x << s;
    let biased = z.e - i32(s) + 150;
    if (biased < 1) {
        return sign;
    }
    return sign | (u32(biased) << 23u) | (m & 0x7fffffu);
}

// a * b for 32-bit words, exact: (low word, high word).
fn mul32(a: u32, b: u32) -> vec2<u32> {
    let p00 = (a & 0xffffu) * (b & 0xffffu);
    let p01 = (a & 0xffffu) * (b >> 16u);
    let p10 = (a >> 16u) * (b & 0xffffu);
    let p11 = (a >> 16u) * (b >> 16u);
    let mid = p01 + p10;
    let mid_carry = select(0u, 0x10000u, mid < p01);
    let lo = p00 + (mid << 16u);
    return vec2<u32>(lo, p11 + (mid >> 16u) + mid_carry + select(0u, 1u, lo < p00));
}

// f64 a * b, rounded to nearest even; both finite and nonzero.
fn mul_f64(a: Soft, b: Soft) -> Soft {
    // Left-align both significands to exactly 53 bits.
    let sa = 53u - bitlen64(a.m);
    let sb = 53u - bitlen64(b.m);
    let x = shl64(a.m, sa);
    let y = shl64(b.m, sb);
    let e = a.e - i32(sa) + b.e - i32(sb);
    // 106-bit product in four words w0..w3.
    let ll = mul32(x.x, y.x);
    let lh = mul32(x.x, y.y);
    let hl = mul32(x.y, y.x);
    let hh = mul32(x.y, y.y);
    var t = add64(vec2<u32>(ll.y, 0u), vec2<u32>(lh.x, 0u));
    t = add64(t, vec2<u32>(hl.x, 0u));
    var u = add64(vec2<u32>(lh.y, 0u), vec2<u32>(hl.y, 0u));
    u = add64(u, vec2<u32>(hh.x, 0u));
    u = add64(u, vec2<u32>(t.y, 0u));
    let lo = vec2<u32>(ll.x, t.x);
    let hi = vec2<u32>(u.x, hh.y + u.y);
    // The product has 105 or 106 bits: round its top 64 bits, the rest sticky.
    let len = 64u + bitlen64(hi);
    let sh = len - 64u;
    let top = shl64(hi, 64u - sh);
    let guard = shr64(lo, sh);
    let sticky = !eq64(shl64(guard, sh), lo);
    let m = vec2<u32>(top.x | guard.x, top.y | guard.y);
    return round_bits(a.neg != b.neg, e + i32(sh), m, sticky, 53u);
}


// Soft -> f64 bits (low, high), nearest even already applied. Results below the f64
// normal range flush to signed zero; above it they are infinite.
fn to_f64_bits(a: Soft) -> vec2<u32> {
    let sign = select(0u, 0x80000000u, a.neg);
    if (zero64(a.m)) {
        return vec2<u32>(0u, sign);
    }
    let s = 53u - bitlen64(a.m);
    let m = shl64(a.m, s);
    let biased = a.e - i32(s) + 1075;
    if (biased < 1) {
        return vec2<u32>(0u, sign);
    }
    if (biased >= 2047) {
        return vec2<u32>(0u, sign | 0x7ff00000u);
    }
    return vec2<u32>(m.x, sign | (u32(biased) << 20u) | (m.y & 0xfffffu));
}

fn finite_f32(x: f32) -> bool {
    return (bitcast<u32>(x) & 0x7f800000u) != 0x7f800000u;
}

// pow without the zero-base hazard: some backends (DX12 WARP) evaluate pow(0, y) as
// exp2(y * log2(0)) and return NaN; for y > 0 the value is 0.
fn pow_nonneg(x: f32, y: f32) -> f32 {
    return select(pow(x, y), 0.0, x <= 0.0);
}
