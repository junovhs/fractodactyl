// f32-only zone keyframe kernel (GPU-28, DEC-25): zone.wgsl's four stages (return map,
// approach, Koenigs/patch, escape finish) with no f64 and no i64, so it runs on
// adapters without SHADER_F64, software ones included (DEC-28). The host prepends
// ff.wgsl (float-float, f32+exponent) and fd-shade's soft_f64.wgsl (exact binary64 in
// integers, used only to write the nu column the host reads as f64).
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

// The zone constants as binary64 bit patterns (low word, high word).
@group(0) @binding(0) var<storage, read> input_data: array<vec2<u32>>;
@group(0) @binding(1) var<storage, read_write> nu_out: array<vec2<u32>>;
@group(0) @binding(2) var<storage, read_write> de_out: array<f32>;
@group(0) @binding(3) var<storage, read_write> word_out: array<u32>;

fn fe_at(i: i32) -> FE { let w = input_data[i]; return fe_from_f64(w.x, w.y); }
// An O(1) constant as a float-float (zero, or exponents within f32's range).
fn ff_at(i: i32) -> FF { return fe_to_ff(fe_at(i)); }
// Small integers and counts.
fn int_at(i: i32) -> i32 { return i32(round(ff_at(i).hi)); }
fn ce_pair(re: FE, im: FE) -> CE {
    return ce_add(CE(re.m, ff(0.0), re.e), CE(ff(0.0), im.m, im.e));
}
fn ce_at(i: i32) -> CE { return ce_pair(fe_at(i), fe_at(i + 1)); }

// ---- CE helpers beyond ff.wgsl ----
fn ce_zero() -> CE { return CE(ff(0.0), ff(0.0), 0); }
fn ce_neg(x: CE) -> CE { return CE(ff_neg(x.re), ff_neg(x.im), x.e); }
fn ce_sub(x: CE, y: CE) -> CE { return ce_add(x, ce_neg(y)); }
fn ce_from_ff(re: FF, im: FF) -> CE { return ce_norm(re, im, 0); }
fn ce_mul_fe(x: CE, s: FE) -> CE {
    return ce_norm(ff_mul(x.re, s.m), ff_mul(x.im, s.m), x.e + s.e);
}
fn ce_mul_f(x: CE, s: f32) -> CE { return ce_norm(ff_mul_f(x.re, s), ff_mul_f(x.im, s), x.e); }
// The value as float-floats (0 below f32's normal range).
fn ce_re(x: CE) -> FF { return ff_scale(x.re, x.e); }
fn ce_im(x: CE) -> FF { return ff_scale(x.im, x.e); }
fn ce_bad(x: CE) -> bool { return !(finite_f32(x.re.hi) && finite_f32(x.im.hi)); }

// ---- R: a real f32 mantissa times 2^e, for magnitudes and comparisons ----
struct R { m: f32, e: i32 }
fn r_norm(m: f32, e: i32) -> R {
    if (m == 0.0) { return R(0.0, 0); }
    let k = i32((bitcast<u32>(abs(m)) >> 23u) & 255u) - 126;
    return R(m * pow2(-k), e + k);
}
fn r_fe(x: FE) -> R { return r_norm(x.m.hi, x.e); }
fn r_mul(a: R, b: R) -> R { return r_norm(a.m * b.m, a.e + b.e); }
fn r_div(a: R, b: R) -> R { return r_norm(a.m / b.m, a.e - b.e); }
fn r_f32(a: R) -> f32 {
    let h = a.e / 2;
    return a.m * pow2(h) * pow2(a.e - h);
}
// a <= b for non-negative values.
fn r_le(a: R, b: R) -> bool {
    if (a.m == 0.0) { return true; }
    if (b.m == 0.0) { return false; }
    if (a.e != b.e) { return a.e < b.e; }
    return a.m <= b.m;
}
fn ce_len(x: CE) -> R { return r_norm(length(vec2<f32>(x.re.hi, x.im.hi)), x.e); }

