//! Guard the shipped renderer and film CLI while pruning dead research tooling.
use std::{fs, process::Command};

#[test]
fn first_atlas_frame_renders_and_film_remains_dispatched() {
    let path = include_str!("../../../bench/path-atlas-v0.txt");
    let first = path
        .lines()
        .find(|line| !line.starts_with('#') && !line.is_empty())
        .unwrap();
    let mut parts = first.split_whitespace();
    let re = parts.next().unwrap();
    let im = parts.next().unwrap();
    let width = parts.next().unwrap();
    let view = ["render", "--re", re, "--im", im, "--width", width];
    let opts = ["--size", "8x4", "--iter", "256", "--threads", "1", "-o"];
    let output = std::env::temp_dir().join(format!("fd-clea01-{}.fds", std::process::id()));
    let render = Command::new(env!("CARGO_BIN_EXE_fd"))
        .args(view)
        .args(opts)
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        render.status.success(),
        "render: {}",
        String::from_utf8_lossy(&render.stderr)
    );
    let bytes = fs::read(&output).unwrap();
    fs::remove_file(&output).unwrap();
    assert!(bytes.starts_with(b"FDSAMPLE"));

    let film = Command::new(env!("CARGO_BIN_EXE_fd"))
        .arg("film")
        .output()
        .unwrap();
    assert_eq!(film.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&film.stderr).contains("usage: fd film"));
}
