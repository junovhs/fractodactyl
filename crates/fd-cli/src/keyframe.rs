//! Keyframe films (FILM-04): `fd film --keyframes M`. Keyframe `j` is the view at width
//! `w0 / 2^j`, rendered once at `M` times the output size (one sample per keyframe pixel)
//! and kept as samples: the cache holds fractal results, never pictures. Every frame
//! between widths `W_k` and `W_{k+1}` is built from keyframes `k` (outer) and `k + 1`
//! (inner): both are shaded at the frame's film time (looks stay late, DEC-07), then each
//! output pixel gathers their samples with a tent filter (half-width [`TENT`] output pixels,
//! never under one keyframe sample). With `M = 2` an output pixel always spans between 1
//! and 4 keyframe samples per axis, and because
//! samples are fixed to the fractal, not the screen, nothing re-rolls between frames.
//!
//! The inner keyframe fades in over the first quarter of each octave and across a thin
//! margin at its edge, so switching to the next keyframe pair is continuous.
//! Keyframe shading is dithered per sample, so the filtered result keeps sub-level
//! precision and the output needs no second dither.
use crate::film::{render_frame, Used};
use crate::path::sig6;
use fd_kernel::{Params, Zone};
use fd_samples::{Header, Samples, View};
use fd_shade::{Appearance, Pass, Rgb8};

/// Tent half-width in output pixels. Probe (FILM-04, 2 s spiral-on-spiral clip vs direct
/// ss 3): 1.0 px is visibly soft (Laplacian 7.0 vs 11.2); 0.5 px matches ss 3 (11.2) and
/// changes between frames no more than ss 3 does (median ratio 0.99, max 1.10).
const TENT: f32 = 0.5;

pub(crate) struct Keyframes<'a> {
    re: String,
    im: String,
    w0: f64,
    /// Output size in pixels.
    out: (usize, usize),
    /// Keyframe render parameters (M x the output size, ss 1).
    p: Params,
    zone: Option<&'a Zone>,
    /// Rendered keyframes still needed: (index, width, header, samples).
    cache: Vec<(usize, f64, Header, Samples)>,
    /// Keyframes rendered so far and their seconds.
    pub rendered: Vec<(usize, f64, Used)>,
}

impl<'a> Keyframes<'a> {
    pub fn new(
        re: &str,
        im: &str,
        w0: f64,
        out: (u32, u32),
        m: u32,
        p: &Params,
        zone: Option<&'a Zone>,
    ) -> Self {
        let p = Params {
            nx: out.0 * m,
            ny: out.1 * m,
            ss: 1,
            ..*p
        };
        let (re, im) = (re.to_string(), im.to_string());
        Keyframes {
            re,
            im,
            w0,
            out: (out.0 as usize, out.1 as usize),
            p,
            zone,
            cache: Vec::new(),
            rendered: Vec::new(),
        }
    }

    fn width_str(&self, j: usize) -> String {
        sig6(self.w0 / 2f64.powi(j as i32))
    }

    fn width(&self, j: usize) -> f64 {
        self.width_str(j).parse().expect("sig6 parses")
    }

    /// Render keyframe `j` unless cached; drop keyframes shallower than `keep`.
    fn ensure(&mut self, j: usize, keep: usize) -> Result<(), String> {
        self.cache.retain(|c| c.0 >= keep);
        if self.cache.iter().any(|c| c.0 == j) {
            return Ok(());
        }
        let view = View {
            center_re: self.re.clone(),
            center_im: self.im.clone(),
            width: self.width_str(j),
            rotation: 0.0,
        };
        let t = std::time::Instant::now();
        let (h, s, used) = render_frame(&view, &self.p, self.zone)?;
        self.rendered.push((j, t.elapsed().as_secs_f64(), used));
        self.cache.push((j, self.width(j), h, s));
        Ok(())
    }

    /// The keyframe pair for frame width `w`: `k` with `W_{k+1} < w <= W_k`.
    fn octave(&self, w: f64) -> usize {
        let mut k = (self.w0 / w).log2().floor().max(0.0) as usize;
        while k > 0 && self.width(k) < w {
            k -= 1;
        }
        while self.width(k + 1) >= w {
            k += 1;
        }
        k
    }

