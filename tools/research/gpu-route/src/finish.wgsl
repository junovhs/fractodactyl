// GPU-12 research arm only. Native Vulkan f64 extension, not WebGPU portable.
// Identical packed checkpoints and recurrence order to the shared Rust arm.
@group(0) @binding(0) var<storage, read> inputs: array<f64>;
@group(0) @binding(1) var<storage, read> iterations: array<u32>;
@group(0) @binding(2) var<storage, read_write> output: array<f64>;
@group(0) @binding(3) var<storage, read_write> words: array<u32>;

@group(0) @binding(4) var<storage, read> camera: array<f64>;
@group(0) @binding(5) var<storage, read_write> de_out: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) pos: vec3<u32>) {
    let i = pos.x;
    if i >= arrayLength(&iterations) { return; }
    let at = i * 4u;
    var zr = inputs[at];
    var zi = inputs[at + 1u];
    var dr = inputs[at + 2u];
    var di = inputs[at + 3u];
    let cr = camera[3];
    let ci = camera[4];
    let escape_sq = camera[5];
    var n = iterations[i];
    var cls = 2u;
    while n < 20000u {
        let next_dr = f64(2.0) * (zr * dr - zi * di) + f64(1.0);
        let next_di = f64(2.0) * (zr * di + zi * dr);
        let next_zr = zr * zr - zi * zi + cr;
        zi = f64(2.0) * zr * zi + ci;
        zr = next_zr;
        dr = next_dr;
        di = next_di;
        n += 1u;
        if zr * zr + zi * zi > escape_sq {
            cls = 0u;
            break;
        }
    }
    var encoded = vec3<f64>(0.0);
    if cls == 0u { encoded = encode(zr, zi, dr, di, n); }
    output[i] = encoded.x;
    de_out[i] = f32(encoded.y);
    let angle = encoded.z;
    var word = cls << 16u;
    if !(angle <= f64(3.141592653589793) && angle >= f64(-3.141592653589793)) { word = 3u << 16u; }
    else {
        let turns = angle * f64(10430.378350470453);
        var rounded = 0i;
        if turns >= f64(0.0) { rounded = i32(turns + f64(0.5)); }
        else { rounded = i32(turns - f64(0.5)); }
        word |= u32(rounded) & 65535u;
    }
    words[i] = word;

}

// Research copy of the original GLSL binary64 log/atan polynomials. Shader
// f64 log/atan are not Vulkan intrinsics; no f32 substitution is made here.
fn log2_positive(x: f64) -> f64 {
    let parts = frexp(x);
    var e = parts.exp - 1;
    var m = f64(2.0) * parts.fract;
    if m > f64(1.4142135623730951) { m *= f64(0.5); e += 1; }
    let t = (m - f64(1.0)) / (m + f64(1.0));
    let t2 = t * t;
    let coef = array<f64, 12>(1.0, 0.3333333333333333, 0.2, 0.14285714285714285,
        0.1111111111111111, 0.09090909090909091, 0.07692307692307693,
        0.06666666666666667, 0.058823529411764705, 0.05263157894736842,
        0.047619047619047616, 0.043478260869565216);
    var sum = coef[11];
    for (var k = 10; k >= 0; k -= 1) { sum = sum * t2 + coef[k]; }
    return (f64(e) * f64(0.6931471805599453) + f64(2.0) * t * sum) * f64(1.4426950408889634);
}
fn atan_small(z: f64) -> f64 {
    let coef = array<f64, 24>(1.0, -0.3333333333333333, 0.2, -0.14285714285714285,
        0.1111111111111111, -0.09090909090909091, 0.07692307692307693,
        -0.06666666666666667, 0.058823529411764705, -0.05263157894736842,
        0.047619047619047616, -0.043478260869565216, 0.04,
        -0.037037037037037035, 0.034482758620689655, -0.03225806451612903,
        0.030303030303030304, -0.02857142857142857, 0.02702702702702703,
        -0.02564102564102564, 0.024390243902439025, -0.023255813953488372,
        0.022222222222222223, -0.02127659574468085);
    let zz = z * z;
    var p = coef[23];
    for (var k = 22; k >= 0; k -= 1) { p = p * zz + coef[k]; }
    return z * p;
}
fn atan_unit(x: f64) -> f64 {
    if x > f64(0.41421356237309503) {
        return f64(0.7853981633974483) + atan_small((x - f64(1.0)) / (x + f64(1.0)));
    }
    return atan_small(x);
}
fn atan2_64(y: f64, x: f64) -> f64 {
    let ax = abs(x); let ay = abs(y);
    if ax == f64(0.0) && ay == f64(0.0) { return f64(0.0); }
    var a = f64(0.0);
    if ay > ax { a = f64(1.5707963267948966) - atan_unit(ax / ay); }
    else { a = atan_unit(ay / ax); }
    if x < f64(0.0) { a = f64(3.141592653589793) - a; }
    if y < f64(0.0) { a = -a; }
    return a;
}
fn encode(zr: f64, zi: f64, dr: f64, di: f64, n: u32) -> vec3<f64> {
    let a = zr * zr + zi * zi;
    let log2z = f64(0.5) * log2_positive(a);
    let nu = f64(n) + f64(1.0) - log2_positive(log2z);
    let de = f64(2.0) * sqrt(a) * (log2z * f64(0.6931471805599453))
        / sqrt(dr * dr + di * di) / camera[0];
    let re = zr * dr + zi * di; let im = zi * dr - zr * di;
    let angle = atan2_64(camera[2] * re - camera[1] * im, camera[1] * re + camera[2] * im);
    return vec3<f64>(nu, de, angle);
}
