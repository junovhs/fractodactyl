// GPU-02's guarded-f32 tail, preserving the binary64 restart checkpoint.
struct Attempt { z: vec4<f64>, n: u32, escaped: bool, accepted: bool }
fn declined(n: u32) -> Attempt { return Attempt(vec4<f64>(0.0), n, false, false); }
fn try_f32(zr: f64, zi: f64, dr: f64, di: f64, cr: f64, ci: f64, start: u32, escape_sq: f64) -> Attempt {
    let parts = frexp(max(abs(dr), abs(di)));
    let e = clamp(parts.exp, i64(0), i64(900));
    var up = ldexp(f64(1.0), i32(e));
    let s = camera[0] * up;
    if !(s > f64(1e-30)) { return declined(start); }
    var x = f32(zr); var y = f32(zi);
    var dx = f32(dr / up); var dy = f32(di / up);
    let c = f32(cr); let d = f32(ci);
    var one = f32(f64(1.0) / up); var inv = f32(f64(1.0) / s);
    var ep = f32(f64(5.960464477539063e-8) * sqrt(zr * zr + zi * zi) / s);
    let cabs = sqrt(c * c + d * d);
    let near = f32(sqrt(f64(0.5) * escape_sq)) * (1.0 - 1e-6);
    var n = start;
    while n < 20000u {
        let lz = sqrt(x * x + y * y);
        let next_dx = 2.0 * (x * dx - y * dy) + one;
        let next_dy = 2.0 * (x * dy + y * dx);
        let next_x = x * x - y * y + c;
        y = 2.0 * x * y + d; x = next_x; dx = next_dx; dy = next_dy; n += 1u;
        ep = 2.0 * lz * ep + 4.76837158203125e-7 * (lz * lz + cabs) * inv;
        if max(abs(dx), abs(dy)) > 1e24 {
            dx *= 8.271806125530277e-25; dy *= 8.271806125530277e-25;
            one *= 8.271806125530277e-25; inv *= 8.271806125530277e-25;
            ep *= 8.271806125530277e-25; up *= f64(1.2089258196146292e24);
        }
        if !(ep < 1e30) { return declined(n); }
        if max(abs(x), abs(y)) < near { continue; }
        let a = f64(x) * f64(x) + f64(y) * f64(y);
        if a > escape_sq {
            let dl = sqrt(f64(dx) * f64(dx) + f64(dy) * f64(dy));
            if !(f64(ep) <= f64(1e-3) * dl) { return declined(n); }
            return Attempt(vec4<f64>(f64(x), f64(y), f64(dx) * up, f64(dy) * up), n, true, true);
        }
    }
    let dl = sqrt(f64(dx) * f64(dx) + f64(dy) * f64(dy));
    return Attempt(vec4<f64>(f64(x), f64(y), f64(dx) * up, f64(dy) * up), n, false, f64(ep) <= f64(1e-3) * dl);
}
