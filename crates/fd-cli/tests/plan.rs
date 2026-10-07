//! Drives the real `fd plan` binary: per-frame tile demand and first-use frames for a
//! known path, and every sample of every frame lands in a predicted tile.
use std::process::{Command, Output};

fn fd(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fd")).args(args).output().unwrap()
}

fn plan(name: &str, path: &str, flags: &[&str]) -> (Output, Vec<String>) {
    let dir = std::env::temp_dir().join(format!("fd-plan-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join(name);
    std::fs::write(&f, path).unwrap();
    let mut args = vec!["plan", f.to_str().unwrap()];
    args.extend_from_slice(flags);
    let out = fd(&args);
    let lines = String::from_utf8(out.stdout.clone()).unwrap().lines().map(String::from).collect();
    (out, lines)
}

fn value<'a>(lines: &'a [String], name: &str) -> &'a str {
    lines.iter().find_map(|l| l.strip_prefix(&format!("{name} "))).unwrap_or_else(|| panic!("no {name} in {lines:?}"))
}

#[test]
fn shallow_path_lists_exact_tiles_and_first_use() {
    // Whole root square, 8x8 samples, tiles of at most 4 samples: level 1 quadrants.
    // Frame 1 zooms into the upper-left quadrant: tile 2/0/0 .. 2/1/1 at level 2.
    let path = "# known path\n0 0 4\n-1 1 2\n0 0 4  # back out\n";
    let (out, l) = plan("shallow.txt", path, &["--size", "8x8", "--tile-px", "4"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(value(&l, "frames"), "3");
    assert_eq!(value(&l, "tiles"), "8");
    assert_eq!(value(&l, "demand"), "12");
    assert!(l.contains(&"frame 0 level 1 tiles 4 new 4".to_string()), "{l:?}");
    assert!(l.contains(&"frame 1 level 2 tiles 4 new 4".to_string()), "{l:?}");
    assert!(l.contains(&"frame 2 level 1 tiles 4 new 0".to_string()), "{l:?}");
    for t in ["1/0/0", "1/1/0", "1/0/1", "1/1/1"] {
        assert!(l.contains(&format!("tile {t} first 0 last 2 frames 2")), "{t}: {l:?}");
    }
    for t in ["2/0/0", "2/1/0", "2/0/1", "2/1/1"] {
        assert!(l.contains(&format!("tile {t} first 1 last 1 frames 1")), "{t}: {l:?}");
    }
    assert_eq!(value(&l, "checked_samples"), "192");
    assert_eq!(value(&l, "unpredicted"), "0");
}

#[test]
fn deep_rotated_zoom_is_fully_predicted() {
    // A zoom on a boundary point (0, i) from 1e-20 to 1e-1000, rotating as it goes.
    let mut path = String::new();
    for f in 0..40 {
        let exp = 20 + f * 25;
        path.push_str(&format!("0 1 1e-{exp} {}\n", f as f64 * 0.1));
    }
    let (out, l) = plan("deep.txt", &path, &["--size", "32x18", "--ss", "2", "--tile-px", "8"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(value(&l, "frames"), "40");
    assert_eq!(value(&l, "checked_samples"), (40 * 64 * 36).to_string());
    assert_eq!(value(&l, "unpredicted"), "0");
    // Every tile is new exactly once; first-use precedes last use.
    let new: usize = l.iter().filter(|s| s.starts_with("frame ")).map(|s| s.rsplit(' ').next().unwrap().parse::<usize>().unwrap()).sum();
    assert_eq!(new.to_string(), value(&l, "tiles"));
    // The last frame's centre tile, located exactly by `fd addr`, is first used there.
    let last = l.iter().find(|s| s.starts_with("frame 39 ")).unwrap();
    let level = last.split(' ').nth(3).unwrap();
    let loc = fd(&["addr", "locate", "--re", "0", "--im", "1", "--level", level]);
    let key = String::from_utf8(loc.stdout).unwrap().trim().strip_prefix("key ").unwrap().to_string();
    assert!(key.starts_with(&format!("{level}/")) && level.parse::<u32>().unwrap() > 3300, "{key}");
    assert!(l.contains(&format!("tile {key} first 39 last 39 frames 1")), "{key}: {l:?}");
}

#[test]
fn bad_path_is_refused() {
    let (out, _) = plan("bad.txt", "0 0\n", &["--size", "8x8"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("bad.txt:1"));
    let (out, _) = plan("empty.txt", "# nothing\n", &["--size", "8x8"]);
    assert!(!out.status.success());
}
