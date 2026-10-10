//! FIX-01: a de or normal miss fails the oracle only if it is also more than a `--px`
//! equivalent displacement in output pixels (docs/spec/SAMPLES.md, "Oracle tolerances").
//! Needs python3 with mpmath; skipped (with a note) where it is missing.
use fd_samples::{ColumnSet, Reader};
use std::path::{Path, PathBuf};
use std::process::Command;

fn fd(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_fd"))
        .args(args)
        .output()
        .unwrap()
}

fn oracle_script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/oracle.py")
}

fn have_mpmath() -> bool {
    let ok = Command::new("python3")
        .args(["-c", "import mpmath"])
        .output()
        .is_ok_and(|o| o.status.success());
    if !ok {
        eprintln!("skipped: python3 with mpmath not available");
    }
    ok
}

/// The number after `"key":` in a flat scan of the report.
fn num(json: &str, key: &str) -> f64 {
    let at = json
        .find(&format!("\"{key}\":"))
        .unwrap_or_else(|| panic!("no {key} in {json}"));
    let rest = json[at + key.len() + 3..].trim_start();
    let end = rest.find([',', '}', ']']).unwrap();
    rest[..end]
        .parse()
        .unwrap_or_else(|_| panic!("{key}: {}", &rest[..end]))
}

/// The 1e-9 frame of bench/path-valley.txt on the f64 tier.
const VALLEY: [&str; 12] = [
    "--re",
    "-0.743643887037158704752191506114774",
    "--im",
    "0.131825904205311970493132056385139",
    "--width",
    "1e-9",
    "--size",
    "32x18",
    "--iter",
    "20000",
    "--columns",
    "nu,de,normal",
];

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fd-oracle-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Sample (24, 13) sits 1.4e-4 px from the set, where the normal turns by ~8000 rad per
/// pixel: the f64 tier's sub-1e-5 px placement error shows as ~4e-3 rad. That is within
/// contract, and the displacement gate passes it.
#[test]
fn ill_conditioned_normal_passes_as_displacement() {
    if !have_mpmath() {
        return;
    }
    let d = dir("valley");
    let out_fds = d.join("valley.fds");
    let script = oracle_script();
    let args = [
        &["bench"][..],
        &VALLEY,
        &["--runs", "1", "--threads", "2", "--k", "2"],
    ]
    .concat();
    let out = fd(&[
        &args[..],
        &[
            "-o",
            out_fds.to_str().unwrap(),
            "--oracle",
            script.to_str().unwrap(),
        ],
    ]
    .concat());
    let json = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{json}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(json.contains("\"kernel\":\"pert-f64/1\""), "{json}");
    assert!(
        num(&json, "normal_err") > 1e-3,
        "the ill-conditioned sample should still be probed: {json}"
    );
    assert!(num(&json, "normal_px_err") < 1e-5, "{json}");
    std::fs::remove_dir_all(&d).ok();
}

/// VIDE-02's frame 150 of bench/path-atlas-v0.txt (f64 tier): the centre's f64 rounding
/// shifts every sample by ~2e-5 px, so a sample 7.5e-8 px from the set reads de 181x
/// too large; as a displacement it is ~1e-5 px, and every column passes.
#[test]
fn atlas_frame_150_passes_as_displacement() {
    if !have_mpmath() {
        return;
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bench/path-atlas-v0.txt");
    let text = std::fs::read_to_string(path).unwrap();
    let line = text
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .nth(150)
        .unwrap();
    let v: Vec<&str> = line.split_whitespace().collect();
    let d = dir("atlas150");
    let out_fds = d.join("f150.fds");
    let script = oracle_script();
    let args = [
        "bench", "--re", v[0], "--im", v[1], "--width", v[2], "--size", "960x540", "--iter",
        "100000",
    ];
    let rest = [
        "--runs",
        "1",
        "--threads",
        "2",
        "-o",
        out_fds.to_str().unwrap(),
        "--oracle",
        script.to_str().unwrap(),
    ];
    let out = fd(&[&args[..], &rest].concat());
    let json = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{json}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(json.contains("\"kernel\":\"pert-f64/1\""), "{json}");
    assert!(
        num(&json, "de_rel_err") > 1e-2 && num(&json, "normal_err") > 1e-3,
        "{json}"
    );
    for key in ["nu_px_err", "de_px_err", "normal_px_err"] {
        assert!(num(&json, key) < 1e-4, "{key}: {json}");
    }
    std::fs::remove_dir_all(&d).ok();
}

/// The gate is not vacuous: turning every stored normal by 100 u16 steps (~0.01 rad)
/// fails on the well-conditioned samples.
#[test]
fn wrong_normals_fail() {
    if !have_mpmath() {
        return;
    }
    let d = dir("wrong");
    let path = d.join("fx.fds");
    let args = [
        &["render"][..],
        &VALLEY,
        &[
            "--kernel",
            "fx",
            "--threads",
            "2",
            "-o",
            path.to_str().unwrap(),
        ],
    ]
    .concat();
    let out = fd(&args);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let oracle = |p: &Path| {
        Command::new("python3")
            .arg(oracle_script())
            .arg(p)
            .args(["--k", "2"])
            .output()
            .unwrap()
    };
    let good = oracle(&path);
    assert!(
        good.status.success(),
        "{}",
        String::from_utf8_lossy(&good.stdout)
    );

    let mut r = Reader::open(&path).unwrap();
    let header = r.header.clone();
    let mut s = r.read(ColumnSet(header.columns.0)).unwrap();
    for a in s.normal.as_mut().unwrap() {
        *a = a.wrapping_add(100);
    }
    let bad_path = d.join("bad.fds");
    fd_samples::write(std::fs::File::create(&bad_path).unwrap(), &header, &s).unwrap();
    let bad = oracle(&bad_path);
    let json = String::from_utf8_lossy(&bad.stdout);
    assert!(!bad.status.success(), "{json}");
    assert!(num(&json, "normal_px_err") > 1e-3, "{json}");
    std::fs::remove_dir_all(&d).ok();
}
