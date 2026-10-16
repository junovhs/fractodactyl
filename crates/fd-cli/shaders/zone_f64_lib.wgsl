// Binary64 complex helpers and transcendentals shared by the wgpu zone kernels
// (zone.wgsl, zone_mid.wgsl; DEC-24). Exponents come from the IEEE bits: pinned
// Naga's f64 frexp crashes the NVIDIA Windows pipeline compiler (GPU-19).

alias C = vec2<f64>;

// Zone constants in; host column formats out: nu, f32 DE in output pixels,
// u16 normal | class << 16.
@group(0) @binding(0) var<storage, read> input_data: array<f64>;
@group(0) @binding(1) var<storage, read_write> nu_out: array<f64>;
@group(0) @binding(2) var<storage, read_write> de_out: array<f32>;
@group(0) @binding(3) var<storage, read_write> word_out: array<u32>;

fn value(i: i32) -> f64 { return input_data[i]; }
fn cx(i: i32) -> C { return C(value(i), value(i + 1)); }

fn mul(a: C, b: C) -> C {
    return C(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x);
}
fn norm2(z: C) -> f64 { return dot(z, z); }
fn divc(a: C, b: C) -> C {
    return C(dot(a, b), a.y * b.x - a.x * b.y) / norm2(b);
}
fn ldexp2(a: C, e: i32) -> C { return C(ldexp(a.x, e), ldexp(a.y, e)); }
// NaN or infinity: all exponent bits set.
fn nonfinite(x: f64) -> bool {
    return ((bitcast<u64>(x) >> 52u) & 2047lu) == 2047lu;
}

const FP64_LN2: f64 = 0.693147180559945309417232121458;
const FP64_INV_LN2: f64 = 1.442695040888963407359924681002;
const FP64_LN2_LO: f64 = 2.3190468138462995584e-17;
const FP64_PI: f64 = 3.141592653589793238462643383280;
const FP64_PI_2: f64 = 1.570796326794896619231321691640;
const FP64_PI_4: f64 = 0.785398163397448309615660845820;
const FP64_INV_PI_2: f64 = 0.636619772367581343075535053490;
const FP64_PI_2_LO: f64 = 6.1232339957367660359e-17;

// Series coefficients are compile-time tables: no sample divides by or
// rebuilds a constant (DEC-10 math unchanged, one rounding per coefficient).
const LOG_INV_ODD = array<f64, 12>(
    1.0, 0.3333333333333333, 0.2, 0.14285714285714285,
    0.1111111111111111, 0.09090909090909091, 0.07692307692307693, 0.06666666666666667,
    0.058823529411764705, 0.05263157894736842, 0.047619047619047616, 0.043478260869565216);
const EXP_INV_FACT = array<f64, 21>(
    1.0, 1.0, 0.5, 0.16666666666666666,
    0.041666666666666664, 0.008333333333333333, 0.001388888888888889, 0.0001984126984126984,
    2.48015873015873e-05, 2.7557319223985893e-06, 2.755731922398589e-07, 2.505210838544172e-08,
    2.08767569878681e-09, 1.6059043836821613e-10, 1.1470745597729725e-11, 7.647163731819816e-13,
    4.779477332387385e-14, 2.8114572543455206e-15, 1.5619206968586225e-16, 8.22063524662433e-18,
    4.110317623312165e-19);
const ATAN_C = array<f64, 24>(
    1.0, -0.3333333333333333, 0.2, -0.14285714285714285,
    0.1111111111111111, -0.09090909090909091, 0.07692307692307693, -0.06666666666666667,
    0.058823529411764705, -0.05263157894736842, 0.047619047619047616, -0.043478260869565216,
    0.04, -0.037037037037037035, 0.034482758620689655, -0.03225806451612903,
    0.030303030303030304, -0.02857142857142857, 0.02702702702702703, -0.02564102564102564,
    0.024390243902439025, -0.023255813953488372, 0.022222222222222223, -0.02127659574468085);
const SIN_C = array<f64, 14>(
    1.0, -0.16666666666666666, 0.008333333333333333, -0.0001984126984126984,
    2.7557319223985893e-06, -2.505210838544172e-08, 1.6059043836821613e-10, -7.647163731819816e-13,
    2.8114572543455206e-15, -8.22063524662433e-18, 1.9572941063391263e-20, -3.868170170630684e-23,
    6.446950284384474e-26, -9.183689863795546e-29);
const COS_C = array<f64, 14>(
    1.0, -0.5, 0.041666666666666664, -0.001388888888888889,
    2.48015873015873e-05, -2.755731922398589e-07, 2.08767569878681e-09, -1.1470745597729725e-11,
    4.779477332387385e-14, -1.5619206968586225e-16, 4.110317623312165e-19, -8.896791392450574e-22,
    1.6117375710961184e-24, -2.4795962632247976e-27);

