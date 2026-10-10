//! Drives the real `fd compile` (VIDE-01) on a short cut of the committed Atlas v0 path:
//! the atlas holds, per frame, exactly the chunks a warm render reads, those chunks
//! render the frame (the deepest bit-identically to a cold render), a recompile stores
//! nothing new, and the byte budget is enforced both before building (estimate over the
//! cap) and while building (the store refuses a put past the cap). Also checks that
//! `fd path zoom` regenerates the committed path.
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fd(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fd"))
        .args(args)
        .output()
        .unwrap()
}

fn text(o: &Output) -> String {
    String::from_utf8(o.stdout.clone()).unwrap()
}

fn ok(o: &Output) -> String {
    assert!(
        o.status.success(),
        "{}\n{}",
        text(o),
        String::from_utf8_lossy(&o.stderr)
    );
    text(o)
}

fn get<'a>(text: &'a str, k: &str) -> &'a str {
    text.lines()
        .find_map(|l| l.strip_prefix(&format!("{k} ")))
        .unwrap_or_else(|| panic!("no {k}: {text}"))
}

fn field<'a>(line: &'a str, k: &str) -> &'a str {
    let f: Vec<&str> = line.split(' ').collect();
    f[f.iter()
        .position(|x| *x == k)
        .unwrap_or_else(|| panic!("no {k} in {line}"))
        + 1]
}

fn committed() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bench/path-atlas-v0.txt")
}

fn frame_lines(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(String::from)
        .collect()
}

#[test]
fn committed_path_regenerates() {
    let file = std::fs::read_to_string(committed()).unwrap();
    let cmd = file
        .lines()
        .find_map(|l| l.strip_prefix("# fd path zoom "))
        .expect("generator line");
    let args: Vec<&str> = ["path", "zoom"].into_iter().chain(cmd.split(' ')).collect();
    let again = ok(&fd(&args));
    let frames = frame_lines(&file);
    assert_eq!(frames.len(), 750);
    assert_eq!(frame_lines(&again), frames);
}

