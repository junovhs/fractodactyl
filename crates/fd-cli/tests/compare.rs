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
