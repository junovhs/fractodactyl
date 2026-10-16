// Zone keyframe kernel (GPU-13, WGSL on wgpu; GPU-12 route). Four fused
// stages: return map, approach, Koenigs/patch, escape finish. The host
// prepends zone_f64_lib.wgsl and appends zone_finish32.wgsl (mixed, f32) or a
// binary64-only stub (f64).
//
// Native binary64 needs wgpu SHADER_F64; IEEE bit reads need SHADER_INT64.
// frexp is never used: pinned Naga's f64 frexp crashes the NVIDIA Windows
// pipeline compiler (GPU-19), so exponents are read from the bits.


// Zone shape fixed for a whole film: pipeline-overridable so the biseries and
// Koenigs loops have constant trip counts.
override DEG: i32 = 7;
override NPHI: i32 = 1;
override NPSI: i32 = 1;
// f32 approach and Koenigs series under an error bound (the f32 variant).
override F32_FRONT: bool = false;


const EPS32: f32 = 5.9604644775390625e-8; // 2^-24, f32 unit roundoff
// Position error bound, in output pixels, above which an f32 stage restarts in
// binary64 (DEC-25 rule 4); the strict 1e-3 px bar of DEC-18 speed claims.
const TOL_PX: f64 = 1e-3;
const SWITCH_PX_Z: f64 = 4096.0 * 5.9604644775390625e-8 / 1e-3;
fn mul32(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x);
}
// frexp's exponent of max(|a.x|, |a.y|), clamped to [-900, 900]: dz/dc grows
// like 1/width and leaves the f32 range near 1e-38, so f32 stages carry it as
// an f32 mantissa times 2^e with e kept here. Subnormals clamp like frexp.
fn exponent_of(a: C) -> i32 {
    let x = max(abs(a.x), abs(a.y));
    if x == 0.0 { return 0; }
    let e = i32((bitcast<u64>(x) >> 52u) & 2047lu) - 1022;
    return clamp(e, -900, 900);
}

struct Series { y: C, dy: C }
fn series(start: i32, count: i32, x: C) -> Series {
    var y = C(0.0, 0.0);
    var dy = C(0.0, 0.0);
    for (var i = count - 1; i >= 0; i -= 1) {
        let a = cx(start + 2 * i);
        y = mul(y + a, x);
        dy = mul(dy, x) + f64(i + 1) * a;
    }
    return Series(y, dy);
}
// f32 series; sa = sum |a_i| |x|^(i+1) bounds its Horner rounding error.
struct Series32 { y: vec2<f32>, dy: vec2<f32>, sa: f32 }
fn series32(start: i32, count: i32, x: vec2<f32>) -> Series32 {
    var y = vec2<f32>(0.0);
    var dy = vec2<f32>(0.0);
    var sa = 0.0f;
    let ax = length(x);
    for (var i = count - 1; i >= 0; i -= 1) {
        let a = vec2<f32>(cx(start + 2 * i));
        y = mul32(y + a, x);
        dy = mul32(dy, x) + f32(i + 1) * a;
        sa = (sa + length(a)) * ax;
    }
    return Series32(y, dy, sa);
}

// Return-map biseries coefficients at this sample's parameter.
var<private> b: array<C, 8>;
var<private> bv: array<C, 8>;
var<private> g_phi_base: i32;
var<private> g_psi_base: i32;
var<private> g_node_base: i32;
var<private> g_leaf_base: i32;

