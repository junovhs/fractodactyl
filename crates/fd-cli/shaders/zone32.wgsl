// f32-only zone keyframe kernel (GPU-28, DEC-25): zone.wgsl's four stages (return map,
// approach, Koenigs/patch, escape finish) with no f64 and no i64, so it runs on
// adapters without SHADER_F64, software ones included (DEC-28). The host prepends
// ff.wgsl (float-float, f32+exponent), fd-shade's soft_f64.wgsl (exact binary64 in
// integers, used only to write the nu column the host reads as f64) and
// zone32_lib.wgsl (bindings, CE/CF/R/ZF helpers, column encoding).
//
// Representations, chosen by the magnitudes the zone constants take (GPU-28 notes):
// - CE (complex float-float, shared exponent): the return map, its biseries
//   coefficients (1e-51..1e25) and parameter (~1e-24), the approach offset and the
//   Koenigs jump. About 48 bits, any exponent.
// - CF (complex f32, shared exponent): dz/dc and every factor that only feeds it
//   (DE and the normal need relative, not absolute, precision; dz/dc reaches ~1/width).
// - ZF (complex float-float): z in the escape finish, O(1).

override DEG: i32 = 7;
override NPHI: i32 = 1;
override NPSI: i32 = 1;

struct Series { y: CE, dy: CF }
fn series(start: i32, count: i32, x: CE) -> Series {
    var y = ce_zero();
    var dy = cf_zero();
    let xf = cf_of(x);
    for (var i = count - 1; i >= 0; i -= 1) {
        let a = ce_at(start + 2 * i);
        y = ce_mul(ce_add(y, a), x);
        dy = cf_add(cf_mul(dy, xf), cf_scale(cf_of(a), f32(i + 1)));
    }
    return Series(y, dy);
}

// Return-map biseries coefficients at this sample's parameter.
var<private> b: array<CE, 8>;
var<private> bv: array<CF, 8>;
var<private> g_phi_base: i32;
var<private> g_psi_base: i32;
var<private> g_node_base: i32;
var<private> g_leaf_base: i32;

struct Jet { y: CE, d: CF, dd: CF }
fn eval2(x: CE) -> Jet {
    var y = b[DEG];
    var dy = cf_zero();
    var ddy = cf_zero();
    let xf = cf_of(x);
    for (var i = DEG - 1; i >= 0; i -= 1) {
        ddy = cf_add(cf_mul(ddy, xf), dy);
        dy = cf_add(cf_mul(dy, xf), cf_of(y));
        y = ce_add(ce_mul(y, x), b[i]);
    }
    return Jet(y, dy, cf_scale(ddy, 2.0));
}
fn compose(x: CE, q: i32) -> Jet {
    var y = x;
    var d = cf_one();
    var dd = cf_zero();
    for (var i = 0; i < q; i += 1) {
        let f = eval2(y);
        dd = cf_add(cf_mul(f.dd, cf_mul(d, d)), cf_mul(f.d, dd));
        d = cf_mul(f.d, d);
        y = f.y;
    }
    return Jet(y, d, dd);
}
fn attracted(u: CE) -> bool {
    let tiny = R(0.5, -46); // sqrt(1e-28) ~ 2^-46.5: |st| <= 1e-14 |x|
    for (var q = 1; q <= 4; q += 1) {
        var x = u;
        for (var i = 0; i < 16; i += 1) {
            let f = compose(x, q);
            let st = ce_mul_cf(ce_sub(f.y, x), cf_inv(cf_add(f.d, CF(vec2<f32>(-0.5, 0.0), 1))));
            x = ce_sub(x, st);
            if (ce_bad(x)) { break; }
            if (r_le(ce_len(st), r_mul(tiny, ce_len(x)))) {
                let g = compose(x, q);
                let l = r_f32(cf_len(g.d));
                if (l < 1.0 && r_le(r_mul(ce_len(ce_sub(u, x)), cf_len(g.dd)), R(0.5 * (1.0 - l), 0))) {
                    return true;
                }
                break;
            }
        }
    }
    return false;
}

