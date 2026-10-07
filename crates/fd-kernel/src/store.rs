//! Turning one sample's outcome into column values, at any depth.
use crate::sample::Outcome;
use crate::view::Plane;
use fd_fixed::exp2i;
use fd_samples::{Class, Evidence, Kind, Samples};

/// One row's slices of every present column.
pub(crate) struct Row<'a> {
    pub(crate) j: usize,
    pub(crate) class: &'a mut [Class],
    nu: Option<&'a mut [f64]>,
    de: Option<&'a mut [f32]>,
    normal: Option<&'a mut [u16]>,
}

impl<'a> Row<'a> {
    /// Split all columns into per-row slices.
    pub(crate) fn split(s: &'a mut Samples, nx: usize) -> Vec<Row<'a>> {
        let mut nu = s.nu.as_deref_mut().map(|v| v.chunks_mut(nx));
        let mut de = s.de.as_deref_mut().map(|v| v.chunks_mut(nx));
        let mut normal = s.normal.as_deref_mut().map(|v| v.chunks_mut(nx));
        (s.class.chunks_mut(nx).enumerate())
            .map(|(j, class)| Row {
                j,
                class,
                nu: nu.as_mut().and_then(Iterator::next),
                de: de.as_mut().and_then(Iterator::next),
                normal: normal.as_mut().and_then(Iterator::next),
            })
            .collect()
    }
}

/// Per-render constants for quantisation.
pub(crate) struct Store {
    plane: Plane,
    /// Output pixel size `px_m * 2^px_e` in the complex plane.
    px_m: f64,
    px_e: i64,
}

impl Store {
    pub(crate) fn new(plane: &Plane, ss: u32) -> Store {
        Store { plane: *plane, px_m: plane.h_m * ss as f64, px_e: plane.h_e }
    }

    #[inline]
    pub(crate) fn put(&self, row: &mut Row, i: usize, o: Outcome) {
        let kind = match o {
            Outcome::Escaped { .. } => Kind::Escaped,
            Outcome::Interior { .. } => Kind::Interior,
            Outcome::Unresolved => Kind::Unresolved,
        };
        row.class[i] = Class::new(kind, Evidence::Heuristic);
        let Outcome::Escaped { n, zr, zi, dr, di, dexp } = o else { return };
        let z2 = zr * zr + zi * zi;
        let log2z = 0.5 * z2.log2();
        if let Some(nu) = row.nu.as_deref_mut() {
            nu[i] = n as f64 + 1.0 - log2z.log2();
        }
        if let Some(de) = row.de.as_deref_mut() {
            de[i] = de_px(z2.sqrt(), log2z, dr, di, dexp, self.px_m, self.px_e);
        }
        if let Some(nm) = row.normal.as_deref_mut() {
            // Direction of z / (dz/dc), i.e. z * conj(dz/dc); the scale is positive.
            let (x, y) = self.plane.to_screen(zr * dr + zi * di, zi * dr - zr * di);
            nm[i] = Samples::angle(x, y);
        }
    }
}

/// `2 |z| ln|z| / |dz/dc|` in output pixels, with `|dz/dc| = |(dr, di)| 2^dexp` and the
/// pixel `px_m 2^px_e`. `hypot` keeps `|dz/dc|` finite where squaring it would
/// overflow (|dz/dc| ~ 1e200 at depth 1e-200).
#[inline]
pub(crate) fn de_px(az: f64, log2z: f64, dr: f64, di: f64, dexp: i64, px_m: f64, px_e: i64) -> f32 {
    let lnz = log2z * std::f64::consts::LN_2;
    let base = 2.0 * az * lnz / (dr.hypot(di) * px_m);
    let k = (-(dexp + px_e)).clamp(-4000, 4000); // beyond this f32 saturates anyway
    (base * exp2i(k / 2) * exp2i(k - k / 2)) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn de_survives_huge_derivatives() {
        // |z| = 1e5, |dz/dc| = 1e250 (f64 squares overflow), pixel 1e-250: de ~ 2.3e6 px.
        let az = 1e5f64;
        let (pm, pe) = (1e-250f64 * exp2i(830), -830);
        let de = de_px(az, az.log2(), 6e249, 8e249, 0, pm, pe);
        let want = 2.0 * az * az.ln() / 1e250 / 1e-250;
        assert!(((de as f64) - want).abs() < 1e-6 * want, "{de} {want}");
    }
}
