struct Params {
    dims: vec4<u32>, // sample nx, ny, ss, unresolved-as-interior
    clock: vec4<f32>, // time, reconstruction magnification (0 = legacy box), pivot x/y
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
// Graph de is in output pixels: sample-grid pixels times the reconstruction
// magnification and the output pixels per grid pixel (origin.z; 0 means 1, STYL-10).
fn style_scale() -> f32 {
    return max(p.clock.y,1.0)*select(bitcast<f32>(p.origin.z),1.0,p.origin.z==0u);
}
// STYL-08 per-sample rates: de (graph units) as a distance in samples, and how far
// de, nu and the normal move from one sample to the next.
fn style_samples(de: f32) -> f32 {
    return max(de,1e-30)*f32(p.dims.z)/style_scale();
}
fn style_de_rate() -> f32 {
    return style_scale()/f32(p.dims.z);
}
fn style_nu_rate(de: f32) -> f32 {
    return 2.0/(style_samples(de)*0.69314718);
}
fn style_rotate(v: vec2<f32>, angle: f32) -> vec2<f32> {
    let c=cos(angle); let s=sin(angle);
    return vec2<f32>(c*v.x-s*v.y, s*v.x+c*v.y);
}
// Phase-spread evaluations per sample; style_j runs over (-1/2, 1/2) in even steps.
const STYLE_TAPS_N: u32 = 1u; // GRAPH_TAPS
fn style_tap(t: u32) -> f32 {
    return (f32(t)+0.5)/f32(STYLE_TAPS_N)-0.5;
}
fn style_band(bits: vec2<u32>, de: f32, frequency: f32, phase: f32) -> f32 {
    let per_sample=abs(frequency)*2.0/(style_samples(de)*0.69314718);
    return 0.5+0.5*cos(6.2831853*(style_phase(bits,frequency)+phase))*band_response(per_sample);
}
fn style_light(normal: vec2<f32>, slope: f32, direction: vec2<f32>, de: f32) -> f32 {
    let fade=tanh_bounded(max(style_samples(de)/0.7,0.0));
    return 0.15+0.85*max(dot(normalize(vec3<f32>(normal*slope*fade,1.0)),normalize(vec3<f32>(direction,1.0))),0.0);
}
fn clean_color(c: vec3<f32>) -> vec3<f32> {
    // Keep finite positive HDR; protect later convolution from invalid graph results.
    return select(vec3<f32>(0.0), min(c, vec3<f32>(65504.0)), c >= vec3<f32>(0.0));
}
struct SampleFields { de: f32, local_nu: f32, normal: vec2<f32> }
fn style_fields(i: u32) -> SampleFields {
    let s=samples[i];
    return SampleFields(max(bitcast<f32>(s.z),0.0)*style_scale(),bitcast<f32>(to_f32_bits(add_f64(from_f64(s.x,s.y),from_f64(p.origin.x,p.origin.y ^ 0x80000000u)))),unit[s.w & 65535u]);
}
// Reconstruction is shared by pointwise and image graphs. Integrate shaded
// samples before any image passes, never interpolate nu/normals before lighting.
// A continuous tent footprint avoids sample-grid popping as the camera moves.
fn style_footprint(pixel: vec2<u32>) -> vec4<f32> {
    let ss=f32(p.dims.z);
    if (p.clock.y==0.0) {
        let lo=vec2<f32>(pixel)*ss;
        return vec4<f32>(lo,lo+ss);
    }
    let pivot=p.clock.zw*vec2<f32>(p.dims.xy);
    let center=pivot+((vec2<f32>(pixel)+0.5)*ss-pivot)/p.clock.y;
    let radius=max(ss/p.clock.y,1.0);
    return vec4<f32>(max(center-radius,vec2<f32>(0.0)),min(center+radius,vec2<f32>(p.dims.xy)));
}
fn style_weight(sample_xy: vec2<u32>, pixel: vec2<u32>) -> f32 {
    if (p.clock.y==0.0) { return 1.0; }
    let ss=f32(p.dims.z);
    let pivot=p.clock.zw*vec2<f32>(p.dims.xy);
    let center=pivot+((vec2<f32>(pixel)+0.5)*ss-pivot)/p.clock.y;
    let w=max(vec2<f32>(0.0),1.0-abs(vec2<f32>(sample_xy)+0.5-center)/max(ss/p.clock.y,1.0));
    return w.x*w.y;
}
// GRAPH_HELPERS
fn evaluate(i: u32, position: vec2<f32>, style_j: f32) -> Layer {
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
    let footprint=style_footprint(gid.xy);
    let lo=vec2<u32>(floor(footprint.xy));
    let hi=vec2<u32>(ceil(footprint.zw));
    var total=0.0;
    for (var y=lo.y; y<hi.y; y++) {
        for (var x=lo.x; x<hi.x; x++) {
            let xy=vec2<u32>(x,y);
            let weight=style_weight(xy,gid.xy);
            if (weight<=0.0) { continue; }
            for (var t=0u; t<STYLE_TAPS_N; t++) {
                let c=evaluate(xy.y*p.dims.x+xy.x,(vec2<f32>(xy)+0.5)/f32(ss),style_tap(t));
                surface+=c.surface*weight;
                emission+=c.emission*weight;
                total+=weight;
            }
        }
    }
    let scale=1.0/max(total,1e-20);
    hdr[gid.y * size.x + gid.x] = Layer(surface * scale, emission * scale);
}
