//! GPU-30: exercise the public inventory, flags and shared zone/studio selection.
use std::process::{Command, Output};

fn fd(args: &[&str], env: Option<&str>) -> Output {
    let bin =
        std::env::var_os("FD_ADAPTER_TEST_BIN").unwrap_or_else(|| env!("CARGO_BIN_EXE_fd").into());
    let mut cmd = Command::new(bin);
    cmd.args(args).env_remove("FD_ADAPTER");
    if matches!(args.first(), Some(&"render" | &"film")) {
        cmd.args(["--threads", "1"]);
    }
    if let Some(value) = env {
        cmd.env("FD_ADAPTER", value);
    }
    let o = cmd.output().unwrap();
    eprintln!(
        "fd {}: exit={}\n{}{}",
        args.join(" "),
        o.status,
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    o
}
fn ok(o: Output) -> Output {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    o
}

#[test]
fn adapter_flag() {
    let inventory = ok(fd(&["adapters"], None));
    let rows = String::from_utf8(inventory.stdout).unwrap();
    let first = rows
        .lines()
        .next()
        .expect("hardware or software adapter is required");
    let words: Vec<_> = first.split_whitespace().collect();
    assert_eq!(words[0], "0");
    assert!(words[2].contains(':') && first.contains(" f64="), "{first}");
    let name = words[3..words.len() - 1].join(" ");
    assert_eq!(rows.as_bytes(), ok(fd(&["adapters"], None)).stdout);
    let dir = std::env::temp_dir().join(format!("fd-adapter-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let samples = dir.join("in.fds");
    let input = samples.to_str().unwrap();
    let output = dir.join("shade");
    let out = output.to_str().unwrap();
    ok(fd(
        &[
            "render",
            "--re",
            "-0.75",
            "--im",
            "0.1",
            "--width",
            "3",
            "--size",
            "8x6",
            "--iter",
            "32",
            "-o",
            input,
            "--adapter",
            "0",
        ],
        None,
    ));
    for (flags, env) in [
        (vec!["--adapter", "0"], None),
        (vec![], Some("0")),
        (vec!["--adapter", "0"], Some("99")),
        (vec!["--adapter", "0"], Some("bad")),
    ] {
        let mut args = vec!["shade", input, "--preset", "ice", "-o", out];
        args.extend(flags);
        let o = ok(fd(&args, env));
        assert!(String::from_utf8_lossy(&o.stderr).contains(&name));
    }
    for (flags, env, diagnostic) in [
        (vec!["--adapter", "99"], None, "valid indices: 0"),
        (vec![], Some("99"), "valid indices: 0"),
        (vec![], Some("bad"), "FD_ADAPTER"),
    ] {
        let mut args = vec!["shade", input, "--preset", "ice", "-o", out];
        args.extend(flags);
        let o = fd(&args, env);
        assert!(!o.status.success());
        assert!(String::from_utf8_lossy(&o.stderr).contains(diagnostic));
    }
    for command in [
        "film", "render", "shade", "style", "play", "control", "explore",
    ] {
        let o = fd(&[command, "--adapter", "99"], None);
        assert!(!o.status.success());
        assert!(String::from_utf8_lossy(&o.stderr).contains("valid indices: 0"));
    }
    // Zone rendering opens the same public adapter index, before capability fallback.
    let zone = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/zones/v0.zone");
    let o = ok(fd(
        &[
            "render",
            "--zone",
            zone,
            "--re",
            "-0.743291890852430202931624158508904039462544013087723088341335644672284698593566",
            "--im",
            "0.131240552308797604770845873815964848019374249266625134372668873249132305318161",
            "--width",
            "1e-35",
            "--size",
            "8x6",
            "--iter",
            "100000",
            "--gpu-kernel",
            "f32only",
            "--adapter",
            "0",
            "-o",
            input,
        ],
        None,
    ));
    let log = String::from_utf8_lossy(&o.stderr);
    assert!(log.contains(&name), "{log}");
    assert!(!log.contains("using the perturbation kernel"), "{log}");
    // Two available adapters prove indices do not collapse to hardware-first ranking.
    for row in rows.lines().skip(1) {
        let words: Vec<_> = row.split_whitespace().collect();
        let expected = words[3..words.len() - 1].join(" ");
        let o = ok(fd(
            &[
                "shade",
                input,
                "--preset",
                "ice",
                "-o",
                out,
                "--adapter",
                words[0],
            ],
            None,
        ));
        assert!(String::from_utf8_lossy(&o.stderr).contains(&expected));
    }
    // Keyframe films reuse the chosen zone device for the studio look.
    let movie = dir.join("chosen.mp4");
    let o = ok(fd(
        &[
            "film",
            "-0.743291890852430202931624158508904039462544013087723088341335644672284698593566",
            "0.131240552308797604770845873815964848019374249266625134372668873249132305318161",
            "--zone",
            zone,
            "--from",
            "2e-35",
            "--to",
            "1e-35",
            "--size",
            "8x6",
            "--iter",
            "100000",
            "--seconds",
            "1",
            "--fps",
            "2",
            "--adapter",
            "0",
            "--gpu-kernel",
            "f32only",
            "--x264",
            "ultrafast",
            "--mp4",
            movie.to_str().unwrap(),
        ],
        None,
    ));
    assert!(String::from_utf8_lossy(&o.stderr).contains(&name));
    let log = String::from_utf8_lossy(&o.stderr);
    assert!(
        log.contains("wgpu f32only zone") && log.contains("(resident)"),
        "{log}"
    );
    assert!(movie.metadata().unwrap().len() > 0);
    std::fs::remove_dir_all(dir).unwrap();
}
