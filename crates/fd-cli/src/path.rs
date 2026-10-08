//! `fd path zoom`: write a constant-rate zoom as a camera path file (docs/spec/PLAN.md
//! "Path file"): one exact centre, widths evenly spaced in log width from `--from` to
//! `--to`, `round(seconds * fps)` frames. Widths are written with 6 significant digits,
//! so a 1-ulp difference in `powf` between platforms does not change the file, and every
//! width round-trips through f64 (frame manifests carry it as f64, ATLAS.md).
use crate::args::Args;
use fd_fixed::Decimal;

const USAGE: &str =
    "usage: fd path zoom --re X --im Y --from W0 --to W1 --seconds S --fps F [--rotation R]";

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    match argv.first().map(String::as_str) {
        Some("zoom") => print!("{}", zoom(&argv[1..])?),
        _ => return Err(USAGE.into()),
    }
    Ok(())
}

fn zoom(rest: &[String]) -> Result<String, String> {
    let a = Args::parse(rest, &["re", "im", "from", "to", "seconds", "fps", "rotation"])?;
    let (re, im) = (a.need("re")?, a.need("im")?);
    Decimal::parse(re)?;
    Decimal::parse(im)?;
    let width = |k: &str| -> Result<f64, String> {
        let w: f64 = a.need(k)?.parse().map_err(|_| format!("--{k}: bad number"))?;
        if !(w.is_normal() && w > 0.0) {
            return Err(format!("--{k}: width must be a positive normal f64"));
        }
        Ok(w)
    };
    let (w0, w1) = (width("from")?, width("to")?);
    let seconds: f64 = a.num("seconds", 0.0)?;
    let fps: f64 = a.num("fps", 30.0)?;
    let rotation: f64 = a.num("rotation", 0.0)?;
    let n = (seconds * fps).round();
    if !(2.0..=1e7).contains(&n) || !rotation.is_finite() {
        return Err("--seconds x --fps must give 2 to 1e7 frames".into());
    }
    let n = n as usize;
    let (l0, l1) = (w0.log10(), w1.log10());
    let mut out = format!(
        "# fd path zoom --re {re} --im {im} --from {} --to {} --seconds {seconds} --fps {fps} --rotation {rotation}\n",
        a.need("from")?,
        a.need("to")?
    );
    out.push_str(&format!("# {n} frames, {:.6} decades/s\n# re im width [rotation]\n", (l0 - l1).abs() * fps / (n - 1) as f64));
    for f in 0..n {
        let x = l0 + (l1 - l0) * f as f64 / (n - 1) as f64;
        let w = sig6(10f64.powf(x));
        if rotation == 0.0 {
            out.push_str(&format!("{re} {im} {w}\n"));
        } else {
            out.push_str(&format!("{re} {im} {w} {rotation}\n"));
        }
    }
    Ok(out)
}

/// `w` with 6 significant digits, trailing zeros dropped: `4.12e-30`, `4e0`.
pub(crate) fn sig6(w: f64) -> String {
    let s = format!("{w:.5e}");
    let (m, e) = s.split_once('e').expect("exponent form");
    let m = if m.contains('.') { m.trim_end_matches('0').trim_end_matches('.') } else { m };
    format!("{m}e{e}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths_are_six_digit_decimals_that_round_trip() {
        assert_eq!(sig6(4.0), "4e0");
        assert_eq!(sig6(1.2345678e-30), "1.23457e-30");
        let argv: Vec<String> =
            "--re 0 --im 1 --from 4 --to 1e-49 --seconds 1 --fps 10".split(' ').map(String::from).collect();
        let text = zoom(&argv).unwrap();
        let widths: Vec<&str> = text.lines().filter(|l| !l.starts_with('#')).map(|l| l.split(' ').nth(2).unwrap()).collect();
        assert_eq!(widths.len(), 10);
        assert_eq!((widths[0], widths[9]), ("4e0", "1e-49"));
        for w in &widths {
            let x: f64 = w.parse().unwrap();
            assert_eq!(format!("{x:e}"), *w);
        }
    }
}
