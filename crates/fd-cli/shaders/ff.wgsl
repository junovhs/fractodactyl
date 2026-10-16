// Float-float and f32+exponent arithmetic for the f32-only zone kernel (GPU-27, DEC-25):
// no f64 anywhere, so it runs on adapters without SHADER_F64. Ported from GPU-24's
// research arm (tools/research/gpu-route/src/ff.wgsl). A float-float x = hi + lo with
// |lo| <= ulp(hi) / 2 carries about 48 significant bits; an FE adds an i32 exponent
// for magnitudes outside f32's range.
//
// Compilers fold the error terms these algorithms depend on (Mesa NIR rewrites
// a + b - a to b; NVIDIA and Intel reassociate too), so every intermediate goes through
// opq(), an XOR with a runtime zero (FF_KEY) the compiler cannot see. An entry point
// must call ff_init(key) with key = 0 read from a buffer before any ff_* call.
// FF_GUARD = false strips the guard; it exists only to prove the guard is needed.

override FF_GUARD: bool = true;
var<private> FF_KEY: u32;
// TwoProd by fma only when fma is fused exactly on this device (Vulkan lets drivers
// split it into a multiply and an add, as RADV does); else Dekker's split.
var<private> FF_FUSED: bool;

fn opq(x: f32) -> f32 {
    if (FF_GUARD) {
        return bitcast<f32>(bitcast<u32>(x) ^ FF_KEY);
    }
    return x;
}

// Run the fma self-test and arm the reassociation guard; key must be a runtime 0.
fn ff_init(key: u32) {
    FF_KEY = key;
    // (1 + 2^-12)^2 = 1 + 2^-11 + 2^-24: the product rounds the 2^-24 tie away, so a
    // fused fma recovers exactly 2^-24 and a split one returns 0.
    let a = opq(1.000244140625);
    let p = opq(a * a);
    FF_FUSED = fma(a, a, -p) == 5.9604644775390625e-8;
}

struct FF { hi: f32, lo: f32 }

fn ff(x: f32) -> FF { return FF(x, 0.0); }
fn ff_neg(x: FF) -> FF { return FF(-x.hi, -x.lo); }

fn two_sum(a: f32, b: f32) -> FF {
    let s = opq(a + b);
    let bb = opq(s - a);
    let av = opq(s - bb);
    return FF(s, opq(a - av) + opq(b - bb));
}
// Needs |a| >= |b| (or a == 0).
fn fast_two_sum(a: f32, b: f32) -> FF {
    let s = opq(a + b);
    return FF(s, b - opq(s - a));
}
fn veltkamp(a: f32) -> FF {
    let c = opq(4097.0 * a);
    let h = opq(c - opq(c - a));
    return FF(h, a - h);
}
// a * b exactly as hi + lo.
fn two_prod(a: f32, b: f32) -> FF {
    let p = opq(a * b);
    if (FF_FUSED) {
        return FF(p, fma(a, b, -p));
    }
    let x = veltkamp(a);
    let y = veltkamp(b);
    let e = opq(opq(opq(opq(x.hi * y.hi - p) + x.hi * y.lo) + x.lo * y.hi) + x.lo * y.lo);
    return FF(p, e);
}

fn ff_add(x: FF, y: FF) -> FF {
    var s = two_sum(x.hi, y.hi);
    let t = two_sum(x.lo, y.lo);
    s.lo += t.hi;
    s = fast_two_sum(s.hi, s.lo);
    s.lo += t.lo;
    return fast_two_sum(s.hi, s.lo);
}
fn ff_sub(x: FF, y: FF) -> FF { return ff_add(x, ff_neg(y)); }
fn ff_mul(x: FF, y: FF) -> FF {
    var p = two_prod(x.hi, y.hi);
    p.lo += x.hi * y.lo + x.lo * y.hi;
    return fast_two_sum(p.hi, p.lo);
}
fn ff_mul_f(x: FF, f: f32) -> FF {
    var p = two_prod(x.hi, f);
    p.lo += x.lo * f;
    return fast_two_sum(p.hi, p.lo);
}
// Three quotient digits: each residual is exact, so GPU division's few-ulp error in
// the digit estimates does not reach the result.
fn ff_div(x: FF, y: FF) -> FF {
    let q1 = x.hi / y.hi;
    var r = ff_sub(x, ff_mul_f(y, q1));
    let q2 = r.hi / y.hi;
    r = ff_sub(r, ff_mul_f(y, q2));
    let q3 = r.hi / y.hi;
    return ff_add(fast_two_sum(q1, q2), ff(q3));
}
// x >= 0.
fn ff_sqrt(x: FF) -> FF {
    if (x.hi <= 0.0) {
        return ff(0.0);
    }
    let s = sqrt(x.hi);
    let r = ff_sub(x, two_prod(s, s));
    return fast_two_sum(s, r.hi / (2.0 * s));
}