struct Jet { y: C, d: C, dd: C }
fn eval2(x: C) -> Jet {
    var y = b[DEG];
    var dy = C(0.0, 0.0);
    var ddy = C(0.0, 0.0);
    for (var i = DEG - 1; i >= 0; i -= 1) {
        ddy = mul(ddy, x) + dy;
        dy = mul(dy, x) + y;
        y = mul(y, x) + b[i];
    }
    return Jet(y, dy, 2.0 * ddy);
}
fn compose(x: C, q: i32) -> Jet {
    var y = x;
    var d = C(1.0, 0.0);
    var dd = C(0.0, 0.0);
    for (var i = 0; i < q; i += 1) {
        let f = eval2(y);
        dd = mul(f.dd, mul(d, d)) + mul(f.d, dd);
        d = mul(f.d, d);
        y = f.y;
    }
    return Jet(y, d, dd);
}
fn attracted(u: C) -> bool {
    for (var q = 1; q <= 4; q += 1) {
        var x = u;
        for (var i = 0; i < 16; i += 1) {
            let f = compose(x, q);
            let st = divc(f.y - x, f.d - C(1.0, 0.0));
            x -= st;
            if nonfinite(x.x) || nonfinite(x.y) { break; }
            if norm2(st) <= 1e-28 * norm2(x) {
                let g = compose(x, q);
                let l = length(g.d);
                if l < 1.0 && length(u - x) * length(g.dd) <= 0.5 * (1.0 - l) {
                    return true;
                }
                break;
            }
        }
    }
    return false;
}

struct Patch { ok: bool, z: C, dz: C, m: i32 }
fn horner_patch(re0: f64, im0: f64, j: f64, root: i32, nodes: i32, leaves: i32) -> Patch {
    let none = Patch(false, C(0.0, 0.0), C(0.0, 0.0), 0);
    if nodes == 0 || j < 0.0 { return none; }
    let tau: f64 = 6.283185307179586476925286766559;
    let re = re0 + j * value(15);
    var im = im0 + j * value(16);
    im -= floor(im / tau) * tau;
    var node = min(i32(im / (tau / f64(root))), root - 1);
    for (var k = 0; k < nodes; k += 1) {
        if node < 0 || node >= nodes { return none; }
        var at = g_node_base + 4 * node;
        let kid = i32(value(at + 2));
        if kid < 0 {
            let leaf = i32(value(at + 3));
            if leaf < 0 || leaf >= leaves { return none; }
            at = g_leaf_base + 36 * leaf;
            let m = i32(value(at + 3));
            if m < 0 { return none; }
            let r = value(at + 2);
            let t = C((re - value(at)) / r, (im - value(at + 1)) / r);
            var z = C(0.0, 0.0);
            var dz = C(0.0, 0.0);
            for (var a = 15; a >= 0; a -= 1) {
                dz = mul(dz, t) + z;
                z = mul(z, t) + cx(at + 4 + 2 * a);
            }
            return Patch(true, z, dz / r, m);
        }
        node = kid + 2 * select(0, 1, re >= value(at)) + select(0, 1, im >= value(at + 1));
    }
    return none;
}


