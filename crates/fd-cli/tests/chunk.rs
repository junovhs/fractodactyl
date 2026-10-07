//! Drives the real `fd chunk` binary: identical chunks are stored once, and a corrupted
//! chunk file is detected on read and by `verify`.
use std::path::{Path, PathBuf};
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
    &lines.iter().find(|(k, _)| k == name).unwrap_or_else(|| panic!("no {name}")).1
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
    let put = |f: &Path| ok(&["chunk", "put", "--store", s(&store), "--kind", "orbit-slab", s(f)]);

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
    let other = ok(&["chunk", "put", "--store", s(&store), "--kind", "orbit-slab", "--precision", "256", s(&a)]);
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
    assert!(String::from_utf8_lossy(&bad.stderr).contains("corrupt"), "{:?}", bad);
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