// f64-only atanh series on the mantissa reduced to [1/sqrt(2), sqrt(2)];
// |(m-1)/(m+1)| <= 0.171573, so 12 odd terms suffice at binary64 precision.
// Exponent and mantissa come from the bits, as frexp gives for positive
// normal x.
fn fp64_log(x: f64) -> f64 {
    let bits = bitcast<u64>(x);
    var e = i32((bits >> 52u) & 2047lu) - 1023;
    var m = bitcast<f64>((bits & 4503599627370495lu) | 4607182418800017408lu);
    if m > 1.4142135623730950488 {
        m *= 0.5;
        e += 1;
    }
    let t = (m - 1.0) / (m + 1.0);
    let t2 = t * t;
    var sum = LOG_INV_ODD[11];
    for (var k = 10; k >= 0; k -= 1) { sum = fma(sum, t2, LOG_INV_ODD[k]); }
    return f64(e) * FP64_LN2 + 2.0 * t * sum;
}
fn fp64_log2(x: f64) -> f64 { return fp64_log(x) * FP64_INV_LN2; }

// x = k ln(2) + r, |r| <= ln(2)/2. Degree-20 exp Taylor in double.
fn fp64_exp(x: f64) -> f64 {
    let k = i32(round(x * FP64_INV_LN2));
    var r = fma(-f64(k), FP64_LN2, x);
    r = fma(-f64(k), FP64_LN2_LO, r);
    var p = EXP_INV_FACT[20];
    for (var i = 19; i >= 0; i -= 1) { p = fma(p, r, EXP_INV_FACT[i]); }
    return ldexp(p, k);
}

// atan(y/x) after octant reduction: argument <= sqrt(2)-1. 24 odd terms.
fn fp64_atan_small(z: f64) -> f64 {
    let zz = z * z;
    var p = ATAN_C[23];
    for (var k = 22; k >= 0; k -= 1) { p = fma(zz, p, ATAN_C[k]); }
    return z * p;
}
fn fp64_atan_unit(x: f64) -> f64 {
    if x > 0.4142135623730950488 {
        return FP64_PI_4 + fp64_atan_small((x - 1.0) / (x + 1.0));
    }
    return fp64_atan_small(x);
}
fn fp64_atan2(y: f64, x: f64) -> f64 {
    let ax = abs(x);
    let ay = abs(y);
    if ax == 0.0 && ay == 0.0 { return 0.0; }
    var a: f64;
    if ay > ax { a = FP64_PI_2 - fp64_atan_unit(ax / ay); } else { a = fp64_atan_unit(ay / ax); }
    if x < 0.0 { a = FP64_PI - a; }
    if y < 0.0 { return -a; }
    return a;
}

// Double Cody-Waite reduction to [-pi/4, pi/4] using fused hi/lo pi/2.
// Horner degrees 27 (sin) and 26 (cos), then restore the quadrant.
fn fp64_sincos(x: f64) -> C {
    let q = i32(round(x * FP64_INV_PI_2));
    var r = fma(-f64(q), FP64_PI_2, x);
    r = fma(-f64(q), FP64_PI_2_LO, r);
    let rr = r * r;
    var sp = SIN_C[13];
    var cp = COS_C[13];
    for (var k = 12; k >= 0; k -= 1) {
        sp = fma(sp, rr, SIN_C[k]);
        cp = fma(cp, rr, COS_C[k]);
    }
    let sr = r * sp;
    let cr = cp;
    let quadrant = (q % 4 + 4) % 4;
    if quadrant == 0 { return C(sr, cr); }
    if quadrant == 1 { return C(cr, -sr); }
    if quadrant == 2 { return C(-sr, -cr); }
    return C(-cr, sr);
}

fn output_sample(gid: i32, cls: u32, nu: f64, de: f64, angle: f64) {
    // The normal is the angle in 1/65536 turns, wrapped; a non-finite angle
    // marks the sample invalid (class 3) so the host rejects the frame.
    var word = cls << 16u;
    if nonfinite(angle) {
        word = 3u << 16u;
    } else {
        // Half away from zero, as GLSL round compiled for GPU-03.
        let turns = angle * 10430.378350470452724949566316381;
        var rounded: i32;
        if turns >= 0.0 { rounded = i32(turns + 0.5); } else { rounded = i32(turns - 0.5); }
        word |= u32(rounded) & 65535u;
    }
    nu_out[gid] = nu;
    de_out[gid] = f32(de);
    word_out[gid] = word;
}
