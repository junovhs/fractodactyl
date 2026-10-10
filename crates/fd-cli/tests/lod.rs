//! Drives the real `fd lod` binary: the screen-space error contract (docs/spec/LOD.md)
//! applied per tile to a real render and to samples carrying stronger evidence.
use fd_samples::{write, Class, Column, ColumnSet, Evidence, Header, Kind, Samples, View, MINOR};
use std::path::PathBuf;
use std::process::Command;

fn fd(args: &[&str]) -> Vec<String> {
    let out = Command::new(env!("CARGO_BIN_EXE_fd"))
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(String::from)
        .collect()
}

fn file(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fd-lod-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn value<'a>(lines: &'a [String], name: &str) -> &'a str {
    lines
        .iter()
        .find_map(|l| l.strip_prefix(&format!("{name} ")))
        .unwrap_or_else(|| panic!("no {name} in {lines:?}"))
}

#[test]
fn heuristic_render_never_accepts() {
    // Without the bound column kernels write Heuristic evidence only: every tile refines.
    let f = file("render.fds");
    let f = f.to_str().unwrap();
    fd(&[
        "render",
        "--re",
        "-0.75",
        "--im",
        "0.1",
        "--width",
        "0.05",
        "--size",
        "16x16",
        "--threads",
        "2",
        "-o",
        f,
    ]);
    let l = fd(&["lod", f, "--tile-px", "8"]);
    assert_eq!(value(&l, "tiles"), "4");
    assert_eq!(value(&l, "accepted"), "0");
    assert_eq!(value(&l, "state.unresolved_boundary"), "4");
    assert_eq!(
        l.iter()
            .filter(|x| x.starts_with("tile ") && x.contains(" e_px inf refine "))
            .count(),
        4,
        "{l:?}"
    );
}

#[test]
fn bounded_render_accepts_tiles_away_from_the_set() {
    // `--columns ...,bound` tracks rigorous error radii: escaped samples far from the set
    // become Bounded and their tiles are accepted; tiles holding the set still refine.
    // Both tiers that carry bounds: f64 (auto here) and fixed-point.
    for kernel in ["auto", "fx"] {
        let f = file(&format!("bounded-{kernel}.fds"));
        let f = f.to_str().unwrap();
        fd(&[
            "render",
            "--re",
            "0.5",
            "--im",
            "0",
            "--width",
            "1",
            "--size",
            "32x32",
            "--columns",
            "nu,de,bound",
            "--kernel",
            kernel,
            "--threads",
            "2",
            "-o",
            f,
        ]);
        let l = fd(&["lod", f, "--tile-px", "8"]);
        assert_eq!(value(&l, "tiles"), "16");
        let approx: u32 = value(&l, "state.certified_approximate").parse().unwrap();
        let accepted: u32 = value(&l, "accepted").parse().unwrap();
        let unresolved: u32 = value(&l, "state.unresolved_boundary").parse().unwrap();
        assert!(
            approx > 0 && accepted > 0 && unresolved > 0,
            "{kernel}: {l:?}"
        );
        // Without the bound column the same render stays heuristic.
        fd(&[
            "render",
            "--re",
            "0.5",
            "--im",
            "0",
            "--width",
            "1",
            "--size",
            "32x32",
            "--columns",
            "nu,de",
            "--kernel",
            kernel,
            "--threads",
            "2",
            "-o",
            f,
        ]);
        assert_eq!(
            value(&fd(&["lod", f, "--tile-px", "8"]), "accepted"),
            "0",
            "{kernel}"
        );
    }
}

#[test]
fn evidence_decides_each_tile() {
    // 6x2 px, ss 1, tiles of 2 px: certified interior | bounded exterior | heuristic exterior.
    let cols = ColumnSet::of(&[Column::Class, Column::De, Column::Bound]);
    let h = Header {
        minor: MINOR,
        columns: cols,
        nx: 6,
        ny: 2,
        ss: 1,
        max_iter: 1000,
        escape_radius: 1e10,
        view: View {
            center_re: "0".into(),
            center_im: "0".into(),
            width: "1".into(),
            rotation: 0.0,
        },
        kernel: "test/1".into(),
    };
    let mut s = Samples::alloc(h.count(), cols);
    for k in 0..h.count() {
        let (kind, ev) = match k % 6 {
            0 | 1 => (Kind::Interior, Evidence::Certified),
            2 | 3 => (Kind::Escaped, Evidence::Bounded),
            _ => (Kind::Escaped, Evidence::Heuristic),
        };
        s.class[k] = Class::new(kind, ev);
        s.de.as_mut().unwrap()[k] = 8.0;
        s.bound.as_mut().unwrap()[k] = 0.01; // E_px = 0.01 * 8 * ln2 / 2 = 0.0277
    }
    let p = file("evidence.fds");
    write(std::fs::File::create(&p).unwrap(), &h, &s).unwrap();
    let p = p.to_str().unwrap();
    let l = fd(&["lod", p, "--tile-px", "2"]);
    assert_eq!(value(&l, "accepted"), "2");
    assert!(
        l.contains(&"tile 0 0 certified_uniform e_px 0 accept uniform".to_string()),
        "{l:?}"
    );
    assert!(
        l.iter().any(
            |x| x.starts_with("tile 1 0 certified_approximate e_px 0.0277")
                && x.ends_with(" accept bounded")
        ),
        "{l:?}"
    );
    assert!(
        l.contains(&"tile 2 0 unresolved_boundary e_px inf refine uncertified".to_string()),
        "{l:?}"
    );
    // A tighter threshold refines the approximate tile.
    let l = fd(&["lod", p, "--tile-px", "2", "--max-px", "0.01"]);
    assert_eq!(value(&l, "accepted"), "1");
    assert!(
        l.iter()
            .any(|x| x.starts_with("tile 1 0 certified_approximate")
                && x.ends_with(" refine over-threshold")),
        "{l:?}"
    );
}
