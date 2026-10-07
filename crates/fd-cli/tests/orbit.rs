//! Drives the real `fd orbit put` and `fd render --orbit`: reference orbits of several
//! slab sizes round-trip through the atlas store, report bytes per iteration, dedupe on
//! a second put, render bit-identical samples when loaded back, and refuse a view at
//! another centre.
use std::process::{Command, Output};

fn fd(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fd")).args(args).output().unwrap()
}

fn ok(args: &[&str]) -> Vec<(String, String)> {
    let out = fd(args);
    assert!(out.status.success(), "fd {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| {
            let (k, v) = l.split_once(' ').unwrap();
            (k.to_string(), v.to_string())
        })
        .collect()
}

fn get<'a>(lines: &'a [(String, String)], name: &str) -> &'a str {
    &lines.iter().find(|(k, _)| k == name).unwrap_or_else(|| panic!("no {name} in {lines:?}")).1
}

#[test]
fn slabs_round_trip_and_render_identically() {
    let dir = std::env::temp_dir().join(format!("fd-orbit-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let st = dir.join("atlas");
    let st = st.to_str().unwrap();
    // c = i stays bounded, so every view below stores the full 20001-point orbit:
    // a shallow f64-tier view and a deep fixed-point one (1e-30).
    let views = [("shallow", "1e-3", 53), ("deep", "1e-30", 233)];
    let mut f64_orbit = String::new();
    for (name, width, bits) in views {
        let view = ["--re", "0", "--im", "1", "--width", width, "--size", "32x24", "--iter", "20000"];
        let plain = dir.join(format!("{name}.fds"));
        ok(&[&["render"][..], &view, &["--threads", "2", "-o", plain.to_str().unwrap()]].concat());
        for size in ["2048", "4096", "8192"] {
            let put = ok(&[&["orbit", "put", "--store", st, "--slab", size][..], &view].concat());
            assert_eq!(get(&put, "points"), "20001");
            assert_eq!(get(&put, "precision"), bits.to_string());
            let n: usize = size.parse().unwrap();
            assert_eq!(get(&put, "slabs"), 20001usize.div_ceil(n).to_string());
            let bpi: f64 = get(&put, "bytes_per_iter").parse().unwrap();
            assert!(bpi > 16.0 && bpi < 16.1, "{bpi}");
            let again = ok(&[&["orbit", "put", "--store", st, "--slab", size][..], &view].concat());
            assert!(again.iter().filter(|(k, _)| k == "slab").all(|(_, v)| v.contains(" deduplicated ")));
            let ids = get(&put, "orbit").to_string();
            let loaded = dir.join(format!("{name}-{size}.fds"));
            let flags = ["--threads", "3", "--store", st, "--orbit", ids.as_str(), "-o", loaded.to_str().unwrap()];
            ok(&[&["render"][..], &view, &flags].concat());
            assert_eq!(std::fs::read(&plain).unwrap(), std::fs::read(&loaded).unwrap(), "{name} slab {size}");
            if bits == 53 {
                f64_orbit = ids;
            }
        }
    }
    // An orbit of the wrong precision for the view is refused.
    let deep = ["render", "--re", "0", "--im", "1", "--width", "1e-30", "--size", "32x24", "--iter", "20000"];
    let out = dir.join("wrong.fds");
    let wrong = fd(&[&deep[..], &["--store", st, "--orbit", f64_orbit.as_str(), "-o", out.to_str().unwrap()]].concat());
    assert!(!wrong.status.success());
    assert!(String::from_utf8_lossy(&wrong.stderr).contains("bits"));
    // An orbit is bound to its exact centre: the same centre written differently loads,
    // any other centre is refused.
    let shallow = ["render", "--re", "0.0", "--im", "1e0", "--width", "1e-3", "--size", "32x24", "--iter", "20000"];
    ok(&[&shallow[..], &["--store", st, "--orbit", f64_orbit.as_str(), "-o", out.to_str().unwrap()]].concat());
    let moved = ["render", "--re", "0", "--im", "0.99", "--width", "1e-3", "--size", "32x24", "--iter", "20000"];
    let moved = fd(&[&moved[..], &["--store", st, "--orbit", f64_orbit.as_str(), "-o", out.to_str().unwrap()]].concat());
    assert!(!moved.status.success());
    assert!(String::from_utf8_lossy(&moved.stderr).contains("centre"));
}