// Approach, Koenigs jump and tail patch from the binary64 return-map
// checkpoint. lo runs the approach and both Koenigs series in f32 and sets
// err to a first-order bound on the absolute error of the landing z.
struct Front { z: C, du: C, n: i32, err: f64 }
fn front(lo: bool, u: C, du_in: C, n_in: i32) -> Front {
    var du = du_in;
    var n = n_in;
    // 23 approach iterates, with dz/dc in unscaled parameter coordinates.
    var d = u * value(0);
    var ed = 0.0lf;
    if lo {
        let e = exponent_of(du);
        var d32 = vec2<f32>(d);
        var du32 = vec2<f32>(ldexp2(du, -e));
        let one32 = vec2<f32>(f32(ldexp(1.0lf, -e)), 0.0);
        ed = f64(EPS32) * length(d);
        for (var i = 0; i < 23; i += 1) {
            let zz = vec2<f32>(cx(160 + 2 * i));
            let lz = length(cx(160 + 2 * i));
            let ld = f64(length(d32));
            ed = f64(2.0 * length(zz + d32)) * ed + 4.0 * f64(EPS32) * (2.0 * lz * ld + ld * ld);
            du32 = 2.0 * mul32(zz + d32, du32) + one32;
            d32 = 2.0 * mul32(zz, d32) + mul32(d32, d32);
        }
        d = C(d32);
        du = ldexp2(C(du32), e);
    } else {
        for (var i = 0; i < 23; i += 1) {
            let zc = cx(160 + 2 * i);
            du = 2.0 * mul(zc + d, du) + C(1.0, 0.0);
            d = 2.0 * mul(zc, d) + mul(d, d);
        }
    }
    n += 23;

    // Koenigs jump; Horner in the tail atlas is the final landing.
    let h0 = cx(17) + d;
    let alpha = cx(13);
    if !(length(h0) < value(2)) { return Front(alpha + h0, du, n, ed); }
    var w0: C;
    var dphi: C;
    var ew = 0.0lf;
    if lo {
        let s = series32(g_phi_base, NPHI, vec2<f32>(h0));
        w0 = C(s.y);
        dphi = C(s.dy);
        ew = length(dphi) * ed + f64((2.0 * f32(NPHI) + 2.0) * EPS32 * s.sa);
    } else {
        let s = series(g_phi_base, NPHI, h0);
        w0 = s.y;
        dphi = s.dy;
    }
    // One log|w0| serves the jump count and the patch; slots 22 and 31 hold
    // log(r0)/log|rho| and 1/log|rho| so j costs no division.
    let lnrho = value(15);
    let lnw = fp64_log(length(w0));
    let j = floor(fma(-lnw, value(31), value(22)));
    let landing = horner_patch(lnw, fp64_atan2(w0.y, w0.x), j, i32(value(23)),
        i32(value(26)), i32(value(27)));
    if landing.ok {
        du = mul(mul(mul(landing.dz, dphi), du), divc(C(1.0, 0.0), w0));
        // dz/dw0 = dzs/w0 through t = (log w0 + j log rho - centre)/r.
        return Front(landing.z, du, n + 2 * i32(j) + landing.m, length(landing.dz) / length(w0) * ew);
    }
    if j > 0.0 {
        let t = j * value(16);
        let mag = fp64_exp(j * lnrho);
        let sc = fp64_sincos(t);
        let rj = mag * C(sc.y, sc.x);
        // w0 is tiny and rho^j huge: their product is O(r0) only in binary64.
        let x = mul(w0, rj);
        var hw: C;
        var dpsi: C;
        var err = ed;
        if lo {
            let s = series32(g_psi_base, NPSI, vec2<f32>(x));
            hw = C(s.y);
            dpsi = C(s.dy);
            err = length(dpsi) * (length(rj) * ew + f64(EPS32) * length(x))
                + f64((2.0 * f32(NPSI) + 2.0) * EPS32 * s.sa);
        } else {
            let s = series(g_psi_base, NPSI, x);
            hw = s.y;
            dpsi = s.dy;
        }
        du = mul(mul(mul(dpsi, rj), dphi), du);
        return Front(alpha + hw, du, n + 2 * i32(j), err);
    }
    return Front(alpha + h0, du, n, ed);
}

