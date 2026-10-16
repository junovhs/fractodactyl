// Shared by the f32-only zone kernels (zone32.wgsl, zone_mid32.wgsl; DEC-24): the
// bindings, number formats on top of ff.wgsl and the column encoding. No f64, no i64.
//
// - CE (ff.wgsl): complex float-float with a shared exponent, any magnitude.
// - CF: complex f32 mantissa with a shared exponent, for dz/dc-only quantities.
// - R: real f32 mantissa with an exponent, for magnitudes and comparisons.
// - ZF: complex float-float, for O(1) orbits.

// The zone constants as binary64 bit patterns (low word, high word).
@group(0) @binding(0) var<storage, read> input_data: array<vec2<u32>>;
@group(0) @binding(1) var<storage, read_write> nu_out: array<vec2<u32>>;
@group(0) @binding(2) var<storage, read_write> de_out: array<f32>;
@group(0) @binding(3) var<storage, read_write> word_out: array<u32>;

fn fe_at(i: i32) -> FE { let w = input_data[i]; return fe_from_f64(w.x, w.y); }
// An O(1) constant as a float-float (zero, or exponents within f32's range).
fn ff_at(i: i32) -> FF { return fe_to_ff(fe_at(i)); }
// Small integers and counts.
fn int_at(i: i32) -> i32 { return i32(round(ff_at(i).hi)); }
fn ce_pair(re: FE, im: FE) -> CE {
    return ce_add(CE(re.m, ff(0.0), re.e), CE(ff(0.0), im.m, im.e));
}
fn ce_at(i: i32) -> CE { return ce_pair(fe_at(i), fe_at(i + 1)); }

// ---- CE helpers beyond ff.wgsl ----
fn ce_zero() -> CE { return CE(ff(0.0), ff(0.0), 0); }
fn ce_neg(x: CE) -> CE { return CE(ff_neg(x.re), ff_neg(x.im), x.e); }
fn ce_sub(x: CE, y: CE) -> CE { return ce_add(x, ce_neg(y)); }
fn ce_from_ff(re: FF, im: FF) -> CE { return ce_norm(re, im, 0); }
fn ce_mul_fe(x: CE, s: FE) -> CE {
    return ce_norm(ff_mul(x.re, s.m), ff_mul(x.im, s.m), x.e + s.e);
}
fn ce_mul_f(x: CE, s: f32) -> CE { return ce_norm(ff_mul_f(x.re, s), ff_mul_f(x.im, s), x.e); }
// The value as float-floats (0 below f32's normal range).
fn ce_re(x: CE) -> FF { return ff_scale(x.re, x.e); }
fn ce_im(x: CE) -> FF { return ff_scale(x.im, x.e); }
fn ce_bad(x: CE) -> bool { return !(finite_f32(x.re.hi) && finite_f32(x.im.hi)); }

// ---- R: a real f32 mantissa times 2^e, for magnitudes and comparisons ----
struct R { m: f32, e: i32 }
fn r_norm(m: f32, e: i32) -> R {
    if (m == 0.0) { return R(0.0, 0); }
    let k = i32((bitcast<u32>(abs(m)) >> 23u) & 255u) - 126;
    return R(m * pow2(-k), e + k);
}
fn r_fe(x: FE) -> R { return r_norm(x.m.hi, x.e); }
fn r_mul(a: R, b: R) -> R { return r_norm(a.m * b.m, a.e + b.e); }
fn r_div(a: R, b: R) -> R { return r_norm(a.m / b.m, a.e - b.e); }
fn r_f32(a: R) -> f32 {
    let h = a.e / 2;
    return a.m * pow2(h) * pow2(a.e - h);
}
// a <= b for non-negative values.
fn r_le(a: R, b: R) -> bool {
    if (a.m == 0.0) { return true; }
    if (b.m == 0.0) { return false; }
    if (a.e != b.e) { return a.e < b.e; }
    return a.m <= b.m;
}
fn ce_len(x: CE) -> R { return r_norm(length(vec2<f32>(x.re.hi, x.im.hi)), x.e); }

