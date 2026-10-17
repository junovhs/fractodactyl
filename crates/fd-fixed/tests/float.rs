//! PROB-30: fd-fixed's Float against mpmath on v0's nucleus orbit
//! (fixture: tools/research/misiurewicz/float_fixture.py).
use fd_fixed::{bits_for_digits, Complex, Float};

fn fixture() -> std::collections::HashMap<String, Vec<String>> {
    include_str!("fixtures/v0_orbit.txt")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let mut f = l.split_whitespace().map(String::from);
            (f.next().unwrap(), f.collect())
        })
        .collect()
}

/// `|a - b| / |b|` as log10.
fn rel_digits(a: &Complex, b: &Complex) -> f64 {
    let (m, e) = a.sub(b).norm2().div(&b.norm2()).frexp();
    -(m.log2() + e as f64) / 2.0 / std::f64::consts::LOG2_10
}

#[test]
fn float_matches_mpmath() {
    let f = fixture();
    let dps: usize = f["dps"][0].parse().unwrap();
    let period: usize = f["period"][0].parse().unwrap();
    // The digits mpmath itself keeps between dps and dps + 40: the reachable agreement.
    let agree: f64 = f["agree_digits"][0].parse().unwrap();
    let bits = bits_for_digits(dps);
    let c = Complex::parse(&f["c"][0], &f["c"][1], bits).unwrap();
    let (mut z, mut dz) = (Complex::zero(bits), Complex::zero(bits));
    let one = Complex::from_i64(1, bits);
    for _ in 0..period {
        dz = z.mul(&dz).ldexp(1).add(&one);
        z = z.sqr().add(&c);
    }
    let want_z = Complex::parse(&f["z"][0], &f["z"][1], bits).unwrap();
    let want_step = Complex::parse(&f["step"][0], &f["step"][1], bits).unwrap();
    let (dz_ok, step_ok) = (rel_digits(&z, &want_z), rel_digits(&z.div(&dz), &want_step));
    eprintln!(
        "PROB-30: z_{period} agrees to {dz_ok:.1} digits, z/dz to {step_ok:.1} (mpmath self-agreement {agree})"
    );
    assert!(dz_ok >= agree - 10.0 && step_ok >= agree - 10.0);
}

#[test]
fn float_spans_deep_zone_ranges() {
    for (s, e2) in [("1e-1000", -3321), ("1e500", 1661), ("-2.5e-37", -121)] {
        let x = Float::parse(s, bits_for_digits(60)).unwrap();
        let (m, e) = x.frexp();
        assert!((0.5..1.0).contains(&m.abs()) && e == e2, "{s}: {m} {e}");
        let back = Float::parse(&x.to_decimal(40), bits_for_digits(60)).unwrap();
        let rel = back.sub(&x).div(&x).abs().to_f64();
        assert!(rel < 1e-38, "{s}: {} -> {rel}", x.to_decimal(40));
    }
    assert_eq!(Float::parse("1e-300", 128).unwrap().to_f64(), 1e-300);
    assert_eq!(Float::parse("1e-1000", 128).unwrap().to_f64(), 0.0);
    assert_eq!(
        Float::parse("-3.5", 128).unwrap().to_decimal(5),
        "-3.5000e0"
    );
}
