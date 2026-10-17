//! PROB-12's lemmas: `fd zone` reproduces the committed Python-generated v0 zone
//! (bench/zones/v0-core.zone).
use fd_fixed::{bits_for_digits, Float};
use std::path::Path;
use std::process::Command;

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

/// The v0 zone's lines, by key.
fn v0_zone() -> Vec<Vec<String>> {
    std::fs::read_to_string(root().join("bench/zones/v0-core.zone"))
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| l.split_whitespace().map(String::from).collect())
        .collect()
}

fn line<'a>(zone: &'a [Vec<String>], key: &str) -> &'a [String] {
    &zone.iter().find(|l| l[0] == key).unwrap()[1..]
}

/// The first value of `"key":` in a one-line JSON report (number or string).
fn field<'a>(json: &'a str, key: &str) -> &'a str {
    let at = json
        .find(&format!("\"{key}\":"))
        .unwrap_or_else(|| panic!("{key}: {json}"));
    let rest = &json[at + key.len() + 3..];
    let rest = rest.trim_start_matches(['[', '"']);
    &rest[..rest.find([',', '"', ']', '}']).unwrap()]
}

/// The pair after `"key":[`.
fn pair(json: &str, key: &str) -> (f64, f64) {
    let at = json.find(&format!("\"{key}\":[")).unwrap() + key.len() + 4;
    let rest = &json[at..];
    let body = &rest[..rest.find(']').unwrap()];
    let mut it = body
        .split(',')
        .map(|s| s.trim_matches('"').parse::<f64>().unwrap());
    (it.next().unwrap(), it.next().unwrap())
}

fn rel(a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1) / b.0.hypot(b.1)
}

#[test]
fn zone_report_v0() {
    let atlas = std::fs::read_to_string(root().join("bench/path-atlas-v0.txt")).unwrap();
    let centre: Vec<&str> = atlas
        .lines()
        .find(|l| !l.starts_with('#') && !l.trim().is_empty())
        .unwrap()
        .split_whitespace()
        .collect();
    let out = Command::new(env!("CARGO_BIN_EXE_fd"))
        .args(["zone", centre[0], centre[1]])
        .output()
        .unwrap();
    let json = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    eprintln!("{json}");
    assert!(json.contains("\"schema\":\"fd-zone/1\""), "{json}");
    assert_eq!(field(&json, "period"), "764");
    assert_eq!((field(&json, "q"), field(&json, "r")), ("24", "2"));
    let zone = v0_zone();
    // c_exact: the file carries 75+ digits; compare there.
    let bits = bits_for_digits(120);
    let c = line(&zone, "c_exact");
    let at = json.find("\"c_exact\":[").unwrap() + 11;
    let got: Vec<&str> = json[at..json[at..].find(']').unwrap() + at]
        .split(',')
        .map(|s| s.trim_matches('"'))
        .collect();
    for k in 0..2 {
        let (a, b) = (
            Float::parse(got[k], bits).unwrap(),
            Float::parse(&c[k], bits).unwrap(),
        );
        let (m, e) = a.sub(&b).div(&b).abs().frexp();
        let digits = -(m.log2() + e as f64) / std::f64::consts::LOG2_10;
        assert!(digits >= 75.0, "c_exact[{k}] agrees to {digits:.1} digits");
    }
    let num = |k: &str| {
        let l = line(&zone, k);
        (l[0].parse().unwrap(), l[1].parse().unwrap())
    };
    for key in ["alpha", "rho"] {
        let r = rel(pair(&json, key), num(key));
        assert!(
            r < 1e-12,
            "{key}: {:?} vs {:?} ({r:e})",
            pair(&json, key),
            num(key)
        );
    }
    assert!(field(&json, "laps").parse::<u64>().unwrap() >= 2);
}

/// `(re, im)` of a `biseries_x`/`diag` mantissa pair and exponent, scaled by 2^-`at`.
fn fx(l: &[String], at: i64) -> (f64, f64) {
    let e: i64 = l[2].parse().unwrap();
    let k = 2f64.powi((e - at) as i32);
    (
        l[0].parse::<f64>().unwrap() * k,
        l[1].parse::<f64>().unwrap() * k,
    )
}