struct Patch { ok: bool, z: ZF, dz: CF, m: i32 }
fn horner_patch(re0: FF, im0: FF, j: f32, root: i32, nodes: i32, leaves: i32) -> Patch {
    let none = Patch(false, ZF(ff(0.0), ff(0.0)), cf_zero(), 0);
    if (nodes == 0 || j < 0.0) { return none; }
    let re = ff_add(re0, ff_mul_f(ff_at(15), j));
    var im = ff_add(im0, ff_mul_f(ff_at(16), j));
    im = ff_sub(im, ff_mul_f(TAU, floor(ff_div(im, TAU).hi)));
    var node = min(i32(ff_div(im, ff_mul_f(TAU, 1.0 / f32(root))).hi), root - 1);
    for (var k = 0; k < nodes; k += 1) {
        if (node < 0 || node >= nodes) { return none; }
        var at = g_node_base + 4 * node;
        let kid = int_at(at + 2);
        if (kid < 0) {
            let leaf = int_at(at + 3);
            if (leaf < 0 || leaf >= leaves) { return none; }
            at = g_leaf_base + 36 * leaf;
            let m = int_at(at + 3);
            if (m < 0) { return none; }
            let r = ff_at(at + 2);
            let t = ce_from_ff(ff_div(ff_sub(re, ff_at(at)), r), ff_div(ff_sub(im, ff_at(at + 1)), r));
            var z = ce_zero();
            var dz = cf_zero();
            let tf = cf_of(t);
            for (var a = 15; a >= 0; a -= 1) {
                dz = cf_add(cf_mul(dz, tf), cf_of(z));
                z = ce_add(ce_mul(z, t), ce_at(at + 4 + 2 * a));
            }
            return Patch(true, zf_of(z), cf_scale(dz, 1.0 / r.hi), m);
        }
        let cre = ff_at(at);
        let cim = ff_at(at + 1);
        node = kid + 2 * select(0, 1, ff_sub(re, cre).hi >= 0.0) + select(0, 1, ff_sub(im, cim).hi >= 0.0);
    }
    return none;
}


struct Front { z: ZF, du: CF, n: i32 }
fn front(u: CE, du_in: CF, n_in: i32) -> Front {
    var du = du_in;
    var n = n_in;
    // 23 approach iterates, with dz/dc in unscaled parameter coordinates.
    var d = ce_mul_fe(u, fe_at(0));
    for (var i = 0; i < 23; i += 1) {
        let zc = ce_at(160 + 2 * i);
        du = cf_add(cf_scale(cf_mul(cf_of(ce_add(zc, d)), du), 2.0), cf_one());
        d = ce_add(ce_mul_f(ce_mul(zc, d), 2.0), ce_mul(d, d));
    }
    n += 23;

    // Koenigs jump; Horner in the tail atlas is the final landing.
    let h0 = ce_add(ce_at(17), d);
    let alpha = ZF(ff_at(13), ff_at(14));
    if (!(r_le(ce_len(h0), r_fe(fe_at(2))) && !r_le(r_fe(fe_at(2)), ce_len(h0)))) {
        return Front(zf_add(alpha, zf_of(h0)), du, n);
    }
    let s = series(g_phi_base, NPHI, h0);
    let w0 = s.y;
    let dphi = s.dy;
    // log|w0| = log|m| + e ln 2, |m| in [0.5, sqrt 2).
    let m2 = ff_add(ff_mul(w0.re, w0.re), ff_mul(w0.im, w0.im));
    let lnw = ff_add(ff_mul_f(ff_log(m2), 0.5), ff_mul_f(FF_LN2, f32(w0.e)));
    let jf = floor(ff_add(ff_neg(ff_mul(lnw, ff_at(31))), ff_at(22)).hi);
    let landing = horner_patch(lnw, ff_atan2(w0.im, w0.re), jf, int_at(23), int_at(26), int_at(27));
    if (landing.ok) {
        du = cf_mul(cf_mul(cf_mul(landing.dz, dphi), du), cf_inv(cf_of(w0)));
        return Front(landing.z, du, n + 2 * i32(jf) + landing.m);
    }
    if (jf > 0.0) {
        let t = ff_mul_f(ff_at(16), jf);
        // |rho|^j = exp(j ln|rho|) kept as an FE: e^x = 2^k e^r.
        let x = ff_mul_f(ff_at(15), jf);
        let k = round(x.hi * 1.4426950408889634);
        let r = ff_sub(ff_sub(x, two_prod(FF_LN2.hi, k)), ff_mul_f(ff(FF_LN2.lo), k));
        let mag = fe_norm(ff_exp(r), i32(k));
        let sc = ff_sincos(t);
        let rj = ce_norm(ff_mul(sc.c, mag.m), ff_mul(sc.s, mag.m), mag.e);
        // w0 is tiny and rho^j huge: their product is O(r0).
        let xw = ce_mul(w0, rj);
        let sp = series(g_psi_base, NPSI, xw);
        du = cf_mul(cf_mul(cf_mul(sp.dy, cf_of(rj)), dphi), du);
        return Front(zf_add(alpha, zf_of(sp.y)), du, n + 2 * i32(jf));
    }
    return Front(zf_add(alpha, zf_of(h0)), du, n);
}

