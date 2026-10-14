//! Existing GPU-02 guarded-f32 finish, shared by CPU and CubeCL. The precise
//! landing checkpoint survives every rejected attempt (DEC-25 rule 4).
use cubecl::prelude::*;

#[cube]
fn try_f32(
    zr: f64,
    zi: f64,
    dr: f64,
    di: f64,
    cr: f64,
    ci: f64,
    mut n: u32,
    escape_sq: f64,
    pixel: f64,
) -> (f64, f64, f64, f64, u32, bool, bool) {
    let mut e = (((dr.abs().max(di.abs())).to_bits() >> 52u64) & 2047u64) as i32 - 1022;
    if e < 0 {
        e = 0;
    }
    if e > 900 {
        e = 900;
    }
    let mut up = f64::from_bits(((e + 1023) as u64) << 52u64);
    let s = pixel * up;
    let mut valid = s > 1e-30f64;
    let mut escaped = false;
    let mut x = zr as f32;
    let mut y = zi as f32;
    let mut dx = (dr / up) as f32;
    let mut dy = (di / up) as f32;
    let c = cr as f32;
    let d = ci as f32;
    let mut one = (1.0f64 / up) as f32;
    let mut inv = (1.0f64 / s) as f32;
    let mut ep = (5.960464477539063e-8f64 * (zr * zr + zi * zi).sqrt() / s) as f32;
    let cabs = (c * c + d * d).sqrt();
    let near = ((0.5f64 * escape_sq).sqrt() as f32) * (1.0f32 - 1e-6f32);
    while n < 20_000u32 && valid {
        let lz = (x * x + y * y).sqrt();
        let next_dx = 2.0f32 * (x * dx - y * dy) + one;
        let next_dy = 2.0f32 * (x * dy + y * dx);
        let next_x = x * x - y * y + c;
        y = 2.0f32 * x * y + d;
        x = next_x;
        dx = next_dx;
        dy = next_dy;
        n += 1;
        ep = 2.0f32 * lz * ep + 4.76837158203125e-7f32 * (lz * lz + cabs) * inv;
        if dx.abs().max(dy.abs()) > 1e24f32 {
            dx *= 8.271806125530277e-25f32;
            dy *= 8.271806125530277e-25f32;
            one *= 8.271806125530277e-25f32;
            inv *= 8.271806125530277e-25f32;
            ep *= 8.271806125530277e-25f32;
            up *= 1.2089258196146292e24f64;
        }
        if !(ep < 1e30f32) {
            valid = false;
            break;
        }
        if x.abs().max(y.abs()) >= near {
            let a = x as f64 * x as f64 + y as f64 * y as f64;
            if a > escape_sq {
                escaped = true;
                break;
            }
        }
    }

    let dl = (dx as f64 * dx as f64 + dy as f64 * dy as f64).sqrt();
    (
        x as f64,
        y as f64,
        dx as f64 * up,
        dy as f64 * up,
        n,
        escaped,
        valid && ep as f64 <= 1e-3f64 * dl,
    )
}

#[cube]
pub fn finish(
    mut zr: f64,
    mut zi: f64,
    mut dr: f64,
    mut di: f64,
    cr: f64,
    ci: f64,
    mut n: u32,
    escape_sq: f64,
    pixel: f64,
    #[comptime] gpu: bool,
) -> (f64, f64, f64, f64, u32, bool) {
    let mut try32 = true;
    let mut escaped_out = false;
    while n < 20_000u32 {
        if try32 && (n & 3u32) == 0u32 {
            let ready = (dr * dr + di * di) * pixel * pixel
                >= 0.059604644775390625f64 * (zr * zr + zi * zi).max(1.0f64);
            let ready = if gpu { plane_all(ready) } else { ready };
            if ready {
                let (x, y, dx, dy, end, escaped, accepted) =
                    try_f32(zr, zi, dr, di, cr, ci, n, escape_sq, pixel);
                if accepted {
                    zr = x;
                    zi = y;
                    dr = dx;
                    di = dy;
                    n = end;
                    escaped_out = escaped;
                    break;
                }
                // Rejected f32 result is discarded; these variables still hold
                // the exact binary64 hand-over checkpoint, never upcast f32.
                try32 = false;
            }
        }
        let next_dr = 2.0f64 * (zr * dr - zi * di) + 1.0f64;
        let next_di = 2.0f64 * (zr * di + zi * dr);
        let next_zr = zr * zr - zi * zi + cr;
        zi = 2.0f64 * zr * zi + ci;
        zr = next_zr;
        dr = next_dr;
        di = next_di;
        n += 1;
        if zr * zr + zi * zi > escape_sq {
            escaped_out = true;
            break;
        }
    }
    (zr, zi, dr, di, n, escaped_out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejected_float_tail_restarts_from_binary64_checkpoint() {
        // A large landing triggers hand-over but overflows f32 squares. The
        // rejected result must not poison the exact binary64 restart.
        let got = finish(1e20, 1e20, 1e100, 1e100, -0.75, 0.1, 0, 1e20, 1e-78, false);
        let expected =
            crate::finish_loop::<f64>(1e20, 1e20, 1e100, 1e100, -0.75, 0.1, 0, 20_000, 1e20);
        assert_eq!(got, expected);
        assert!(!try_f32(1e20, 1e20, 1e100, 1e100, -0.75, 0.1, 0, 1e20, 1e-78).6);
    }
    #[test]
    fn float_tail_rejects_insufficient_pixel_spacing() {
        assert!(!try_f32(0.1, 0.1, 1.0, 0.0, -0.75, 0.1, 0, 1e20, 1e-100).6);
    }
}
