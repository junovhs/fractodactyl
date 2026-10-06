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
pub(crate) fn resolve(h: &Header, f: impl Fn(usize) -> [f32; 3]) -> Rgb8 {
    let (w, ht) = h.pixels();
    let (ss, nx) = (h.ss as usize, h.nx as usize);
    let inv = 1.0 / (ss * ss) as f32;
    let mut data = Vec::with_capacity(w * ht * 3);
    for py in 0..ht {
        for px in 0..w {
            let mut acc = [0.0f32; 3];
            for sy in 0..ss {
                let base = (py * ss + sy) * nx + px * ss;
                for k in base..base + ss {
                    let c = f(k);
                    acc = [acc[0] + c[0], acc[1] + c[1], acc[2] + c[2]];
                }
            }
            data.extend(acc.map(|v| ((v * inv).clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0 + 0.5) as u8));
        }
    }
    Rgb8 { w, h: ht, data }
}
