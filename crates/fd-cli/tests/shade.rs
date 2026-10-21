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

/// Runs a just-copied executable. On Linux a parallel test that forks while the
/// copy's write descriptor is open leaves it open in the child, and exec fails
/// with ETXTBSY until that child execs or exits, so retry briefly (FIX-57).
fn output_of_copied(command: &mut Command) -> std::process::Output {
    for _ in 0..50 {
        match command.output() {
            Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            result => return result.unwrap(),
        }
    }
    command.output().unwrap()
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

#[test]
fn preset_from_release_layout() {
    let dir = std::env::temp_dir().join(format!("fd-release-looks-{}", std::process::id()));
    let package = dir.join("release");
    let elsewhere = dir.join("elsewhere");
    std::fs::create_dir_all(package.join("looks")).unwrap();
    std::fs::create_dir_all(&elsewhere).unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../looks");
    for entry in std::fs::read_dir(source).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|ext| ext == "look") {
            std::fs::copy(&path, package.join("looks").join(path.file_name().unwrap())).unwrap();
        }
    }
    assert!(!package.join(".git").exists());
    let executable = package.join(format!("fd{}", std::env::consts::EXE_SUFFIX));
    std::fs::copy(env!("CARGO_BIN_EXE_fd"), &executable).unwrap();
    let samples = elsewhere.join("tiny.fds");
    fd(&[
        "render",
        "--re",
        "0",
        "--im",
        "0",
        "--width",
        "4",
        "--size",
        "8x4",
        "--ss",
        "1",
        "--iter",
        "64",
        "--threads",
        "1",
        "-o",
        samples.to_str().unwrap(),
    ]);
    let output = output_of_copied(
        Command::new(&executable)
            .current_dir(&elsewhere)
            .env_remove("FD_LOOKS")
            .args([
                "shade",
                "tiny.fds",
                "--preset",
                "master-ice",
                "-o",
                "shaded",
            ]),
    );
    assert!(
        output.status.success(),
        "release shade exited {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(elsewhere.join("shaded/tiny.studio.png").is_file());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn preset_survives_removed_build_worktree() {
    let dir = std::env::temp_dir().join(format!("fd-runtime-looks-{}", std::process::id()));
    std::fs::create_dir_all(dir.join(".git/worktrees/probe")).unwrap();
    std::fs::create_dir_all(dir.join("looks")).unwrap();
    let worktree = dir.join("scratch/probe");
    std::fs::create_dir_all(worktree.join("bin")).unwrap();
    std::fs::write(
        worktree.join(".git"),
        "gitdir: ../../.git/worktrees/probe\n",
    )
    .unwrap();
    std::fs::write(dir.join(".git/worktrees/probe/commondir"), "../..\n").unwrap();
    let preset = dir.join("looks/runtime-probe.look");
    std::fs::write(&preset, fd_shade::Look::builtin("ice").unwrap().to_text()).unwrap();
    let samples = dir.join("tiny.fds");
    fd(&[
        "render",
        "--re",
        "0",
        "--im",
        "0",
        "--width",
        "4",
        "--size",
        "8x4",
        "--ss",
        "1",
        "--iter",
        "64",
        "--threads",
        "1",
        "-o",
        samples.to_str().unwrap(),
    ]);
    let executable = worktree.join(format!("bin/fd{}", std::env::consts::EXE_SUFFIX));
    std::fs::copy(env!("CARGO_BIN_EXE_fd"), &executable).unwrap();
    let shade = |exe: &std::path::Path, cwd: &std::path::Path, name: &str, out: &str| {
        let output = output_of_copied(
            Command::new(exe)
                .current_dir(cwd)
                .env_remove("FD_LOOKS")
                .args([
                    "shade",
                    samples.to_str().unwrap(),
                    "--preset",
                    name,
                    "-o",
                    dir.join(out).to_str().unwrap(),
                ]),
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::fs::read(dir.join(out).join("tiny.studio.png")).unwrap()
    };
    let before = shade(&executable, &worktree, "runtime-probe", "before");
    let retained = dir.join(format!("fd{}", std::env::consts::EXE_SUFFIX));
    std::fs::rename(executable, &retained).unwrap();
    std::fs::remove_dir_all(worktree).unwrap();
    let after = shade(&retained, &dir, "runtime-probe", "after");
    let explicit = shade(&retained, &dir, preset.to_str().unwrap(), "explicit");
    assert_eq!(
        before, after,
        "removed build checkout must not lose its preset"
    );
    assert_eq!(
        after, explicit,
        "named preset must load the main checkout's file"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

/// Every channel within one level of the frozen reference (GPU exp, pow and cos are not
/// correctly rounded; a wrong band at a terraced edge is tens of levels).
fn within_one_level(what: &str, got: &[u8], want: &[u8], w: usize) -> u8 {
    assert_eq!(got.len(), want.len(), "{what}: size");
    let mut worst = 0;
    for (k, (&g, &c)) in got.iter().zip(want).enumerate() {
        worst = worst.max(g.abs_diff(c));
        assert!(
            g.abs_diff(c) <= 1,
            "{what}: pixel ({},{}) channel {}: WGSL {g}, frozen CPU {c}",
            k / 3 % w,
            k / 3 / w,
            k % 3
        );
    }
    worst
}

/// SHAD-11: `fd shade` writes the studio still (ss 2, lines, haze, breathing, flow) the
/// CPU studio wrote before it was deleted, within one level per channel. The reference,
/// tests/fixtures/studio-still.rgb, is that CPU output (`fd shade --gpu off` at fcab2b5).
#[test]
fn studio_matches_frozen_reference() {
    let dir = std::env::temp_dir().join(format!("fd-shade-frozen-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let fds = dir.join("still.fds");
    fd(&[
        "render",
        "--re",
        "-0.7453",
        "--im",
        "0.1127",
        "--width",
        "0.0065",
        "--size",
        "128x72",
        "--ss",
        "2",
        "--iter",
        "5000",
        "--threads",
        "2",
        "-o",
        fds.to_str().unwrap(),
    ]);
    let look = dir.join("still.look");
    std::fs::write(
        &look,
        "stops #08152e #0b3d91 #1e7fe0 #7cc4ff #eef6fc #c9c3bc #7d7a78 #2a3140\n\
         interior #05070d\ndensity 0.05\nterrace 0.5\nslope 1.1\nlight 135\nlines 1\n\
         line_px 2.2\nhaze #1e7fe0\nflow 0.11\nbreathe 0.08\nbrate 0.5\n",
    )
    .unwrap();
    let out = dir.join("out");
    let run = Command::new(env!("CARGO_BIN_EXE_fd"))
        .arg("shade")
        .arg(&fds)
        .arg("--preset")
        .arg(&look)
        .args(["--aa", "on", "--dither", "on", "--time", "0.7", "-o"])
        .arg(&out)
        .output()
        .unwrap();
    let log = String::from_utf8_lossy(&run.stderr);
    assert!(run.status.success(), "fd shade: {log}");
    let device = log.lines().find(|l| l.contains("GPU shading on"));
    eprintln!("{}", device.expect("fd shade names its wgpu adapter"));
    let png = out.join("still.studio.png");
    let rgb = Command::new("ffmpeg")
        .args(["-loglevel", "error", "-i"])
        .arg(&png)
        .args(["-f", "rawvideo", "-pix_fmt", "rgb24", "-"])
        .output()
        .expect("ffmpeg is required to decode the stills");
    assert!(rgb.status.success(), "decode {}", png.display());
    let frozen = include_bytes!("fixtures/studio-still.rgb");
    let worst = within_one_level("ss 2 still", &rgb.stdout, frozen, 128);
    eprintln!("ss 2 still: within {worst} level(s) of the frozen CPU studio");
    std::fs::remove_dir_all(&dir).unwrap();
}
