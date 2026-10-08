//! The default look: a muted umber palette from `Nu`, lit by soft, wide-band relief
//! from `Normal` and `De`. Reads `Class`, `Nu`, `De` and `Normal`; no fractal math.
use crate::resolve::{resolve, Rgb8};
use crate::{Appearance, Pass};
use fd_samples::{Column, ColumnSet, Header, Kind, Samples};

pub(crate) struct Umber;

/// Linear-light stops: dark umber, raw umber, warm ochre. No white or grey.
const DARK: [f32; 3] = [0.035, 0.020, 0.010];
const MID: [f32; 3] = [0.20, 0.11, 0.05];
const LIGHT: [f32; 3] = [0.36, 0.22, 0.10];
/// Interior colour (and unresolved, when drawn as interior).
const INTERIOR: [f32; 3] = [0.012, 0.007, 0.004];
/// Bands per natural-log unit of `nu`: low, so bands stay wide.
const BANDS: f32 = 0.3;
/// Light from the upper left, high in the sky, so the relief stays soft.
const SUN: [f32; 3] = [-0.42, -0.42, 0.80];
/// Surface height scale: larger is flatter, so the shading reads as broad swells.
const HEIGHT: f32 = 3.0;

impl Pass for Umber {
    fn columns(&self) -> ColumnSet {
        ColumnSet::of(&[Column::Class, Column::Nu, Column::De, Column::Normal])
    }

    fn shade_with(&self, h: &Header, s: &Samples, a: &Appearance) -> Rgb8 {
        let nu = s.nu.as_deref().expect("umber look needs the Nu column");
        let de = s.de.as_deref().expect("umber look needs the De column");
        let nm = s.normal.as_deref().expect("umber look needs the Normal column");
        let k = 1.0 / (1.0 + HEIGHT * HEIGHT).sqrt();
        let (bands, phase) = (BANDS * a.density(), a.phase());
        resolve(h, a.dither, |i| match s.class[i].kind() {
            Some(Kind::Escaped) => {
                let w = 0.5 - 0.5 * (std::f32::consts::TAU * (bands * nu[i].max(1.0).ln() as f32 + phase)).cos();
                let base = mix(mix(DARK, MID, (2.0 * w).min(1.0)), LIGHT, (2.0 * w - 1.0).max(0.0));
                let (x, y) = Samples::unit(nm[i]);
                let flat = HEIGHT * k * SUN[2];
                let lit = ((x * SUN[0] + y * SUN[1]) * k + flat).clamp(0.0, 1.0);
                let lit = if a.aa { flat + (lit - flat) * a.relief_gain(de[i], h.ss) } else { lit };
                let edge = 0.45 + 0.55 * (de[i] * 0.35).tanh();
                let v = (0.55 + 0.6 * lit) * edge;
                base.map(|c| c * v)
            }
            Some(Kind::Interior) => INTERIOR,
            _ if a.unresolved_interior => INTERIOR,
            // Uncertainty stays visible instead of masquerading as interior.
            _ => [0.25, 0.0, 0.25],
        })
    }
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [0, 1, 2].map(|j| a[j] + (b[j] - a[j]) * t)
}
