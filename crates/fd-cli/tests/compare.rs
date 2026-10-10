//! BENC-01 benchmark tools on a short cut of the Atlas v0 path: `fd control --bla
//! per-frame` (each frame builds its own reference and BLA table, nothing carried) and
//! `fd compare` (sample-exact class agreement and `nu` displacement between two frame
//! directories).
use std::path::Path;
use std::process::{Command, Output};

fn fd(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fd")).args(args).output().unwrap()
}

fn ok(o: &Output) -> String {
    let text = String::from_utf8(o.stdout.clone()).unwrap();
    assert!(o.status.success(), "{text}\n{}", String::from_utf8_lossy(&o.stderr));
    text
}

/// The number after the first `"key":` in a flat scan of one JSON line.
fn num(json: &str, key: &str) -> f64 {
    let at = json.find(&format!("\"{key}\":")).unwrap_or_else(|| panic!("no {key} in {json}"));
    let rest = &json[at + key.len() + 3..];
    let end = rest.find([',', '}', ']']).unwrap();
    rest[..end].parse().unwrap_or_else(|_| panic!("{key}: {}", &rest[..end]))
}

#[test]
fn per_frame_bla_control_matches_plain_control() {
    let dir = std::env::temp_dir().join(format!("fd-compare-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let committed = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bench/path-atlas-v0.txt");
    let text = std::fs::read_to_string(committed).unwrap();
    let first = text.lines().find(|l| !l.starts_with('#')).unwrap();
    let (re, im) = (first.split(' ').next().unwrap(), first.split(' ').nth(1).unwrap());
    // An f64-tier frame (empty table) and deep fx frames (tables with valid blocks).
    let path = dir.join("cut.txt");
    let lines: String = ["1e-3", "1e-25", "1e-40", "2e-49"].iter().map(|w| format!("{re} {im} {w}\n")).collect();
    std::fs::write(&path, lines).unwrap();
    let p = path.to_str().unwrap();
    let grid = ["--size", "32x18", "--iter", "20000", "--threads", "2"];
    let (da, db) = (dir.join("A"), dir.join("B"));
    let (a, b) = (da.to_str().unwrap(), db.to_str().unwrap());
    ok(&fd(&[&["control", p, "-o", a][..], &grid].concat()));
    let out = ok(&fd(&[&["control", p, "--bla", "per-frame", "--runs", "2", "-o", b][..], &grid].concat()));
    let records: Vec<&str> = out.lines().collect();
    assert_eq!(records.len(), 5, "{out}");
    let (mut used, mut empty) = (0, 0);
    for rec in &records[..4] {
        for k in ["\"bla\":{\"mode\":\"per-frame\"", "\"atlas\":\"none\",\"cross_frame_reuse\":false", "\"state\":\"repeat\"", "\"deterministic\":true", "\"ok\":true}"] {
            assert!(rec.contains(k), "missing {k} in {rec}");
        }
        // The table is built inside the frame's clock, and the orbit computed fresh.
        let at = &rec[rec.find("\"bla\":{").unwrap()..];
        assert!(num(at, "reference_seconds") > 0.0 && num(at, "operator_seconds") > 0.0, "{rec}");
        if rec.contains("\"use\":\"used\"") {
            used += 1;
            assert!(num(at, "blocks") > 0.0 && num(at, "valid_blocks") > 0.0 && num(rec, "macro_operators_per_pixel") > 0.0, "{rec}");
            assert!(num(rec, "pixel_fraction") < 1.0, "{rec}");
        } else {
            assert!(rec.contains("\"use\":\"skipped_empty\""), "{rec}");
            empty += 1;
            assert_eq!(num(rec, "pixel_fraction"), 1.0, "{rec}");
        }
    }
    assert!(used > 0 && empty > 0, "used {used} empty {empty}\n{out}");
    let t = records[4];
    assert!(t.contains("\"bla\":\"per-frame\"") && t.contains("\"mode\":\"per-frame\""), "{t}");
    assert_eq!(num(t, "frames_used"), used as f64);
    assert_eq!(num(t, "frames_empty_table_skipped"), empty as f64);

    // Classes agree exactly; frames without a table are byte-identical to plain control.
    let cmp = ok(&fd(&["compare", a, b]));
    let lines: Vec<&str> = cmp.lines().collect();
    assert_eq!(lines.len(), 5, "{cmp}");
    let totals = lines[4];
    assert!(totals.starts_with("{\"schema\":\"fd-compare/1\",\"record\":\"totals\""), "{totals}");
    assert_eq!(num(totals, "class_mismatches"), 0.0);
    assert_eq!(num(totals, "frames_class_identical"), 4.0);
    assert!(num(totals, "frames_bytes_identical") >= empty as f64, "{totals}");
    assert!(lines[0].contains("\"bytes_identical\":true"), "{}", lines[0]);
    assert!(totals.ends_with("\"ok\":true}"), "{totals}");

    // A directory against itself: everything identical. A corrupted class: exit 1.
    let same = ok(&fd(&["compare", a, a]));
    assert_eq!(num(same.lines().last().unwrap(), "frames_bytes_identical"), 4.0);
    let f = db.join("frame-00001.fds");
    let mut bytes = std::fs::read(&f).unwrap();
    let h = fd_samples::Reader::open(&f).unwrap().header.encode().unwrap().len();
    bytes[h] ^= 1; // escaped <-> interior on sample 0
    std::fs::write(&f, bytes).unwrap();
    let bad = fd(&["compare", a, b]);
    assert_eq!(bad.status.code(), Some(1));
    let text = String::from_utf8(bad.stdout).unwrap();
    assert_eq!(num(text.lines().last().unwrap(), "class_mismatches"), 1.0, "{text}");

    for bad in [&["control", p, "--bla", "frame"][..], &["compare", a], &["control", p, "--every", "0"]] {
        assert_eq!(fd(bad).status.code(), Some(2), "{bad:?}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// Write one 2x1 frame (both samples escaped) into `dir` as `frame-00000.fds`.
fn synth(dir: &Path, width: &str, nu: [f64; 2], de: [f32; 2]) {
    use fd_samples::{Class, Column, ColumnSet, Evidence, Header, Kind, Samples, View};
    std::fs::create_dir_all(dir).unwrap();
    let columns = ColumnSet::of(&[Column::Class, Column::Nu, Column::De]);
    let view = View { center_re: "-0.75".into(), center_im: "0.1".into(), width: width.into(), rotation: 0.0 };
    let h = Header { minor: fd_samples::MINOR, columns, nx: 2, ny: 1, ss: 1, max_iter: 1000, escape_radius: 1e10, view, kernel: "test".into() };
    let mut s = Samples::alloc(2, columns);
    s.class = vec![Class::new(Kind::Escaped, Evidence::Heuristic); 2];
    s.nu = Some(nu.to_vec());
    s.de = Some(de.to_vec());
    fd_samples::write(std::fs::File::create(dir.join("frame-00000.fds")).unwrap(), &h, &s).unwrap();
}

#[test]
fn non_finite_values_and_deep_widths_fail() {
    let dir = std::env::temp_dir().join(format!("fd-compare-gate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let run = |name: &str, aw: &str, bw: &str, b_nu: [f64; 2], a_de: [f32; 2]| {
        let (a, b) = (dir.join(name).join("A"), dir.join(name).join("B"));
        synth(&a, aw, [10.5, 11.25], a_de);
        synth(&b, bw, b_nu, [1.0, 2.0]);
        fd(&["compare", a.to_str().unwrap(), b.to_str().unwrap()])
    };
    let good = [10.5, 11.25];
    let de = [1.0, 2.0];

    // Identical frames pass, at an f64-underflowing depth too.
    for w in ["1e-3", "1e-1000"] {
        let o = run(&format!("same{w}"), w, w, good, de);
        assert!(ok(&o).lines().last().unwrap().ends_with("\"ok\":true}"));
    }
    // Width agreement within a relative 1e-15 is the same view.
    let o = run("near", "1.0000000000000001e-1000", "1e-1000", good, de);
    let t = ok(&o);
    assert_eq!(num(t.lines().last().unwrap(), "non_finite"), 0.0, "{t}");

    // 1e-1000 vs 1e-2000: both are 0 as f64, but they are different views.
    let o = run("deep", "1e-1000", "1e-2000", good, de);
    assert_eq!(o.status.code(), Some(1));
    let text = String::from_utf8(o.stdout).unwrap();
    assert!(text.contains("widths differ"), "{text}");

    // Non-finite nu in B (NaN, +inf) and non-finite or negative de in A each fail.
    for (name, b_nu, a_de) in [
        ("nan", [f64::NAN, 11.25], de),
        ("inf", [10.5, f64::INFINITY], de),
        ("de-nan", good, [f32::NAN, 2.0]),
        ("de-neg", good, [1.0, -2.0]),
    ] {
        let o = run(name, "1e-40", "1e-40", b_nu, a_de);
        assert_eq!(o.status.code(), Some(1), "{name}");
        let text = String::from_utf8(o.stdout).unwrap();
        let totals = text.lines().last().unwrap();
        assert_eq!(num(totals, "non_finite"), 1.0, "{name}: {totals}");
        assert_eq!(num(totals, "class_mismatches"), 0.0, "{name}: {totals}");
        assert!(totals.ends_with("\"ok\":false}"), "{name}: {totals}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
