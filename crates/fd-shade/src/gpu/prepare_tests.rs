//! Studio prepare pass on wgpu (SHAD-03/05): runs film_prepare.wgsl alone, plus the CPU
//! mirror of fd-shade Studio's per-sample terms it is checked against.

use super::testkit::{bytes, dispatch, words, Binding};
use super::{shade_push, PREPARE};
use crate::{Appearance, Look};

const LN_2: f32 = std::f32::consts::LN_2;
const BAND_K: f32 = -2.0 * (std::f32::consts::PI * 0.5) * (std::f32::consts::PI * 0.5);

/// Run the WGSL prepare pass over a `w` x `h` keyframe of packed raw samples with
/// look `l` at appearance `a`. Returns the adapter name and two cooked words per sample.
pub(super) fn run(
    samples: &[[u32; 4]],
    (w, h): (usize, usize),
    l: &Look,
    a: &Appearance,
) -> Result<(String, Vec<[u32; 8]>), String> {
    if w * h == 0 || samples.len() != w * h {
        return Err(format!("expected {} samples, got {}", w * h, samples.len()));
    }
    let input = bytes(samples.as_flattened());
    let push = shade_push(l, a, (w, h), (0, 0, w, h)).bytes();
    let (name, out) = dispatch(
        "fd studio prepare",
        PREPARE,
        "main",
        &[
            Binding::Read(&input),
            Binding::Out((w * h * 32) as u64),
            Binding::Uniform(&push),
        ],
        ((w as u32).div_ceil(8), (h as u32).div_ceil(8)),
    )?;
    let cooked = words(&out)
        .chunks_exact(8)
        .map(|c| std::array::from_fn(|k| c[k]))
        .collect();
    Ok((name, cooked))
}

