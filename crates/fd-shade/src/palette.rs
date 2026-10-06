//! Pass 1: hue from the smooth escape value. Reads `Class` and `Nu` only.
use crate::resolve::{resolve, Rgb8};
use crate::Pass;
use fd_samples::{Column, ColumnSet, Header, Kind, Samples};

pub(crate) struct Palette;

impl Pass for Palette {
    fn columns(&self) -> ColumnSet {
        ColumnSet::of(&[Column::Class, Column::Nu])
    }

    fn shade(&self, h: &Header, s: &Samples) -> Rgb8 {
        let nu = s.nu.as_deref().expect("palette pass needs the Nu column");
        resolve(h, |i| match s.class[i].kind() {
            Some(Kind::Escaped) => {
                // Cyclic cosine palette over ln(nu): scale-free banding density.
                let t = 0.35 * nu[i].max(1.0).ln() as f32;
                let ch = |phase: f32| 0.5 + 0.5 * (std::f32::consts::TAU * (t + phase)).cos();
                [ch(0.0), ch(0.15), ch(0.35)].map(|v| v * v)
            }
            Some(Kind::Interior) => [0.0; 3],
            // Uncertainty stays visible instead of masquerading as interior.
            _ => [0.25, 0.0, 0.25],
        })
    }
}
