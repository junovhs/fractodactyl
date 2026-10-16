//! Film tent resample on wgpu (SHAD-06): the WGSL port of `keyframe::resample`,
//! including rotated frames. Test-only until the film GPU path is wired to it.

use crate::keyframe::Resample;
use crate::wgpu_compute::{bytes, dispatch, words, Binding};

const SHADER: &str = include_str!("../shaders/film_resample.wgsl");

/// Resample full-frame packed RGB keyframes (`r | g << 8 | b << 16`; only each `rect`
/// is read) into the output frame. Returns the adapter name and the packed output.
pub(crate) fn run(
    r: &Resample,
    outer: &[u32],
    inner: &[u32],
    lut: &[f32; 256],
) -> Result<(String, Vec<u32>), String> {
    let (((owidth, oheight), ro), ((iwidth, iheight), ri)) = (r.outer, r.inner);
    if outer.len() != owidth * oheight || inner.len() != iwidth * iheight {
        return Err("keyframe sizes do not match the resample".into());
    }
    let u = |v: usize| v as u32;
    let rect = |(x, y, w, h): (usize, usize, usize, usize)| [u(x), u(y), u(w), u(h)];
    let ints = [
        u(r.out.0),
        u(r.out.1),
        u(owidth),
        u(oheight),
        u(iwidth),
        u(iheight),
        0,
        0,
    ];
    let floats = [
        r.zoom.0,
        r.zoom.1,
        r.outer_at.0,
        r.outer_at.1,
        r.inner_at.0,
        r.inner_at.1,
        r.ramp,
        r.margin,
        r.sin,
        r.cos,
        0.0,
        0.0,
    ];
    let params: Vec<u8> = bytes(&ints)
        .into_iter()
        .chain(floats.iter().flat_map(|f| f.to_ne_bytes()))
        .chain(bytes(&rect(ro)))
        .chain(bytes(&rect(ri)))
        .collect();
    let lut: Vec<u8> = lut.iter().flat_map(|f| f.to_ne_bytes()).collect();
    let (name, out) = dispatch(
        "fd film resample",
        SHADER,
        "main",
        &[
            Binding::Read(&bytes(outer)),
            Binding::Read(&bytes(inner)),
            Binding::Out((r.out.0 * r.out.1 * 4) as u64),
            Binding::Read(&lut),
            Binding::Uniform(&params),
        ],
        ((r.out.0 as u32).div_ceil(8), (r.out.1 as u32).div_ceil(8)),
    )?;
    Ok((name, words(&out)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyframe::{needed_rect, resample};
    use fd_shade::Rgb8;

    /// A smooth gradient with per-texel hash noise, so a misplaced tap or weight shows.
    fn image(w: usize, h: usize, seed: usize) -> Vec<[u8; 3]> {
        (0..w * h)
            .map(|i| {
                let (x, y) = (i % w, i / w);
                let n = (i * 2_654_435_761 + seed) >> 7;
                [
                    ((x * 255 / w) as u8).wrapping_add((n & 15) as u8),
                    ((y * 255 / h) as u8).wrapping_add((n >> 4 & 15) as u8),
                    ((n >> 8) & 255) as u8,
                ]
            })
            .collect()
    }

    fn crop(img: &[[u8; 3]], w: usize, rect: (usize, usize, usize, usize)) -> Rgb8 {
        let mut data = Vec::new();
        for y in rect.1..rect.1 + rect.3 {
            for x in rect.0..rect.0 + rect.2 {
                data.extend_from_slice(&img[y * w + x]);
            }
        }
        Rgb8 {
            w: rect.2,
            h: rect.3,
            data,
        }
    }

    /// Full-frame words with only `rect` holding the image; junk elsewhere, so a read
    /// outside the shaded rectangle cannot pass unnoticed.
    fn packed(img: &[[u8; 3]], w: usize, rect: (usize, usize, usize, usize)) -> Vec<u32> {
        img.iter()
            .enumerate()
            .map(|(i, c)| {
                let (x, y) = (i % w, i / w);
                let inside =
                    x >= rect.0 && x < rect.0 + rect.2 && y >= rect.1 && y < rect.1 + rect.3;
                if inside {
                    u32::from(c[0]) | u32::from(c[1]) << 8 | u32::from(c[2]) << 16
                } else {
                    0x00ff_00ff
                }
            })
            .collect()
    }

    #[test]
    fn wgsl_resample_matches_cpu() {
        let lut: [f32; 256] = std::array::from_fn(|v| (v as f32 / 255.0).powf(2.2));
        let out = (64usize, 36usize);
        let (outer_size, inner_size) = ((112usize, 63usize), (112usize, 63usize));
        let outer_img = image(outer_size.0, outer_size.1, 1);
        let inner_img = image(inner_size.0, inner_size.1, 7);
        let mut name = String::new();
        let (mut worst, mut blended, mut rotated_px) = (0u8, 0usize, 0usize);
        for (zo, at) in [
            (0.55f32, (0.0f32, 0.0f32)),
            (0.81, (0.11, -0.07)),
            (0.99, (-0.2, 0.15)),
        ] {
            let zi = 2.0 * zo;
            let inner_at = (at.0 * 2.0, at.1 * 2.0);
            for angle in [
                0.0f32,
                0.3,
                std::f32::consts::FRAC_PI_4,
                -1.2,
                std::f32::consts::FRAC_PI_2,
            ] {
                for (ramp, margin) in [(1.0f32, 0.1f32), (0.6, 0.03)] {
                    let (sin, cos) = angle.sin_cos();
                    let ro = needed_rect(out, outer_size, zo, at.0, at.1, sin, cos);
                    let ri = needed_rect(out, inner_size, zi, inner_at.0, inner_at.1, sin, cos);
                    let r = Resample {
                        out,
                        sin,
                        cos,
                        zoom: (zo, zi),
                        outer_at: at,
                        inner_at,
                        ramp,
                        margin,
                        outer: (outer_size, ro),
                        inner: (inner_size, ri),
                    };
                    let shaded = [
                        crop(&outer_img, outer_size.0, ro),
                        crop(&inner_img, inner_size.0, ri),
                    ];
                    let mut cpu = Rgb8 {
                        w: out.0,
                        h: out.1,
                        data: vec![0; out.0 * out.1 * 3],
                    };
                    resample(&r, &shaded, &lut, &mut cpu);
                    let (adapter, gpu) = run(
                        &r,
                        &packed(&outer_img, outer_size.0, ro),
                        &packed(&inner_img, inner_size.0, ri),
                        &lut,
                    )
                    .unwrap_or_else(|e| panic!("wgsl resample: {e}"));
                    name = adapter;
                    for (i, &word) in gpu.iter().enumerate() {
                        let (px, py) = (i % out.0, i / out.0);
                        for c in 0..3 {
                            let got = (word >> (8 * c)) as u8;
                            let want = cpu.data[i * 3 + c];
                            worst = worst.max(got.abs_diff(want));
                            assert!(
                                got.abs_diff(want) <= 1,
                                "zo {zo} angle {angle} ramp {ramp} at ({px},{py}) channel {c}: CPU {want}, WGSL {got}"
                            );
                        }
                        // Count pixels where both keyframes contribute, to show the blend ran.
                        let xn = (px as f32 + 0.5) / out.0 as f32 - 0.5;
                        let yn = (py as f32 + 0.5) / out.1 as f32 - 0.5;
                        let aspect = out.0 as f32 / out.1 as f32;
                        let (rx, ry) = (cos * xn + sin * yn / aspect, cos * yn - sin * xn * aspect);
                        let d = (2.0 * (rx * zi + inner_at.0).abs())
                            .max(2.0 * (ry * zi + inner_at.1).abs());
                        let wi = ramp * ((1.0 - d) / margin).clamp(0.0, 1.0);
                        if wi > 0.0 && wi < 1.0 {
                            blended += 1;
                        }
                        if angle != 0.0 {
                            rotated_px += 1;
                        }
                    }
                }
            }
        }
        eprintln!(
            "adapter {name}: worst {worst} level(s); {blended} blended pixels, {rotated_px} rotated-frame pixels"
        );
        assert!(blended > 0 && rotated_px > 0);
    }
}