// ---- CF: complex f32 mantissa times 2^e ----
struct CF { m: vec2<f32>, e: i32 }
fn cmul(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x);
}
fn cf_norm(m: vec2<f32>, e: i32) -> CF {
    let big = max(abs(m.x), abs(m.y));
    if (big == 0.0) { return CF(vec2<f32>(0.0), 0); }
    let k = i32((bitcast<u32>(big) >> 23u) & 255u) - 126;
    return CF(m * pow2(-k), e + k);
}
fn cf_zero() -> CF { return CF(vec2<f32>(0.0), 0); }
fn cf_one() -> CF { return CF(vec2<f32>(0.5, 0.0), 1); }
fn cf_mul(a: CF, b: CF) -> CF { return cf_norm(cmul(a.m, b.m), a.e + b.e); }
fn cf_add(a: CF, b: CF) -> CF {
    if (a.m.x == 0.0 && a.m.y == 0.0) { return b; }
    if (b.m.x == 0.0 && b.m.y == 0.0) { return a; }
    let e = max(a.e, b.e);
    return cf_norm(a.m * pow2(max(a.e - e, -150)) + b.m * pow2(max(b.e - e, -150)), e);
}
fn cf_scale(a: CF, s: f32) -> CF { return cf_norm(a.m * s, a.e); }
fn cf_of(x: CE) -> CF { return cf_norm(vec2<f32>(x.re.hi, x.im.hi), x.e); }
fn cf_len(a: CF) -> R { return r_norm(length(a.m), a.e); }
fn cf_inv(a: CF) -> CF { return cf_norm(vec2<f32>(a.m.x, -a.m.y) / dot(a.m, a.m), -a.e); }
fn ce_mul_cf(x: CE, y: CF) -> CE {
    let re = ff_sub(ff_mul_f(x.re, y.m.x), ff_mul_f(x.im, y.m.y));
    let im = ff_add(ff_mul_f(x.re, y.m.y), ff_mul_f(x.im, y.m.x));
    return ce_norm(re, im, x.e + y.e);
}
fn cf_bad(a: CF) -> bool { return !(finite_f32(a.m.x) && finite_f32(a.m.y)); }

// ---- ZF: complex float-float for the escape finish ----
struct ZF { re: FF, im: FF }
fn zf_of(x: CE) -> ZF { return ZF(ce_re(x), ce_im(x)); }
fn zf_add(a: ZF, b: ZF) -> ZF { return ZF(ff_add(a.re, b.re), ff_add(a.im, b.im)); }

const TAU = FF(6.2831854820251465, -1.7484555314695172e-07);

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

fn output_sample(gid: i32, cls: u32, nu: FF, de: f32, angle: f32) {
    // The normal is the angle in 1/65536 turns, wrapped; a non-finite angle marks the
    // sample invalid (class 3) so the host rejects the keyframe.
    var word = cls << 16u;
    if (!finite_f32(angle) || !finite_f32(de) || !finite_f32(nu.hi)) {
        word = 3u << 16u;
    } else {
        let turns = angle * 10430.378350470453;
        var rounded: i32;
        if (turns >= 0.0) { rounded = i32(turns + 0.5); } else { rounded = i32(turns - 0.5); }
        word |= u32(rounded) & 65535u;
    }
    // nu as the host's f64: hi + lo summed exactly in software binary64.
    nu_out[gid] = to_f64_bits(add_f64(from_f32(bitcast<u32>(nu.hi)), from_f32(bitcast<u32>(nu.lo))));
    de_out[gid] = de;
    word_out[gid] = word;
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
        // The escape test in plain f32: on the escaping step |z|^2 can reach ~1e40,
        // past f32's range, where the float-float product's error term would be
        // inf - inf = NaN; f32 overflows to inf, which still compares above the
        // radius. Within 2^-20 of the radius the exact float-float value decides.
        let a32 = z.re.hi * z.re.hi + z.im.hi * z.im.hi;
        var out = a32 > esc * 1.000001;
        if (!out && a32 > esc * 0.999999) {
            out = ff_add(ff_mul(z.re, z.re), ff_mul(z.im, z.im)).hi > esc;
        }
        // The values below use z rescaled by 2^-k, so nothing overflows.
        if (out) {
            let big = max(abs(z.re.hi), abs(z.im.hi));
            let k = i32((bitcast<u32>(big) >> 23u) & 255u) - 126;
            let zr = ff_scale(z.re, -k);
            let zi = ff_scale(z.im, -k);
            let as_ = ff_add(ff_mul(zr, zr), ff_mul(zi, zi));
            // ln a = ln(as) + 2 k ln 2; log2|z| = ln a / (2 ln 2).
            let ln_a = ff_add(ff_log(as_), ff_mul_f(FF_LN2, 2.0 * f32(k)));
            let log2z = ff_mul_f(ff_div(ln_a, FF_LN2), 0.5);
            let nu = ff_sub(ff(f32(n + 1)), ff_div(ff_log(log2z), FF_LN2));
            // DE = 2 |z| ln|z| / |dz/dc| / pixel, in output pixels; |z| = sqrt(as) 2^k.
            let num = R(2.0 * sqrt(as_.hi) * (log2z.hi * 0.6931471805599453), k);
            let de = r_f32(r_div(num, r_mul(cf_len(du), px)));
            let re = zr.hi * du.m.x + zi.hi * du.m.y;
            let im = zi.hi * du.m.x - zr.hi * du.m.y;
            let sx = cs * re + sn * im;
            let sy = sn * re - cs * im;
            output_sample(gid, 0u, nu, de, atan2_32(sy, sx));
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
