//! Pass 1: hue from the smooth escape value. Reads `Class` and `Nu` only.
use crate::resolve::{resolve, Rgb8};
use crate::{Appearance, Pass};
use fd_samples::{Column, ColumnSet, Header, Kind, Samples};

pub(crate) struct Palette;

impl Pass for Palette {
    fn columns(&self) -> ColumnSet {
        ColumnSet::of(&[Column::Class, Column::Nu])
    }

    fn shade_with(&self, h: &Header, s: &Samples, a: &Appearance) -> Rgb8 {
        let nu = s.nu.as_deref().expect("palette pass needs the Nu column");
        let (bands, phase, hue) = (0.35 * a.density(), a.phase(), a.hue());
        resolve(h, |i| match s.class[i].kind() {
            Some(Kind::Escaped) => {
                // Cyclic cosine palette over ln(nu): scale-free banding density.
                let t = bands * nu[i].max(1.0).ln() as f32 + phase;
                let ch = |p: f32| 0.5 + 0.5 * (std::f32::consts::TAU * (t + p)).cos();
                [ch(0.0), ch(0.15 + 0.37 * hue), ch(0.35 + 0.71 * hue)].map(|v| v * v)
            }
            Some(Kind::Interior) => [0.0; 3],
            _ if a.unresolved_interior => [0.0; 3],
            // Uncertainty stays visible instead of masquerading as interior.
            _ => [0.25, 0.0, 0.25],
        })
    }
}
