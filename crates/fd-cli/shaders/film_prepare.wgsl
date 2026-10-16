// Time-invariant per-sample studio relief and band anti-aliasing (SHAD-03): the WGSL
// port of film_prepare.comp, f32 arithmetic only. One dispatch per newly uploaded
// keyframe, not once per output frame.

struct Params {
    dims: vec4<u32>,  // width, height, unused, unused
    tone: vec4<f32>,  // unused, density, unused, slope
    light: vec4<f32>, // light x, light y, unused, unused
    extra: vec4<f32>, // unused, unused, anti-alias flag, unused
}

@group(0) @binding(0) var<storage, read> samples: array<vec4<u32>>;
@group(0) @binding(1) var<storage, read_write> cooked: array<vec4<u32>>;
@group(0) @binding(2) var<uniform> p: Params;

const ANGLE_PER_STEP: f32 = 6.2831853 / 65536.0;
const LN_2: f32 = 0.6931471805599453;
// -2 * (pi * sigma)^2 with sigma = 0.5
const BAND_K: f32 = -4.9348022;

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
        // Encode interior=-1 and unresolved=-2 in the band slot; retain both exact
        // f64 phase words for escaped samples.
        cooked[i] = vec4<u32>(s.x, s.y, bitcast<u32>(-f32(kind)), bitcast<u32>(0.0));
        return;
    }
    let d = max(bitcast<f32>(s.z), 0.0);
    let angle = f32(s.w & 65535u) * ANGLE_PER_STEP;
    let normal = vec2<f32>(cos(angle), sin(angle));
    let facing = -dot(normal, p.light.xy);
    let cycles = p.tone.y * 2.0 / (max(d, 1e-30) * LN_2);
    let aa = p.extra.z > 0.5;
    var band: f32 = 1.0;
    if (aa) {
        band = exp(BAND_K * cycles * cycles);
    }
    let steep = tanh(0.6 * log(1.0 + cycles / 0.02));
    var fade: f32 = 1.0;
    if (aa) {
        fade = tanh(d / 0.7);
    }
    let v = (1.0 + p.tone.w * facing * steep * fade) * (1.0 - 0.6 * exp(-d * 1.5));
    cooked[i] = vec4<u32>(s.x, s.y, bitcast<u32>(band), bitcast<u32>(v));
}
