//! Drives the real `fd manifest` binary: thousands of frame manifests reference the same
//! tile manifests and math chunks, the store keeps each chunk's bytes once, and
//! `fd manifest walk` reports the sharing and refuses a corrupted or inconsistent DAG.
use std::path::PathBuf;
use std::process::{Command, Output};

const FRAMES: usize = 2000;

fn fd(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fd"))
        .args(args)
        .output()
        .unwrap()
}

fn ok(args: &[&str]) -> Vec<(String, String)> {
    let out = fd(args);
    assert!(
        out.status.success(),
        "fd {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
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
    &lines
        .iter()
        .find(|(k, _)| k == name)
        .unwrap_or_else(|| panic!("no {name} in {lines:?}"))
        .1
}

fn num(lines: &[(String, String)], name: &str) -> u64 {
    get(lines, name).parse().unwrap()
}

#[test]
fn thousands_of_frames_share_math_chunks() {
    let dir = std::env::temp_dir().join(format!("fd-manifest-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let store = dir.join("atlas");
    let st = store.to_str().unwrap();

    // Three math chunks.
    let mut math = Vec::new();
    for (kind, body) in [
        ("orbit-slab", "orbit"),
        ("bla", "operators"),
        ("certificate", "claim"),
    ] {
        let f: PathBuf = dir.join(kind);
        std::fs::write(&f, body.repeat(500)).unwrap();
        let put = ok(&[
            "chunk",
            "put",
            "--store",
            st,
            "--kind",
            kind,
            f.to_str().unwrap(),
        ]);
        math.push((get(&put, "id").to_string(), num(&put, "bytes")));
    }
    let math_bytes: u64 = math.iter().map(|(_, b)| b).sum();
    let (orbit, bla, cert) = (&math[0].0, &math[1].0, &math[2].0);

    // Tile 3/3/3 is child 3 of tile 2/1/1; 2/1/2 is a sibling sharing the orbit slab.
    let child = ok(&[
        "manifest",
        "tile",
        "--store",
        st,
        "--tile",
        "3/3/3",
        "--refs",
        &format!("{cert},{bla}"),
    ]);
    let child = get(&child, "id").to_string();
    let parent = ok(&[
        "manifest",
        "tile",
        "--store",
        st,
        "--tile",
        "2/1/1",
        "--evidence",
        "heuristic",
        "--children",
        &format!("3:{child}"),
        "--refs",
        orbit,
    ]);
    let parent = get(&parent, "id").to_string();
    let sibling = ok(&[
        "manifest", "tile", "--store", st, "--tile", "2/1/2", "--refs", orbit,
    ]);
    let sibling = get(&sibling, "id").to_string();

    // Inconsistent references are refused at build time.
    let wrong_quadrant = fd(&[
        "manifest",
        "tile",
        "--store",
        st,
        "--tile",
        "2/1/1",
        "--children",
        &format!("0:{child}"),
    ]);
    assert!(!wrong_quadrant.status.success());
    let manifest_as_math = fd(&[
        "manifest", "tile", "--store", st, "--tile", "2/1/1", "--refs", &child,
    ]);
    assert!(!manifest_as_math.status.success());
    let math_as_tile = fd(&[
        "manifest", "frame", "--store", st, "--anchor", "2/1/1", "--tiles", orbit,
    ]);
    assert!(!math_as_tile.status.success());

    // A zoom path: every frame is a distinct camera over the same tiles.
    let mut first = String::new();
    for i in 0..FRAMES {
        let offset = format!("{},{}", i as f64 / 8192.0, -(i as f64) / 16384.0);
        let width = format!("{}", 1.0 / (1.0 + i as f64 / 64.0));
        let tiles = if i % 2 == 0 {
            parent.clone()
        } else {
            format!("{sibling},{parent}")
        };
        let f = ok(&[
            "manifest", "frame", "--store", st, "--anchor", "2/1/1", "--offset", &offset,
            "--width", &width, "--size", "320x180", "--iter", "5000", "--tiles", &tiles,
        ]);
        assert_eq!(get(&f, "result"), "stored", "frame {i}");
        if i == 0 {
            first = get(&f, "id").to_string();
        }
    }
    let again = ok(&[
        "manifest", "frame", "--store", st, "--anchor", "2/1/1", "--offset", "0,0", "--width", "1",
        "--size", "320x180", "--iter", "5000", "--tiles", &parent,
    ]);
    assert_eq!(
        (get(&again, "id"), get(&again, "result")),
        (first.as_str(), "deduplicated")
    );

    let shown = ok(&["manifest", "show", "--store", st, &first]);
    assert_eq!(get(&shown, "kind"), "frame-manifest");
    assert_eq!(get(&shown, "anchor"), "2/1/1");
    assert_eq!(get(&shown, "tile-ref"), parent);
    let shown = ok(&["manifest", "show", "--store", st, &parent]);
    assert_eq!(get(&shown, "child"), format!("3 {child}"));
    assert_eq!(get(&shown, "ref"), format!("orbit-slab {orbit}"));

    // Every frame reaches all three math chunks, stored once.
    let w = ok(&["manifest", "walk", "--store", st]);
    let n = FRAMES as u64;
    assert_eq!(num(&w, "frames"), n);
    assert_eq!(num(&w, "tiles"), 3);
    assert_eq!(num(&w, "math_chunks"), 3);
    assert_eq!(num(&w, "math_bytes"), math_bytes);
    assert_eq!(num(&w, "math_refs"), 3 * n);
    assert_eq!(num(&w, "math_bytes_per_frame"), n * math_bytes);
    assert_eq!(get(&w, "sharing"), format!("{n}.00"));
    assert_eq!(num(&w, "store_chunks"), 3 + 3 + n);
    assert_eq!(
        num(&w, "store_bytes"),
        math_bytes + num(&w, "manifest_bytes")
    );
    let stats = ok(&["chunk", "stats", "--store", st]);
    assert_eq!(num(&stats, "bytes"), num(&w, "store_bytes"));
    // Frames stay tiny next to the math they name.
    assert!(num(&w, "manifest_bytes") / n < 256, "{w:?}");

    // One corrupted math chunk breaks every frame's walk.
    let path = PathBuf::from(get(&ok(&["chunk", "show", "--store", st, bla]), "path"));
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[40] ^= 1;
    std::fs::write(&path, bytes).unwrap();
    let bad = fd(&["manifest", "walk", "--store", st, &first]);
    assert!(!bad.status.success());
    assert!(
        String::from_utf8_lossy(&bad.stderr).contains("corrupt"),
        "{bad:?}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
