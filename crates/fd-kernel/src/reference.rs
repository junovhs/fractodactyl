//! Reference orbit Z_0 = 0, Z_{n+1} = Z_n^2 + C, stored structure-of-arrays as f64.
//! Computed either in f64 (shallow) or in fixed point at the exact centre (deep).
use fd_fixed::{Complex, Fixed};

/// Reference orbit points `Z_0..Z_len-1`.
#[derive(Clone)]
pub struct Reference {
    /// Real parts.
    pub re: Vec<f64>,
    /// Imaginary parts.
    pub im: Vec<f64>,
}

impl Reference {
    /// Memory cap: 2^20 points = 16 MiB. Orbit slabs (REF-01) replace this.
    pub const MAX_LEN: usize = 1 << 20;

    /// Iterate until the orbit escapes `escape_radius`, `max_iter` steps are taken, or
    /// `MAX_LEN` points are stored. The final point is kept: a sample that reaches it
    /// rebases to Z_0, so a shorter reference is always valid, just less reused.
    pub fn new(c_re: f64, c_im: f64, max_iter: u64, escape_radius: f64) -> Reference {
        let (mut x, mut y) = (0.0f64, 0.0f64);
        Self::collect(max_iter, escape_radius, || {
            let t = x * x - y * y + c_re;
            y = 2.0 * x * y + c_im;
            x = t;
            (x, y)
        })
    }

    // One collection/termination driver for the f64 and arbitrary-exponent number
    // stages; Complex::sqr/add already own the latter's arithmetic (DEC-24).
    fn collect(max_iter: u64, escape_radius: f64, mut step: impl FnMut() -> (f64, f64)) -> Self {
        let steps = max_iter.min(Self::MAX_LEN as u64 - 1);
        let cap = steps.min(1 << 16) as usize + 1;
        let (mut re, mut im) = (Vec::with_capacity(cap), Vec::with_capacity(cap));
        re.push(0.0);
        im.push(0.0);
        for _ in 0..steps {
            let (x, y) = step();
            re.push(x);
            im.push(y);
            if x * x + y * y > escape_radius * escape_radius {
                break;
            }
        }
        Self { re, im }
    }

    /// Opt-in full-escape reference using existing arbitrary-exponent arithmetic.
    /// The returned orbit meets `error_radius(Some(bits),...)`'s absolute defect
    /// contract. Default CPU references retain the Fixed builder below.
    ///
    /// Domain: b>=128, 2<=R<=2^40, rounded centre L1<=4 (true L1<8).
    /// P>=b+128 RN bits: while collecting, |Z|<=2R before a continued step;
    /// every sqr/add intermediate is <2^85. Summing scalar rounding and decimal
    /// parse defects conservatively gives <2^(96-P), hence <2^(-b-32), well below
    /// the existing absolute per-step budget 2^(4-b). f64 storage rounding is
    /// enclosed separately by error_radius. See the GPU-32 derivation/handoff.
    pub fn from_decimal_full_escape(
        re: &str,
        im: &str,
        bits: u32,
        max_iter: u64,
        radius: f64,
    ) -> Result<Self, String> {
        if !(128..=16384).contains(&bits) || !(2.0..=1099511627776.0).contains(&radius) {
            return Err("full-escape reference outside proved precision/radius domain".into());
        }
        let precision = bits as usize + 128;
        let c = Complex::parse(re, im, precision)?;
        let size = c.re.to_f64().abs() + c.im.to_f64().abs();
        if !size.is_finite() || size > 4.0 {
            return Err("full-escape reference outside proved centre domain".into());
        }
        let mut z = Complex::zero(precision);
        Ok(Self::collect(max_iter, radius, || {
            z = z.sqr().add(&c);
            (z.re.to_f64(), z.im.to_f64())
        }))
    }

    /// Same orbit computed in fixed point from the exact centre. Stops at `|Z| > 2^16`
    /// (samples rebase there; their own escape test uses the full radius).
    pub fn from_fixed(cr: &Fixed, ci: &Fixed, max_iter: u64) -> Reference {
        let steps = max_iter.min(Self::MAX_LEN as u64 - 1);
        let cap = steps.min(1 << 16) as usize + 1;
        let (mut re, mut im) = (Vec::with_capacity(cap), Vec::with_capacity(cap));
        re.push(0.0);
        im.push(0.0);
        fd_fixed::orbit(cr, ci, steps, 4294967296.0, |a, b| {
            re.push(a);
            im.push(b);
        });
        Reference { re, im }
    }

    /// FIX-04 / DEC-10: admit only a deep, near-superattracting record return.
    /// A finite residual and its second return must be visible in this reference;
    /// zeros/subnormal returns decline rather than inventing a nucleus. This is a
    /// heuristic gate, not a certified interior test or reference re-placement.
    pub(crate) fn nucleus_period(&self) -> Option<usize> {
        let mut record = self.re.get(1)?.hypot(*self.im.get(1)?);
        let mut log_beta = 0.0;
        for p in 2..self.len() / 2 {
            let previous = self.re[p - 1].hypot(self.im[p - 1]);
            log_beta += 1.0 + previous.log2();
            let radius = self.re[p].hypot(self.im[p]);
            if radius > 1e-140
                && radius < 1e-24
                && radius < record * 1e-12
                && 1.0 + log_beta + radius.log2() < -40.0
                && self.re[2 * p].hypot(self.im[2 * p]) <= 2.0 * radius
            {
                return Some(p);
            }
            record = record.min(radius);
        }
        None
    }