// 2^k from the IEEE bits (no ldexp; GPU-19), 0 below the normal range.
fn pow2(k: i32) -> f32 {
    return select(0.0, bitcast<f32>(u32(clamp(k, -126, 127) + 127) << 23u), k >= -126);
}
// x * 2^k exactly while the result stays normal; k may exceed f32's exponent range.
fn ff_scale(x: FF, k: i32) -> FF {
    let a = k / 2;
    let b = k - a;
    return FF(x.hi * pow2(a) * pow2(b), x.lo * pow2(a) * pow2(b));
}

// f32+exponent: value = m * 2^e with |m.hi| in [0.5, 1), or m = 0 (e = 0).
struct FE { m: FF, e: i32 }

fn fe_norm(m: FF, e: i32) -> FE {
    if (m.hi == 0.0) {
        return FE(ff(0.0), 0);
    }
    let k = i32((bitcast<u32>(m.hi) >> 23u) & 255u) - 126;
    return FE(ff_scale(m, -k), e + k);
}
// An f64 given as its bits (low word, high word), to the nearest FE (48 of its 53
// bits; the dropped tail is below 2^-48 relative). Zero, subnormals, inf and NaN map
// to zero; callers screen them first.
fn fe_from_f64(lo_word: u32, hi_word: u32) -> FE {
    let biased = i32((hi_word >> 20u) & 2047u);
    if (biased == 0 || biased == 2047) {
        return FE(ff(0.0), 0);
    }
    let sign = select(1.0, -1.0, (hi_word >> 31u) == 1u);
    // Mantissa 1.f as three exact pieces: 21 + 24 + 8 bits, each an exact f32.
    let top = f32((hi_word & 1048575u) | 1048576u); // 21 bits, value * 2^20
    let mid = f32(lo_word >> 8u); // 24 bits, value * 2^20 * 2^24
    let low = f32(lo_word & 255u); // 8 bits, value * 2^20 * 2^32
    var m = ff_add(ff(top * pow2(-20)), ff(mid * pow2(-44)));
    m = ff_add(m, ff(low * pow2(-52)));
    return fe_norm(FF(sign * m.hi, sign * m.lo), biased - 1023);
}
fn fe_mul(x: FE, y: FE) -> FE { return fe_norm(ff_mul(x.m, y.m), x.e + y.e); }
fn fe_add(x: FE, y: FE) -> FE {
    if (x.m.hi == 0.0) { return y; }
    if (y.m.hi == 0.0) { return x; }
    if (x.e >= y.e) {
        return fe_norm(ff_add(x.m, ff_scale(y.m, max(y.e - x.e, -150))), x.e);
    }
    return fe_norm(ff_add(ff_scale(x.m, max(x.e - y.e, -150)), y.m), y.e);
}
// The FE as a float-float (0 below f32's normal range, inf above).
fn fe_to_ff(x: FE) -> FF { return ff_scale(x.m, x.e); }

// Complex FE with one shared exponent: (re + i im) * 2^e, max(|re.hi|, |im.hi|) in
// [0.5, 1) unless zero.
struct CE { re: FF, im: FF, e: i32 }

fn ce_norm(re: FF, im: FF, e: i32) -> CE {
    let big = max(abs(re.hi), abs(im.hi));
    if (big == 0.0) {
        return CE(ff(0.0), ff(0.0), 0);
    }
    let k = i32((bitcast<u32>(big) >> 23u) & 255u) - 126;
    return CE(ff_scale(re, -k), ff_scale(im, -k), e + k);
}
fn ce_mul(x: CE, y: CE) -> CE {
    let re = ff_sub(ff_mul(x.re, y.re), ff_mul(x.im, y.im));
    let im = ff_add(ff_mul(x.re, y.im), ff_mul(x.im, y.re));
    return ce_norm(re, im, x.e + y.e);
}
fn ce_add(x: CE, y: CE) -> CE {
    // A zero carries exponent 0, which must not decide the alignment.
    if (x.re.hi == 0.0 && x.im.hi == 0.0) { return y; }
    if (y.re.hi == 0.0 && y.im.hi == 0.0) { return x; }
    if (x.e >= y.e) {
        let d = max(y.e - x.e, -150);
        return ce_norm(ff_add(x.re, ff_scale(y.re, d)), ff_add(x.im, ff_scale(y.im, d)), x.e);
    }
    let d = max(x.e - y.e, -150);
    return ce_norm(ff_add(ff_scale(x.re, d), y.re), ff_add(ff_scale(x.im, d), y.im), y.e);
}

const FF_LN2 = FF(0.6931471824645996, -1.9046542121259336e-09);
const FF_PI = FF(3.1415927410125732, -8.742277657347586e-08);
const FF_PI_2 = FF(1.5707963705062866, -4.371138828673793e-08);
// pi/2 in four parts for Cody-Waite reduction; the first two have 12 trailing zero
// bits, so k * part is exact for |k| < 2^12.
const PI_2_A: f32 = 1.5703125;
const PI_2_B: f32 = 0.0004837512969970703;
const PI_2_C: f32 = 7.549790126404332e-08;
const PI_2_D: f32 = -1.7151245100058819e-15;