    /// The frame at width `w`, centred `off` (complex, re/im) from the keyframe centre,
    /// shaded by `look` at appearance `a`.
    pub fn frame(
        &mut self,
        w: f64,
        off: (f64, f64),
        look: &dyn Pass,
        a: &Appearance,
    ) -> Result<Rgb8, String> {
        let req = coverage(w, off, (self.out.0 as u32, self.out.1 as u32));
        let k = self.octave(req);
        self.ensure(k, k)?;
        self.ensure(k + 1, k)?;
        let get = |j: usize| self.cache.iter().find(|c| c.0 == j).expect("ensured");
        let (outer, inner) = (get(k), get(k + 1));
        let sa = Appearance { dither: true, ..*a };
        let (so, si) = (
            look.shade_with(&outer.2, &outer.3, &sa),
            look.shade_with(&inner.2, &inner.3, &sa),
        );
        let lut: Vec<f32> = (0..256).map(|v| (v as f32 / 255.0).powf(2.2)).collect();
        let (ow, oh) = self.out;
        let u = (outer.1 / req).log2().clamp(0.0, 1.0) as f32;
        let ramp = (u / 0.25).min(1.0);
        let margin = 0.1 * (1.0 - u) + 1e-3;
        let (zo, zi) = ((w / outer.1) as f32, (w / inner.1) as f32);
        // The frame centre in each keyframe, in its widths (x) and heights (y, down).
        let aspect = ow as f64 / oh as f64;
        let at = |wj: f64| ((off.0 / wj) as f32, (-off.1 / wj * aspect) as f32);
        let ((oxo, oyo), (oxi, oyi)) = (at(outer.1), at(inner.1));
        let mut data = vec![0u8; ow * oh * 3];
        let threads = std::thread::available_parallelism()
            .map_or(1, |n| n.get())
            .clamp(1, oh);
        let rows_per = oh.div_ceil(threads);
        std::thread::scope(|sc| {
            for (c, chunk) in data.chunks_mut(rows_per * ow * 3).enumerate() {
                let (so, si, lut) = (&so, &si, &lut);
                sc.spawn(move || {
                    for (r, row) in chunk.chunks_mut(ow * 3).enumerate() {
                        let py = c * rows_per + r;
                        let yn = (py as f32 + 0.5) / oh as f32 - 0.5;
                        for px in 0..ow {
                            let xn = (px as f32 + 0.5) / ow as f32 - 0.5;
                            let (xi, yi) = (xn * zi + oxi, yn * zi + oyi);
                            let d = (2.0 * xi.abs()).max(2.0 * yi.abs());
                            let wi = ramp * ((1.0 - d) / margin).clamp(0.0, 1.0);
                            let mut col = [0f32; 3];
                            if wi < 1.0 {
                                let o = gather(
                                    so,
                                    lut,
                                    xn * zo + oxo,
                                    yn * zo + oyo,
                                    TENT * zo * so.w as f32 / ow as f32,
                                );
                                col = o.map(|v| v * (1.0 - wi));
                            }
                            if wi > 0.0 {
                                let i =
                                    gather(si, lut, xi, yi, TENT * zi * si.w as f32 / ow as f32);
                                col = [0, 1, 2].map(|j| col[j] + i[j] * wi);
                            }
                            for (o, v) in row[px * 3..px * 3 + 3].iter_mut().zip(col) {
                                *o = (v.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0 + 0.5) as u8;
                            }
                        }
                    }
                });
            }
        });
        Ok(Rgb8 { w: ow, h: oh, data })
    }
}

/// The keyframe width needed to hold a frame of width `w` centred `off` (complex) from
/// the keyframe centre, for output size `out` (same aspect as the keyframes).
pub(crate) fn coverage(w: f64, off: (f64, f64), out: (u32, u32)) -> f64 {
    (w + 2.0 * off.0.abs()).max(w + 2.0 * off.1.abs() * out.0 as f64 / out.1 as f64)
}

