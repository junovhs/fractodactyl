//! Drives the real `fd schedule` on a known path: every job's chunk is built into the
//! store after its dependencies, the log says when each became ready against its
//! first-use frame, a lead too short for the machine fails the compile (or only reports
//! with `--on-miss report`), and the stored chunks are the ones `fd render` uses.
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

#[test]
fn path_chunks_are_built_by_their_first_use() {
    let dir = std::env::temp_dir().join(format!("fd-schedule-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("path.txt");
    // An f64-tier group, a deep group at the same centre, and a frame elsewhere.
    std::fs::write(
        &path,
        "0 1 1e-2\n0 1 1e-3\n0 1 1e-20\n0 1.0 1e-30\n-0.75 0.1 0.01\n",
    )
    .unwrap();
    let st = dir.join("atlas");
    let (p, s) = (path.to_str().unwrap(), st.to_str().unwrap());
    let common = [
        "schedule",
        p,
        "--store",
        s,
        "--size",
        "32x24",
        "--iter",
        "20000",
        "--columns",
        "nu,de",
        "--fps",
        "30",
    ];
    let run = |extra: &[&str]| fd(&[&common[..], extra].concat());

    // A generous lead: every chunk is ready before its frame; exit 0.
    let out = run(&["--lead", "60", "--workers", "2"]);
    let t = text(&out);
    assert!(
        out.status.success(),
        "{t}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(get(&t, "jobs"), "8");
    assert_eq!(get(&t, "jobs.orbit"), "3");
    assert_eq!(get(&t, "jobs.bla"), "5");
    assert_eq!(get(&t, "jobs.certificate"), "0");
    assert_eq!((get(&t, "met"), get(&t, "missed")), ("8", "0"));
    let jobs: Vec<&str> = t.lines().filter(|l| l.starts_with("job ")).collect();
    assert_eq!(jobs.len(), 8);
    let num = |l: &str, k: &str| field(l, k).parse::<f64>().unwrap();
    // Seconds are printed with 6 decimals: a comparison of sums of printed values may be
    // off by a few half-units of the last digit, so derived inequalities allow 2e-6.
    // Comparisons of single printed values need no slack (rounding is monotone).
    const PRINTED: f64 = 2e-6;
    let by_name = |n: &str| *jobs.iter().find(|l| field(l, "job") == n).unwrap();
    for l in &jobs {
        assert_eq!(field(l, "ready"), "yes", "{l}");
        assert!(
            num(l, "finish") <= num(l, "deadline") && num(l, "slack") >= 0.0,
            "{l}"
        );
        let first: f64 = num(l, "first_use");
        assert!(
            (num(l, "deadline") - (60.0 + first / 30.0)).abs() < PRINTED,
            "{l}"
        );
        assert!(num(l, "effective") <= num(l, "deadline"), "{l}");
        // Not > 0: a fast enough job may print as 0.000000.
        assert!(
            num(l, "est_seconds") >= 0.0 && num(l, "actual_seconds") >= 0.0,
            "{l}"
        );
        match field(l, "deps") {
            "-" => assert_eq!(field(l, "kind"), "orbit"),
            d => {
                assert_eq!(field(l, "kind"), "bla");
                let parent = by_name(d);
                assert!(num(l, "start") >= num(parent, "finish"), "{l}\n{parent}");
                assert!(
                    num(parent, "effective")
                        <= num(l, "effective") - num(l, "est_seconds") + PRINTED,
                    "{l}\n{parent}"
                );
            }
        }
    }
    // The deep group's orbit is first used by frame 2, the frame elsewhere by frame 4.
    assert_eq!(field(by_name("orbit.1"), "first_use"), "2");
    assert_eq!(field(by_name("orbit.2"), "first_use"), "4");
    for pol in ["slack", "edf", "first-use"] {
        for costs in ["measured", "estimated"] {
            assert!(
                t.lines()
                    .any(|l| l.starts_with(&format!("compare {pol} costs {costs} missed "))),
                "{t}"
            );
        }
    }

    // The stored table renders its frame: the chunks are the real ones.
    let bla = field(by_name("bla.3"), "chunk");
    let fds = dir.join("f3.fds");
    let r = fd(&[
        "render", "--re", "0", "--im", "1.0", "--width", "1e-30", "--size", "32x24", "--iter",
        "20000",
    ]
    .iter()
    .copied()
    .chain([
        "--columns",
        "nu,de",
        "--store",
        s,
        "--bla",
        bla,
        "-o",
        fds.to_str().unwrap(),
    ])
    .collect::<Vec<_>>());
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));

    // Playback starting with the compile cannot have frame 0's chunks ready at t = 0.
    let out = run(&["--policy", "first-use"]);
    // Exit 1: a missed deadline, distinct from usage and build errors (2).
    assert_eq!(
        out.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let t = text(&out);
    assert_ne!(get(&t, "missed"), "0");
    assert!(get(&t, "min_lead_seconds").parse::<f64>().unwrap() > 0.0);
    assert!(String::from_utf8_lossy(&out.stderr).contains("not ready by their first-use frame"));
    let out = run(&["--on-miss", "report"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    // Rebuilding the same path stores nothing new.
    assert_eq!(get(&text(&out), "stored_bytes"), "0");
    // A usage error is exit 2.
    assert_eq!(run(&["--policy", "fastest"]).status.code(), Some(2));
    // A probe directory left by a killed run (a PID that is not running) is removed.
    let stale = dir.join(format!("atlas.schedule-probe-{}", u32::MAX - 1));
    std::fs::create_dir_all(&stale).unwrap();
    assert!(run(&["--on-miss", "report"]).status.success());
    assert!(!stale.exists());
}
