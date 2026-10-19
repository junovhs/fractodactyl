struct Params {
    dims: vec4<u32>, // sample nx, ny, ss, unresolved-as-interior
    clock: vec4<f32>,
    origin: vec4<u32>,
    knobs: array<vec4<f32>, 64>,
    optics: vec4<f32>,
    grain: vec4<u32>,
    tonal: vec4<f32>,
    image_offsets: array<vec4<u32>,16>,
}
struct Layer { surface: vec4<f32>, emission: vec4<f32> }
@group(0) @binding(0) var<storage, read> samples: array<vec4<u32>>;
@group(0) @binding(1) var<storage, read_write> hdr: array<Layer>;
@group(0) @binding(2) var<uniform> p: Params;
@group(0) @binding(3) var<storage, read> unit: array<vec2<f32>>;

fn style_phase(bits: vec2<u32>, frequency: f32) -> f32 {
    return bitcast<f32>(to_f32_bits(rem_euclid_1(mul_f64(from_f64(bits.x, bits.y), from_f32(bitcast<u32>(frequency))))));
}
fn style_band(bits: vec2<u32>, de: f32, frequency: f32, phase: f32) -> f32 {
    let per_sample=abs(frequency)*2.0/(max(de,1e-30)*0.69314718*f32(p.dims.z));
    return 0.5+0.5*cos(6.2831853*(style_phase(bits,frequency)+phase))*band_response(per_sample);
}
fn style_light(normal: vec2<f32>, slope: f32, direction: vec2<f32>, de: f32) -> f32 {
    let fade=tanh_bounded(max(de*f32(p.dims.z)/0.7,0.0));
    return 0.15+0.85*max(dot(normalize(vec3<f32>(normal*slope*fade,1.0)),normalize(vec3<f32>(direction,1.0))),0.0);
}
fn clean_color(c: vec3<f32>) -> vec3<f32> {
    // Keep finite positive HDR; protect later convolution from invalid graph results.
    return select(vec3<f32>(0.0), min(c, vec3<f32>(65504.0)), c >= vec3<f32>(0.0));
}
struct SampleFields { de: f32, local_nu: f32, normal: vec2<f32> }
fn style_fields(i: u32) -> SampleFields {
    let s=samples[i];
    return SampleFields(max(bitcast<f32>(s.z),0.0),bitcast<f32>(to_f32_bits(add_f64(from_f64(s.x,s.y),from_f64(p.origin.x,p.origin.y ^ 0x80000000u)))),unit[s.w & 65535u]);
}
// GRAPH_HELPERS
fn evaluate(i: u32, position: vec2<f32>) -> Layer {
    let s = samples[i];
    let kind = s.w >> 16u;
    if (kind != 0u) {
        var color = vec3<f32>(0.0);
        if (kind != 1u && p.dims.w == 0u) { color = vec3<f32>(0.25, 0.0, 0.25); }
        return Layer(vec4<f32>(color, 1.0), vec4<f32>(0.0));
    }
    let fields = style_fields(i);
    let de = fields.de;
    let normal = fields.normal;
    let local_nu = fields.local_nu;
// GRAPH_BODY
// GRAPH_RETURN
}
@compute @workgroup_size(8, 8)
fn shade(@builtin(global_invocation_id) gid: vec3<u32>) {
    let ss = p.dims.z;
    let size = p.dims.xy / ss;
    if (any(gid.xy >= size)) { return; }
    var surface = vec4<f32>(0.0);
    var emission = vec4<f32>(0.0);
    for (var y = 0u; y < ss; y++) {
        for (var x = 0u; x < ss; x++) {
            let xy = gid.xy * ss + vec2<u32>(x,y);
            let c = evaluate(xy.y * p.dims.x + xy.x, (vec2<f32>(xy) + 0.5) / f32(ss));
            surface += c.surface;
            emission += c.emission;
        }
    }
    let scale = 1.0 / f32(ss * ss);
    hdr[gid.y * size.x + gid.x] = Layer(surface * scale, emission * scale);
}