// Finish at the fixed nucleus and encode nu, pixel DE and screen normal.
fn finish(gid: i32, z_in: ZF, du_in: CF, n_in: i32) {
    var z = z_in;
    var du = du_in;
    var n = n_in;
    let cre = ff_at(11);
    let cim = ff_at(12);
    let cs = ff_at(28).hi;
    let sn = ff_at(29).hi;
    let maxit = int_at(5);
    let esc = ff_at(21).hi;
    let px = r_fe(fe_at(30));
    while (n < maxit) {
        du = cf_add(cf_scale(cf_mul(CF(vec2<f32>(z.re.hi, z.im.hi), 0), du), 2.0), cf_one());
        let r2 = ff_mul(z.re, z.re);
        let i2 = ff_mul(z.im, z.im);
        let ri = ff_mul(z.re, z.im);
        z = ZF(ff_add(ff_sub(r2, i2), cre), ff_add(ff_mul_f(ri, 2.0), cim));
        n += 1;
        if (escaped32(z, esc)) {
            emit_escape(gid, n, z, du, px, cs, sn);
            return;
        }
    }
    output_sample(gid, 2u, ff(0.0), 0.0, 0.0);
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>,
        @builtin(num_workgroups) groups: vec3<u32>) {
    let gid = i32(id.x + id.y * (groups.x * 64u));
    // Runtime zero for the float-float reassociation guard: the high word of the
    // pixel count nx is 0x40... never 0, so take its low word, which is 0 for the
    // small integers the host packs (exact f64 integers below 2^20).
    ff_init(input_data[6].x);
    let nx = int_at(6);
    let ny = int_at(7);
    if (gid >= nx * ny) { return; }
    let period = int_at(3);
    let maxit = int_at(5);
    let nn = int_at(26);
    g_phi_base = 206;
    g_psi_base = g_phi_base + 2 * NPHI;
    g_node_base = g_psi_base + 2 * NPSI;
    g_leaf_base = g_node_base + 4 * nn;
    let col = gid % nx;
    let row = gid / nx;
    let x = f32(col) + 0.5 - f32(nx) * 0.5;
    let y = f32(row) + 0.5 - f32(ny) * 0.5;
    let cs = ff_at(28);
    let sn = ff_at(29);
    let uv = ce_from_ff(ff_add(ff_mul_f(cs, x), ff_mul_f(sn, y)), ff_sub(ff_mul_f(sn, x), ff_mul_f(cs, y)));
    let v = ce_add(ce_at(8), ce_mul_fe(uv, fe_at(10)));
    let vf = cf_of(v);
    for (var i = 0; i <= DEG; i += 1) {
        var s = ce_zero();
        var ds = cf_zero();
        for (var j = DEG - i; j >= 0; j -= 1) {
            ds = cf_add(cf_mul(ds, vf), cf_of(s));
            s = ce_add(ce_mul(s, v), ce_at(32 + 2 * (8 * i + j)));
        }
        b[i] = s;
        bv[i] = ds;
    }
    b[0] = ce_add(b[0], ce_at(19));
    var u = v;
    var du = cf_one();
    var n = 1;
    var work = 0;
    let scale = r_fe(fe_at(0));
    let guard = r_fe(fe_at(1));
    let eps = R(0.5, -39); // sqrt(1e-24) ~ 2^-39.9: |delta| <= 1e-12 |u|
    while (r_le(r_mul(ce_len(u), scale), guard)) {
        if (n + period > maxit) {
            output_sample(gid, 2u, ff(0.0), 0.0, 0.0);
            return;
        }
        var s = b[DEG];
        var su = cf_zero();
        var sv = bv[DEG];
        let uf = cf_of(u);
        for (var i = DEG - 1; i >= 0; i -= 1) {
            su = cf_add(cf_mul(su, uf), cf_of(s));
            s = ce_add(ce_mul(s, u), b[i]);
            sv = cf_add(cf_mul(sv, uf), bv[i]);
        }
        let delta = ce_sub(s, u);
        du = cf_add(cf_mul(su, du), sv);
        if ((r_f32(cf_len(su)) < 1.0 && r_le(ce_len(delta), r_mul(eps, ce_len(u))))
            || (work >= 16 && (work & (work - 1)) == 0 && attracted(u))) {
            output_sample(gid, 1u, ff(0.0), 0.0, 0.0);
            return;
        }
        u = s;
        n += period;
        work += 1;
    }
    let f = front(u, du, n);
    finish(gid, f.z, f.du, f.n);
}
