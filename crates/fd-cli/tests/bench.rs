//! Drives the real `fd bench` binary: one JSON report with cold and warm runs and every
//! oracle-independent metric populated.
use std::process::Command;

fn fd(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_fd"))
        .args(args)
        .output()
        .unwrap()
}

/// The number after `"key":` in a flat scan of the report.
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

#[test]
fn reports_baseline_metrics() {
    let view = [
        "--re", "-0.7453", "--im", "0.1127", "--width", "0.0065", "--size", "32x18", "--ss", "2",
    ];
    let out = fd(&[
        &["bench"][..],
        &view,
        &["--iter", "5000", "--runs", "3", "--threads", "2"],
    ]
    .concat());
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json = String::from_utf8(out.stdout).unwrap();
    let json = json.trim();
    assert!(
        json.starts_with("{\"schema\":\"fd-bench/1\"")
            && json.ends_with('}')
            && !json.contains('\n')
    );
    for s in [
        "\"run\":1,\"state\":\"cold\"",
        "\"run\":3,\"state\":\"warm\"",
        "\"atlas\":\"none\"",
        "\"kernel\":\"pert-f64/1\"",
        "\"warm_statistic\":\"median\"",
        "\"peak_rss_scope\":\"run\"",
        "\"deterministic\":true",
        "\"oracle\":null",
        "\"ok\":true",
    ] {
        assert!(json.contains(s), "missing {s} in {json}");
    }
    assert!(num(json, "cold_seconds") > 0.0 && num(json, "warm_seconds") > 0.0);
    assert_eq!(num(json, "pixels"), 32.0 * 18.0);
    let total = num(json, "total");
    assert!(total > 0.0 && (num(json, "per_pixel") - total / 576.0).abs() < 1e-9);
    assert_eq!(num(json, "pixel_fraction"), 1.0);
    assert!(num(json, "reference_length") > 1.0 && num(json, "fds_bytes") > 0.0);
    let classes = num(json, "escaped") + num(json, "interior") + num(json, "unresolved");
    assert_eq!(classes, 64.0 * 36.0);
}

#[test]
fn iterations_do_not_depend_on_threads() {
    let base = [
        "bench", "--re", "0", "--im", "1", "--width", "1e-300", "--size", "24x12", "--iter",
        "20000", "--runs", "1",
    ];
    let run = |t: &str| {
        let out = fd(&[&base[..], &["--threads", t]].concat());
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let json = String::from_utf8(out.stdout).unwrap();
        assert!(json.contains("\"kernel\":\"pert-fx-scaled/1"), "{json}");
        assert!(
            json.contains("\"warm_seconds\":null,\"warm_statistic\":null"),
            "{json}"
        );
        num(&json, "total")
    };
    assert_eq!(run("1"), run("3"));
}

#[test]
fn oracle_needs_an_output_file() {
    let out = fd(&[
        "bench",
        "--re",
        "0",
        "--im",
        "0",
        "--width",
        "4",
        "--oracle",
        "tools/oracle.py",
    ]);
    assert_eq!(out.status.code(), Some(2));
}
