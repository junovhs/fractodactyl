// GPU-09/GPU-05 research arm: float-float finish with no f64 anywhere, so it
// also runs on adapters without SHADER_F64 (DEC-25). Port of finff in
// tools/research/twolevel/zone_finish/zfin.c: z is float-float (TwoSum adds,
// fma TwoProd, about 48 bits), dz/dc is an f32 mantissa with an i32 exponent.
// Inputs and camera arrive as u32 bit patterns; nu leaves as an exact
// (integer, fraction) f32 pair that the host sums in binary64.
@group(0) @binding(0) var<storage, read> inputs: array<vec4<u32>>;
@group(0) @binding(1) var<storage, read> camera: array<u32>;
@group(0) @binding(2) var<storage, read_write> nu_out: array<vec2<f32>>;
@group(0) @binding(3) var<storage, read_write> de_out: array<f32>;
@group(0) @binding(4) var<storage, read_write> words: array<u32>;

// Reassociation guard: XOR with a runtime zero the compiler cannot see, so
// inexact algebraic rules (e.g. Mesa NIR's a + b - a -> b) cannot fold the
// TwoSum/TwoProd error terms. FD_GPU_ROUTE_FF_NOGUARD=1 strips it to test.
var<private> KEY: u32;
fn opq(x: f32) -> f32 { return bitcast<f32>(bitcast<u32>(x) ^ KEY); }

struct FF { hi: f32, lo: f32 }

// Every intermediate is opaque: otherwise (a - (s - bb)) + (b - bb) may be
// reassociated to (a + b) - s, which is 0 (seen on the NVIDIA driver).
fn two_sum(a: f32, b: f32) -> FF {
    let s = opq(a + b);
    let bb = opq(s - a);
    let av = opq(s - bb);
    return FF(s, opq(a - av) + opq(b - bb));
}
fn fast_two_sum(a: f32, b: f32) -> FF {
    let s = opq(a + b);
    return FF(s, b - opq(s - a));
}
fn ff_add(x: FF, y: FF) -> FF {
    var s = two_sum(x.hi, y.hi);
    let t = two_sum(x.lo, y.lo);
    s.lo += t.hi;
    s = fast_two_sum(s.hi, s.lo);
    s.lo += t.lo;
    return fast_two_sum(s.hi, s.lo);
}
fn ff_mul(x: FF, y: FF) -> FF {
    let p = opq(x.hi * y.hi);
    var e = fma(x.hi, y.hi, -p);
    e = fma(x.hi, y.lo, e);
    e = fma(x.lo, y.hi, e);
    return fast_two_sum(p, e);
}
// Dekker/Veltkamp TwoProd without fma (Vulkan lets drivers split fma into a
// multiply and an add). Selected by FD_GPU_ROUTE_FF_TWOPROD=dekker.
fn veltkamp(a: f32) -> FF {
    let c = opq(4097.0 * a);
    let h = opq(c - opq(c - a));
    return FF(h, a - h);
}
fn ff_mul_dekker(x: FF, y: FF) -> FF {
    let p = opq(x.hi * y.hi);
    let a = veltkamp(x.hi);
    let b = veltkamp(y.hi);
    var e = opq(opq(opq(a.hi * b.hi - p) + a.hi * b.lo) + a.lo * b.hi) + a.lo * b.lo;
    e += x.hi * y.lo + x.lo * y.hi;
    return fast_two_sum(p, e);
}

// 2^k from the IEEE bits (no ldexp/frexp; GPU-19); 0 below the normal range.
fn pow2(k: i32) -> f32 {
    return select(0.0, bitcast<f32>(u32(clamp(k, -126, 127) + 127) << 23u), k >= -126);
}
// frexp exponent (mantissa in [0.5, 1)) of a positive normal f32.
fn exponent_of(x: f32) -> i32 { return i32((bitcast<u32>(x) >> 23u) & 255u) - 126; }

// log2(hi + lo) for hi + lo in [sqrt(1/2), sqrt(2)]: atanh series, |t| <= 0.172.
fn log2_near1(hi: f32, lo: f32) -> f32 {
    let t = ((hi - 1.0) + lo) / ((hi + 1.0) + lo);
    let t2 = t * t;
    var s = 0.07692307692307693;
    s = s * t2 + 0.09090909090909091;
    s = s * t2 + 0.1111111111111111;
    s = s * t2 + 0.14285714285714285;
    s = s * t2 + 0.2;
    s = s * t2 + 0.3333333333333333;
    s = s * t2 + 1.0;
    return 2.0 * t * s * 1.4426950408889634;
}
// log2 x = e + f, e an integer (as f32), |f| <= 0.5, for positive normal x.
fn log2_split(x: f32) -> vec2<f32> {
    let bits = bitcast<u32>(x);
    var e = i32((bits >> 23u) & 255u) - 127;
    var m = bitcast<f32>((bits & 8388607u) | 1065353216u);
    if m > 1.4142135 { m *= 0.5; e += 1; }
    return vec2<f32>(f32(e), log2_near1(m, 0.0));
}

