//! Drives the real `fd control` on a known path: one fd-control/1 frame record per path
//! frame with the fd-bench/1 metrics, a totals record that sums them, every frame
//! rendered exactly as an independent `fd bench` render of that view, and no way to
//! hand it stored work.
use std::process::Command;

fn fd(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_fd"))
        .args(args)
        .output()
        .unwrap()
}

/// The number after the first `"key":` in a flat scan of one JSON line.
fn num(json: &str, key: &str) -> f64 {
    let at = json
        .find(&format!("\"{key}\":"))
        .unwrap_or_else(|| panic!("no {key} in {json}"));
    let rest = &json[at + key.len() + 3..];
    let end = rest.find([',', '}', ']']).unwrap();
    rest[..end]
        .parse()
        .unwrap_or_else(|_| panic!("{key}: {}", &rest[..end]))
}

/// The iteration total of the `"iterations"` object (`"total"` also names other sums).
fn iterations(json: &str) -> f64 {
    num(&json[json.find("\"iterations\":").unwrap()..], "total")
}

#[test]
fn renders_every_frame_independently() {
    let dir = std::env::temp_dir().join(format!("fd-control-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("path.txt");
    // f64, fixed-point and scaled frames; two share a centre (no reuse between them).
    let lines = [
        ("-0.7453", "0.1127", "0.0065"),
        ("0", "1", "1e-20"),
        ("0", "1", "1e-30"),
        ("0", "1", "1e-300"),
    ];
    let text: String = lines
        .iter()
        .map(|(r, i, w)| format!("{r} {i} {w}\n"))
        .collect();
    std::fs::write(&path, format!("# re im width\n{text}")).unwrap();
    let out_dir = dir.join("frames");
    let grid = ["--size", "24x12", "--iter", "20000", "--threads", "2"];
    let out = fd(&[
        &[
            "control",
            path.to_str().unwrap(),
            "--runs",
            "2",
            "-o",
            out_dir.to_str().unwrap(),
        ][..],
        &grid,
    ]
    .concat());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        out.status.success(),
        "{text}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let records: Vec<&str> = text.lines().collect();
    assert_eq!(records.len(), lines.len() + 1, "{text}");
    let mut sum = 0.0;
    for (f, (rec, (re, im, w))) in records.iter().zip(lines).enumerate() {
        assert!(
            rec.starts_with(&format!(
                "{{\"schema\":\"fd-control/1\",\"record\":\"frame\",\"frame\":{f},"
            )),
            "{rec}"
        );
        for s in [
            "\"run\":1,\"state\":\"cold\"",
            "\"run\":2,\"state\":\"repeat\"",
            "\"warm_seconds\":null,\"warm_statistic\":null",
            "\"peak_rss_scope\":\"run\"",
            "\"atlas\":\"none\",\"cross_frame_reuse\":false",
            "\"atlas_bytes_read\":0",
            "\"tiles_touched\":0",
            "\"deterministic\":true",
            "\"oracle\":null",
            "\"ok\":true}",
        ] {
            assert!(rec.contains(s), "missing {s} in {rec}");
        }
        assert!(num(rec, "cold_seconds") > 0.0 && num(rec, "precision_bits") >= 53.0);
        assert!(num(rec, "peak_rss_bytes") > 0.0 && num(rec, "sample_bytes") > 0.0);
        assert_eq!(num(rec, "pixel_fraction"), 1.0);
        let classes = num(rec, "escaped") + num(rec, "interior") + num(rec, "unresolved");
        assert_eq!(classes, 24.0 * 12.0);
        // Same render as an independent `fd bench` of the frame's view.
        let fds = out_dir.join(format!("frame-{f:05}.fds"));
        let bench_fds = dir.join("bench.fds");
        let b = fd(&[
            &[
                "bench",
                "--re",
                re,
                "--im",
                im,
                "--width",
                w,
                "--runs",
                "1",
                "-o",
                bench_fds.to_str().unwrap(),
            ][..],
            &grid,
        ]
        .concat());
        assert!(b.status.success());
        let b = String::from_utf8(b.stdout).unwrap();
        assert_eq!(iterations(rec), iterations(&b), "frame {f}");
        assert_eq!(
            std::fs::read(&fds).unwrap(),
            std::fs::read(&bench_fds).unwrap(),
            "frame {f}"
        );
        sum += iterations(rec);
    }
    let totals = records[lines.len()];
    assert!(
        totals.starts_with("{\"schema\":\"fd-control/1\",\"record\":\"totals\""),
        "{totals}"
    );
    assert_eq!(num(totals, "frames"), lines.len() as f64);
    assert_eq!(iterations(totals), sum);
    assert!(
        totals.contains("\"deterministic\":true") && totals.ends_with("\"ok\":true}"),
        "{totals}"
    );
    assert!(!totals.contains("\"kernel\"") && totals.contains("\"warm_seconds\":null"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn refuses_stored_work_and_unchecked_oracle() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../bench/path-valley.txt");
    for extra in [
        &["--store", "x", "--orbit", "y"][..],
        &["--bla", "y"],
        &["--re", "0"],
        &["--oracle", "tools/oracle.py"],
    ] {
        let out = fd(&[&["control", path][..], extra].concat());
        assert_eq!(out.status.code(), Some(2), "{extra:?}");
    }
}

#[test]
fn runtime_failure_exits_1_with_partial_totals() {
    let dir = std::env::temp_dir().join(format!("fd-control-fail-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("path.txt");
    // Line 2 parses as a path line but is not a renderable view.
    std::fs::write(&path, "-0.75 0.1 0.01\n0 1 not-a-width\n0 1 1e-3\n").unwrap();
    let out = fd(&[
        "control",
        path.to_str().unwrap(),
        "--size",
        "16x8",
        "--iter",
        "2000",
        "--threads",
        "2",
    ]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8(out.stdout).unwrap();
    let records: Vec<&str> = text.lines().collect();
    assert_eq!(records.len(), 2, "{text}");
    assert!(
        records[0].contains("\"frame\":0,") && records[0].ends_with("\"ok\":true}"),
        "{text}"
    );
    let totals = records[1];
    assert!(
        totals.contains("\"record\":\"totals\"") && totals.contains("\"frames\":1,"),
        "{totals}"
    );
    assert!(
        totals.contains("\"error\":\"frame 1 (line 2): ") && totals.ends_with("\"ok\":false}"),
        "{totals}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