/// `fd zone --emit lines` at `centre` against a committed zone file's return map:
/// `scale` and `bias`, then every `biseries`/`biseries_x`/`diag` value within `tol`.
/// Returns the worst relative difference.
fn compare_map(centre: (&str, &str), fixture: &str, tol: f64) -> f64 {
    let out = Command::new(env!("CARGO_BIN_EXE_fd"))
        .args(["zone", centre.0, centre.1, "--emit", "lines"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let got: Vec<Vec<String>> = text
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split_whitespace().map(String::from).collect())
        .collect();
    let want: Vec<Vec<String>> = std::fs::read_to_string(root().join(fixture))
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| l.split_whitespace().map(String::from).collect())
        .collect();
    // Lines keyed by their leading key and indices.
    let keyed = |v: &[Vec<String>], key: &str, idx: usize| -> Vec<(String, Vec<String>)> {
        v.iter()
            .filter(|l| l[0] == key)
            .map(|l| (l[1..1 + idx].join(" "), l[1 + idx..].to_vec()))
            .collect()
    };
    assert_eq!(line(&got, "scale"), line(&want, "scale"), "{fixture}");
    let (bg, bw) = (line(&got, "bias"), line(&want, "bias"));
    for k in 0..2 {
        let (g, w): (f64, f64) = (bg[k].parse().unwrap(), bw[k].parse().unwrap());
        assert!((g - w).abs() <= tol, "bias: {bg:?} vs {bw:?}");
    }
    let mut worst = 0.0f64;
    for (key, idx) in [("biseries", 2), ("biseries_x", 2), ("diag", 1)] {
        let (g, w) = (keyed(&got, key, idx), keyed(&want, key, idx));
        assert_eq!(g.len(), w.len(), "{fixture}: {key} count");
        for ((gi, gv), (wi, wv)) in g.iter().zip(&w) {
            assert_eq!(gi, wi, "{key} order");
            // `unit`: what one unit of the compared pair is worth (2^e for `_x` lines).
            let (a, b, unit) = if key == "biseries" {
                let p = |v: &[String]| (v[0].parse().unwrap(), v[1].parse().unwrap());
                (p(gv), p(wv), 1.0)
            } else {
                let at: i64 = wv[2].parse().unwrap();
                (fx(gv, at), fx(wv, at), 2f64.powi(at as i32))
            };
            if !(a.0.is_finite() && a.1.is_finite() && b.0.is_finite() && b.1.is_finite()) {
                // Deep rungs' plain `biseries` doubles overflow; their `_x` lines carry them.
                assert_eq!(gv, wv, "{key} {gi}");
                continue;
            }
            // A biseries term is judged by its contribution at the guard radius
            // R = guard/scale = 1e-3 (u, v units), against the leading a_01 R ~ R:
            // a_10 and Im a_01 are the nucleus residual (~1e-51 from the Python build's
            // unrefined 100-digit centre, ~1e-144 here), zero to every digit that renders.
            let r = if key.starts_with("biseries") {
                let deg: i32 = gi.split(' ').map(|n| n.parse::<i32>().unwrap()).sum();
                let d = (a.0 - b.0).hypot(a.1 - b.1) * unit;
                rel(a, b).min(d * 1e-3f64.powi(deg) / 1e-3)
            } else {
                rel(a, b)
            };
            worst = worst.max(r);
            assert!(r <= tol, "{fixture}: {key} {gi}: {gv:?} vs {wv:?} ({r:e})");
        }
    }
    worst
}

/// The v0 centre from bench/path-atlas-v0.txt.
fn v0_centre() -> (String, String) {
    let atlas = std::fs::read_to_string(root().join("bench/path-atlas-v0.txt")).unwrap();
    let f: Vec<String> = atlas
        .lines()
        .find(|l| !l.starts_with('#') && !l.trim().is_empty())
        .unwrap()
        .split_whitespace()
        .map(String::from)
        .collect();
    (f[0].clone(), f[1].clone())
}

#[test]
fn zone_biseries_v0() {
    // The committed v0 map was built in doubles (returns_exit_tail.py `build_map`),
    // measured within 1.5e-14 of an exact build.
    let (re, im) = v0_centre();
    let worst = compare_map(
        (&re, &im),
        "crates/fd-cli/tests/fixtures/zones/v0.zone",
        1e-12,
    );
    eprintln!("PROB-32 v0: worst biseries/diag relative difference {worst:e}");
}

#[test]
fn zone_biseries_rung409() {
    // The k = 409 ladder rung (period 1582) was built at high precision (`build_map_mp`).
    let rung = std::fs::read_to_string(root().join("tools/research/misiurewicz/ladder_rungs.txt"))
        .unwrap();
    let f: Vec<&str> = rung
        .lines()
        .find(|l| l.starts_with("409 "))
        .unwrap()
        .split_whitespace()
        .collect();
    let worst = compare_map(
        (f[2], f[3]),
        "crates/fd-cli/tests/fixtures/zones/rung409.zone",
        1e-30,
    );
    eprintln!("PROB-32 rung409: worst biseries/diag relative difference {worst:e}");
}
