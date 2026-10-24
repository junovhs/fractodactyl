//! Turning one sample's outcome into column values, at any depth.
use crate::sample::{Outcome, U};
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
    bound: Option<&'a mut [f32]>,
    stripe: Option<&'a mut [f32]>,
}

impl<'a> Row<'a> {
    /// Split all columns into per-row slices.
    pub(crate) fn split(s: &'a mut Samples, nx: usize) -> Vec<Row<'a>> {
        let mut nu = s.nu.as_deref_mut().map(|v| v.chunks_mut(nx));
        let mut de = s.de.as_deref_mut().map(|v| v.chunks_mut(nx));
        let mut normal = s.normal.as_deref_mut().map(|v| v.chunks_mut(nx));
        let mut bound = s.bound.as_deref_mut().map(|v| v.chunks_mut(nx));
        let mut stripe = s.stripe.as_deref_mut().map(|v| v.chunks_mut(nx));
        (s.class.chunks_mut(nx).enumerate())
            .map(|(j, class)| Row {
                j,
                class,
                nu: nu.as_mut().and_then(Iterator::next),
                de: de.as_mut().and_then(Iterator::next),
                normal: normal.as_mut().and_then(Iterator::next),
                bound: bound.as_mut().and_then(Iterator::next),
                stripe: stripe.as_mut().and_then(Iterator::next),
            })
            .collect()
    }

    /// Store sample `i`'s stripe average, if the column is present.
    #[inline]
    pub(crate) fn set_stripe(&mut self, i: usize, v: f32) {
        if let Some(s) = self.stripe.as_deref_mut() {
            s[i] = v;
        }
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
        Store {
            plane: *plane,
            px_m: plane.h_m * ss as f64,
            px_e: plane.h_e,
        }
    }

    #[inline]
    pub(crate) fn put(&self, row: &mut Row, i: usize, o: Outcome) {
        let Outcome::Escaped {
            n,
            zr,
            zi,
            dr,
            di,
            dexp,
            ez,
            ed,
        } = o
        else {
            let kind = if o == Outcome::Unresolved {
                Kind::Unresolved
            } else {
                Kind::Interior
            };
            row.class[i] = Class::new(kind, Evidence::Heuristic);
            return;
        };
        let z2 = zr * zr + zi * zi;
        let log2z = 0.5 * z2.log2();
        let b = bounds(n, z2.sqrt(), ez, dr.hypot(di), ed);
        row.class[i] = Class::new(
            Kind::Escaped,
            if b.is_some() {
                Evidence::Bounded
            } else {
                Evidence::Heuristic
            },
        );
        if let Some(nu) = row.nu.as_deref_mut() {
            nu[i] = n as f64 + 1.0 - log2z.log2();
        }
        if let Some(de) = row.de.as_deref_mut() {
            let d = de_px(z2.sqrt(), log2z, dr, di, dexp, self.px_m, self.px_e);
            de[i] = b.map_or(d, |(lo, _)| (d as f64 * lo) as f32);
        }
        if let Some(bound) = row.bound.as_deref_mut() {
            bound[i] = b.map_or(0.0, |(_, nu)| nu as f32);
        }
        if let Some(nm) = row.normal.as_deref_mut() {
            // Direction of z / (dz/dc), i.e. z * conj(dz/dc); the scale is positive.
            let (x, y) = self.plane.to_screen(zr * dr + zi * di, zi * dr - zr * di);
            nm[i] = Samples::angle(x, y);
        }
    }
}

/// Bounded evidence for an escaped sample with `|z| = az`, `|dz/dc| = ad` (any scale) and
/// error radii `ez`, `ed` on them: `(lo, bound)` where `de * lo` is a lower estimate safe
/// for the Koebe test (true distance >= `de * lo / 4`) and `bound` covers the nu error
/// widened by `de_high / de_low`, so `bound * de * lo * ln2 / 2` bounds the displacement.
/// `None` unless the radii prove escape (`|z| - ez > 2`) and `ed < ad / 2`.
fn bounds(n: u64, az: f64, ez: f64, ad: f64, ed: f64) -> Option<(f64, f64)> {
    let lo = az - ez;
    if !(lo > 2.0 && ed < 0.5 * ad) {
        return None;
    }
    let (lnz, llo, lhi) = (az.ln(), lo.ln(), (az + ez).ln());
    // nu = n + 1 - log2(ln|z| / ln 2): |z| within ez moves it by at most
    // log2(ln|z| / ln(|z| - ez)); plus the rounding of nu itself.
    let nu =
        (-(-ez / az).ln_1p() / llo).ln_1p() / std::f64::consts::LN_2 + 8.0 * U * (n as f64 + 8.0);
    // Koebe: distance >= e^-G de / 4 with G <= ln|z| 2^-n; (1 - 4/|z|^2) covers the
    // finite-n estimate of G and G' (|c| <= 4).
    let g = lhi * (-(n as f64)).exp2();
    let de_lo = (lo * llo) / (az * lnz) * ad / (ad + ed)
        * (-g).exp()
        * (1.0 - 4.0 / (lo * lo))
        * (1.0 - 1e-6);
    let de_hi = ((az + ez) * lhi) / (az * lnz) * ad / (ad - ed) * (1.0 + 1e-6);
    Some((de_lo, nu * de_hi / de_lo * (1.0 + 1e-6)))
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

/// Guard an externally computed escaped state using this encoder's uncertainty and
/// output-pixel definitions (DEC-10/17). Radii are in z units and derivative-mantissa
/// units respectively. This does not confer certified evidence on external radii.
pub fn external_escape_ok(
    s: &crate::grid::EscapedState,
    ez: f64,
    ed: f64,
    pixel: (f64, i64),
) -> bool {
    let az = s.z.0.hypot(s.z.1);
    let ad = s.derivative.0.hypot(s.derivative.1);
    if !(ez >= 0.0
        && ed >= 0.0
        && ez.is_finite()
        && ed.is_finite()
        && az.is_finite()
        && ad.is_finite())
    {
        return false;
    }
    let Some((lo, bound)) = bounds(s.n, az, ez, ad, ed) else {
        return false;
    };
    let de = f64::from(de_px(
        az,
        az.log2(),
        s.derivative.0,
        s.derivative.1,
        s.derivative_exponent,
        pixel.0,
        pixel.1,
    ));
    let displacement = bound * de * lo * std::f64::consts::LN_2 / 2.0;
    // Relative uncertainty in |z| ln|z| / |dz/dc|, the DE encoded above.
    let base = az * az.ln();
    let de_hi = (az + ez) * (az + ez).ln() / base * ad / (ad - ed);
    let de_lo = (az - ez) * (az - ez).ln() / base * ad / (ad + ed);
    let angle = (ez / az).asin() + (ed / ad).asin();
    // Half fd compare/SAMPLES defaults: 1e-3 px, 0.2% DE, 0.2 degrees.
    // Include f32 DE encoding and one normal quantization bin in this half budget.
    displacement.is_finite()
        && displacement <= 5e-4
        && de.is_finite()
        && de_hi - 1.0 + f64::from(f32::EPSILON) <= 1e-3
        && 1.0 - de_lo + f64::from(f32::EPSILON) <= 1e-3
        && angle + std::f64::consts::TAU / 65536.0 <= 0.1f64.to_radians()
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
