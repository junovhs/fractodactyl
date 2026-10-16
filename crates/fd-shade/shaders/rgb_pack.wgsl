// Packs the shade/resample output (one u32 per pixel: r | g << 8 | b << 16) into
// RGB24 bytes, four bytes per word, so the host reads back 3/4 of the bytes and copies
// them with one memcpy (SHAD-13).
@group(0) @binding(0) var<storage, read> rgba: array<u32>;
@group(0) @binding(1) var<storage, read_write> rgb: array<u32>;
@group(0) @binding(2) var<uniform> pixels: vec4<u32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let w = id.x + id.y * 65535u * 64u;
    let bytes = pixels.x * 3u;
    if (w * 4u >= bytes) {
        return;
    }
    var word = 0u;
    for (var k = 0u; k < 4u; k = k + 1u) {
        let j = w * 4u + k;
        if (j < bytes) {
            word = word | (((rgba[j / 3u] >> (8u * (j % 3u))) & 255u) << (8u * k));
        }
    }
    rgb[w] = word;
}
