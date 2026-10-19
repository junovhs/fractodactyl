// Half-resolution emission extraction and a separable mixture of Gaussian kernels.
// Weights are calculated once on the host when the radius changes.
struct BlurParams { dims: vec4<u32>, full: vec4<u32>, weights: array<vec4<f32>, 33> }
struct Layer { surface: vec4<f32>, emission: vec4<f32> }
@group(0) @binding(0) var<storage, read> layers: array<Layer>;
@group(0) @binding(1) var<storage, read_write> extracted: array<vec4<f32>>;
@group(0) @binding(2) var<uniform> p: BlurParams;
@group(0) @binding(3) var<storage, read> source: array<vec4<f32>>;

@compute @workgroup_size(8, 8)
fn extract(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (any(gid.xy >= p.dims.xy)) { return; }
    var value = vec4<f32>(0.0);
    for (var y = 0u; y < 2u; y++) {
        for (var x = 0u; x < 2u; x++) {
            let xy = min(2u * gid.xy + vec2<u32>(x,y), p.full.xy - 1u);
            value += layers[xy.y * p.full.x + xy.x].emission;
        }
    }
    extracted[gid.y * p.dims.x + gid.x] = value * 0.25;
}

@compute @workgroup_size(8, 8)
fn blur(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (any(gid.xy >= p.dims.xy)) { return; }
    let i = gid.y * p.dims.x + gid.x;
    var value = source[i] * p.weights[0].x;
    let axis = select(vec2<i32>(1,0),vec2<i32>(0,1), p.dims.z == 1u);
    for (var k = 1u; k <= p.dims.w; k++) {
        let a = vec2<u32>(clamp(vec2<i32>(gid.xy) - axis * i32(k),vec2<i32>(0),vec2<i32>(p.dims.xy)-1));
        let b = vec2<u32>(clamp(vec2<i32>(gid.xy) + axis * i32(k),vec2<i32>(0),vec2<i32>(p.dims.xy)-1));
        value += (source[a.y * p.dims.x + a.x] + source[b.y * p.dims.x + b.x]) * p.weights[k].x;
    }
    extracted[i] = value;
}