fn atan_small(z: f32) -> f32 {
    let zz = z * z;
    var p = -0.043478260869565216;
    p = p * zz + 0.047619047619047616;
    p = p * zz - 0.05263157894736842;
    p = p * zz + 0.058823529411764705;
    p = p * zz - 0.06666666666666667;
    p = p * zz + 0.07692307692307693;
    p = p * zz - 0.09090909090909091;
    p = p * zz + 0.1111111111111111;
    p = p * zz - 0.14285714285714285;
    p = p * zz + 0.2;
    p = p * zz - 0.3333333333333333;
    p = p * zz + 1.0;
    return z * p;
}
fn atan_unit(x: f32) -> f32 {
    if x > 0.41421356 { return 0.78539816 + atan_small((x - 1.0) / (x + 1.0)); }
    return atan_small(x);
}
fn atan2_32(y: f32, x: f32) -> f32 {
    let ax = abs(x); let ay = abs(y);
    if ax == 0.0 && ay == 0.0 { return 0.0; }
    var a = 0.0;
    if ay > ax { a = 1.5707963 - atan_unit(ax / ay); } else { a = atan_unit(ay / ax); }
    if x < 0.0 { a = 3.1415927 - a; }
    if y < 0.0 { a = -a; }
    return a;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) pos: vec3<u32>) {
    let i = pos.x;
    if i >= arrayLength(&words) { return; }
    KEY = camera[9];
    let a = inputs[2u * i];
    let b = inputs[2u * i + 1u];
    var zr = FF(bitcast<f32>(a.x), bitcast<f32>(a.y));
    var zi = FF(bitcast<f32>(a.z), bitcast<f32>(a.w));
    var dr = bitcast<f32>(b.x);
    var di = bitcast<f32>(b.y);
    var kd = bitcast<i32>(b.z);
    var n = b.w;
    let cr = FF(bitcast<f32>(camera[0]), bitcast<f32>(camera[1]));
    let ci = FF(bitcast<f32>(camera[2]), bitcast<f32>(camera[3]));
    let escape_sq = bitcast<f32>(camera[6]);
    var cls = 2u;
    while n < 20000u {
        // dz/dc' = 2 z D + 1, z rounded to its hi word, D = (dr, di) 2^kd.
        let pr = 2.0 * (zr.hi * dr - zi.hi * di);
        let pi = 2.0 * (zr.hi * di + zi.hi * dr);
        let kn = max(kd, 1);
        let sc = pow2(kd - kn);
        dr = pr * sc + pow2(-kn);
        di = pi * sc;
        let big = max(abs(dr), abs(di));
        let e = select(0, exponent_of(big), big > 0.0);
        dr *= pow2(-e);
        di *= pow2(-e);
        kd = kn + e;
        // z' = z^2 + C in float-float.
        let r2 = ff_mul(zr, zr);
        let i2 = ff_mul(zi, zi);
        let ri = ff_mul(zr, zi);
        let next_zr = ff_add(ff_add(r2, FF(-i2.hi, -i2.lo)), cr);
        zi = ff_add(FF(2.0 * ri.hi, 2.0 * ri.lo), ci);
        zr = next_zr;
        n += 1u;
        if zr.hi * zr.hi + zi.hi * zi.hi > escape_sq {
            cls = 0u;
            break;
        }
    }
    var nu = vec2<f32>(0.0);
    var de = 0.0;
    var angle = 0.0;
    if cls == 0u {
        // log2|z| = log2 M + log2 sqrt(1 + r^2), kept as float-float (33..67).
        let ax = abs(zr.hi); let ay = abs(zi.hi);
        let big = max(ax, ay);
        let r = min(ax, ay) / big;
        let root = sqrt(1.0 + r * r);
        let lm = log2_split(big);
        let lq = log2_split(root);
        let l = two_sum(lm.x, lm.y + (lq.x + lq.y));
        // nu = n + 1 - log2(log2|z|), integer and fraction kept apart.
        let k = exponent_of(l.hi) - 1;
        var mh = l.hi * pow2(-k);
        var ml = l.lo * pow2(-k);
        var kk = k;
        if mh > 1.4142135 { mh *= 0.5; ml *= 0.5; kk += 1; }
        nu = vec2<f32>(f32(n + 1u) - f32(kk), -log2_near1(mh, ml));
        // DE = 2|z| ln|z| / |D| / h, with D = (dr, di) 2^kd and h = hm 2^kh.
        let hm = bitcast<f32>(camera[7]);
        let kh = bitcast<i32>(camera[8]);
        let q = 2.0 * big * root * (l.hi * 0.6931472) / (sqrt(dr * dr + di * di) * hm);
        let s = -(kd + kh);
        de = q * pow2(s / 2) * pow2(s - s / 2);
        let cs = bitcast<f32>(camera[4]);
        let sn = bitcast<f32>(camera[5]);
        let re = zr.hi * dr + zi.hi * di;
        let im = zi.hi * dr - zr.hi * di;
        angle = atan2_32(sn * re - cs * im, cs * re + sn * im);
    }
    nu_out[i] = nu;
    de_out[i] = de;
    var word = cls << 16u;
    if !(angle <= 3.1415927 && angle >= -3.1415927) { word = 3u << 16u; }
    else {
        let turns = angle * 10430.378;
        var rounded = 0i;
        if turns >= 0.0 { rounded = i32(turns + 0.5); } else { rounded = i32(turns - 0.5); }
        word |= u32(rounded) & 65535u;
    }
    words[i] = word;
}
