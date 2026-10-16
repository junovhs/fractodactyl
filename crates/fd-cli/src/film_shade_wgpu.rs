//! Studio shade pass on wgpu (SHAD-04/05): runs film_shade.wgsl alone (FilmGpu runs it
//! in the film path), for tests against fd-shade's CPU shading.

use crate::film_gpu::{ShadePush, SHADE as SHADER};
use crate::wgpu_compute::{bytes, dispatch, words, Binding};

/// Shade the push block's rectangle of `cooked` (the prepare pass output, full frame)
/// with linear-light `stops`. Returns the adapter name and the full-frame packed
/// RGB words (`r | g << 8 | b << 16`); words outside the rectangle are zero.
pub(crate) fn run(
    cooked: &[[u32; 8]],
    stops: &[[f32; 3]],
    push: &ShadePush,
    full: (usize, usize),
    rect: (usize, usize),
) -> Result<(String, Vec<u32>), String> {
    let count = full.0 * full.1;
    if count == 0 || cooked.len() != count {
        return Err(format!(
            "expected {count} cooked samples, got {}",
            cooked.len()
        ));
    }
    if !(2..=64).contains(&stops.len()) {
        return Err(format!(
            "{} stops; the shade pass takes 2..=64",
            stops.len()
        ));
    }
    let input = bytes(cooked.as_flattened());
    let palette: Vec<u8> = stops
        .iter()
        .flat_map(|c| c.iter().chain(&[0.0]).flat_map(|v| v.to_ne_bytes()))
        .collect();
    let (name, out) = dispatch(
        "fd studio shade",
        SHADER,
        "main",
        &[
            Binding::Read(&input),
            Binding::Out(count as u64 * 4),
            Binding::Read(&palette),
            Binding::Uniform(&push.bytes()),
        ],
        ((rect.0 as u32).div_ceil(8), (rect.1 as u32).div_ceil(8)),
    )?;
    Ok((name, words(&out)))
}

