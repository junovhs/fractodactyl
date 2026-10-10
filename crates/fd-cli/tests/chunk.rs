//! Drives the real `fd chunk` binary: identical chunks are stored once, and a corrupted
//! chunk file is detected on read and by `verify`.
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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
        .unwrap_or_else(|| panic!("no {name}"))
        .1
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fd-chunk-cli-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

#[test]
fn identical_chunks_stored_once_and_corruption_detected() {
    let dir = scratch("e2e");
    let store = dir.join("atlas");
    let (a, b) = (dir.join("a.bin"), dir.join("b.bin"));
    std::fs::write(&a, b"reference orbit slab bytes").unwrap();
    std::fs::write(&b, b"a different slab").unwrap();
    let put = |f: &Path| {
        ok(&[
            "chunk",
            "put",
            "--store",
            s(&store),
            "--kind",
            "orbit-slab",
            s(f),
        ])
    };

    let first = put(&a);
    assert_eq!(get(&first, "result"), "stored");
    let second = put(&a);
    assert_eq!(get(&second, "result"), "deduplicated");
    let id = get(&first, "id").to_string();
    assert_eq!(get(&second, "id"), id);
    assert_eq!(get(&put(&b), "result"), "stored");

    let stats = ok(&["chunk", "stats", "--store", s(&store)]);
    assert_eq!(get(&stats, "chunks"), "2");

    // The contract is part of the identity: same payload, other precision, new chunk.
    let other = ok(&[
        "chunk",
        "put",
        "--store",
        s(&store),
        "--kind",
        "orbit-slab",
        "--precision",
        "256",
        s(&a),
    ]);
    assert_ne!(get(&other, "id"), id);

    let shown = ok(&["chunk", "show", "--store", s(&store), &id]);
    assert_eq!(get(&shown, "kind"), "orbit-slab");
    assert_eq!(get(&shown, "precision"), "53");
    let back = dir.join("back.bin");
    ok(&["chunk", "get", "--store", s(&store), &id, "-o", s(&back)]);
    assert_eq!(std::fs::read(&back).unwrap(), std::fs::read(&a).unwrap());

    // Flip one payload bit in the stored file.
    let path = PathBuf::from(get(&shown, "path"));
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[40] ^= 1;
    std::fs::write(&path, bytes).unwrap();

    let bad = fd(&["chunk", "get", "--store", s(&store), &id, "-o", s(&back)]);
    assert!(!bad.status.success());
    assert!(
        String::from_utf8_lossy(&bad.stderr).contains("corrupt"),
        "{:?}",
        bad
    );
    let verify = fd(&["chunk", "verify", "--store", s(&store)]);
    assert!(!verify.status.success());
    let text = String::from_utf8_lossy(&verify.stdout);
    assert!(text.contains(&format!("corrupt {id}")), "{text}");
    assert!(text.contains("bad 1"), "{text}");

    // Putting the true bytes again repairs it.
    assert_eq!(get(&put(&a), "result"), "repaired");
    ok(&["chunk", "verify", "--store", s(&store)]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn hard_cap_refuses_and_stats_report_bytes_by_kind() {
    let dir = scratch("budget");
    let store = dir.join("atlas");
    let st = s(&store);
    let defaults = ok(&["chunk", "budget", "--store", st]);
    assert_eq!(get(&defaults, "target"), (5u64 << 29).to_string());
    assert_eq!(get(&defaults, "cap"), (3u64 << 30).to_string());
    assert!(
        !fd(&["chunk", "budget", "--store", st, "--target", "2KiB", "--cap", "1KiB"])
            .status
            .success()
    );

    // Chunks are 32 header bytes + payload padded to 8: 64, 48 and 64 bytes here.
    let set = ok(&[
        "chunk", "budget", "--store", st, "--target", "100", "--cap", "150",
    ]);
    assert_eq!((get(&set, "target"), get(&set, "cap")), ("100", "150"));
    let (a, b, c) = (dir.join("a.bin"), dir.join("b.bin"), dir.join("c.bin"));
    std::fs::write(&a, b"reference orbit slab bytes").unwrap();
    std::fs::write(&b, b"certificate data").unwrap();
    std::fs::write(&c, b"one more orbit slab, too many").unwrap();
    let put = |k: &str, f: &Path| fd(&["chunk", "put", "--store", st, "--kind", k, s(f)]);

    let first = put("orbit-slab", &a);
    assert!(first.status.success());
    assert!(String::from_utf8_lossy(&first.stdout).contains("over_target 0"));
    let second = put("certificate", &b);
    assert!(second.status.success(), "{second:?}");
    assert!(String::from_utf8_lossy(&second.stdout).contains("atlas_bytes 112\nover_target 1"));
    assert!(String::from_utf8_lossy(&second.stderr).contains("over its 100 byte target"));

    // 112 + 64 > 150: refused, nothing written; a duplicate still succeeds.
    let refused = put("orbit-slab", &c);
    assert!(!refused.status.success());
    let err = String::from_utf8_lossy(&refused.stderr);
    assert!(
        err.contains("hard cap of 150") && err.contains("nothing written"),
        "{err}"
    );
    assert!(put("orbit-slab", &a).status.success());

    let stats = ok(&["chunk", "stats", "--store", st]);
    assert_eq!(get(&stats, "chunks"), "2");
    assert_eq!(get(&stats, "bytes"), "112");
    assert_eq!(get(&stats, "headroom"), "38");
    assert_eq!(get(&stats, "over_target"), "1");
    assert_eq!(get(&stats, "kind.orbit-slab"), "1 64");
    assert_eq!(get(&stats, "kind.certificate"), "1 48");
    assert_eq!(get(&stats, "class.operators"), "64");
    assert_eq!(get(&stats, "class.evidence"), "48");
    assert_eq!(get(&stats, "class.manifests"), "0");

    // Manifest builds go through the same cap.
    let tile = fd(&["manifest", "tile", "--store", st, "--tile", "0/0/0"]);
    assert!(!tile.status.success());
    assert!(
        String::from_utf8_lossy(&tile.stderr).contains("hard cap"),
        "{tile:?}"
    );

    // Raising the cap admits it.
    ok(&["chunk", "budget", "--store", st, "--cap", "1MiB"]);
    assert_eq!(
        get(
            &ok(&["chunk", "put", "--store", st, "--kind", "orbit-slab", s(&c)]),
            "result"
        ),
        "stored"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
