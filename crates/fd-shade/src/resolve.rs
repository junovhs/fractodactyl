//! Sample grid to output pixels: average each `ss x ss` block in linear light.
use fd_samples::Header;

/// Packed RGB8 image.
pub struct Rgb8 {
    /// Width in pixels.
    pub w: usize,
    /// Height in pixels.
    pub h: usize,
    /// Row-major RGB triples.
    pub data: Vec<u8>,
}

/// `f(sample index)` gives linear RGB in 0..1; the result is gamma-encoded RGB8.
/// Rows of pixels are shaded on all cores (each row is independent; the output is
/// identical to a sequential pass).
pub(crate) fn resolve(h: &Header, f: impl Fn(usize) -> [f32; 3] + Sync) -> Rgb8 {
    let (w, ht) = h.pixels();
    let mut data = vec![0u8; w * ht * 3];
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get()).clamp(1, ht.max(1));
    let rows_per = ht.div_ceil(threads).max(1);
    std::thread::scope(|sc| {
        for (k, chunk) in data.chunks_mut(rows_per * w * 3).enumerate() {
            let f = &f;
            sc.spawn(move || {
                for (r, row) in chunk.chunks_mut(w * 3).enumerate() {
                    pixel_row(h, k * rows_per + r, row, f);
                }
            });
        }
    });
    Rgb8 { w, h: ht, data }
}

/// Average each `ss x ss` block of output row `py` in linear light into `out`.
fn pixel_row(h: &Header, py: usize, out: &mut [u8], f: &impl Fn(usize) -> [f32; 3]) {
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
        for (d, v) in o.iter_mut().zip(acc) {
            *d = ((v * inv).clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0 + 0.5) as u8;
        }
    }
}