#[test]
fn compiles_a_cut_of_the_v0_path_within_budget() {
    let dir = std::env::temp_dir().join(format!("fd-compile-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // 8 frames over the whole depth range of the v0 path, same centre.
    let first = frame_lines(&std::fs::read_to_string(committed()).unwrap())[0].clone();
    let (re, im) = (
        first.split(' ').next().unwrap(),
        first.split(' ').nth(1).unwrap(),
    );
    let path = dir.join("cut.txt");
    let cut = ok(&fd(&[
        "path",
        "zoom",
        "--re",
        re,
        "--im",
        im,
        "--from",
        "4",
        "--to",
        "2e-49",
        "--seconds",
        "8",
        "--fps",
        "1",
    ]));
    std::fs::write(&path, &cut).unwrap();
    let views: Vec<Vec<String>> = frame_lines(&cut)
        .iter()
        .map(|l| l.split(' ').map(String::from).collect())
        .collect();
    let p = path.to_str().unwrap();
    let render = [
        "--size",
        "32x18",
        "--iter",
        "20000",
        "--columns",
        "nu,de",
        "--threads",
        "2",
    ];
    let compile = |store: &Path, extra: &[&str]| {
        let mut args = vec![
            "compile",
            p,
            "--store",
            store.to_str().unwrap(),
            "--workers",
            "2",
        ];
        args.extend(render);
        args.extend(extra);
        fd(&args)
    };
    let st = dir.join("atlas");
    let s = st.to_str().unwrap();
    let t = ok(&compile(&st, &[]));
    assert_eq!(get(&t, "frames"), "8");
    assert_eq!(get(&t, "orbits"), "2");
    assert_eq!(get(&t, "certificates"), "0");
    assert_eq!(get(&t, "seconds.certification"), "0");
    assert_eq!(get(&t, "budget.within_cap"), "1");
    assert_eq!(get(&t, "frames.camera_exact"), "8");
    assert_eq!(get(&t, "walk.frames"), "8");
    assert_eq!(get(&t, "tile_manifests.split"), "0");
    assert!(get(&t, "frames.fx").parse::<u32>().unwrap() >= 5, "{t}");
    let n = |k: &str| get(&t, k).parse::<u64>().unwrap();
    assert_eq!(n("met") + n("missed"), n("jobs"));
    // Bytes by kind add up to the atlas; the store holds exactly this atlas.
    let by_kind: u64 = t
        .lines()
        .filter(|l| l.starts_with("atlas.kind."))
        .map(|l| l.split(' ').nth(2).unwrap().parse::<u64>().unwrap())
        .sum();
    assert_eq!(by_kind, n("atlas.bytes"));
    assert_eq!(n("atlas.bytes"), n("stored_bytes"));
    let walked = n("walk.math_bytes") + n("walk.manifest_bytes");
    // The walk counts orbit manifests as math chunks and frame + tile manifests as manifests.
    assert_eq!(walked, n("atlas.bytes"));
    let stats = ok(&fd(&["chunk", "verify", "--store", s]));
    assert_eq!(get(&stats, "bad"), "0");

    // The deepest frame, warm from the atlas: frame manifest -> tile -> orbit and table.
    let line = t.lines().find(|l| l.starts_with("frame 7 ")).unwrap();
    let fm = ok(&fd(&[
        "manifest",
        "show",
        "--store",
        s,
        field(line, "manifest"),
    ]));
    let tile = fm
        .lines()
        .find_map(|l| l.strip_prefix("tile-ref "))
        .unwrap();
    let tm = ok(&fd(&["manifest", "show", "--store", s, tile]));
    let reff = |kind: &str| {
        tm.lines()
            .find_map(|l| l.strip_prefix(&format!("ref {kind} ")))
            .unwrap_or_else(|| panic!("no {kind}: {tm}"))
    };
    let (orbit, bla) = (reff("orbit-manifest"), reff("bla"));
    let v = &views[7];
    let view = ["--re", &v[0], "--im", &v[1], "--width", &v[2]];
    let cold = dir.join("cold.fds");
    let warm = dir.join("warm.fds");
    ok(&fd(&[
        &["render"][..],
        &view,
        &render,
        &["-o", cold.to_str().unwrap()],
    ]
    .concat()));
    ok(&fd(&[
        &["render"][..],
        &view,
        &render,
        &["--store", s, "--orbit", orbit, "-o", warm.to_str().unwrap()],
    ]
    .concat()));
    assert_eq!(std::fs::read(&cold).unwrap(), std::fs::read(&warm).unwrap());
    let r = ok(&fd(&[
        &["render"][..],
        &view,
        &render,
        &["--store", s, "--bla", bla, "-o", warm.to_str().unwrap()],
    ]
    .concat()));
    assert!(r.contains("bla.skipped"), "{r}");

    // A recompile writes nothing new.
    let again = ok(&compile(&st, &[]));
    assert_eq!(get(&again, "stored_bytes"), "0");
    assert_eq!(get(&again, "atlas.bytes"), get(&t, "atlas.bytes"));
    assert_eq!(get(&again, "dedup.manifest_puts.stored"), "0");

    // Budget, before building: an estimate over the cap is refused and nothing is written.
    let small = dir.join("small");
    let o = compile(&small, &["--cap", "1MiB", "--target", "1MiB"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&o.stderr).contains("atlas over budget"),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    assert_eq!(
        get(
            &ok(&fd(&["chunk", "stats", "--store", small.to_str().unwrap()])),
            "bytes"
        ),
        "0"
    );

    // Budget, while building: the atlas fits the cap alone, but not on top of what the
    // store already holds, so a put is refused and the compile fails.
    let full = dir.join("full");
    let junk = dir.join("junk.bin");
    std::fs::write(&junk, vec![7u8; 1 << 20]).unwrap();
    ok(&fd(&[
        "chunk",
        "put",
        "--store",
        full.to_str().unwrap(),
        "--kind",
        "samples",
        junk.to_str().unwrap(),
    ]));
    let cap = (n("atlas.bytes") + (1 << 19)).to_string();
    let o = compile(&full, &["--cap", &cap, "--target", &cap]);
    assert_eq!(o.status.code(), Some(2), "{}", text(&o));
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(
        err.contains("over budget") && err.contains("hard cap"),
        "{err}"
    );

    // Usage errors are exit 2.
    assert_eq!(compile(&st, &["--bla", "tile"]).status.code(), Some(2));
    let _ = std::fs::remove_dir_all(&dir);
}