// Finish at the fixed nucleus and encode nu, pixel DE and screen normal.
// errpx bounds the landing position error in output pixels (0 for binary64
// fronts). With f32 finishing, the binary64 loop hands over once the pixel
// spacing |dz/dc| * pixel exceeds SWITCH_PX_Z * max(|z|, 1): from there f32
// rounding stays far below the tolerance. A failed f32 tail resumes here.
fn finish(gid: i32, z_in: C, du_in: C, n_in: i32, errpx: f64) {
    var z = z_in;
    var du = du_in;
    var n = n_in;
    let c = cx(11);
    let cs = value(28);
    let sn = value(29);
    let maxit = i32(value(5));
    var try32 = F32_FINISH;
    let pw2 = value(30) * value(30);
    while n < maxit {
        if try32 && (n & 3) == 0 {
            let ready = norm2(du) * pw2 >= SWITCH_PX_Z * SWITCH_PX_Z * max(norm2(z), 1.0);
            if wave_all(ready) {
                if finish32(gid, z, du, n, errpx) { return; }
                try32 = false;
            }
        }
        du = 2.0 * mul(z, du) + C(1.0, 0.0);
        z = mul(z, z) + c;
        n += 1;
        let a = norm2(z);
        if a > value(21) {
            let log2z = 0.5 * fp64_log2(a);
            let nu = f64(n) + 1.0 - fp64_log2(log2z);
            let de = 2.0 * sqrt(a) * (log2z * FP64_LN2) / length(du) / value(30);
            let re = z.x * du.x + z.y * du.y;
            let im = z.y * du.x - z.x * du.y;
            let sx = cs * re + sn * im;
            let sy = sn * re - cs * im;
            output_sample(gid, 0u, nu, de, fp64_atan2(sy, sx));
            return;
        }
    }
    output_sample(gid, 2u, 0.0, 0.0, 0.0);
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>,
        @builtin(num_workgroups) groups: vec3<u32>) {
    let gid = i32(id.x + id.y * (groups.x * 64u));
    let nx = i32(value(6));
    let ny = i32(value(7));
    if gid >= nx * ny { return; }
    let period = i32(value(3));
    let maxit = i32(value(5));
    let nn = i32(value(26));
    g_phi_base = 206;
    g_psi_base = g_phi_base + 2 * NPHI;
    g_node_base = g_psi_base + 2 * NPSI;
    g_leaf_base = g_node_base + 4 * nn;
    let col = gid % nx;
    let row = gid / nx;
    let x = f64(col) + 0.5 - f64(nx) * 0.5;
    let y = f64(row) + 0.5 - f64(ny) * 0.5;
    let cs = value(28);
    let sn = value(29);
    let uv = C(cs * x + sn * y, sn * x - cs * y);
    let v = cx(8) + uv * value(10);
    for (var i = 0; i <= DEG; i += 1) {
        var s = C(0.0, 0.0);
        var ds = C(0.0, 0.0);
        for (var j = DEG - i; j >= 0; j -= 1) {
            ds = mul(ds, v) + s;
            s = mul(s, v) + cx(32 + 2 * (8 * i + j));
        }
        b[i] = s;
        bv[i] = ds;
    }
    b[0] += cx(19);
    var u = v;
    var du = C(1.0, 0.0);
    var n = 1;
    var work = 0;
    let scale = value(0);
    let guard = value(1);
    while length(u) * scale <= guard {
        if n + period > maxit {
            output_sample(gid, 2u, 0.0, 0.0, 0.0);
            return;
        }
        var s = b[DEG];
        var su = C(0.0, 0.0);
        var sv = bv[DEG];
        for (var i = DEG - 1; i >= 0; i -= 1) {
            su = mul(su, u) + s;
            s = mul(s, u) + b[i];
            sv = mul(sv, u) + bv[i];
        }
        let delta = s - u;
        du = mul(su, du) + sv;
        if (norm2(su) < 1.0 && norm2(delta) <= 1e-24 * norm2(u))
            || (work >= 16 && (work & (work - 1)) == 0 && attracted(u)) {
            output_sample(gid, 1u, 0.0, 0.0, 0.0);
            return;
        }
        u = s;
        n += period;
        work += 1;
    }

    // DEC-25 rule 4: a front whose f32 error bound exceeds a quarter of the
    // tolerance restarts from the binary64 return-map checkpoint (u, du, n).
    var f = front(F32_FRONT, u, du, n);
    if F32_FRONT && !(f.err <= 0.25 * TOL_PX * length(f.du) * value(30)) {
        f = front(false, u, du, n);
    }
    var errpx = 0.0lf;
    if f.err != 0.0 { errpx = f.err / (length(f.du) * value(30)); }
    finish(gid, f.z, f.du, f.n, errpx);
}
