// Mid-band zone kernel, binary64 (GPU-22): crates/fd-kernel/src/zone/mid.rs's
// co-moving two-cycle jump on the GPU, ported operation for operation. The host
// prepends zone_f64_lib.wgsl. Constants (Zone::gpu_mid_input): 0 nx, 1 ny, 2 max_iter,
// 3 r^2, 4 sample spacing h, 5 output pixel, 6..7 centre offset to the nucleus,
// 8..9 nucleus c0, 10 cos, 11 sin, 12 period, 13..14 u, 15..16 s, 17..18 p, 19 sign,
// 20..21 lambda; from 32: Z_0..Z_24, k[0..18], k'[0..18], lambda^j for j < 512, then
// the reference orbit.

const APPROACH: i32 = 24;
const NK: i32 = 19;
const RADIUS: f64 = 0.03;
const MAX_DC: f64 = 1e-6;
const OUTER_MIN_DC: f64 = 1e-7;
const INNER_MAX_DC: f64 = 7e-12;
const Z_AT: i32 = 32;
const K_AT: i32 = 32 + 2 * 25;
const KP_AT: i32 = K_AT + 2 * 19;
const T_AT: i32 = KP_AT + 2 * 19;
const REF_AT: i32 = T_AT + 2 * 512;

fn chart_parameter_ok(dc: f64) -> bool {
    return !nonfinite(dc) && (dc <= INNER_MAX_DC || (dc >= OUTER_MIN_DC && dc <= MAX_DC));
}
fn cabs(z: C) -> f64 { return length(z); }
fn finite_c(z: C) -> bool { return !nonfinite(z.x) && !nonfinite(z.y); }

var<private> k: array<C, 19>;

struct Ev { v: C, d: C }
// Horner with derivative over k (own == true) or the constant k' table.
fn eval_k(w: C) -> Ev {
    var v = C(0.0, 0.0);
    var d = C(0.0, 0.0);
    for (var i = NK - 1; i >= 0; i -= 1) {
        d = mul(d, w) + v;
        v = mul(v, w) + k[i];
    }
    return Ev(v, d);
}
fn eval_kp(w: C) -> Ev {
    var v = C(0.0, 0.0);
    var d = C(0.0, 0.0);
    for (var i = NK - 1; i >= 0; i -= 1) {
        d = mul(d, w) + v;
        v = mul(v, w) + cx(KP_AT + 2 * i);
    }
    return Ev(v, d);
}

fn emit(gid: i32, n: i32, z: C, dz: C) {
    let a = norm2(z);
    let log2z = 0.5 * fp64_log2(a);
    let nu = f64(n) + 1.0 - fp64_log2(log2z);
    let de = 2.0 * sqrt(a) * (log2z * FP64_LN2) / length(dz) / value(5);
    let re = z.x * dz.x + z.y * dz.y;
    let im = z.y * dz.x - z.x * dz.y;
    let cs = value(10);
    let sn = value(11);
    output_sample(gid, 0u, nu, de, fp64_atan2(sn * re - cs * im, cs * re + sn * im));
}

