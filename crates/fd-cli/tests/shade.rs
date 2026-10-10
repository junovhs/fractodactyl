//! SHAD-01: several looks from one stored mathematical result, with no fractal
//! iterations. One render, then `fd shade` applies two looks to that one `.fds`;
//! both report the same input hash and zero iterations, and the input is untouched.
use std::process::Command;

fn fd(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_fd"))
        .args(args)
        .output()
        .unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        out.status.success(),
        "{args:?}\n{text}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    text
}

/// The token after `key` on a space-separated report line.
fn field<'a>(line: &'a str, key: &str) -> &'a str {
    let mut it = line.split(' ');
    it.find(|t| *t == key)
        .unwrap_or_else(|| panic!("no {key} in {line}"));
    it.next().unwrap()
}

#[test]
fn two_looks_from_one_result_without_iterating() {
    let dir = std::env::temp_dir().join(format!("fd-shade-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("frames")).unwrap();
    let fds = |n: &str| dir.join("frames").join(n).to_str().unwrap().to_string();
    let view = [
        "--re",
        "-0.7453",
        "--im",
        "0.1127",
        "--size",
        "32x18",
        "--ss",
        "2",
        "--iter",
        "5000",
        "--threads",
        "2",
    ];
    for (n, w) in [("frame-00000.fds", "0.0065"), ("frame-00001.fds", "0.003")] {
        fd(&[&["render", "--width", w, "-o", &fds(n)][..], &view].concat());
    }
    let one = fds("frame-00000.fds");
    let before = std::fs::read(&one).unwrap();
    let looks = dir.join("looks");
    let text = fd(&[
        "shade",
        &one,
        "--look",
        "umber,relief",
        "-o",
        looks.to_str().unwrap(),
    ]);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 4, "{text}");
    assert!(
        lines[0].starts_with("input ") && field(lines[0], "columns") == "class,nu,de,normal",
        "{text}"
    );
    let sha = field(lines[0], "sha256");
    assert_eq!(sha.len(), 64);
    for (line, look) in lines[1..3].iter().zip(["umber", "relief"]) {
        assert_eq!(field(line, "look"), look, "{line}");
        assert_eq!(field(line, "input_sha256"), sha, "{line}");
        assert_eq!(field(line, "iterations"), "0", "{line}");
        assert_eq!(field(line, "px"), "32x18", "{line}");
    }
    assert!(
        lines[3].starts_with("totals frames 1 looks 2 images 2 "),
        "{text}"
    );
    assert!(lines[3].ends_with(" iterations 0 kernel_calls 0"), "{text}");
    assert_eq!(
        std::fs::read(&one).unwrap(),
        before,
        "shading must not modify the stored result"
    );
    let png = |n: &str| std::fs::read(looks.join(n)).unwrap();
    let (umber, relief) = (png("frame-00000.umber.png"), png("frame-00000.relief.png"));
    assert!(
        umber.starts_with(b"\x89PNG") && umber != relief,
        "two distinct looks"
    );

    // A directory of path frames (as `fd control -o DIR` writes) recolours in one call.
    let out = dir.join("path-looks");
    let text = fd(&[
        "shade",
        dir.join("frames").to_str().unwrap(),
        "--look",
        "umber,palette",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(
        text.lines().filter(|l| l.starts_with("input ")).count(),
        2,
        "{text}"
    );
    assert!(
        text.lines()
            .last()
            .unwrap()
            .starts_with("totals frames 2 looks 2 images 4 "),
        "{text}"
    );
    assert!(out.join("frame-00001.palette.png").exists());

    // Unknown looks are refused; the legacy single-image form still works.
    let bad = Command::new(env!("CARGO_BIN_EXE_fd"))
        .args(["shade", &one, "--look", "bone", "-o", "x"])
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(2));
    let legacy = dir.join("legacy.png");
    fd(&["shade", "umber", &one, legacy.to_str().unwrap()]);
    assert_eq!(
        std::fs::read(&legacy).unwrap(),
        umber,
        "same look, same pixels"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
