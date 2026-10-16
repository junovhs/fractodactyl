// Per-sample studio terms (SHAD-03/05): the WGSL counterpart of the per-sample half of
// fd-shade's Studio, f32 and integer arithmetic only. It turns each raw sample into
// the palette base t = density * nu (exact f64, emulated) and the relief, band-AA,
// haze and line-density terms. One dispatch per keyframe upload, and again whenever a
// breathing look changes the density.
// soft_f64.wgsl is prepended by the Rust side.

// The shade pass's parameter block (film_gpu::ShadePush).
struct Params {
    dims: vec4<u32>,      // full width, full height, rectangle x, rectangle y
    rect: vec4<u32>,      // rectangle width, rectangle height, samples per pixel side
    tone: vec4<f32>,      // phase, density (f32), terrace, slope
    light: vec4<f32>,     // light x, light y, inside r, inside g
    color: vec4<f32>,     // inside b, mean r, mean g, mean b
    extra: vec4<f32>,     // ramp mean, stop count, aa, unresolved-as-interior
    density64: vec4<u32>, // density as f64 bits (low, high)
    lines: vec4<f32>,     // lines, line_px, 1 to dither the output
    haze: vec4<f32>,      // haze tint r, g, b, 1 when the look has haze
}

// Raw samples: (nu f64 low, nu f64 high, de f32 bits, normal | kind << 16).
@group(0) @binding(0) var<storage, read> samples: array<vec4<u32>>;
// Two words per sample: (t low, t high, band, light) then (haze weight, haze tone,
// cycles, 0); band is -1 interior and -2 unresolved.
@group(0) @binding(1) var<storage, read_write> cooked: array<vec4<u32>>;
@group(0) @binding(2) var<uniform> p: Params;

const ANGLE_PER_STEP: f32 = 6.2831853 / 65536.0;
const LN_2: f32 = 0.6931471805599453;
// -2 * (pi * sigma)^2 with sigma = 0.5
const BAND_K: f32 = -4.9348022;

// density * nu as f64 bits, as Rust evaluates it.
fn base(lo: u32, hi: u32) -> vec2<u32> {
    let nu = from_f64(lo, hi);
    let d = from_f64(p.density64.x, p.density64.y);
    if (((hi >> 20u) & 0x7ffu) == 0x7ffu) {
        return vec2<u32>(lo, hi ^ select(0u, 0x80000000u, d.neg));
    }
    if (zero64(nu.m)) {
        return vec2<u32>(0u, select(0u, 0x80000000u, nu.neg != d.neg));
    }
    return to_f64_bits(mul_f64(nu, d));
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let xy = gid.xy;
    if (xy.x >= p.dims.x || xy.y >= p.dims.y) {
        return;
    }
    let i = xy.y * p.dims.x + xy.x;
    let s = samples[i];
    let kind = s.w >> 16u;
    if (kind != 0u) {
        cooked[2u * i] = vec4<u32>(0u, 0u, bitcast<u32>(-f32(kind)), bitcast<u32>(0.0));
        cooked[2u * i + 1u] = vec4<u32>(0u);
        return;
    }
    let t = base(s.x, s.y);
    let ss = f32(max(p.rect.z, 1u));
    let d = max(bitcast<f32>(s.z), 0.0);
    let angle = f32(s.w & 65535u) * ANGLE_PER_STEP;
    let normal = vec2<f32>(cos(angle), sin(angle));
    let facing = -dot(normal, p.light.xy);
    let cycles = p.tone.y * 2.0 / (max(d, 1e-30) * LN_2);
    let aa = p.extra.z > 0.5;
    var band: f32 = 1.0;
    if (aa) {
        let per_sample = cycles / ss;
        band = exp(BAND_K * per_sample * per_sample);
    }
    let steep = tanh(0.6 * log(1.0 + cycles / 0.02));
    var fade: f32 = 1.0;
    if (aa) {
        fade = tanh(max(d * ss / 0.7, 0.0));
    }
    let v = (1.0 + p.tone.w * facing * steep * fade) * (1.0 - 0.6 * exp(-d * ss * 1.5));
    // fd-shade haze_terms.
    var weight = 0.0;
    var tone = 0.0;
    if (p.haze.w > 0.5 && d > 0.0 && finite_f32(d) && finite_f32(cycles)) {
        let e = clamp(cycles / ss - 1.0, 0.0, 1.0);
        weight = e * e * (3.0 - 2.0 * e);
        let x = d * ss;
        tone = 0.2 + 2.8 * x / (x + 0.02);
    }
    cooked[2u * i] = vec4<u32>(t.x, t.y, bitcast<u32>(band), bitcast<u32>(v));
    cooked[2u * i + 1u] = vec4<u32>(bitcast<u32>(weight), bitcast<u32>(tone), bitcast<u32>(cycles), 0u);
}
