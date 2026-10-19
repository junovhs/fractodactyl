struct Params {
    dims: vec4<u32>,
    clock: vec4<f32>,
    origin: vec4<u32>,
    knobs: array<vec4<f32>, 64>,
}
struct Layer { surface: vec4<f32>, emission: vec4<f32> }
@group(0) @binding(0) var<storage, read> hdr: array<Layer>;
@group(0) @binding(1) var<storage, read_write> pixels: array<u32>;
@group(0) @binding(2) var<uniform> p: Params;

@compute @workgroup_size(8, 8)
fn finish(@builtin(global_invocation_id) gid: vec3<u32>) {
    let size = p.dims.xy / p.dims.z;
    if (any(gid.xy >= size)) { return; }
    let i = gid.y * size.x + gid.x;
    let c = max(hdr[i].surface.rgb + hdr[i].emission.rgb, vec3<f32>(0.0));
    let mapped = c / (vec3<f32>(1.0) + c);
    let srgb = select(1.055 * pow(mapped, vec3<f32>(1.0/2.4)) - 0.055, 12.92 * mapped, mapped <= vec3<f32>(0.0031308));
    let b = vec3<u32>(clamp(srgb * 255.0 + 0.5, vec3<f32>(0.0), vec3<f32>(255.0)));
    pixels[i] = b.x | (b.y << 8u) | (b.z << 16u);
}