/// CPU mirror of fd-shade Studio's per-sample terms at one sample per pixel, in its f32
/// operation order: t = density * nu (f64), band weight, light, haze weight and tone,
/// cycles.
pub(super) fn reference(samples: &[[u32; 4]], l: &Look, a: &Appearance) -> Vec<[u32; 8]> {
    let density = l.density * f64::from(a.density());
    let (ls, lc) = l.light.to_radians().sin_cos();
    let (lx, ly) = (lc, -ls);
    samples
        .iter()
        .map(|s| {
            let kind = s[3] >> 16;
            if kind != 0 {
                return [0, 0, (-(kind as f32)).to_bits(), 0, 0, 0, 0, 0];
            }
            let nu = f64::from_bits(u64::from(s[0]) | u64::from(s[1]) << 32);
            let t = (density * nu).to_bits();
            let d = f32::from_bits(s[2]).max(0.0);
            let angle = (s[3] & 65535) as f32 * (std::f32::consts::TAU / 65536.0);
            let facing = -(angle.cos() * lx + angle.sin() * ly);
            let cycles = density as f32 * 2.0 / (d.max(1e-30) * LN_2);
            let band = if a.aa {
                (BAND_K * cycles * cycles).exp()
            } else {
                1.0
            };
            let steep = (0.6 * (1.0 + cycles / 0.02).ln()).tanh();
            let fade = if a.aa { (d / 0.7).tanh() } else { 1.0 };
            let v = (1.0 + l.slope * facing * steep * fade) * (1.0 - 0.6 * (-d * 1.5).exp());
            let (weight, tone) =
                if l.haze.is_some() && d > 0.0 && d.is_finite() && cycles.is_finite() {
                    let e = (cycles - 1.0).clamp(0.0, 1.0);
                    (e * e * (3.0 - 2.0 * e), 0.2 + 2.8 * d / (d + 0.02))
                } else {
                    (0.0, 0.0)
                };
            [
                t as u32,
                (t >> 32) as u32,
                band.to_bits(),
                v.to_bits(),
                weight.to_bits(),
                tone.to_bits(),
                cycles.to_bits(),
                0,
            ]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::testkit::or_skip;
    use super::*;

    /// Largest float-bit distance, in ulps, the WGSL pass may differ from the reference.
    /// GPU exp/tanh/sin/cos are not correctly rounded, and exp's error grows with its
    /// argument: on RADV NAVI10 the worst is 6 ulp without AA and 17 with it, while a
    /// 1% error in any shader constant costs 800+ ulp. Near underflow (band weights
    /// ~1e-37) relative error grows without bound, so differences below an absolute
    /// floor far under one 16-bit output step are not counted.
    const MAX_ULP: u32 = 32;
    const ABS_FLOOR: f32 = 1.0 / 16_777_216.0;

    /// Interior, unresolved and escaped samples over a grid: varied distance and angle,
    /// and nu from arbitrary f64 bit patterns (huge, tiny, subnormal, infinite, NaN)
    /// alongside realistic smooth iteration counts.
    fn fixture(w: u32, h: u32) -> Vec<[u32; 4]> {
        (0..h)
            .flat_map(|y| {
                (0..w).map(move |x| {
                    let i = y * w + x;
                    let kind = if (x + y) % 17 == 0 {
                        1
                    } else if (3 * x + y) % 29 == 0 {
                        2
                    } else {
                        0
                    };
                    // 0.01..~650 geometrically, so band weights span 0..1 rather than
                    // all underflowing, and the near-zero fade and dimming terms are hit.
                    let d = 0.01 * (x as f32 / 16.0 + y as f32 / 4096.0).exp2();
                    let angle = (x * 37 + y * 91) & 0xffff;
                    let nu = if i.is_multiple_of(2) {
                        f64::from_bits(
                            u64::from(i.wrapping_mul(2_654_435_761))
                                | u64::from(i ^ 0x5bd1_e995) << 32,
                        )
                    } else {
                        20.0 + f64::from(i) * 0.0137
                    };
                    let nu = nu.to_bits();
                    [
                        nu as u32,
                        (nu >> 32) as u32,
                        d.to_bits(),
                        (kind << 16) | angle,
                    ]
                })
            })
            .collect()
    }

    /// Distance in ulps between two non-negative float bit patterns. GPUs flush
    /// subnormals to zero, so both sides are flushed before comparing.
    fn ulp(a: u32, b: u32) -> u32 {
        let ftz = |x: u32| if x & 0x7f80_0000 == 0 { 0 } else { x };
        ftz(a).abs_diff(ftz(b))
    }

    /// The same f64, both NaN, or a CPU subnormal the shader flushed to zero.
    fn same_f64(gpu: u64, cpu: u64) -> bool {
        let (g, c) = (f64::from_bits(gpu), f64::from_bits(cpu));
        gpu == cpu || (g.is_nan() && c.is_nan()) || (c.abs() < f64::MIN_POSITIVE && g == 0.0)
    }

    #[test]
    fn wgsl_prepare_matches_reference() {
        let (w, h) = (256, 256);
        let samples = fixture(w as u32, h as u32);
        let mut look = Look::builtin("ice").unwrap();
        look.haze = Some(0x1e7fe0);
        look.slope = 0.45;
        for (aa, breathe) in [(false, 0.0), (true, 0.0), (true, 0.08)] {
            let a = Appearance {
                time: 0.7,
                breathe,
                brate: 0.5,
                aa,
                ..Appearance::STILL
            };
            let Some((name, gpu)) = or_skip(run(&samples, (w, h), &look, &a), "wgsl prepare")
            else {
                return;
            };
            let cpu = reference(&samples, &look, &a);
            eprintln!("adapter {name}, aa {aa}, breathe {breathe}");
            let mut worst = 0;
            for (i, (g, c)) in gpu.iter().zip(&cpu).enumerate() {
                let t = |w: &[u32; 8]| u64::from(w[0]) | u64::from(w[1]) << 32;
                assert!(
                    same_f64(t(g), t(c)),
                    "pixel {i}: t = density * nu: gpu {:e}, cpu {:e}",
                    f64::from_bits(t(g)),
                    f64::from_bits(t(c))
                );
                // The haze weight is ill-conditioned in cycles near one cycle per
                // sample (it uses cycles - 1): check its formula on the GPU's own cycles.
                let mut c = *c;
                if c[4] != 0 {
                    let e = (f32::from_bits(g[6]) - 1.0).clamp(0.0, 1.0);
                    c[4] = (e * e * (3.0 - 2.0 * e)).to_bits();
                }
                for k in 2..7 {
                    if (f32::from_bits(g[k]) - f32::from_bits(c[k])).abs() < ABS_FLOOR {
                        continue;
                    }
                    let u = ulp(g[k], c[k]);
                    worst = worst.max(u);
                    assert!(
                        u <= MAX_ULP,
                        "pixel {i} word {k}: gpu {:#010x} ({}), cpu {:#010x} ({}), {u} ulp",
                        g[k],
                        f32::from_bits(g[k]),
                        c[k],
                        f32::from_bits(c[k]),
                    );
                }
            }
            eprintln!(
                "aa {aa}: t bit-exact, worst {worst} ulp over {} samples",
                gpu.len()
            );
        }
    }
}
