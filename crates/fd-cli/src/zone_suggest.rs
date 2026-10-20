//! PROB-27: construct a Misiurewicz ladder, then refine actual critical returns.
//! DEC-16: the predictor follows this seed's dynamics; it never substitutes a location.
//! DEC-10: predictors are not admitted as targets until nucleus refinement and the
//! existing zone seed detector confirm a repelling cycle with at least 100 laps.
use super::{cycle, log10_size, log2_norm2, misiurewicz, nucleus, orbit, seed};
use crate::args::Args;
use fd_fixed::{bits_for_digits, Complex, Float};

fn sqrt(z: &Complex) -> Complex {
    if z.is_zero() {
        return z.clone();
    }
    let bits = z.re.precision();
    // Only a state-space Newton initializer is rounded; all subsequent operations,
    // including the parameter displacement and nucleus, stay at working precision.
    let (re, im) = (z.re.to_f64(), z.im.to_f64());
    let a = ((re.hypot(im) + re) * 0.5).max(0.0).sqrt();
    let b = ((re.hypot(im) - re) * 0.5).max(0.0).sqrt().copysign(im);
    let mut x = Complex {
        re: Float::from_f64(a, bits),
        im: Float::from_f64(b, bits),
    };
    for _ in 0..12 {
        x = x.add(&z.div(&x)).ldexp(-1);
    }
    x
}

/// Continue the cycle's inverse branch from zero. rho^j (x_j-alpha) tends to
/// its Koenigs coordinate V; no truncated chart is evaluated outside its radius.
fn landing(m: &Complex, alpha: &Complex, rho: &Complex, r: usize) -> Result<Complex, String> {
    let bits = m.re.precision();
    let mut points = vec![alpha.clone()];
    for _ in 1..r {
        points.push(points.last().unwrap().sqr().add(m));
    }
    let mut x = Complex::zero(bits);
    let mut power = Complex::from_i64(1, bits);
    let mut previous = Complex::zero(bits);
    for j in 0..2048 {
        for point in points.iter().rev() {
            let root = sqrt(&x.sub(m));
            x = if log2_norm2(&root.sub(point)) < log2_norm2(&root.neg().sub(point)) {
                root
            } else {
                root.neg()
            };
        }
        power = power.mul(rho);
        let v = power.mul(&x.sub(alpha));
        if j > 32
            && !v.is_zero()
            && log2_norm2(&v.sub(&previous)) - log2_norm2(&v)
                < -2.0 * 40.0 * std::f64::consts::LOG2_10
        {
            return Ok(v);
        }
        previous = v;
    }
    Err("the inverse branch did not settle at this cycle; no suggestions produced".into())
}

pub(super) fn run(argv: &[String]) -> Result<(), String> {
    let args = Args::parse(argv, &["count", "max-iter"])?;
    let [re, im] = args.positional.as_slice() else {
        return Err(super::USAGE.into());
    };
    let count: usize = args.num("count", 3)?;
    if !(1..=20).contains(&count) {
        return Err("--count must be 1..=20".into());
    }
    let max: usize = args.num("max-iter", 3000)?;
    if !(64..=10000).contains(&max) {
        return Err("--max-iter must be 64..=10000 for suggestions".into());
    }
    let digits = re.len().max(im.len()).max(180) + 80;
    let bits = bits_for_digits(digits);
    let mut m = Complex::parse(re, im, bits)?;
    let (initial, _) = orbit(&m, max + 10, bits);
    if initial.len() < max + 10 {
        return Err("the seed escapes before its cycle can be identified".into());
    }
    let (q, r) = misiurewicz(&m, max, bits)?;
    let tolerance = -2.0 * (digits as f64 - 30.0) * std::f64::consts::LOG2_10;
    let mut converged = false;
    for _ in 0..60 {
        let (z, u) = orbit(&m, q + r, bits);
        if z.len() <= q + r {
            break;
        }
        let dh = u[q + r].sub(&u[q]);
        if dh.is_zero() {
            break;
        }
        let step = z[q + r].sub(&z[q]).div(&dh);
        m = m.sub(&step);
        if log2_norm2(&step) < tolerance {
            converged = true;
            break;
        }
    }
    if !converged {
        return Err("Misiurewicz seed Newton did not converge".into());
    }
    let (z, u) = orbit(&m, q + r, bits);
    let (alpha, rho) = cycle(&m, &z[q], r, digits, bits)?;
    let ln_rho = log2_norm2(&rho) * 0.5 * std::f64::consts::LN_2;
    if !ln_rho.is_finite() || ln_rho <= 0.0 {
        return Err("the seed needs a repelling cycle".into());
    }
    let d = u[q + r]
        .sub(&u[q])
        .div(&rho.sub(&Complex::from_i64(1, bits)));
    let first = (28.0 * std::f64::consts::LN_10 / ln_rho).ceil().max(140.0) as usize;
    if first > max.saturating_sub(q) / r {
        return Err(
            "this cycle's ladder exceeds --max-iter; increase it or choose another seed".into(),
        );
    }
    let a = landing(&m, &alpha, &rho, r)?.div(&d);
    let mut inv = Complex::from_i64(1, bits);
    for _ in 0..first {
        inv = inv.div(&rho);
    }
    let mut suggestions = Vec::new();
    // A failed refinement declines that rung. Never publish an unchecked predictor.
    for k in 0..count * 8 {
        let laps = first + k;
        let period = q + r * laps;
        if period > max {
            break;
        }
        let guess = m.add(&a.mul(&inv));
        inv = inv.div(&rho);
        let Ok(c) = nucleus(&guess, period, digits, bits) else {
            continue;
        };
        let (cre, cim) = (c.re.to_decimal(digits - 2), c.im.to_decimal(digits - 2));
        let Ok(s) = seed(&cre, &cim, max, Some(period)) else {
            continue;
        };
        if s.laps < 100 || (s.q, s.r) != (q, r) {
            continue;
        }
        if suggestions
            .iter()
            .any(|old: &super::Seed| log2_norm2(&old.c.sub(&s.c)) < tolerance)
        {
            continue;
        }
        suggestions.push(s);
        if suggestions.len() == count {
            break;
        }
    }
    if suggestions.len() < count {
        return Err(format!("found {} validated targets, fewer than {count}; try a larger --max-iter or another seed", suggestions.len()));
    }
    for s in suggestions {
        let size = log10_size(&s);
        let exponent = size.floor() as i64;
        // Decimal mantissa + explicit exponent, including sizes below f64 range (DEC-21).
        let width = format!(
            "{:.12}e{exponent}",
            5.0 * 10.0f64.powf(size - exponent as f64)
        );
        println!("{{\"schema\":\"fd-zone-suggestion/1\",\"c_exact\":[\"{}\",\"{}\"],\"period\":{},\"q\":{},\"r\":{},\"laps\":{},\"end_width\":\"{}\"}}",
            s.c.re.to_decimal(s.digits-2), s.c.im.to_decimal(s.digits-2), s.period, s.q, s.r, s.laps, width);
    }
    Ok(())
}
