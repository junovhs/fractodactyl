//! Studio prepare pass on wgpu (SHAD-03): the WGSL port of `film_prepare.comp`, plus
//! the CPU mirror of that GLSL math the port is checked against. Test-only until the
//! film GPU path is wired to it.

use crate::wgpu_compute::{bytes, dispatch, words, Binding};

const SHADER: &str = include_str!("../shaders/film_prepare.wgsl");
const LN_2: f32 = std::f32::consts::LN_2;
// -2 * (pi * sigma)^2 with sigma = 0.5, as in film_prepare.comp.
const BAND_K: f32 = -2.0 * (std::f32::consts::PI * 0.5) * (std::f32::consts::PI * 0.5);

/// Frame and look terms the prepare pass reads; mirrors `Params` in the shader.
#[derive(Clone, Copy)]
pub(crate) struct Params {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) density: f32,
    pub(crate) slope: f32,
    pub(crate) light: [f32; 2],
    pub(crate) aa: bool,
}

impl Params {
    /// The uniform block: four vec4s, 64 bytes, std140 layout.
    fn uniform(&self) -> Vec<u8> {
        let dims = [self.width, self.height, 0, 0];
        let tone = [0.0, self.density, 0.0, self.slope];
        let light = [self.light[0], self.light[1], 0.0, 0.0];
        let extra = [0.0, 0.0, if self.aa { 1.0 } else { 0.0 }, 0.0];
        let mut out = Vec::with_capacity(64);
        for w in dims {
            out.extend(w.to_ne_bytes());
        }
        for f in tone.into_iter().chain(light).chain(extra) {
            out.extend(f.to_ne_bytes());
        }
        out
    }
}

/// Run the WGSL prepare pass on the wgpu Vulkan adapter. Returns the adapter name and
/// the cooked words.
pub(crate) fn run(samples: &[[u32; 4]], p: &Params) -> Result<(String, Vec<[u32; 4]>), String> {
    let count = p.width as usize * p.height as usize;
    if count == 0 || samples.len() != count {
        return Err(format!("expected {count} samples, got {}", samples.len()));
    }
    let input = bytes(samples.as_flattened());
    let (name, out) = dispatch(
        "fd studio prepare",
        SHADER,
        "main",
        &[
            Binding::Read(&input),
            Binding::Out(count as u64 * 16),
            Binding::Uniform(&p.uniform()),
        ],
        (p.width.div_ceil(8), p.height.div_ceil(8)),
    )?;
    let cooked = words(&out)
        .chunks_exact(4)
        .map(|c| [c[0], c[1], c[2], c[3]])
        .collect();
    Ok((name, cooked))
}

/// CPU mirror of `film_prepare.comp`, in the same f32 operation order. The reference
/// the WGSL pass is compared against.
pub(crate) fn reference(samples: &[[u32; 4]], p: &Params) -> Vec<[u32; 4]> {
    samples
        .iter()
        .map(|s| {
            let kind = s[3] >> 16;
            if kind != 0 {
                return [s[0], s[1], (-(kind as f32)).to_bits(), 0.0f32.to_bits()];
            }
            let d = f32::from_bits(s[2]).max(0.0);
            let angle = (s[3] & 65535) as f32 * (std::f32::consts::TAU / 65536.0);
            let facing = -(angle.cos() * p.light[0] + angle.sin() * p.light[1]);
            let cycles = p.density * 2.0 / (d.max(1e-30) * LN_2);
            let aa = p.aa;
            let band = if aa {
                (BAND_K * cycles * cycles).exp()
            } else {
                1.0
            };
            let steep = (0.6 * (1.0 + cycles / 0.02).ln()).tanh();
            let fade = if aa { (d / 0.7).tanh() } else { 1.0 };
            let v = (1.0 + p.slope * facing * steep * fade) * (1.0 - 0.6 * (-d * 1.5).exp());
            [s[0], s[1], band.to_bits(), v.to_bits()]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Largest float-bit distance, in ulps, the WGSL pass may differ from the reference.
    /// GPU exp/tanh/sin/cos are not correctly rounded, and exp's error grows with its
    /// argument: on RADV NAVI10 the worst is 6 ulp without AA and 17 with it, while a
    /// 1% error in any shader constant costs 800+ ulp. Near underflow (band weights
    /// ~1e-37) relative error grows without bound, so differences below an absolute
    /// floor far under one 16-bit output step are not counted.
    const MAX_ULP: u32 = 32;
    const ABS_FLOOR: f32 = 1.0 / 16_777_216.0;

    /// Interior, unresolved and escaped samples over a grid, with varied distance and angle.
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
                    [
                        i.wrapping_mul(2_654_435_761),
                        i ^ 0x5bd1_e995,
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

    #[test]
    fn wgsl_prepare_matches_reference() {
        let (w, h) = (256, 256);
        let samples = fixture(w, h);
        for aa in [false, true] {
            let p = Params {
                width: w,
                height: h,
                density: 3.7,
                slope: 0.45,
                light: [0.6, 0.8],
                aa,
            };
            let (name, gpu) = run(&samples, &p).unwrap_or_else(|e| panic!("wgsl prepare: {e}"));
            let cpu = reference(&samples, &p);
            eprintln!("adapter {name}, aa {aa}");
            let mut worst = 0;
            for (i, (g, c)) in gpu.iter().zip(&cpu).enumerate() {
                assert_eq!(&g[..2], &c[..2], "pixel {i}: copied phase words differ");
                for k in [2, 3] {
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
            eprintln!("aa {aa}: worst {worst} ulp over {} samples", gpu.len());
        }
    }
}
