// Mid-band zone kernel, f32-only (GPU-22): zone_mid.wgsl's KERN-02 operator with no
// f64 or i64. The host prepends ff.wgsl, soft_f64.wgsl and zone32_lib.wgsl. Same
// constants layout as zone_mid.wgsl (Zone::gpu_mid_input).
//
// dc (5e-27..1e-6), the approach offset d, the cycle-point shift and the chart
// variable live in CE; dz/dc stays in CE through the jump, where the bracket
// dz - p'(c) - k'(w0) + ... cancels, and drops to CF for the finish.

const APPROACH: i32 = 24;
const NK: i32 = 19;
const Z_AT: i32 = 32;
const K_AT: i32 = 32 + 2 * 25;
const KP_AT: i32 = K_AT + 2 * 19;
const T_AT: i32 = KP_AT + 2 * 19;
const REF_AT: i32 = T_AT + 2 * 512;

// ln|x| = 0.5 ln|m|^2 + e ln 2.
fn ce_ln_abs(x: CE) -> FF {
    let m2 = ff_add(ff_mul(x.re, x.re), ff_mul(x.im, x.im));
    return ff_add(ff_mul_f(ff_log(m2), 0.5), ff_mul_f(FF_LN2, f32(x.e)));
}
fn r_of(x: f32) -> R { return r_norm(x, 0); }
fn r_lt(a: R, b: R) -> bool { return !r_le(b, a); }
fn r_ok(a: R) -> bool { return finite_f32(a.m); }

fn chart_parameter_ok(dc: R) -> bool {
    return r_ok(dc) && (r_le(dc, r_of(7e-12)) || (r_le(r_of(1e-7), dc) && r_le(dc, r_of(1e-6))));
}

var<private> k: array<CE, 19>;

struct Ev { v: CE, d: CE }
fn eval_k(w: CE) -> Ev {
    var v = ce_zero();
    var d = ce_zero();
    for (var i = NK - 1; i >= 0; i -= 1) {
        d = ce_add(ce_mul(d, w), v);
        v = ce_add(ce_mul(v, w), k[i]);
    }
    return Ev(v, d);
}
fn eval_kp(w: CE) -> CE {
    var v = ce_zero();
    for (var i = NK - 1; i >= 0; i -= 1) {
        v = ce_add(ce_mul(v, w), ce_at(KP_AT + 2 * i));
    }
    return v;
}

