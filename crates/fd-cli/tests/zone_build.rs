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
    for (key, idx) in [
        ("biseries", 2),
        ("biseries_x", 2),
        ("diag", 1),
        ("phi", 1),
        ("psi", 1),
    ] {
        let (g, w) = (keyed(&got, key, idx), keyed(&want, key, idx));
        assert_eq!(g.len(), w.len(), "{fixture}: {key} count");
        for ((gi, gv), (wi, wv)) in g.iter().zip(&w) {
            assert_eq!(gi, wi, "{key} order");
            // `unit`: what one unit of the compared pair is worth (2^e for `_x` lines).
            let (a, b, unit) = if matches!(key, "biseries" | "phi" | "psi") {
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

#[test]
fn zone_koenigs_v0() {
    // phi/psi are compared inside compare_map (with the return map); the claim is
    // PROB-33's: v0's 12 phi and 18 psi terms within 1e-12 relative.
    let (re, im) = v0_centre();
    let worst = compare_map(
        (&re, &im),
        "crates/fd-cli/tests/fixtures/zones/v0.zone",
        1e-12,
    );
    eprintln!("PROB-33 v0: worst biseries/diag/phi/psi relative difference {worst:e}");
}

/// The patch_root/node/leaf lines of `fd zone --emit lines` at `depth` on the v0 centre.
fn v0_patch_lines(depth: usize) -> Vec<String> {
    let (re, im) = v0_centre();
    let out = Command::new(env!("CARGO_BIN_EXE_fd"))
        .args([
            "zone",
            &re,
            &im,
            "--emit",
            "lines",
            "--patch-depth",
            &depth.to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| {
            ["patch_root ", "node ", "leaf "]
                .iter()
                .any(|k| l.starts_with(k))
        })
        .map(String::from)
        .collect()
}

#[test]
fn zone_patches_v0() {
    // Depth 2 against tail_patches.py's tree (crates/fd-cli/tests/fixtures/zones/
    // v0_patches_depth2.txt): the same nodes, leaf geometry and step counts n, and each
    // leaf's polynomial within half the fit tolerance (TOL |c_1|, TOL = 1e-11) on its
    // covering disk |t| <= sqrt 2. Both builds are binary64; the FFT differs.
    let got = v0_patch_lines(2);
    let want: Vec<String> = std::fs::read_to_string(
        root().join("crates/fd-cli/tests/fixtures/zones/v0_patches_depth2.txt"),
    )
    .unwrap()
    .lines()
    .filter(|l| !l.starts_with('#'))
    .map(String::from)
    .collect();
    assert_eq!(got.len(), want.len());
    let (mut worst, mut flips) = (0.0f64, 0);
    for (g, w) in got.iter().zip(&want) {
        let (gf, wf): (Vec<&str>, Vec<&str>) = (
            g.split_whitespace().collect(),
            w.split_whitespace().collect(),
        );
        // Numbers within 1e-12 relative, integers (kid, leaf, nx, D) exact: the libm
        // log/exp/sin/cos differ in the last bit between Linux and Windows MSVC.
        assert_eq!(gf[0], wf[0]);
        let close = |a: &str, b: &str| match (a.parse::<i64>(), b.parse::<i64>()) {
            (Ok(x), Ok(y)) => x == y,
            _ => {
                let (x, y): (f64, f64) = (a.parse().unwrap(), b.parse().unwrap());
                (x - y).abs() <= 1e-12 * x.abs().max(y.abs()).max(1e-300)
            }
        };
        if gf[0] != "leaf" {
            assert!(
                gf.len() == wf.len() && gf[1..].iter().zip(&wf[1..]).all(|(a, b)| close(a, b)),
                "{g} vs {w}"
            );
            continue;
        }
        assert!((1..4).all(|i| close(gf[i], wf[i])), "leaf geometry: {g}");
        // n is the largest step count within TOL: a fit on its threshold can flip by one
        // step between platforms. Such a leaf is equally valid; skip its coefficients.
        let (gn, wn): (i64, i64) = (gf[4].parse().unwrap(), wf[4].parse().unwrap());
        assert!((gn - wn).abs() <= 1, "leaf n {gn} vs {wn}");
        if gn != wn {
            flips += 1;
            continue;
        }
        let c = |f: &[&str], k: usize| -> (f64, f64) {
            (f[5 + 2 * k].parse().unwrap(), f[6 + 2 * k].parse().unwrap())
        };
        let c1 = c(&wf, 1);
        let mut d = 0.0;
        for k in 0..16 {
            let (a, b) = (c(&gf, k), c(&wf, k));
            d += (a.0 - b.0).hypot(a.1 - b.1) * 2f64.sqrt().powi(k as i32);
        }
        worst = worst.max(d / (1e-11 * c1.0.hypot(c1.1)));
    }
    eprintln!(
        "PROB-34: depth-2 atlas, worst polynomial difference {worst:.3} TOL, {flips} n flips"
    );
    assert!(worst <= 0.5 && flips <= 2);
}

/// fd compare's strict bar between two frames: class mismatches, nu over 1e-3 px, DE
/// over 0.2%, normal over 0.2 degrees (where DE > 1e-3 px), and the worst nu in px.
fn strict_diff(
    a: &fd_samples::Samples,
    b: &fd_samples::Samples,
) -> (usize, usize, usize, usize, f64) {
    let (an, bn) = (a.nu.as_ref().unwrap(), b.nu.as_ref().unwrap());
    let (ad, bd) = (a.de.as_ref().unwrap(), b.de.as_ref().unwrap());
    let (aw, bw) = (a.normal.as_ref().unwrap(), b.normal.as_ref().unwrap());
    let mut o = (0, 0, 0, 0, 0.0f64);
    for i in 0..a.class.len() {
        if a.class[i].kind() != b.class[i].kind() {
            o.0 += 1;
            continue;
        }
        if !a.class[i].escaped() {
            continue;
        }
        let de = f64::from(ad[i]);
        let px = (an[i] - bn[i]).abs() * std::f64::consts::LN_2 * de / 2.0;
        o.4 = o.4.max(px);
        o.1 += usize::from(px.is_nan() || px > 1e-3);
        if de > 1e-3 {
            let rel = (f64::from(bd[i]) / de - 1.0).abs();
            let deg = f64::from(aw[i].wrapping_sub(bw[i]).min(bw[i].wrapping_sub(aw[i]))) * 360.0
                / 65536.0;
            o.2 += usize::from(rel.is_nan() || rel > 2e-3);
            o.3 += usize::from(deg > 0.2);
        }
    }
    o
}

fn read_fds(path: &Path) -> fd_samples::Samples {
    let mut r = fd_samples::Reader::open(path).unwrap();
    let cols = r.header.columns;
    r.read(cols).unwrap()
}

#[test]
fn zone_patches_render_v0() {
    // The full depth-6 atlas, appended to the committed v0 fixture zone (which has
    // none), renders PROB-08's six deep frames to the frozen CPU-zone frames (made
    // without patches) at the strict bar: the patches move nothing that renders.
    let patches = v0_patch_lines(6);
    let dir = std::env::temp_dir().join(format!("fd-prob34-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut zone =
        std::fs::read_to_string(root().join("crates/fd-cli/tests/fixtures/zones/v0.zone")).unwrap();
    for l in &patches {
        zone.push_str(l);
        zone.push('\n');
    }
    let zpath = dir.join("v0-rust-patches.zone");
    std::fs::write(&zpath, zone).unwrap();
    let (re, im) = v0_centre();
    let mut failed = Vec::new();
    for w in ["1e-35", "1e-38", "1e-40", "1e-43", "1e-46", "2e-48"] {
        let out = dir.join(format!("{w}.fds"));
        let o = Command::new(env!("CARGO_BIN_EXE_fd"))
            .args(["render", "--re", &re, "--im", &im, "--width", w])
            .args([
                "--size",
                "96x54",
                "--iter",
                "100000",
                "--columns",
                "nu,de,normal",
            ])
            .arg("--zone")
            .arg(&zpath)
            .arg("-o")
            .arg(&out)
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        let frozen = root().join(format!(
            "crates/fd-cli/tests/fixtures/zones/frames/v0-deep-{w}.fds"
        ));
        let d = strict_diff(&read_fds(&frozen), &read_fds(&out));
        eprintln!(
            "PROB-34 {w}: class {}, nu > 1e-3 px {}, de > 0.2% {}, normal > 0.2 deg {}; worst nu {:.1e} px",
            d.0, d.1, d.2, d.3, d.4
        );
        if (d.0, d.1, d.2, d.3) != (0, 0, 0, 0) {
            failed.push(w);
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(failed.is_empty(), "{failed:?}");
}

/// A fixture frame: (name, re, im, width, max_iter).
type FixtureFrame = (String, String, String, String, u64);

/// Write a zone with `fd zone -o` (optionally at `period`), then render each fixture
/// frame `(name, re, im, width, iter)` from it and hold it to the frozen frame at the
/// strict bar. Returns the failing frame names.
fn written_zone_renders(
    centre: (&str, &str),
    args: &[&str],
    frames: &[FixtureFrame],
    tag: &str,
) -> Vec<String> {
    let dir = std::env::temp_dir().join(format!("fd-prob12-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let zone = dir.join(format!("{tag}.zone"));
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_fd"));
    cmd.args(["zone", centre.0, centre.1, "-o"])
        .arg(&zone)
        .args(args);
    let o = cmd.output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    eprintln!(
        "PROB-12 {tag}: {}",
        String::from_utf8_lossy(&o.stdout).trim()
    );
    let mut failed = Vec::new();
    for (name, re, im, width, iter) in frames {
        let out = dir.join(format!("{name}.fds"));
        let o = Command::new(env!("CARGO_BIN_EXE_fd"))
            .args(["render", "--re", re, "--im", im, "--width", width])
            .args([
                "--size",
                "96x54",
                "--iter",
                &iter.to_string(),
                "--columns",
                "nu,de,normal",
            ])
            .arg("--zone")
            .arg(&zone)
            .arg("-o")
            .arg(&out)
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        let frozen = root().join(format!(
            "crates/fd-cli/tests/fixtures/zones/frames/{name}.fds"
        ));
        let d = strict_diff(&read_fds(&frozen), &read_fds(&out));
        eprintln!(
            "PROB-12 {name}: class {}, nu > 1e-3 px {}, de > 0.2% {}, normal > 0.2 deg {}; worst nu {:.1e} px",
            d.0, d.1, d.2, d.3, d.4
        );
        if (d.0, d.1, d.2, d.3) != (0, 0, 0, 0) {
            failed.push(name.clone());
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    failed
}

/// The ladder rung `k`'s centre, period and fixture frames (zone_fixtures.rs: 5000, 500,
/// 50 and 5 minibrot sizes, max_iter 40 periods).
fn rung_frames(k: u32, size: &str) -> (String, String, u64, Vec<FixtureFrame>) {
    let rungs = std::fs::read_to_string(root().join("tools/research/misiurewicz/ladder_rungs.txt"))
        .unwrap();
    let line = rungs
        .lines()
        .find(|l| l.starts_with(&format!("{k} ")))
        .unwrap();
    let f: Vec<&str> = line.split_whitespace().collect();
    let period: u64 = f[1].parse().unwrap();
    let (m, e) = size.split_once('e').unwrap();
    let frames = [5000.0, 500.0, 50.0, 5.0]
        .iter()
        .map(|mult| {
            (
                format!("rung{k}-{mult}"),
                f[2].to_string(),
                f[3].to_string(),
                format!("{}e{e}", m.parse::<f64>().unwrap() * mult),
                40 * period,
            )
        })
        .collect();
    (f[2].to_string(), f[3].to_string(), period, frames)
}

#[test]
fn zone_written_renders() {
    // v0 from its centre alone (period by record return), then the k = 409 rung.
    let (re, im) = v0_centre();
    let deep: Vec<_> = ["1e-35", "1e-38", "1e-40", "1e-43", "1e-46", "2e-48"]
        .iter()
        .map(|w| {
            (
                format!("v0-deep-{w}"),
                re.clone(),
                im.clone(),
                w.to_string(),
                100_000,
            )
        })
        .collect();
    let mut failed = written_zone_renders((&re, &im), &[], &deep, "v0");
    // The tail atlas only saves steps; depth 2 keeps the default test run short.
    let (r, i, p, frames) = rung_frames(409, "9.2e-101");
    let args = ["--period", &p.to_string(), "--patch-depth", "2"].map(String::from);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    failed.extend(written_zone_renders((&r, &i), &args, &frames, "rung409"));
    assert!(failed.is_empty(), "{failed:?}");
}

#[test]
#[ignore = "builds a 2088-digit, period-16116 zone (about 66 s release); run with --ignored"]
fn zone_written_renders_rung7676() {
    let (r, i, p, frames) = rung_frames(7676, "1.01e-1000");
    let failed = written_zone_renders((&r, &i), &["--period", &p.to_string()], &frames, "rung7676");
    assert!(failed.is_empty(), "{failed:?}");
}

#[test]
fn zone_other_relation() {
    // The leftmost period-128 nucleus approaches M(2,1) = -2. Its initial
    // displacement is (4*pi^2/3) * 4^-P; Newton refines the exact centre in fd zone.
    let bits = bits_for_digits(240);
    let re = Float::from_i64(-2, bits)
        .add(&Float::parse("13.1594725348", bits).unwrap().ldexp(-256))
        .to_decimal(230);
    let dir = std::env::temp_dir().join(format!("fd-prob36-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let zone = dir.join("other.zone");
    let o = Command::new(env!("CARGO_BIN_EXE_fd"))
        .args([
            "zone",
            &re,
            "0",
            "--period",
            "128",
            "--patch-depth",
            "2",
            "-o",
        ])
        .arg(&zone)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let report = String::from_utf8_lossy(&o.stdout);
    eprintln!("{report}");
    assert_eq!(field(&report, "q"), "2");
    assert_eq!(field(&report, "r"), "1");
    assert!(field(&report, "laps").parse::<usize>().unwrap() >= 100);
    let text = std::fs::read_to_string(&zone).unwrap();
    let centre: Vec<_> = text
        .lines()
        .find(|l| l.starts_with("c_exact "))
        .unwrap()
        .split_whitespace()
        .skip(1)
        .collect();
    // An exterior frame near the nucleus avoids the separate deep-interior
    // classification gap tracked in FIX-04. Every pixel must still match BLA.
    let view = [
        "--re",
        centre[0],
        "--im",
        "2e-152",
        "--width",
        "1e-152",
        "--size",
        "480x270",
        "--iter",
        "10000",
        "--columns",
        "nu,de,normal",
        "--threads",
        "4",
    ];
    let store = dir.join("store");
    let mut orbit = String::new();
    let mut bla = String::new();
    for (sub, key) in [("put", "orbit"), ("bla", "bla")] {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_fd"));
        cmd.args(["orbit", sub, "--store"]).arg(&store).args(view);
        if sub == "bla" {
            cmd.args(["--orbit", &orbit]);
        }
        let o = cmd.output().unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        let report = String::from_utf8_lossy(&o.stdout);
        let id = report
            .lines()
            .find_map(|l| l.strip_prefix(&format!("{key} ")))
            .unwrap()
            .to_string();
        if sub == "put" {
            orbit = id;
        } else {
            bla = id;
        }
    }
    for mode in [None, Some("f32only")] {
        let out = dir.join(if mode.is_none() {
            "bla.fds"
        } else {
            "zone.fds"
        });
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_fd"));
        cmd.args([
            "render",
            "--re",
            centre[0],
            "--im",
            "2e-152",
            "--width",
            "1e-152",
            "--size",
            "480x270",
            "--iter",
            "10000",
            "--columns",
            "nu,de,normal",
            "--threads",
            "4",
        ]);
        if let Some(mode) = mode {
            cmd.arg("--zone").arg(&zone).args(["--gpu-kernel", mode]);
        } else {
            cmd.arg("--store").arg(&store).args(["--bla", &bla]);
        }
        let o = cmd.arg("-o").arg(&out).output().unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        if mode.is_some() {
            let reader = fd_samples::Reader::open(&out).unwrap();
            assert!(
                reader.header.kernel.contains("wgpu"),
                "{}",
                reader.header.kernel
            );
        }
    }
    let truth = read_fds(&dir.join("bla.fds"));
    assert!(
        truth.class.iter().any(|c| c.escaped()),
        "must exercise escaping pixels"
    );
    let d = strict_diff(&truth, &read_fds(&dir.join("zone.fds")));
    eprintln!("M(2,1) whole frame: {d:?}");
    assert_eq!((d.0, d.1, d.2, d.3), (0, 0, 0, 0));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn zone_suggest_three_pass() {
    // M(24,2), independently Newton-refined with mpmath at 190 digits.
    let re = "-0.7432918908524302029316243259725107571763485581207765818323304541609283472574785977186532571331974372312015364725883055839296440736897644315554785878367182908611270251456349093";
    let im = "0.1312405523087976047708459065814781430771141511585840067313339426441224485804463828704809547062528759706770520637320224159702666378411381293727697380576634323825795329443433122";
    let output = |cmd: &mut Command| {
        let o = cmd.output().unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        String::from_utf8(o.stdout).unwrap()
    };
    let text = output(Command::new(env!("CARGO_BIN_EXE_fd")).args([
        "zone",
        "--suggest",
        re,
        im,
        "--count",
        "3",
    ]));
    let rows: Vec<_> = text.lines().collect();
    assert_eq!(rows.len(), 3, "{text}");
    let dir = std::env::temp_dir().join(format!("fd-prob27-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut periods = std::collections::BTreeSet::new();
    for (i, row) in rows.iter().enumerate() {
        let period = field(row, "period");
        assert!(periods.insert(period.to_string()), "duplicate target");
        assert!(field(row, "laps").parse::<usize>().unwrap() >= 100);
        let start = row.find("\"c_exact\":[\"").unwrap() + "\"c_exact\":[\"".len();
        let coords: Vec<_> = row[start..].split('"').collect();
        let (re, im) = (coords[0], coords[2]);
        let (mant, exponent) = field(row, "end_width").split_once('e').unwrap();
        // The end width is five minibrot sizes. Compare a centered deep frame
        // 5000 sizes wide, whose sample spacing exceeds the minibrot's size.
        let width = format!("{}e{exponent}", mant.parse::<f64>().unwrap() * 1000.0);
        let zone = dir.join(format!("{i}.zone"));
        output(
            Command::new(env!("CARGO_BIN_EXE_fd"))
                .args([
                    "zone",
                    re,
                    im,
                    "--period",
                    period,
                    "--patch-depth",
                    "2",
                    "-o",
                ])
                .arg(&zone),
        );
        let iter = (period.parse::<usize>().unwrap() * 100).to_string();
        let view = [
            "--re",
            re,
            "--im",
            im,
            "--width",
            &width,
            "--size",
            "480x270",
            "--iter",
            &iter,
            "--columns",
            "nu,de,normal",
            "--threads",
            "4",
        ];
        let store = dir.join(format!("store-{i}"));
        let mut orbit = String::new();
        let mut bla = String::new();
        for (sub, key) in [("put", "orbit"), ("bla", "bla")] {
            let mut cmd = Command::new(env!("CARGO_BIN_EXE_fd"));
            cmd.args(["orbit", sub, "--store"]).arg(&store).args(view);
            if sub == "bla" {
                cmd.args(["--orbit", &orbit]);
            }
            let report = output(&mut cmd);
            let id = report
                .lines()
                .find_map(|l| l.strip_prefix(&format!("{key} ")))
                .unwrap()
                .to_string();
            if sub == "put" {
                orbit = id;
            } else {
                bla = id;
            }
        }
        for fast in [false, true] {
            let out = dir.join(format!("{i}-{fast}.fds"));
            let mut cmd = Command::new(env!("CARGO_BIN_EXE_fd"));
            cmd.arg("render").args(view);
            if fast {
                cmd.arg("--zone")
                    .arg(&zone)
                    .args(["--gpu-kernel", "f32only"]);
            } else {
                cmd.arg("--store").arg(&store).args(["--bla", &bla]);
            }
            output(cmd.arg("-o").arg(&out));
            if fast {
                let r = fd_samples::Reader::open(&out).unwrap();
                assert!(r.header.kernel.contains("wgpu"), "{}", r.header.kernel);
            }
        }
        let truth = read_fds(&dir.join(format!("{i}-false.fds")));
        assert!(truth.class.iter().any(|c| c.escaped()));
        let d = strict_diff(&truth, &read_fds(&dir.join(format!("{i}-true.fds"))));
        eprintln!(
            "PROB-27 period {period}, laps {}, width {width}: {d:?}",
            field(row, "laps")
        );
        assert_eq!((d.0, d.1, d.2, d.3), (0, 0, 0, 0));
    }
    std::fs::remove_dir_all(dir).unwrap();
}
