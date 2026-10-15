// GPU-09 ff self-test entry point, appended after the ff.wgsl helpers.
// Per case: in = a, b, z.hi/lo (re, im), pad, pad; out = 12 f32 (see ff.rs).
@group(0) @binding(0) var<storage, read> cases: array<f32>;
@group(0) @binding(1) var<storage, read> key: array<u32>;
@group(0) @binding(2) var<storage, read_write> out: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) pos: vec3<u32>) {
    let i = pos.x;
    if i * 8u >= arrayLength(&cases) { return; }
    KEY = key[0];
    let a = cases[8u * i];
    let b = cases[8u * i + 1u];
    let s = two_sum(a, b);
    // FastTwoSum needs |x| >= |y|.
    var x = a;
    var y = b;
    if abs(b) > abs(a) { x = b; y = a; }
    let f = fast_two_sum(x, y);
    let p = opq(a * b);
    let e = fma(a, b, -p);
    let m = ff_mul(FF(a, 0.0), FF(b, 0.0));
    let zr = FF(cases[8u * i + 2u], cases[8u * i + 3u]);
    let zi = FF(cases[8u * i + 4u], cases[8u * i + 5u]);
    let cr = FF(bitcast<f32>(key[1]), bitcast<f32>(key[2]));
    let ci = FF(bitcast<f32>(key[3]), bitcast<f32>(key[4]));
    let r2 = ff_mul(zr, zr);
    let i2 = ff_mul(zi, zi);
    let ri = ff_mul(zr, zi);
    let nr = ff_add(ff_add(r2, FF(-i2.hi, -i2.lo)), cr);
    let ni = ff_add(FF(2.0 * ri.hi, 2.0 * ri.lo), ci);
    let o = 12u * i;
    out[o] = s.hi;
    out[o + 1u] = s.lo;
    out[o + 2u] = f.hi;
    out[o + 3u] = f.lo;
    out[o + 4u] = e;
    out[o + 5u] = m.hi;
    out[o + 6u] = m.lo;
    out[o + 7u] = nr.hi;
    out[o + 8u] = nr.lo;
    out[o + 9u] = ni.hi;
    out[o + 10u] = ni.lo;
    out[o + 11u] = 0.0;
}