// Unaccelerated orbit at the pixel's own parameter, with periodic rebasing.
fn fallback(gid: i32, dc: CE, maxit: i32, esc: f32, px: R, cs: f32, sn: f32) {
    let period = int_at(12);
    var d = ce_zero();
    var dz = cf_zero();
    var at = 0;
    for (var n = 1; n <= maxit; n += 1) {
        let r = ZF(ff_at(REF_AT + 2 * at), ff_at(REF_AT + 2 * at + 1));
        var z = zf_add(r, zf_of(d));
        dz = cf_add(cf_scale(cf_mul(CF(vec2<f32>(z.re.hi, z.im.hi), 0), dz), 2.0), cf_one());
        let rc = ce_from_ff(r.re, r.im);
        d = ce_add(ce_add(ce_mul_f(ce_mul(rc, d), 2.0), ce_mul(d, d)), dc);
        at += 1;
        if (at == period) { at = 0; }
        z = zf_add(ZF(ff_at(REF_AT + 2 * at), ff_at(REF_AT + 2 * at + 1)), zf_of(d));
        if (escaped32(z, esc)) {
            if (!cf_bad(dz) && (dz.m.x != 0.0 || dz.m.y != 0.0)) {
                emit_escape(gid, n, z, dz, px, cs, sn);
            } else {
                output_sample(gid, 2u, ff(0.0), 0.0, 0.0);
            }
            return;
        }
        // Rebase when the orbit comes closer to 0 than the offset is large.
        let zl = r_norm(length(vec2<f32>(z.re.hi, z.im.hi)), 0);
        if (r_lt(zl, ce_len(d))) {
            d = ce_from_ff(z.re, z.im);
            at = 0;
        }
    }
    output_sample(gid, 2u, ff(0.0), 0.0, 0.0);
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>,
        @builtin(num_workgroups) groups: vec3<u32>) {
    let gid = i32(id.x + id.y * (groups.x * 64u));
    // Runtime zero for the reassociation guard: nx is an exact small f64 integer,
    // whose low word is 0.
    ff_init(input_data[0].x);
    let nx = int_at(0);
    let ny = int_at(1);
    if (gid >= nx * ny) { return; }
    let maxit = int_at(2);
    let esc = ff_at(3).hi;
    let h = r_fe(fe_at(4));
    let px = r_fe(fe_at(5));
    let x = f32(gid % nx) + 0.5 - f32(nx) * 0.5;
    let y = f32(gid / nx) + 0.5 - f32(ny) * 0.5;
    let csf = ff_at(10);
    let snf = ff_at(11);
    let cs = csf.hi;
    let sn = snf.hi;
    let uv = ce_from_ff(ff_add(ff_mul_f(csf, x), ff_mul_f(snf, y)), ff_sub(ff_mul_f(snf, x), ff_mul_f(csf, y)));
    let dc = ce_add(ce_at(6), ce_mul_fe(uv, fe_at(4)));
    let dcl = ce_len(dc);
    if (!chart_parameter_ok(dcl) || maxit <= APPROACH) {
        fallback(gid, dc, maxit, esc, px, cs, sn);
        return;
    }
    var d = ce_zero();
    var dz = ce_zero();
    for (var i = 0; i < APPROACH; i += 1) {
        let zi = ce_at(Z_AT + 2 * i);
        dz = ce_add(ce_mul_f(ce_mul(ce_add(zi, d), dz), 2.0), ce_one());
        d = ce_add(ce_add(ce_mul_f(ce_mul(zi, d), 2.0), ce_mul(d, d)), dc);
    }
    // Closed-form shift p(c) - p(C), without subtracting two rounded cycle points.
    let s2 = ce_mul_f(ce_at(15), 2.0);
    let m4dc = ce_mul_f(dc, -4.0);
    var ds = ce_div(m4dc, s2);
    for (var i = 0; i < 2; i += 1) {
        ds = ce_div(m4dc, ce_add(s2, ds));
    }
    let dp = ce_mul_f(ds, 0.5 * ff_at(19).hi);
    let pc = ce_add(ce_at(17), dp);
    let pprime = ce_div(ce_neg(ce_one()), ce_add(ce_mul_f(pc, 2.0), ce_one()));
    let u0 = ce_sub(ce_add(ce_at(13), d), dp);
    let lambda = ce_at(20);
    let lam = ce_add(lambda, ce_mul_f(dc, 4.0));
    let laml = ce_len(lam);
    if (!(r_lt(r_of(1.0), laml) && r_lt(ce_len(u0), r_div(r_of(0.03), laml)) && r_ok(ce_len(u0)))) {
        fallback(gid, dc, maxit, esc, px, cs, sn);
        return;
    }
    for (var i = 0; i < NK; i += 1) {
        k[i] = ce_add(ce_at(K_AT + 2 * i), ce_mul(dc, ce_at(KP_AT + 2 * i)));
    }
    var w0 = u0;
    for (var i = 0; i < 3; i += 1) {
        let e = eval_k(w0);
        if (!(e.d.re.hi != 0.0 || e.d.im.hi != 0.0) || ce_bad(e.d)) {
            fallback(gid, dc, maxit, esc, px, cs, sn);
            return;
        }
        w0 = ce_sub(w0, ce_div(ce_sub(e.v, u0), e.d));
    }
    if (ce_bad(w0) || (w0.re.hi == 0.0 && w0.im.hi == 0.0)) {
        fallback(gid, dc, maxit, esc, px, cs, sn);
        return;
    }
    // j = ln(RADIUS / |w0|) / ln|lambda|.
    let jf = ff_div(ff_sub(ff_log(ff(0.03)), ce_ln_abs(w0)), ce_ln_abs(lam)).hi;
    if (!(finite_f32(jf) && jf >= 1.0 && jf < 512.0)) {
        fallback(gid, dc, maxit, esc, px, cs, sn);
        return;
    }
    let j = i32(floor(jf));
    let n0 = APPROACH + 2 * j;
    if (n0 >= maxit) {
        fallback(gid, dc, maxit, esc, px, cs, sn);
        return;
    }
    // Tabulated lambda(C)^j with a first-order parameter correction.
    let factor = ce_add(ce_one(), ce_div(ce_mul_f(dc, 4.0 * f32(j)), lambda));
    let lj = ce_mul(ce_at(T_AT + 2 * j), factor);
    let q = ce_mul(w0, lj);
    if (!(r_ok(ce_len(q)) && r_le(ce_len(q), r_of(0.031)))) {
        fallback(gid, dc, maxit, esc, px, cs, sn);
        return;
    }
    let dkw = eval_k(w0).d;
    let eq = eval_k(q);
    let pcw = eval_kp(w0);
    let pcq = eval_kp(q);
    if (!(dkw.re.hi != 0.0 || dkw.im.hi != 0.0) || ce_bad(dkw)) {
        fallback(gid, dc, maxit, esc, px, cs, sn);
        return;
    }
    let jl = ce_div(ce_from_ff(ff(4.0 * f32(j)), ff(0.0)), lam);
    let bracket = ce_add(ce_sub(ce_sub(dz, pprime), pcw), ce_mul(ce_mul(w0, dkw), jl));
    let dzj = ce_add(ce_add(pprime, ce_mul(ce_div(ce_mul(lj, eq.d), dkw), bracket)), pcq);
    let zj = ce_add(pc, eq.v);
    if (ce_bad(zj) || ce_bad(dzj)) {
        fallback(gid, dc, maxit, esc, px, cs, sn);
        return;
    }
    // The finish MUST use the pixel's own c, not the nucleus C (PROB-14).
    let cre = ff_add(ff_at(8), ce_re(dc));
    let cim = ff_add(ff_at(9), ce_im(dc));
    var z = zf_of(zj);
    var du = cf_of(dzj);
    let far = r_lt(r_of(1e-8), dcl);
    for (var n = n0 + 1; n <= maxit; n += 1) {
        du = cf_add(cf_scale(cf_mul(CF(vec2<f32>(z.re.hi, z.im.hi), 0), du), 2.0), cf_one());
        let r2 = ff_mul(z.re, z.re);
        let i2 = ff_mul(z.im, z.im);
        let ri = ff_mul(z.re, z.im);
        z = ZF(ff_add(ff_sub(r2, i2), cre), ff_add(ff_mul_f(ri, 2.0), cim));
        if (escaped32(z, esc)) {
            if (cf_bad(du) || (du.m.x == 0.0 && du.m.y == 0.0)) {
                fallback(gid, dc, maxit, esc, px, cs, sn);
                return;
            }
            // A long chaotic finish near the boundary needs the full orbit:
            // de_px = 2 |z| ln|z| / (|dz| h) < 1 falls back.
            if (far) {
                let big = max(abs(z.re.hi), abs(z.im.hi));
                let kk = i32((bitcast<u32>(big) >> 23u) & 255u) - 126;
                let zs = vec2<f32>(ff_scale(z.re, -kk).hi, ff_scale(z.im, -kk).hi);
                let lnz = log(length(zs)) + f32(kk) * 0.6931471805599453;
                let de_px = r_div(R(2.0 * length(zs) * lnz, kk), r_mul(cf_len(du), h));
                if (r_lt(de_px, r_of(1.0)) || !r_ok(de_px)) {
                    fallback(gid, dc, maxit, esc, px, cs, sn);
                    return;
                }
            }
            emit_escape(gid, n, z, du, px, cs, sn);
            return;
        }
    }
    output_sample(gid, 2u, ff(0.0), 0.0, 0.0);
}
