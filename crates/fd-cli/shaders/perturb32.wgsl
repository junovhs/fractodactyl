// Bounded generic perturbation checkpoint interface (GPU-31), not a frame admission
// policy. Caller supplies Z_m and Z_(m+1), retaining the CPU's high-precision centre.
// Requires cf.wgsl. Each input CF must be finite, normalized (or zero), |e| <= 16000.
// No output from this interface is a classified sample: a later frame route MUST
// accumulate pixel error and restart from a pre-loss checkpoint before admitting it.
// A small relative delta error can be MANY initial sample spacings at depth; it
// is not a screen-space error bound (that also needs the derivative).
struct PerturbState {
    delta: CF,
    m: u32,
    n: u32,
    rebases: u32,
    // 0 active; 1 escaped (|z|>2); 2 invalid; 3 outside supported exponent range.
    status: u32,
}
fn perturb_input_ok(x: CF) -> bool {
    let a = max(abs(x.m.x), abs(x.m.y));
    return !cf_bad(x) && (a == 0.0 || (a >= 0.5 && a < 1.0));
}
fn perturb_in_range(x: CF) -> bool { return x.e >= -16000 && x.e <= 16000; }
fn perturb_norm_le(a: CF, b: CF) -> bool {
    let aa = dot(a.m, a.m);
    let bb = dot(b.m, b.m);
    if (aa == 0.0) { return true; }
    if (bb == 0.0) { return false; }
    let e = a.e - b.e;
    if (e > 1) { return false; }
    if (e < -1) { return true; }
    return aa * pow2(2 * e) <= bb;
}
fn perturb_step(s: PerturbState, dc: CF, z: CF, next: CF, last: bool) -> PerturbState {
    if (s.status != 0u) { return s; }
    var o = s;
    if (!(perturb_input_ok(s.delta) && perturb_input_ok(dc) && perturb_input_ok(z) && perturb_input_ok(next)) || s.n == 0xffffffffu || s.m == 0xffffffffu || s.rebases == 0xffffffffu) {
        o.status = 2u;
        return o;
    }
    if (!(perturb_in_range(s.delta) && perturb_in_range(dc) && perturb_in_range(z) && perturb_in_range(next))) {
        o.status = 3u;
        return o;
    }
    // delta' = 2 Z delta + delta^2 + dc. Exponent-carrying products never
    // materialize the tiny physical delta or dc as an ordinary f32.
    o.delta = cf_add(cf_add(cf_scale(cf_mul(z, s.delta), 2.0), cf_mul(s.delta, s.delta)), dc);
    o.n += 1u;
    o.m += 1u;
    let physical = cf_add(next, o.delta);
    if (cf_bad(o.delta) || cf_bad(physical)) { o.status = 2u; return o; }
    if (!(perturb_in_range(o.delta) && perturb_in_range(physical))) { o.status = 3u; return o; }
    if (!perturb_norm_le(physical, CF(vec2<f32>(0.5, 0.0), 2))) {
        o.status = 1u;
        return o;
    }
    // Match the exponent CPU tier's <= rule, including exact cancellation.
    if (last || perturb_norm_le(physical, o.delta)) {
        o.delta = physical;
        o.m = 0u;
        o.rebases += 1u;
    }
    return o;
}