    /// Upper bounds `q[m] >= |A_m - Z_m|` on each stored point's distance from the
    /// exact orbit `A` of the exact view centre. `bits`: the fixed-point fraction bits
    /// the orbit was computed with, or `None` for an f64 orbit from `(c_re, c_im)`, the
    /// correctly rounded centre.
    pub fn error_radius(&self, bits: Option<u64>, c_re: f64, c_im: f64) -> Vec<f64> {
        const U: f64 = f64::EPSILON / 2.0;
        let c1 = c_re.abs() + c_im.abs();
        // Fixed point: three truncated squares per component plus the centre's own
        // truncation, each under one unit of 2^-bits.
        let fixed = bits.map(|b| fd_fixed::exp2i(4 - b as i64).max(f64::MIN_POSITIVE));
        let mut q = Vec::with_capacity(self.len());
        let mut a = 0.0f64; // |A_m - F_m|, F the orbit as computed before rounding to f64
        for m in 0..self.len() {
            let s = (self.re[m] * self.re[m] + self.im[m] * self.im[m]).sqrt() * (1.0 + 4.0 * U);
            let (round, defect) = match fixed {
                Some(d) => (2.0 * U * s + f64::MIN_POSITIVE, d),
                // f64: rounding of x^2 - y^2 + c and 2xy + c, plus the centre's rounding.
                None => (
                    0.0,
                    8.0 * U * (s * s + c1) + 2.0 * U * c1 + f64::MIN_POSITIVE,
                ),
            };
            q.push(a + round);
            a = (a * (2.0 * (s + round) + a) + defect) * (1.0 + 8.0 * U);
        }
        q
    }

    /// Number of stored points (at least 2 unless `max_iter == 0`).
    #[inline]
    pub fn len(&self) -> usize {
        self.re.len()
    }

    /// Always false: `Z_0` is always stored.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.re.is_empty()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn existing_locations_decline_the_nucleus_heuristic() {
        use crate::grid::{reference, Params};
        use crate::view::Tier;
        use fd_samples::{Column, ColumnSet, View};
        for line in include_str!("../../../bench/locations.txt")
            .lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        {
            let f: Vec<_> = line.split_whitespace().collect();
            let view = View {
                center_re: f[1].into(),
                center_im: f[2].into(),
                width: f[3].into(),
                rotation: f[6].parse().unwrap(),
            };
            let p = Params {
                nx: 16,
                ny: 9,
                ss: 1,
                max_iter: f[4].parse().unwrap(),
                escape_radius: 1e10,
                columns: ColumnSet::of(&[Column::Nu]),
                threads: 1,
                tier: match f[7] {
                    "fx" => Some(Tier::Fixed),
                    "scaled" => Some(Tier::Scaled),
                    _ => None,
                },
                stripe: Default::default(),
            };
            let (r, _) = reference(&view, &p).unwrap();
            assert_eq!(
                r.nucleus_period(),
                None,
                "{} must retain the old path",
                f[0]
            );
        }
    }
}

#[cfg(test)]
mod full_escape_tests {
    use super::*;
    use crate::Plane;
    use fd_samples::View;
    use std::fmt::Write as _;

    #[test]
    fn full_escape_rejects_unproved_domain() {
        for (re, bits, radius) in [
            ("0", 127, 1e10),
            ("0", 128, 1.0),
            ("0", 128, f64::NAN),
            ("0", 128, 2f64.powi(41)),
            ("5", 128, 1e10),
        ] {
            assert!(Reference::from_decimal_full_escape(re, "0", bits, 100, radius).is_err());
        }
    }

    /// Actual consumed builder/radius output for the independent mpmath check.
    #[test]
    #[ignore = "writes bounded highprecision reference validation data"]
    fn gpu_full_escape_radius_probe() {
        let path = std::env::var("FD_REFERENCE_REPORT").expect("FD_REFERENCE_REPORT output path");
        let mut report = String::new();
        for (name, re, im, width, radius) in [
            ("seahorse-deep", "-0.7453", "0.1127", "1e-14", 1e10),
            (
                "valley",
                "-0.743643887037158704752191506114774",
                "0.131825904205311970493132056385139",
                "1e-28",
                1e10,
            ),
            ("large-escape", "4", "0", "1e-40", 2f64.powi(40)),
            ("zero", "0", "0", "1e-100", 2.0),
        ] {
            let v = View {
                center_re: re.into(),
                center_im: im.into(),
                width: width.into(),
                rotation: 0.0,
            };
            let plane = Plane::new(&v, 128, 72).unwrap();
            let bits = plane.bits as u32;
            let r = Reference::from_decimal_full_escape(re, im, bits, 10000, radius).unwrap();
            let q = r.error_radius(Some(u64::from(bits)), plane.c_re, plane.c_im);
            writeln!(report, "camera|{name}|{re}|{im}|{bits}|{radius:.17e}").unwrap();
            for (n, ((&x, &y), &e)) in r.re.iter().zip(&r.im).zip(&q).enumerate() {
                writeln!(report, "point|{n}|{x:.17e}|{y:.17e}|{e:.17e}").unwrap();
            }
        }
        let path = std::path::Path::new(&path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, report).unwrap();
        eprintln!(
            "GPU-32 full-escape references/radii written: {}",
            path.display()
        );
    }
}
