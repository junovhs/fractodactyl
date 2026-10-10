//! Sample grid to output pixels: average each `ss x ss` block in linear light, then
//! gamma-encode to 8 bits, optionally dithered (FX-04).
use fd_samples::Header;

/// Peak dither amplitude in 8-bit levels.
pub const DITHER_LEVELS: f32 = 2.0;

/// Packed RGB8 image.
pub struct Rgb8 {
    /// Width in pixels.
    pub w: usize,
    /// Height in pixels.
    pub h: usize,
    /// Row-major RGB triples.
    pub data: Vec<u8>,
}

/// `f(sample index)` gives linear RGB in 0..1; the result is gamma-encoded RGB8. With
/// `dither`, a triangular-PDF offset of up to ±[`DITHER_LEVELS`] levels, fixed per pixel and channel (a
/// hash of the position, so it never crawls between frames), is added before rounding:
/// a slow gradient then reads as a smooth ramp instead of a stack of one-level rings.
/// Two levels, not one: x264 at crf 16 4:2:0 smooths a ±1 dither back into rings, ±2
/// survives it (checked on a dark master-ice gradient, FX-04).
/// Rows of pixels are shaded on all cores (each row is independent; the output is
/// identical to a sequential pass).
pub(crate) fn resolve(h: &Header, dither: bool, f: impl Fn(usize) -> [f32; 3] + Sync) -> Rgb8 {
    let (w, ht) = h.pixels();
    let mut data = vec![0u8; w * ht * 3];
    let threads = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .clamp(1, ht.max(1));
    let rows_per = ht.div_ceil(threads).max(1);
    std::thread::scope(|sc| {
        for (k, chunk) in data.chunks_mut(rows_per * w * 3).enumerate() {
            let f = &f;
            sc.spawn(move || {
                for (r, row) in chunk.chunks_mut(w * 3).enumerate() {
                    pixel_row(h, k * rows_per + r, row, dither, f);
                }
            });
        }
    });
    Rgb8 { w, h: ht, data }
}

/// Average each `ss x ss` block of output row `py` in linear light into `out`.
fn pixel_row(h: &Header, py: usize, out: &mut [u8], dither: bool, f: &impl Fn(usize) -> [f32; 3]) {
    let (ss, nx) = (h.ss as usize, h.nx as usize);
    let inv = 1.0 / (ss * ss) as f32;
    for (px, o) in out.chunks_mut(3).enumerate() {
        let mut acc = [0.0f32; 3];
        for sy in 0..ss {
            let base = (py * ss + sy) * nx + px * ss;
            for k in base..base + ss {
                let c = f(k);
                acc = [acc[0] + c[0], acc[1] + c[1], acc[2] + c[2]];
            }
        }
        for (c, (d, v)) in o.iter_mut().zip(acc).enumerate() {
            let t = if dither {
                DITHER_LEVELS * tpdf((py as u32) << 16 ^ (px as u32) << 2 ^ c as u32)
            } else {
                0.0
            };
            *d = ((v * inv).clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0 + 0.5 + t).clamp(0.0, 255.0)
                as u8;
        }
    }
}

/// Triangular-PDF dither in (-1, 1) levels from a position key: the sum of two
/// independent uniforms drawn from an integer hash (lowbias32).
fn tpdf(key: u32) -> f32 {
    let mut x = key.wrapping_add(0x9e37_79b9);
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    ((x & 0xffff) as f32 + (x >> 16) as f32) / 65535.0 - 1.0
}

#[cfg(test)]
mod tests {
    use super::tpdf;

    #[test]
    fn tpdf_is_centred_triangular_and_bounded() {
        let v: Vec<f32> = (0..1u32 << 16).map(|k| tpdf(k << 2)).collect();
        let mean = v.iter().sum::<f32>() / v.len() as f32;
        let var = v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32;
        assert!(
            mean.abs() < 0.01 && (var - 1.0 / 6.0).abs() < 0.01,
            "mean {mean} var {var}"
        );
        assert!(v.iter().all(|x| (-1.0..=1.0).contains(x)));
    }
}
