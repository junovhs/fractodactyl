// Per-frame studio shade (SHAD-04/05): fd-shade's Studio colouring with no f64. The
// palette band comes from the f64 phase base t exactly as the CPU picks it,
// ((t + phase).rem_euclid(1.0) * n as f64) as f32, and terrain lines from the same
// exact t + phase, both with soft_f64.wgsl (prepended by the Rust side). Every float
// operation after that is f32, in the CPU's operation order.

struct Params {
    dims: vec4<u32>,      // full width, full height, rectangle x, rectangle y
    rect: vec4<u32>,      // rectangle width, rectangle height
    tone: vec4<f32>,      // phase, density (f32), terrace, slope
    light: vec4<f32>,     // light x, light y, inside r, inside g
    color: vec4<f32>,     // inside b, mean r, mean g, mean b
    extra: vec4<f32>,     // ramp mean, stop count, aa, unresolved-as-interior
    density64: vec4<u32>, // density as f64 bits (low, high)
    lines: vec4<f32>,     // lines, line_px
    haze: vec4<f32>,      // haze tint r, g, b, 1 when the look has haze
}

@group(0) @binding(0) var<storage, read> prepared: array<vec4<u32>>;
@group(0) @binding(1) var<storage, read_write> pixels: array<u32>;
@group(0) @binding(2) var<storage, read> stops: array<vec4<f32>>;
@group(0) @binding(3) var<uniform> p: Params;

const PI: f32 = 3.14159265358979323846;
// Rust's f32 1.0 / 2.2; a WGSL constant expression would fold it one ulp higher.
const INV_GAMMA_BITS: u32 = 0x3ee8ba2eu;
const DITHER_LEVELS: f32 = 2.0;
// Contour lines fade out where band edges are closer than this many pixels.
const LINE_SPACE: f32 = 3.0;

// The bits of ((t + phase).rem_euclid(1.0) * n as f64) as f32 for f64 t = (lo, hi)
// and f32 phase; NaN when t is not finite.
fn band_x(lo: u32, hi: u32, phase_bits: u32, n: u32) -> u32 {
    if (((hi >> 20u) & 0x7ffu) == 0x7ffu) {
        return 0x7fc00000u;
    }
    let r = rem_euclid_1(add_f64(from_f64(lo, hi), from_f32(phase_bits)));
    return to_f32_bits(round_bits(r.neg, r.e, mul64_small(r.m, n), false, 53u));
}

// |c - c.round()| as f32 for c = (t + phase) * n in f64, as fd-shade's terrain lines.
fn edge_distance(lo: u32, hi: u32, phase_bits: u32, n: u32) -> f32 {
    let sum = add_f64(from_f64(lo, hi), from_f32(phase_bits));
    let c = round_bits(false, sum.e, mul64_small(sum.m, n), false, 53u);
    // By symmetry use |c|; its fraction r is exact, and 1 - r is exact for r > 1/2.
    let r = rem_euclid_1(c);
    if (to_f32_bits(r) <= 0x3f000000u) {
        return bitcast<f32>(to_f32_bits(r));
    }
    let rest = add_f64(Soft(false, 0, vec2<u32>(1u, 0u)), Soft(true, r.e, r.m));
    return bitcast<f32>(to_f32_bits(rest));
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
    let s = prepared[2u * i];
    let terms = prepared[2u * i + 1u];
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
        let weight = bitcast<f32>(terms.x);
        if (weight > 0.0) {
            col = (1.0 - weight) * col + weight * p.haze.rgb * bitcast<f32>(terms.y);
        }
        var v = bitcast<f32>(s.w);
        if (p.lines.x > 0.0) {
            let per_px = max(bitcast<f32>(terms.z) * f32(n), 1e-30);
            let dpx = edge_distance(s.x, s.y, bitcast<u32>(p.tone.x), n) / per_px;
            let line = clamp(p.lines.y / 2.0 + 0.5 - dpx, 0.0, 1.0);
            let vis = clamp((1.0 / per_px - LINE_SPACE) / LINE_SPACE, 0.0, 1.0);
            v = v * (1.0 - p.lines.x * line * vis);
        }
        col = col * v;
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
