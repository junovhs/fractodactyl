// Complex f32 mantissa + shared i32 exponent. Shared by zone and generic
// perturbation kernels (DEC-24/25). Requires pow2 from ff.wgsl.
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
fn cf_inv(a: CF) -> CF { return cf_norm(vec2<f32>(a.m.x, -a.m.y) / dot(a.m, a.m), -a.e); }
fn cf_bad(a: CF) -> bool { return (bitcast<u32>(a.m.x) & 0x7f800000u) == 0x7f800000u || (bitcast<u32>(a.m.y) & 0x7f800000u) == 0x7f800000u; }