// ---- CF: complex f32 mantissa times 2^e ----
struct CF { m: vec2<f32>, e: i32 }
fn cmul(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x);
}
fn cf_norm(m: vec2<f32>, e: i32) -> CF {
    let big = max(abs(m.x), abs(m.y));
    if (big == 0.0) { return CF(vec2<f32>(0.0), 0); }
    let k = i32((bitcast<u32>(big) >> 23u) & 255u) - 126;
    return CF(m * pow2(-k), e + k);
}
fn cf_zero() -> CF { return CF(vec2<f32>(0.0), 0); }
fn cf_one() -> CF { return CF(vec2<f32>(0.5, 0.0), 1); }
fn cf_mul(a: CF, b: CF) -> CF { return cf_norm(cmul(a.m, b.m), a.e + b.e); }
fn cf_add(a: CF, b: CF) -> CF {
    if (a.m.x == 0.0 && a.m.y == 0.0) { return b; }
    if (b.m.x == 0.0 && b.m.y == 0.0) { return a; }
    let e = max(a.e, b.e);
    return cf_norm(a.m * pow2(max(a.e - e, -150)) + b.m * pow2(max(b.e - e, -150)), e);
}
fn cf_scale(a: CF, s: f32) -> CF { return cf_norm(a.m * s, a.e); }
fn cf_of(x: CE) -> CF { return cf_norm(vec2<f32>(x.re.hi, x.im.hi), x.e); }
fn cf_len(a: CF) -> R { return r_norm(length(a.m), a.e); }
fn cf_inv(a: CF) -> CF { return cf_norm(vec2<f32>(a.m.x, -a.m.y) / dot(a.m, a.m), -a.e); }
fn ce_mul_cf(x: CE, y: CF) -> CE {
    let re = ff_sub(ff_mul_f(x.re, y.m.x), ff_mul_f(x.im, y.m.y));
    let im = ff_add(ff_mul_f(x.re, y.m.y), ff_mul_f(x.im, y.m.x));
    return ce_norm(re, im, x.e + y.e);
}
fn cf_bad(a: CF) -> bool { return !(finite_f32(a.m.x) && finite_f32(a.m.y)); }

// ---- ZF: complex float-float for the escape finish ----
struct ZF { re: FF, im: FF }
fn zf_of(x: CE) -> ZF { return ZF(ce_re(x), ce_im(x)); }
fn zf_add(a: ZF, b: ZF) -> ZF { return ZF(ff_add(a.re, b.re), ff_add(a.im, b.im)); }

const TAU = FF(6.2831854820251465, -1.7484555314695172e-07);


fn output_sample(gid: i32, cls: u32, nu: FF, de: f32, angle: f32) {
    // The normal is the angle in 1/65536 turns, wrapped; a non-finite angle marks the
    // sample invalid (class 3) so the host rejects the keyframe.
    var word = cls << 16u;
    if (!finite_f32(angle) || !finite_f32(de) || !finite_f32(nu.hi)) {
        word = 3u << 16u;
    } else {
        let turns = angle * 10430.378350470453;
        var rounded: i32;
        if (turns >= 0.0) { rounded = i32(turns + 0.5); } else { rounded = i32(turns - 0.5); }
        word |= u32(rounded) & 65535u;
    }
    // nu as the host's f64: hi + lo summed exactly in software binary64.
    nu_out[gid] = to_f64_bits(add_f64(from_f32(bitcast<u32>(nu.hi)), from_f32(bitcast<u32>(nu.lo))));
    de_out[gid] = de;
    word_out[gid] = word;
}

// Whether z has escaped the radius whose square is esc. The test runs in plain f32: on
// the escaping step |z|^2 can reach ~1e40, past f32's range, where the float-float
// product's error term would be inf - inf = NaN; f32 overflows to inf, which still
// compares above the radius. Within 1e-6 of the radius the float-float value decides.
fn escaped32(z: ZF, esc: f32) -> bool {
    let a32 = z.re.hi * z.re.hi + z.im.hi * z.im.hi;
    if (a32 > esc * 1.000001) { return true; }
    if (a32 > esc * 0.999999) {
        return ff_add(ff_mul(z.re, z.re), ff_mul(z.im, z.im)).hi > esc;
    }
    return false;
}

// Encode an escaped sample after n steps: nu = n + 1 - log2(log2|z|), DE =
// 2 |z| ln|z| / |dz/dc| / pixel in output pixels, and the normal z conj(dz/dc) in screen
// axes (cs, sn). z is rescaled by 2^-k first, so nothing overflows.
fn emit_escape(gid: i32, n: i32, z: ZF, du: CF, px: R, cs: f32, sn: f32) {
    let big = max(abs(z.re.hi), abs(z.im.hi));
    let k = i32((bitcast<u32>(big) >> 23u) & 255u) - 126;
    let zr = ff_scale(z.re, -k);
    let zi = ff_scale(z.im, -k);
    let as_ = ff_add(ff_mul(zr, zr), ff_mul(zi, zi));
    // ln a = ln(as) + 2 k ln 2; log2|z| = ln a / (2 ln 2).
    let ln_a = ff_add(ff_log(as_), ff_mul_f(FF_LN2, 2.0 * f32(k)));
    let log2z = ff_mul_f(ff_div(ln_a, FF_LN2), 0.5);
    let nu = ff_sub(ff(f32(n + 1)), ff_div(ff_log(log2z), FF_LN2));
    let num = R(2.0 * sqrt(as_.hi) * (log2z.hi * 0.6931471805599453), k);
    let de = r_f32(r_div(num, r_mul(cf_len(du), px)));
    let re = zr.hi * du.m.x + zi.hi * du.m.y;
    let im = zi.hi * du.m.x - zr.hi * du.m.y;
    output_sample(gid, 0u, nu, de, atan2_32(sn * re - cs * im, cs * re + sn * im));
}
