//! Pass 2: lit relief from the normal, darkened near the set by the distance
//! estimate. Reads `Class`, `De` and `Normal` only: never touches `Nu`.
use crate::resolve::{resolve, Rgb8};
use crate::Pass;
use fd_samples::{Column, ColumnSet, Header, Kind, Samples};

pub(crate) struct Relief;

/// Light from the upper left, 45 degrees above the surface (screen y points down).
const LIGHT: [f32; 3] = [-0.5, -0.5, std::f32::consts::FRAC_1_SQRT_2];
const HEIGHT: f32 = 1.5;

impl Pass for Relief {
    fn columns(&self) -> ColumnSet {
        ColumnSet::of(&[Column::Class, Column::De, Column::Normal])
    }

    fn shade(&self, h: &Header, s: &Samples) -> Rgb8 {
        let de = s.de.as_deref().expect("relief pass needs the De column");
        let nm = s.normal.as_deref().expect("relief pass needs the Normal column");
        resolve(h, |i| match s.class[i].kind() {
            Some(Kind::Escaped) => {
                let (x, y) = Samples::unit(nm[i]);
                let k = 1.0 / (1.0 + HEIGHT * HEIGHT).sqrt();
                let lit = ((x * LIGHT[0] + y * LIGHT[1]) * k + HEIGHT * k * LIGHT[2]).clamp(0.0, 1.0);
                let edge = (de[i] * 0.5).tanh();
                let v = (0.15 + 0.85 * lit) * edge;
                [v, v * 0.96, v * 0.9]
            }
            Some(Kind::Interior) => [0.0; 3],
            _ => [0.25, 0.0, 0.25],
        })
    }
}
