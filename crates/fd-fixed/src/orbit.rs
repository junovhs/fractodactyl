//! Reference orbit Z_{n+1} = Z_n^2 + C in fixed point.
//!
//! Three squarings per step and no multiplications:
//! `x' = x^2 - y^2 + cr`, `y' = (x+y)^2 - x^2 - y^2 + ci`. A square costs about half a
//! general product, so this is ~25% cheaper than `x^2, y^2, 2xy`. All buffers are
//! allocated once; the loop itself never allocates.
use crate::limbs::{add, add_signed, sqr};
use crate::{to_f64, Fixed};

/// Iterate from Z_0 = 0, calling `emit(re, im)` with each Z_1, Z_2, ... as f64.
/// Stops after `steps` points or after emitting the first point with `|Z|^2 > escape2`.
/// Keep `escape2 <= 2^32` so every intermediate fits the integer limb.
/// `cr` and `ci` must have the same number of limbs.
pub fn orbit(cr: &Fixed, ci: &Fixed, steps: u64, escape2: f64, mut emit: impl FnMut(f64, f64)) {
    let l = cr.mag.len();
    assert_eq!(l, ci.mag.len(), "orbit: coordinates need equal precision");
    assert!(
        escape2 <= 4294967296.0,
        "orbit: escape radius too large for the integer limb"
    );
    let z = || vec![0u64; l];
    let (mut x, mut y, mut x2, mut y2, mut s, mut s2, mut t) = (z(), z(), z(), z(), z(), z(), z());
    let mut prod = vec![0u64; 2 * l];
    let (mut xn, mut yn) = (false, false);
    for _ in 0..steps {
        sqr(&x, &mut prod, &mut x2);
        sqr(&y, &mut prod, &mut y2);
        add_signed(xn, &x, yn, &y, &mut s); // sign irrelevant: it is squared next
        sqr(&s, &mut prod, &mut s2);
        // x' = (x2 - y2) + cr
        let tn = add_signed(false, &x2, true, &y2, &mut t);
        xn = add_signed(tn, &t, cr.neg, &cr.mag, &mut x);
        // y' = (s2 - (x2 + y2)) + ci
        add(&x2, &y2, &mut s);
        let tn = add_signed(false, &s2, true, &s, &mut t);
        yn = add_signed(tn, &t, ci.neg, &ci.mag, &mut y);
        let (fx, fy) = (to_f64(xn, &x), to_f64(yn, &y));
        emit(fx, fy);
        if fx * fx + fy * fy > escape2 {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::limbs_for;

    fn run(c: (&str, &str), steps: u64) -> Vec<(f64, f64)> {
        let l = limbs_for(256);
        let (cr, ci) = (Fixed::parse(c.0, l).unwrap(), Fixed::parse(c.1, l).unwrap());
        let mut out = Vec::new();
        orbit(&cr, &ci, steps, 65536.0, |a, b| out.push((a, b)));
        out
    }

    #[test]
    fn matches_f64_iteration_at_shallow_points() {
        for (re, im) in [("-0.75", "0.1"), ("0.3", "-0.5"), ("-1.8", "0.01")] {
            let (cr, ci): (f64, f64) = (re.parse().unwrap(), im.parse().unwrap());
            let (mut x, mut y) = (0.0f64, 0.0f64);
            for (n, (a, b)) in run((re, im), 30).into_iter().enumerate() {
                (x, y) = (x * x - y * y + cr, 2.0 * x * y + ci);
                assert!(
                    (a - x).abs() + (b - y).abs() < 1e-9 * (1.0 + x.abs() + y.abs()),
                    "{re},{im} step {n}"
                );
            }
        }
    }

    #[test]
    fn misiurewicz_i_is_preperiodic() {
        // c = i: 0, i, -1+i, -i, -1+i, -i, ...
        let z = run(("0", "1"), 6);
        assert_eq!(
            z,
            vec![
                (0.0, 1.0),
                (-1.0, 1.0),
                (0.0, -1.0),
                (-1.0, 1.0),
                (0.0, -1.0),
                (-1.0, 1.0)
            ]
        );
    }

    #[test]
    fn stops_after_escape() {
        let z = run(("1", "1"), 100);
        assert!(z.len() < 10);
        let (a, b) = *z.last().unwrap();
        assert!(a * a + b * b > 65536.0);
    }
}
