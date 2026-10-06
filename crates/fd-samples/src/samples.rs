//! In-memory columns. A column is `None` when it was not requested: nothing is
//! computed, stored or read for it.
use crate::class::Class;
use crate::column::{Column, ColumnSet};

/// One vector per present column, each `nx * ny` long (see `Column` for meanings).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Samples {
    /// Always present.
    pub class: Vec<Class>,
    /// Smooth escape value.
    pub nu: Option<Vec<f64>>,
    /// Distance estimate, output pixels.
    pub de: Option<Vec<f32>>,
    /// Screen-space normal angle.
    pub normal: Option<Vec<u16>>,
    /// Error bound on `nu`.
    pub bound: Option<Vec<f32>>,
}

impl Samples {
    /// Zeroed storage for `n` samples with exactly the columns in `set`.
    pub fn alloc(n: usize, set: ColumnSet) -> Samples {
        Samples {
            class: vec![Class(0); n],
            nu: set.has(Column::Nu).then(|| vec![0.0; n]),
            de: set.has(Column::De).then(|| vec![0.0; n]),
            normal: set.has(Column::Normal).then(|| vec![0; n]),
            bound: set.has(Column::Bound).then(|| vec![0.0; n]),
        }
    }

    /// The set of columns present.
    pub fn columns(&self) -> ColumnSet {
        let mut s = ColumnSet::of(&[Column::Class]);
        for (c, on) in [
            (Column::Nu, self.nu.is_some()),
            (Column::De, self.de.is_some()),
            (Column::Normal, self.normal.is_some()),
            (Column::Bound, self.bound.is_some()),
        ] {
            if on {
                s = s.with(c);
            }
        }
        s
    }

    /// Normal angle as a unit vector `(x, y)` in screen orientation (y down).
    #[inline]
    pub fn unit(angle: u16) -> (f32, f32) {
        let (s, c) = (angle as f32 * (std::f32::consts::TAU / 65536.0)).sin_cos();
        (c, s)
    }

    /// Quantise a screen-space direction to the stored angle.
    #[inline]
    pub fn angle(x: f64, y: f64) -> u16 {
        let t = y.atan2(x) * (65536.0 / std::f64::consts::TAU);
        t.round() as i64 as u16 // wraps negative turns into [0, 65536)
    }
}
