//! Binary64 encoding shared by ordinary Rust and CubeCL. Temporary research
//! counterpart of zone_f64.comp's log/atan polynomials (DEC-24 rule 4).
//! Vulkan does not provide binary64 log/atan transcendental instructions.
use cubecl::prelude::*;

/// Positive normal inputs only: escape norm2 and log2(|z|) for this probe.
#[cube]
fn log2_positive(x: f64) -> f64 {
    let bits = x.to_bits();
    let mut e = ((bits >> 52u64) & 2047u64) as i32 - 1023i32;
    let mut m = f64::from_bits((bits & 0x000f_ffff_ffff_ffffu64) | 0x3ff0_0000_0000_0000u64);
    if m > 1.4142135623730951f64 {
        m *= 0.5f64;
        e += 1;
    }
    let t = (m - 1.0f64) / (m + 1.0f64);
    let t2 = t * t;
    let mut sum = 0.043478260869565216f64;
    sum = sum * t2 + 0.047619047619047616f64;
    sum = sum * t2 + 0.05263157894736842f64;
    sum = sum * t2 + 0.058823529411764705f64;
    sum = sum * t2 + 0.06666666666666667f64;
    sum = sum * t2 + 0.07692307692307693f64;
    sum = sum * t2 + 0.09090909090909091f64;
    sum = sum * t2 + 0.1111111111111111f64;
    sum = sum * t2 + 0.14285714285714285f64;
    sum = sum * t2 + 0.2f64;
    sum = sum * t2 + 0.3333333333333333f64;
    sum = sum * t2 + 1.0f64;
    (e as f64 * 0.6931471805599453f64 + 2.0f64 * t * sum) * 1.4426950408889634f64
}

#[cube]
fn atan_small(z: f64) -> f64 {
    let zz = z * z;
    let mut p = 0.0f64;
    p -= 0.02127659574468085f64;
    p = p * zz + 0.022222222222222223f64;
    p = p * zz + -0.023255813953488372f64;
    p = p * zz + 0.024390243902439025f64;
    p = p * zz + -0.02564102564102564f64;
    p = p * zz + 0.02702702702702703f64;
    p = p * zz + -0.02857142857142857f64;
    p = p * zz + 0.030303030303030304f64;
    p = p * zz + -0.03225806451612903f64;
    p = p * zz + 0.034482758620689655f64;
    p = p * zz + -0.037037037037037035f64;
    p = p * zz + 0.04f64;
    p = p * zz + -0.043478260869565216f64;
    p = p * zz + 0.047619047619047616f64;
    p = p * zz + -0.05263157894736842f64;
    p = p * zz + 0.058823529411764705f64;
    p = p * zz + -0.06666666666666667f64;
    p = p * zz + 0.07692307692307693f64;
    p = p * zz + -0.09090909090909091f64;
    p = p * zz + 0.1111111111111111f64;
    p = p * zz + -0.14285714285714285f64;
    p = p * zz + 0.2f64;
    p = p * zz + -0.3333333333333333f64;
    p = p * zz + 1.0f64;
    z * p
}

#[cube]
fn atan_unit(x: f64) -> f64 {
    if x > 0.41421356237309503f64 {
        0.7853981633974483f64 + atan_small((x - 1.0f64) / (x + 1.0f64))
    } else {
        atan_small(x)
    }
}

#[cube]
fn atan2(y: f64, x: f64) -> f64 {
    let ax = x.abs();
    let ay = y.abs();
    let mut a = 0.0f64;
    if ax != 0.0f64 || ay != 0.0f64 {
        a = if ay > ax {
            1.5707963267948966f64 - atan_unit(ax / ay)
        } else {
            atan_unit(ay / ax)
        };
        if x < 0.0f64 {
            a = 3.141592653589793f64 - a;
        }
        if y < 0.0f64 {
            a = -a;
        }
    }
    a
}

/// nu, DE in output pixels, and screen-space normal angle. Normal quantisation
/// remains a host step so this checkpoint can measure angular error directly.
#[cube]
pub fn encode(
    zr: f64,
    zi: f64,
    dr: f64,
    di: f64,
    n: u32,
    pixel: f64,
    cs: f64,
    sn: f64,
) -> (f64, f64, f64) {
    let a = zr * zr + zi * zi;
    let log2z = 0.5f64 * log2_positive(a);
    let nu = n as f64 + 1.0f64 - log2_positive(log2z);
    // Derivatives at the six GPU-12 depths fit comfortably in binary64 squares.
    let de =
        2.0f64 * a.sqrt() * (log2z * 0.6931471805599453f64) / (dr * dr + di * di).sqrt() / pixel;
    let re = zr * dr + zi * di;
    let im = zi * dr - zr * di;
    let angle = atan2(sn * re - cs * im, cs * re + sn * im);
    (nu, de, angle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary64_log_covers_escape_and_log_magnitudes() {
        for exponent in -4..=140 {
            for mantissa in [1.0, 1.3, 1.4142135623730951, 1.9] {
                let x = mantissa * 2.0f64.powi(exponent);
                assert!((log2_positive(x) - x.log2()).abs() < 6e-14, "{x}");
            }
        }
    }

    #[test]
    fn binary64_atan_covers_octants_and_axis_edges() {
        for (x, y) in [
            (1.0, 0.0),
            (0.0, 1.0),
            (1.0, 1.0),
            (1.0, 0.41421356237309503),
            (1e100, 1e99),
            (1e99, 1e100),
        ] {
            for sx in [-1.0, 1.0] {
                for sy in [-1.0, 1.0] {
                    let (x, y): (f64, f64) = (x * sx, y * sy);
                    let actual = atan2(y, x);
                    // Signed zero chooses opposite representations of the same
                    // normal on the negative real axis (+pi versus -pi).
                    let difference = (actual - y.atan2(x)).abs();
                    assert!(
                        difference.min((std::f64::consts::TAU - difference).abs()) < 1e-15,
                        "x={x} y={y} actual={actual} expected={}",
                        y.atan2(x)
                    );
                }
            }
        }
    }
}

#[cube]
pub fn packed_word(angle: f64, escaped: bool) -> u32 {
    let mut word = if escaped { 0u32 } else { 2u32 << 16u32 };
    if !(angle <= 3.141592653589793f64 && angle >= -3.141592653589793f64) {
        word = 3u32 << 16u32;
    } else {
        let turns = angle * 10430.378350470453f64;
        let rounded = if turns >= 0.0f64 {
            (turns + 0.5f64) as i32
        } else {
            (turns - 0.5f64) as i32
        };
        word |= rounded as u32 & 65535u32;
    }
    word
}

#[cfg(test)]
mod packing_tests {
    use super::*;
    #[test]
    fn packed_normal_wraps_and_invalid_angle_marks_class() {
        for angle in [
            -std::f64::consts::PI,
            -0.5,
            -0.0,
            0.0,
            0.5,
            std::f64::consts::PI,
        ] {
            let expected = (angle * (65536.0 / std::f64::consts::TAU)).round() as i64 as u16;
            assert_eq!(packed_word(angle, true) & 65535, u32::from(expected));
            assert_eq!(packed_word(angle, false) >> 16, 2);
        }
        for angle in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(packed_word(angle, true) >> 16, 3);
        }
    }
}