/// Tent-filtered linear colour of image `img` around the point `x` image widths and `y`
/// image heights from its centre, over a footprint of half-width `f` image pixels (at
/// least one, so the filter never degenerates to point sampling).
fn gather(img: &Rgb8, lut: &[f32], x: f32, y: f32, f: f32) -> [f32; 3] {
    let f = f.max(1.0);
    let (iw, ih) = (img.w as i64, img.h as i64);
    let cx = x * img.w as f32 + img.w as f32 / 2.0;
    let cy = y * img.h as f32 + img.h as f32 / 2.0;
    let (x0, x1) = ((cx - f).floor() as i64, (cx + f).ceil() as i64);
    let (y0, y1) = ((cy - f).floor() as i64, (cy + f).ceil() as i64);
    let (mut acc, mut wsum) = ([0f32; 3], 0f32);
    for sy in y0..=y1 {
        let wy = 1.0 - ((sy as f32 + 0.5 - cy).abs() / f);
        if wy <= 0.0 {
            continue;
        }
        let row = sy.clamp(0, ih - 1) as usize * img.w;
        for sx in x0..=x1 {
            let wx = 1.0 - ((sx as f32 + 0.5 - cx).abs() / f);
            if wx <= 0.0 {
                continue;
            }
            let k = (row + sx.clamp(0, iw - 1) as usize) * 3;
            let wgt = wx * wy;
            for (a, &v) in acc.iter_mut().zip(&img.data[k..k + 3]) {
                *a += lut[v as usize] * wgt;
            }
            wsum += wgt;
        }
    }
    acc.map(|a| a / wsum.max(1e-12))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gather_keeps_flat_colour_and_weights_by_distance() {
        let flat = Rgb8 {
            w: 8,
            h: 4,
            data: [10u8, 128, 250].repeat(32),
        };
        let lut: Vec<f32> = (0..256).map(|v| v as f32).collect();
        for (x, y, f) in [(0.0, 0.0, 1.0), (0.49, -0.49, 3.0), (-0.3, 0.2, 0.2)] {
            let g = gather(&flat, &lut, x, y, f);
            assert!(
                g.iter()
                    .zip([10.0, 128.0, 250.0])
                    .all(|(a, b)| (a - b).abs() < 1e-3),
                "{g:?}"
            );
        }
        // Left half 0, right half 200: a point a quarter pixel right of the edge, with a
        // one-pixel tent, sees both sides, more of the right.
        let mut img = Rgb8 {
            w: 8,
            h: 4,
            data: vec![0; 96],
        };
        for r in 0..4 {
            for c in 4..8 {
                img.data[(r * 8 + c) * 3..(r * 8 + c) * 3 + 3].copy_from_slice(&[200; 3]);
            }
        }
        let v = gather(&img, &lut, 0.25 / 8.0, 0.0, 1.0)[0];
        assert!(v > 100.0 && v < 200.0, "{v}");
    }

    #[test]
    fn octave_picks_the_pair_around_the_frame_width() {
        let p = Params {
            nx: 2,
            ny: 2,
            ss: 1,
            max_iter: 10,
            escape_radius: 1e10,
            columns: crate::render::columns("nu").unwrap(),
            threads: 1,
            tier: None,
        };
        let k = Keyframes::new("0", "0", 4.0, (2, 2), 2, &p, None);
        assert_eq!(k.octave(4.0), 0);
        assert_eq!(k.octave(2.0001), 0);
        assert_eq!(k.octave(2.0), 1);
        assert_eq!(k.octave(1e-80), (4e80f64).log2().floor() as usize);
        let j = k.octave(3e-50);
        assert!(k.width(j) >= 3e-50 && k.width(j + 1) < 3e-50);
        // Off-centre frames need wider keyframes: a frame 1 wide, 0.3 right of centre,
        // needs 1.6; 0.1 above centre in a 2:1 output needs 1 + 0.4.
        assert!((coverage(1.0, (0.3, 0.0), (2, 1)) - 1.6).abs() < 1e-12);
        assert!((coverage(1.0, (0.0, 0.1), (2, 1)) - 1.4).abs() < 1e-12);
        assert_eq!(coverage(2.0, (0.0, 0.0), (16, 9)), 2.0);
    }
}
