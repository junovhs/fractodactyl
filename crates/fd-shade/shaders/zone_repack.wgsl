// Zone kernel columns to the studio's packed samples, on the device (GPU-21): the zone
// kernel writes nu (f64 bits), de (f32) and normal | class << 16 as separate columns;
// the prepare pass reads one vec4 per sample, as fd_shade::gpu::pack_samples packs it on
// the host (nu bits only for escaped samples, else 0). A sample with an invalid class,
// or an escaped one with non-finite nu or de, sets the flag word: the host reads only
// that word and rejects the keyframe (DEC-10), as the CPU unpack did.
@group(0) @binding(0) var<storage, read> nu: array<vec2<u32>>;
@group(0) @binding(1) var<storage, read> de: array<u32>;
@group(0) @binding(2) var<storage, read> word: array<u32>;
@group(0) @binding(3) var<storage, read_write> samples: array<vec4<u32>>;
@group(0) @binding(4) var<storage, read_write> invalid: atomic<u32>;
@group(0) @binding(5) var<uniform> count: vec4<u32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x + id.y * 65535u * 64u;
    if (i >= count.x) {
        return;
    }
    let w = word[i];
    let kind = w >> 16u;
    let n = nu[i];
    let d = de[i];
    let escaped = kind == 0u;
    let finite = ((n.y >> 20u) & 0x7ffu) != 0x7ffu && ((d >> 23u) & 0xffu) != 0xffu;
    if (kind > 2u || (escaped && !finite)) {
        atomicOr(&invalid, 1u);
    }
    var bits = vec2<u32>(0u, 0u);
    if (escaped) {
        bits = n;
    }
    samples[i] = vec4<u32>(bits.x, bits.y, d, w);
}
