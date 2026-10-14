// Guarded f32 finish for the mixed and f32 zone kernels, appended to
// zone.wgsl (needs wgpu SUBGROUP). The f64 kernel appends a stub instead.
const F32_FINISH: bool = true;

// The hand-over is subgroup-uniform: per-lane hand-over diverges waves and
// runs slower than f64 (GPU-02 round 2).
fn wave_all(ready: bool) -> bool { return subgroupAll(ready); }

// f32 finish with a running first-order error bound (DEC-10/25). ep is the
// absolute error of z in units of the pixel spacing s = |dz/dc| * pixel at the
// landing; dv is dz/dc over 2^e, renormalised by 2^-80 so neither leaves the
// f32 range (one step multiplies by 2|z| <= 2e10). |z|^2 can reach 1e40, so
// the escape test squares in binary64 once a component nears the radius. At
// escape ep/|dv| is the position error in pixels; above the tolerance the
// sample returns false and restarts from the binary64 hand-over checkpoint.
fn finish32(gid: i32, z: C, du: C, n_in: i32, errpx: f64) -> bool {
    let maxit = i32(value(5));
    let e = max(exponent_of(du), 0);
    let s = ldexp(value(30), e);
    if !(s > 1e-30) { return false; }
    var z32 = vec2<f32>(z);
    var dv = vec2<f32>(ldexp2(du, -e));
    let c32 = vec2<f32>(cx(11));
    var one = f32(ldexp(1.0lf, -e));
    var inv = f32(1.0 / s);
    var ep = f32(errpx * length(ldexp2(du, -e)) + f64(EPS32) * length(z) / s);
    let cabs = length(c32);
    let tol = 1e-3f; // f32(TOL_PX), as the GLSL rounded it
    let near = f32(sqrt(0.5 * value(21))) * (1.0f - 1e-6f);
    let K = 8.0f * EPS32;
    let DOWN = 8.271806125530277e-25f; // 2^-80
    var n = n_in;
    while n < maxit {
        let lz = length(z32);
        dv = 2.0 * mul32(z32, dv) + vec2<f32>(one, 0.0);
        z32 = mul32(z32, z32) + c32;
        n += 1;
        ep = 2.0 * lz * ep + K * (lz * lz + cabs) * inv;
        if max(abs(dv.x), abs(dv.y)) > 1e24f {
            dv *= DOWN;
            one *= DOWN;
            inv *= DOWN;
            ep *= DOWN;
        }
        // Once ep/|dv| exceeds the tolerance it cannot recover (ep grows at
        // least as fast as dv); bail out early to the binary64 restart.
        if !(ep < 1e30f) { return false; }
        if max(abs(z32.x), abs(z32.y)) < near { continue; }
        let zd = C(z32);
        let a = norm2(zd);
        if a > value(21) {
            // Squares of dv overflow f32; the one-off escape arithmetic is binary64.
            let dd = C(dv);
            let dl = length(dd);
            if !(f64(ep) <= f64(tol) * dl) { return false; }
            let l2 = 0.5 * fp64_log2(a);
            let nu = f64(n) + 1.0 - fp64_log2(l2);
            let de = 2.0 * sqrt(a) * (l2 * FP64_LN2) * f64(inv) / dl;
            let re = zd.x * dd.x + zd.y * dd.y;
            let im = zd.y * dd.x - zd.x * dd.y;
            let cs = value(28);
            let sn = value(29);
            let sx = cs * re + sn * im;
            let sy = sn * re - cs * im;
            output_sample(gid, 0u, nu, de, fp64_atan2(sy, sx));
            return true;
        }
    }
    if !(f64(ep) <= f64(tol) * length(C(dv))) { return false; }
    output_sample(gid, 2u, 0.0, 0.0, 0.0);
    return true;
}
