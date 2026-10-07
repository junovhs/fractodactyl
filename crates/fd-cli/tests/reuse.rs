//! Drives the real `fd reuse` on a known path: frames at one exact centre share one
//! stored orbit (the deepest frame's), a frame at another centre falls back to its own,
//! and frames whose precision matches the shared orbit render bit-identically.
use std::process::Command;

#[test]
fn path_frames_share_orbits() {
    let dir = std::env::temp_dir().join(format!("fd-reuse-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("path.txt");
    // Two f64-tier frames, three deep frames at the same centre written two ways, and
    // one frame elsewhere.
    std::fs::write(&path, "# re im width\n0 1 1e-2\n0 1 1e-3\n0 1.0 1e-20\n0 1 1e-30\n0 1 1e-40\n-0.75 0.1 0.01\n").unwrap();
    let st = dir.join("atlas");
    let out = Command::new(env!("CARGO_BIN_EXE_fd"))
        .args(["reuse", path.to_str().unwrap(), "--store", st.to_str().unwrap()])
        .args(["--size", "32x24", "--iter", "20000", "--columns", "nu,de,bound", "--threads", "2"])
        .output()
        .unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(out.status.success(), "{text}\n{}", String::from_utf8_lossy(&out.stderr));
    let get = |k: &str| text.lines().find_map(|l| l.strip_prefix(&format!("{k} "))).unwrap_or_else(|| panic!("no {k}: {text}"));
    assert_eq!(get("frames"), "6");
    assert_eq!(get("orbits"), "3");
    assert_eq!(get("reused"), "3");
    assert_eq!(get("fallback"), "1");
    let frames: Vec<Vec<&str>> = text.lines().filter(|l| l.starts_with("frame ")).map(|l| l.split(' ').collect()).collect();
    let field = |f: &[&str], k: &str| f[f.iter().position(|x| *x == k).unwrap() + 1].to_string();
    let roles: Vec<&str> = frames.iter().map(|f| f[4]).collect();
    assert_eq!(roles, ["lead", "reused", "reused", "reused", "lead", "fallback"]);
    for f in &frames {
        let (need, bits): (u32, u32) = (field(f, "need_bits").parse().unwrap(), field(f, "orbit_bits").parse().unwrap());
        assert!(bits >= need, "{f:?}");
        if bits == need {
            assert_eq!(field(f, "differing"), "0", "{f:?}");
        }
    }
    // The deep frames all use the 1e-40 frame's orbit, at more precision than they need.
    assert!(frames[2..5].iter().all(|f| field(f, "orbit_bits") == field(&frames[4], "need_bits")));
    assert!(field(&frames[2], "need_bits").parse::<u32>().unwrap() < field(&frames[4], "need_bits").parse().unwrap());
}