/// Run the shader's band select alone: each case is (t low word, t high word, phase
/// bits, stop count); returns the bits of `((t + phase).rem_euclid(1.0) * n) as f32`.
pub(crate) fn band_select(cases: &[[u32; 4]]) -> Result<(String, Vec<u32>), String> {
    let input = bytes(cases.as_flattened());
    let (name, out) = dispatch(
        "fd studio band select",
        SHADER,
        "band_select",
        &[Binding::Read(&input), Binding::Out(cases.len() as u64 * 4)],
        ((cases.len() as u32).div_ceil(64), 1),
    )?;
    Ok((name, words(&out)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::film_gpu::{linear, pack_samples, shade_push};
    use crate::film_prepare_wgpu as prepare;
    use crate::wgpu_compute::or_skip;
    use fd_samples::{Class, Column, ColumnSet, Evidence, Header, Kind, Samples, View, MINOR};
    use fd_shade::{Appearance, Look, PreparedStudio, Rgb8};

    /// PreparedStudio's palette coordinate, the f64 expression the shader emulates.
    fn cpu_x(t: f64, phase: f32, n: u32) -> f32 {
        ((t + f64::from(phase)).rem_euclid(1.0) * f64::from(n)) as f32
    }

    /// Same f32 value, or both NaN, or a CPU f32 subnormal the shader flushes to zero.
    fn same(cpu: f32, gpu: u32) -> bool {
        cpu.to_bits() == gpu
            || (cpu.is_nan() && f32::from_bits(gpu).is_nan())
            || (cpu.abs() < f32::MIN_POSITIVE && gpu & 0x7fff_ffff == 0)
    }

    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
    }

    /// Adversarial (t, phase, n): t on every stop edge and whole-cycle wrap, nudged a
    /// few f64 ulps either side, at integer parts from 0 to 2^52 and negative; t at
    /// random magnitudes 2^-60..2^60; and the special values.
    fn band_cases() -> Vec<(f64, f32, u32)> {
        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        let stops = [2u32, 3, 5, 7, 8, 12, 16, 31, 32];
        let mut phases = vec![0.0f32, 1.0, 0.5, 1e-30, f32::MIN_POSITIVE, 0.999_999_94];
        phases.push(
            Appearance {
                time: 2.0 / 30.0,
                flow: 0.11,
                ..Appearance::STILL
            }
            .phase(),
        );
        for _ in 0..25 {
            phases.push((rng.next() >> 40) as f32 / (1u64 << 24) as f32);
        }
        let wholes = [
            0.0,
            1.0,
            7.0,
            2048.0,
            1e6,
            (1u64 << 40) as f64,
            ((1u64 << 52) - 3) as f64,
            -1.0,
            -3.0,
            -2048.0,
        ];
        let mut cases = Vec::new();
        for &n in &stops {
            for &phase in &phases {
                for &whole in &wholes {
                    for j in 0..=n {
                        let edge = whole + f64::from(j) / f64::from(n) - f64::from(phase);
                        for step in -3i64..=3 {
                            let t = f64::from_bits((edge.to_bits() as i64 + step) as u64);
                            cases.push((t, phase, n));
                        }
                    }
                }
                for _ in 0..64 {
                    let mantissa = (rng.next() >> 11) as f64 / (1u64 << 53) as f64 + 0.5;
                    let exp = rng.below(121) as i32 - 60;
                    let sign = if rng.below(4) == 0 { -1.0 } else { 1.0 };
                    cases.push((sign * mantissa * 2f64.powi(exp), phase, n));
                }
            }
        }
        // Rounding ties of t + phase: phase sits exactly half a t-ulp off t's grid (ties
        // to even) or one f32 ulp either side of that (the sticky bits decide), for t of
        // both signs and ulps from 2^-23 to 2^-1.
        for _ in 0..20_000 {
            let ulp_exp = -(rng.below(23) as i32) - 1;
            let sign = if rng.below(2) == 0 { -1.0 } else { 1.0 };
            let whole = 2f64.powi(52 + ulp_exp) + rng.below(1 << 20) as f64;
            let half_steps = 2 * rng.below(1 << (-ulp_exp - 1).min(20) as u64) + 1;
            let tie = half_steps as f64 * 2f64.powi(ulp_exp - 1);
            let phase = (tie as f32).min(0.999_999_94);
            let nudge = [0.0, 1.0, -1.0][rng.below(3) as usize];
            let phase = f32::from_bits((phase.to_bits() as i64 + nudge as i64) as u32);
            let t = sign * (whole + rng.below(1 << 10) as f64 * 2f64.powi(ulp_exp));
            cases.push((t, phase, stops[rng.below(stops.len() as u64) as usize]));
        }
        // Double rounding of r * n: r = fl64(y / n) for y on a 24-bit tie, so r * n is a
        // hair off the tie and only the intermediate f64 rounding lands it on it.
        for _ in 0..20_000 {
            let n = [3u32, 5, 7, 12, 31][rng.below(5) as usize];
            let m = (1u64 << 23) | rng.below(1 << 23);
            let scale = 2f64.powi(-(rng.below(30) as i32) - 24);
            let y = (2 * m + 1) as f64 * scale;
            let r = y / f64::from(n);
            if r < 1.0 {
                cases.push((r, 0.0, n));
                cases.push((r - 1.0, 0.0, n));
            }
        }
        for t in [
            0.0,
            -0.0,
            f64::from_bits(1),
            f64::MIN_POSITIVE,
            1e-300,
            -1e-300,
            f64::MAX,
            f64::MIN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ] {
            for &n in &stops {
                for &phase in &phases[..7] {
                    cases.push((t, phase, n));
                }
            }
        }
        cases
    }

    fn encode_case(t: f64, phase: f32, n: u32) -> [u32; 4] {
        let bits = t.to_bits();
        [bits as u32, (bits >> 32) as u32, phase.to_bits(), n]
    }

    #[test]
    fn wgsl_shade_band_select_matches_f64() {
        let cases = band_cases();
        let encoded: Vec<[u32; 4]> = cases
            .iter()
            .map(|&(t, phase, n)| encode_case(t, phase, n))
            .collect();
        let Some((name, got)) = or_skip(band_select(&encoded), "band select") else {
            return;
        };
        eprintln!("adapter {name}");
        // How many cases an f32 shortcut gets wrong: proof the fixture bites.
        let mut naive_wrong = 0;
        for (&(t, phase, n), &g) in cases.iter().zip(&got) {
            let cpu = cpu_x(t, phase, n);
            assert!(
                same(cpu, g),
                "t {t:e} ({:#018x}) phase {phase:e} n {n}: cpu {cpu:e} ({:#010x}), wgsl {:e} ({g:#010x})",
                t.to_bits(),
                cpu.to_bits(),
                f32::from_bits(g),
            );
            let naive = ((t.rem_euclid(1.0) as f32 + phase).fract() * n as f32) as u32;
            if t.is_finite() && naive.min(n - 1) != (cpu as u32).min(n - 1) {
                naive_wrong += 1;
            }
        }
        eprintln!(
            "band select: {} cases bit-exact; an f32 shortcut picks the wrong band on {naive_wrong}",
            cases.len()
        );
        assert!(naive_wrong > 0, "fixture never reaches a band edge");
    }

    /// Escaped samples on stop edges, plus interior and unresolved ones; varied distance
    /// (so AA band weights and relief span their ranges) and normal.
    fn fixture(look: &Look, phase: f64, w: usize, h: usize) -> (Header, Samples) {
        let columns = ColumnSet::of(&[Column::Class, Column::Nu, Column::De, Column::Normal]);
        let mut s = Samples::alloc(w * h, columns);
        let n = look.stops.len() as f64;
        let offsets = [
            -2e-7, -5e-8, -1e-8, -1e-10, -1e-13, 0.0, 1e-13, 1e-10, 1e-8, 5e-8,
        ];
        for i in 0..w * h {
            let kind = if i % 53 == 0 {
                Kind::Interior
            } else if i % 59 == 0 {
                Kind::Unresolved
            } else {
                Kind::Escaped
            };
            s.class[i] = Class::new(kind, Evidence::Heuristic);
            // Odd samples sit mid-band, so the blend and terrace curves are seen whole.
            let edge = if i % 2 == 0 {
                (i % (look.stops.len() + 1)) as f64 / n + offsets[i % offsets.len()]
            } else {
                (i.wrapping_mul(2_654_435_761) % 1_000_003) as f64 / 1_000_003.0
            };
            let t = 2048.0 + edge - phase;
            s.nu.as_mut().unwrap()[i] = t / look.density;
            s.de.as_mut().unwrap()[i] = 0.01 * ((i % w) as f32 / 4.0).exp2();
            s.normal.as_mut().unwrap()[i] = (i * 1373) as u16;
        }
        let header = Header {
            minor: MINOR,
            columns,
            nx: w as u32,
            ny: h as u32,
            ss: 1,
            max_iter: 100_000,
            escape_radius: 1e10,
            view: View {
                center_re: "0".into(),
                center_im: "0".into(),
                width: "1".into(),
                rotation: 0.0,
            },
            kernel: "test".into(),
        };
        (header, s)
    }

    #[test]
    fn wgsl_shade_matches_prepared_studio() {
        let mut look = Look::parse(include_str!("../../../looks/master-ice.look")).unwrap();
        // Haze is SHAD-05's; terrace stays, so a wrong band is a visible jump.
        look.haze = None;
        assert!(look.terrace > 0.0 && look.lines == 0.0);
        let (w, h) = (64usize, 48usize);
        for aa in [false, true] {
            let a = Appearance {
                time: 2.0 / 30.0,
                flow: 0.11,
                aa,
                dither: true,
                unresolved_interior: false,
                ..Appearance::STILL
            };
            let phase = a.phase();
            let (header, samples) = fixture(&look, f64::from(phase), w, h);
            let mut packed = vec![0u32; w * h * 4];
            pack_samples(&samples, &mut packed).unwrap();
            let packed: Vec<[u32; 4]> = packed
                .chunks_exact(4)
                .map(|c| [c[0], c[1], c[2], c[3]])
                .collect();
            let Some((name, cooked)) =
                or_skip(prepare::run(&packed, (w, h), &look, &a), "wgsl prepare")
            else {
                return;
            };
            let stops: Vec<[f32; 3]> = look.stops.iter().map(|&c| linear(c)).collect();
            let push = shade_push(&look, &a, (w, h), (0, 0, w, h));
            let (_, gpu) = run(&cooked, &stops, &push, (w, h), (w, h)).unwrap();
            eprintln!("adapter {name}, aa {aa}");

            // Band index: exact on every escaped sample.
            let n = stops.len() as u32;
            let cases: Vec<[u32; 4]> = cooked
                .iter()
                .map(|s| [s[0], s[1], phase.to_bits(), n])
                .collect();
            let (_, xs) = band_select(&cases).unwrap();
            let mut edges = 0;
            for (i, s) in cooked.iter().enumerate() {
                if samples.class[i].kind() != Some(Kind::Escaped) {
                    continue;
                }
                let t = f64::from_bits(u64::from(s[0]) | u64::from(s[1]) << 32);
                let cpu = cpu_x(t, phase, n);
                let k_cpu = (cpu as u32).min(n - 1);
                let k_gpu = (f32::from_bits(xs[i]) as u32).min(n - 1);
                assert_eq!(k_gpu, k_cpu, "sample {i}: band {k_gpu}, CPU band {k_cpu}");
                if cpu.fract() > 0.999 || cpu.fract() < 0.001 {
                    edges += 1;
                }
            }

            // Packed RGB: within one level of PreparedStudio (GPU cos/pow/exp are not
            // correctly rounded); a wrong band at a terraced edge is tens of levels.
            let prepared = PreparedStudio::new(&look, &header, &samples, &a).unwrap();
            let mut cpu = Rgb8 {
                w: 0,
                h: 0,
                data: Vec::new(),
            };
            prepared.shade_rect_film_into((0, 0, w, h), &a, &mut cpu);
            let mut worst = 0;
            for (i, &word) in gpu.iter().enumerate() {
                for c in 0..3 {
                    let got = (word >> (8 * c)) as u8;
                    let want = cpu.data[i * 3 + c];
                    worst = worst.max(got.abs_diff(want));
                    assert!(
                        got.abs_diff(want) <= 1,
                        "pixel ({},{}) channel {c}: CPU {want}, WGSL {got}",
                        i % w,
                        i / w
                    );
                }
            }
            eprintln!(
                "aa {aa}: {} pixels within {worst} level(s); {edges} samples within 0.001 of a stop edge",
                w * h
            );
        }
    }
}