// Unaccelerated orbit at the pixel's own parameter, with periodic rebasing.
fn fallback(gid: i32, dc: C, maxit: i32, r2: f64) {
    let period = i32(value(12));
    var d = C(0.0, 0.0);
    var dz = C(0.0, 0.0);
    var at = 0;
    for (var n = 1; n <= maxit; n += 1) {
        var z = cx(REF_AT + 2 * at) + d;
        dz = 2.0 * mul(z, dz) + C(1.0, 0.0);
        let r = cx(REF_AT + 2 * at);
        d = 2.0 * mul(r, d) + mul(d, d) + dc;
        at += 1;
        if at == period { at = 0; }
        z = cx(REF_AT + 2 * at) + d;
        let a = norm2(z);
        if a > r2 {
            if !nonfinite(a) && finite_c(dz) && cabs(dz) > 0.0 {
                emit(gid, n, z, dz);
            } else {
                output_sample(gid, 2u, 0.0, 0.0, 0.0);
            }
            return;
        }
        if a < norm2(d) {
            d = z;
            at = 0;
        }
    }
    output_sample(gid, 2u, 0.0, 0.0, 0.0);
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>,
        @builtin(num_workgroups) groups: vec3<u32>) {
    let gid = i32(id.x + id.y * (groups.x * 64u));
    let nx = i32(value(0));
    let ny = i32(value(1));
    if gid >= nx * ny { return; }
    let maxit = i32(value(2));
    let r2 = value(3);
    let h = value(4);
    let x = f64(gid % nx) + 0.5 - f64(nx) / 2.0;
    let y = f64(gid / nx) + 0.5 - f64(ny) / 2.0;
    let cs = value(10);
    let sn = value(11);
    let dc = C(value(6) + (cs * x + sn * y) * h, value(7) + (sn * x - cs * y) * h);
    if !chart_parameter_ok(cabs(dc)) || maxit <= APPROACH {
        fallback(gid, dc, maxit, r2);
        return;
    }
    var d = C(0.0, 0.0);
    var dz = C(0.0, 0.0);
    for (var i = 0; i < APPROACH; i += 1) {
        let zi = cx(Z_AT + 2 * i);
        let at = zi + d;
        dz = 2.0 * mul(at, dz) + C(1.0, 0.0);
        d = 2.0 * mul(zi, d) + mul(d, d) + dc;
    }
    // Closed-form shift p(c) - p(C), without subtracting two rounded cycle points.
    let s = cx(15);
    var ds = divc(-4.0 * dc, 2.0 * s);
    for (var i = 0; i < 2; i += 1) {
        ds = divc(-4.0 * dc, 2.0 * s + ds);
    }
    let dp = ds * (0.5 * value(19));
    let pc = cx(17) + dp;
    let pprime = divc(C(-1.0, 0.0), 2.0 * pc + C(1.0, 0.0));
    let u0 = cx(13) + d - dp;
    let lambda = cx(20);
    let lam = lambda + 4.0 * dc;
    if !(cabs(lam) > 1.0 && cabs(u0) < RADIUS / cabs(lam) && !nonfinite(cabs(u0))) {
        fallback(gid, dc, maxit, r2);
        return;
    }
    for (var i = 0; i < NK; i += 1) {
        k[i] = cx(K_AT + 2 * i) + mul(dc, cx(KP_AT + 2 * i));
    }
    var w0 = u0;
    for (var i = 0; i < 3; i += 1) {
        let e = eval_k(w0);
        if !(cabs(e.d) > 0.0 && !nonfinite(cabs(e.d))) {
            fallback(gid, dc, maxit, r2);
            return;
        }
        w0 = w0 - divc(e.v - u0, e.d);
    }
    let r = cabs(w0);
    if !(r > 0.0 && !nonfinite(r)) {
        fallback(gid, dc, maxit, r2);
        return;
    }
    let jf = fp64_log(RADIUS / r) / fp64_log(cabs(lam));
    if !(!nonfinite(jf) && jf >= 1.0 && jf < 512.0) {
        fallback(gid, dc, maxit, r2);
        return;
    }
    let j = i32(floor(jf));
    let n0 = APPROACH + 2 * j;
    if n0 >= maxit {
        fallback(gid, dc, maxit, r2);
        return;
    }
    // Tabulated lambda(C)^j with a first-order parameter correction.
    let factor = C(1.0, 0.0) + divc(dc * (4.0 * f64(j)), lambda);
    let lj = mul(cx(T_AT + 2 * j), factor);
    let q = mul(w0, lj);
    if !(!nonfinite(cabs(q)) && cabs(q) <= 0.031) {
        fallback(gid, dc, maxit, r2);
        return;
    }
    let dkw = eval_k(w0).d;
    let eq = eval_k(q);
    let pcw = eval_kp(w0).v;
    let pcq = eval_kp(q).v;
    if !(cabs(dkw) > 0.0 && !nonfinite(cabs(dkw))) {
        fallback(gid, dc, maxit, r2);
        return;
    }
    let bracket = dz - pprime - pcw + mul(mul(w0, dkw), divc(C(4.0 * f64(j), 0.0), lam));
    dz = pprime + mul(divc(mul(lj, eq.d), dkw), bracket) + pcq;
    var z = pc + eq.v;
    if !finite_c(z) || nonfinite(cabs(z)) || nonfinite(cabs(dz)) {
        fallback(gid, dc, maxit, r2);
        return;
    }
    // The finish MUST use the pixel's own c, not the nucleus C (PROB-14).
    let c = cx(8) + dc;
    for (var n = n0 + 1; n <= maxit; n += 1) {
        dz = 2.0 * mul(z, dz) + C(1.0, 0.0);
        z = mul(z, z) + c;
        let a = norm2(z);
        if a > r2 {
            if nonfinite(a) || !(!nonfinite(cabs(dz)) && cabs(dz) > 0.0) {
                fallback(gid, dc, maxit, r2);
                return;
            }
            // A long chaotic finish near the boundary needs the full orbit.
            if cabs(dc) > 1e-8 {
                let de_px = 2.0 * sqrt(a) * (0.5 * fp64_log(a)) / (cabs(dz) * h);
                if de_px < 1.0 || nonfinite(de_px) {
                    fallback(gid, dc, maxit, r2);
                    return;
                }
            }
            emit(gid, n, z, dz);
            return;
        }
    }
    output_sample(gid, 2u, 0.0, 0.0, 0.0);
}
