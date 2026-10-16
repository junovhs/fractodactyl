// Per-frame studio shade (SHAD-04): the WGSL port of film_shade.comp with no f64.
// The palette band comes from the f64 phase base t exactly as PreparedStudio picks it,
// ((t + phase).rem_euclid(1.0) * n as f64) as f32, evaluated bit-exactly with integer
// mantissas held in vec2<u32> (x = low word, y = high word). Every float operation
// after that is f32, in the CPU's operation order.

struct Params {
    dims: vec4<u32>,  // full width, full height, rectangle x, rectangle y
    rect: vec4<u32>,  // rectangle width, rectangle height
    tone: vec4<f32>,  // phase, density, terrace, slope
    light: vec4<f32>, // light x, light y, inside r, inside g
    color: vec4<f32>, // inside b, mean r, mean g, mean b
    extra: vec4<f32>, // ramp mean, stop count, aa, unresolved-as-interior
}

@group(0) @binding(0) var<storage, read> prepared: array<vec4<u32>>;
@group(0) @binding(1) var<storage, read_write> pixels: array<u32>;
@group(0) @binding(2) var<storage, read> stops: array<vec4<f32>>;
@group(0) @binding(3) var<uniform> p: Params;

const PI: f32 = 3.14159265358979323846;
// Rust's f32 1.0 / 2.2; a WGSL constant expression would fold it one ulp higher.
const INV_GAMMA_BITS: u32 = 0x3ee8ba2eu;
const DITHER_LEVELS: f32 = 2.0;

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

// The bits of ((t + phase).rem_euclid(1.0) * n as f64) as f32 for f64 t = (lo, hi)
// and f32 phase; NaN when t is not finite.
fn band_x(lo: u32, hi: u32, phase_bits: u32, n: u32) -> u32 {
    if (((hi >> 20u) & 0x7ffu) == 0x7ffu) {
        return 0x7fc00000u;
    }
    let r = rem_euclid_1(add_f64(from_f64(lo, hi), from_f32(phase_bits)));
    return to_f32_bits(round_bits(r.neg, r.e, mul64_small(r.m, n), false, 53u));
}

// ---- encode, as fd-shade's resolve ----

fn tpdf(key: u32) -> f32 {
    var x = key + 0x9e3779b9u;
    x ^= x >> 16u;
    x *= 0x7feb352du;
    x ^= x >> 15u;
    x *= 0x846ca68bu;
    x ^= x >> 16u;
    return f32((x & 65535u) + (x >> 16u)) / 65535.0 - 1.0;
}

fn encode(rgb: vec3<f32>, x: u32, y: u32) -> u32 {
    var out = 0u;
    for (var j = 0u; j < 3u; j++) {
        let dither = DITHER_LEVELS * tpdf((y << 16u) ^ (x << 2u) ^ j);
        let v = clamp(rgb[j], 0.0, 1.0);
        let level = clamp(pow(v, bitcast<f32>(INV_GAMMA_BITS)) * 255.0 + 0.5 + dither, 0.0, 255.0);
        out |= u32(level) << (8u * j);
    }
    return out;
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= p.rect.x || gid.y >= p.rect.y) {
        return;
    }
    let xy = gid.xy + p.dims.zw;
    let i = xy.y * p.dims.x + xy.x;
    let s = prepared[i];
    let band = bitcast<f32>(s.z);
    var col = vec3<f32>(p.light.z, p.light.w, p.color.x);
    if (band >= 0.0) {
        let n = u32(p.extra.y);
        let xb = band_x(s.x, s.y, bitcast<u32>(p.tone.x), n);
        if (xb == 0x7fc00000u) {
            // A non-finite phase base turns every channel NaN on the CPU: black.
            pixels[i] = 0u;
            return;
        }
        let x = bitcast<f32>(xb);
        let k = min(u32(x), n - 1u);
        let fr = x - f32(k);
        let f = 0.5 - 0.5 * cos(PI * fr);
        let c0 = stops[k].rgb;
        let c1 = stops[(k + 1u) % n].rgb;
        col = c0 + (c1 - c0) * f;
        if (p.tone.z > 0.0) {
            col = col * (1.0 - p.tone.z * pow(1.0 - fr, 1.5));
        }
        if (p.extra.z > 0.5) {
            let mean = vec3<f32>(p.color.y, p.color.z, p.color.w);
            col = band * col + (1.0 - band) * mean * p.extra.x;
        }
        col = col * bitcast<f32>(s.w);
    } else if (band <= -2.0 && p.extra.w < 0.5) {
        col = vec3<f32>(0.25, 0.0, 0.25);
    }
    pixels[i] = encode(col, xy.x, xy.y);
}

// Test entry: case = (t low word, t high word, phase bits, stop count) -> band_x bits.
@compute @workgroup_size(64)
fn band_select(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= arrayLength(&pixels)) {
        return;
    }
    let c = prepared[gid.x];
    pixels[gid.x] = band_x(c.x, c.y, c.z, c.w);
}
