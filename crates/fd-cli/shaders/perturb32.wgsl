// Bounded generic perturbation checkpoint interface (GPU-31), not a frame admission
// policy. Caller supplies Z_m and Z_(m+1), retaining the CPU's high-precision centre.
// Requires cf.wgsl. Each input PN must be finite, normalized (or zero), |e| <= 16000.
// No output from this interface is a classified sample: a later frame route MUST
// accumulate pixel error and restart from a pre-loss checkpoint before admitting it.
// A small relative delta error can be MANY initial sample spacings at depth; it
// is not a screen-space error bound (that also needs the derivative).
struct PerturbState {
    delta: PN,
    m: u32,
    n: u32,
    rebases: u32,
    // 0 active; 1 escaped (|z|>2); 2 invalid; 3 outside supported exponent range.
    status: u32,
}
fn perturb_step_radius(s: PerturbState, dc: PN, z: PN, next: PN, last: bool, radius: PN) -> PerturbState {
    if (s.status != 0u) { return s; }
    var o = s;
    if (!(pn_input_ok(s.delta) && pn_input_ok(dc) && pn_input_ok(z) && pn_input_ok(next)) || s.n == 0xffffffffu || s.m == 0xffffffffu || s.rebases == 0xffffffffu) {
        o.status = 2u;
        return o;
    }
    if (!(pn_in_range(s.delta) && pn_in_range(dc) && pn_in_range(z) && pn_in_range(next))) {
        o.status = 3u;
        return o;
    }
    // delta' = 2 Z delta + delta^2 + dc. Exponent-carrying products never
    // materialize the tiny physical delta or dc as an ordinary f32.
    o.delta = pn_add(pn_add(pn_scale(pn_mul(z, s.delta), 2.0), pn_mul(s.delta, s.delta)), dc);
    o.n += 1u;
    o.m += 1u;
    return perturb_finish(o,next,last,radius);
}

// Shared endpoint handling for a single step or an affine block; no shortcut may
// skip an escape/rebase at its landing iterate.
fn perturb_finish(state:PerturbState,next:PN,last:bool,radius:PN)->PerturbState {
    var o=state;
    let physical = pn_add(next, o.delta);
    if (pn_bad(o.delta) || pn_bad(physical)) { o.status = 2u; return o; }
    if (!(pn_in_range(o.delta) && pn_in_range(physical))) { o.status = 3u; return o; }
    if (!pn_norm_le(physical, radius)) {
        o.status = 1u;
        return o;
    }
    // Match the exponent CPU tier's <= rule, including exact cancellation.
    if (last || pn_norm_le(physical, o.delta)) {
        o.delta = physical;
        o.m = 0u;
        o.rebases += 1u;
    }
    return o;
}

// GPU-31 checkpoint compatibility: radius two.
fn perturb_step(s: PerturbState, dc: PN, z: PN, next: PN, last: bool) -> PerturbState {
    return perturb_step_radius(s, dc, z, next, last, pn_scale(pn_one(),2.0));
}
