//! FIX-04: real no-zone fd render and per-frame BLA at the deep v0 nucleus.
use fd_samples::{Kind, Reader};
use std::{path::Path, process::Command};
fn output(cmd: &mut Command) -> String {
    let o = cmd.output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8(o.stdout).unwrap()
}
fn all_interior(path: &Path) {
    let mut r = Reader::open(path).unwrap();
    assert!(!r.header.kernel.contains("zone"));
    let cols = r.header.columns;
    let s = r.read(cols).unwrap();
    assert!(
        s.class.iter().all(|c| c.kind() == Some(Kind::Interior)),
        "interior {}/{}",
        s.class
            .iter()
            .filter(|c| c.kind() == Some(Kind::Interior))
            .count(),
        s.class.len()
    );
}
#[test]
fn deep_nucleus_interior() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let text = std::fs::read_to_string(root.join("bench/path-atlas-v0.txt")).unwrap();
    let f: Vec<_> = text
        .lines()
        .find(|l| !l.starts_with('#') && !l.trim().is_empty())
        .unwrap()
        .split_whitespace()
        .collect();
    let dir = std::env::temp_dir().join(format!("fd-fix04-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let view = [
        "--re",
        f[0],
        "--im",
        f[1],
        "--width",
        "1e-90",
        "--iter",
        "100000",
        "--columns",
        "nu,de,normal",
        "--threads",
        "12",
    ];
    let probe = dir.join("probe.fds");
    output(
        Command::new(env!("CARGO_BIN_EXE_fd"))
            .arg("render")
            .args(view)
            .args(["--size", "16x9", "-o"])
            .arg(&probe),
    );
    all_interior(&probe); // tiny red/green probe before the full frame
    let full = dir.join("plain.fds");
    let report = output(
        Command::new(env!("CARGO_BIN_EXE_fd"))
            .arg("render")
            .args(view)
            .args(["--size", "960x540", "-o"])
            .arg(&full),
    );
    eprintln!("FIX-04 plain: {report}");
    all_interior(&full);
    let store = dir.join("store");
    let mut orbit = String::new();
    let mut bla = String::new();
    for (sub, key) in [("put", "orbit"), ("bla", "bla")] {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_fd"));
        cmd.args(["orbit", sub, "--store"])
            .arg(&store)
            .args(view)
            .args(["--size", "960x540"]);
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
    let fast = dir.join("bla.fds");
    let report = output(
        Command::new(env!("CARGO_BIN_EXE_fd"))
            .arg("render")
            .args(view)
            .args(["--size", "960x540", "--store"])
            .arg(&store)
            .args(["--bla", &bla, "-o"])
            .arg(&fast),
    );
    eprintln!("FIX-04 BLA: {report}");
    all_interior(&fast);
    // Exact exponent offsets also retain the new interior result below f64 range.
    let deep = dir.join("scaled.fds");
    output(
        Command::new(env!("CARGO_BIN_EXE_fd"))
            .arg("render")
            .args(view)
            .args(["--width", "1e-1000", "--size", "16x9", "-o"])
            .arg(&deep),
    );
    all_interior(&deep);
    // Keep the files for an optional external oracle run and report their paths.
    eprintln!(
        "FIX-04 oracle inputs: {} {}",
        full.display(),
        deep.display()
    );
}