// 1 / n! for n = 0..14.
const INV_FACT = array<FF, 15>(
    FF(1.0, 0.0), FF(1.0, 0.0), FF(0.5, 0.0),
    FF(0.1666666716337204, -4.967053879312289e-09),
    FF(0.0416666679084301, -1.2417634698280722e-09),
    FF(0.008333333767950535, -4.34617203337595e-10),
    FF(0.0013888889225199819, -3.3631094437103215e-11),
    FF(0.00019841270113829523, -2.725596874933456e-12),
    FF(2.4801587642286904e-05, -3.40699609366682e-13),
    FF(2.7557318844628753e-06, 3.793571224297229e-14),
    FF(2.755731998149713e-07, -7.575112209051195e-15),
    FF(2.5052107943679403e-08, 4.4176230446483665e-16),
    FF(2.0876755879584152e-09, 1.1082839147459852e-16),
    FF(1.6059044372074283e-10, -5.352526511562726e-18),
    FF(1.147074536050896e-11, 2.372207689231238e-19),
);

// exp(x) for |x| < 87: x = k ln2 + r, |r| <= ln2 / 2, Taylor to r^14 (tail < 2^-57).
fn ff_exp(x: FF) -> FF {
    let k = round(x.hi * 1.4426950408889634);
    let r = ff_sub(ff_sub(x, two_prod(FF_LN2.hi, k)), ff_mul_f(ff(FF_LN2.lo), k));
    var s = INV_FACT[14];
    for (var n = 13; n >= 0; n = n - 1) {
        s = ff_add(ff_mul(s, r), INV_FACT[n]);
    }
    return ff_scale(s, i32(k));
}
// ln x for x > 0: two Newton steps y += x e^-y - 1 from f32 log.
fn ff_log(x: FF) -> FF {
    var y = ff(log(x.hi));
    for (var i = 0; i < 2; i = i + 1) {
        y = ff_add(y, ff_sub(ff_mul(x, ff_exp(ff_neg(y))), ff(1.0)));
    }
    return y;
}

struct SinCos { s: FF, c: FF }

// sin and cos of x for |x| < 2^12 * pi / 2: Cody-Waite reduction to |r| <= pi / 4,
// then Taylor to r^13 / r^14 (tails below 2^-45).
fn ff_sincos(x: FF) -> SinCos {
    let k = round(x.hi * 0.6366197723675814);
    var r = ff_sub(x, ff(k * PI_2_A));
    r = ff_sub(r, ff(k * PI_2_B));
    r = ff_sub(r, two_prod(k, PI_2_C));
    r = ff_sub(r, ff(k * PI_2_D));
    let r2 = ff_mul(r, r);
    var s = INV_FACT[13];
    var c = INV_FACT[14];
    for (var n = 11; n >= 1; n = n - 2) {
        s = ff_add(ff_neg(ff_mul(s, r2)), INV_FACT[n]);
    }
    for (var n = 12; n >= 0; n = n - 2) {
        c = ff_add(ff_neg(ff_mul(c, r2)), INV_FACT[n]);
    }
    // Horner with alternating signs: s = 1/1! - r2/3! + ..., c = 1 - r2/2! + ...
    s = ff_mul(s, r);
    let q = i32(k) & 3;
    if (q == 0) { return SinCos(s, c); }
    if (q == 1) { return SinCos(c, ff_neg(s)); }
    if (q == 2) { return SinCos(ff_neg(s), ff_neg(c)); }
    return SinCos(ff_neg(c), s);
}

fn atan_small32(z: f32) -> f32 {
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
// atan2 in f32 from a fixed polynomial (driver atan2 accuracy varies; WARP's is poor).
fn atan2_32(y: f32, x: f32) -> f32 {
    let ax = abs(x);
    let ay = abs(y);
    if (ax == 0.0 && ay == 0.0) {
        return 0.0;
    }
    let t = min(ax, ay) / max(ax, ay);
    var a = select(atan_small32(t), 0.7853981633974483 + atan_small32((t - 1.0) / (t + 1.0)), t > 0.41421356);
    if (ay > ax) { a = 1.5707963267948966 - a; }
    if (x < 0.0) { a = 3.141592653589793 - a; }
    if (y < 0.0) { a = -a; }
    return a;
}
// atan2(y, x) in (-pi, pi]: one Newton step on the f32 angle,
// a += (y cos a - x sin a) / (x cos a + y sin a), which squares its ~1e-7 error.
fn ff_atan2(y: FF, x: FF) -> FF {
    if (x.hi == 0.0 && y.hi == 0.0) {
        return ff(0.0);
    }
    let a = ff(atan2_32(y.hi, x.hi));
    let sc = ff_sincos(a);
    let num = ff_sub(ff_mul(y, sc.c), ff_mul(x, sc.s));
    let den = ff_add(ff_mul(x, sc.c), ff_mul(y, sc.s));
    return ff_add(a, ff_div(num, den));
}
