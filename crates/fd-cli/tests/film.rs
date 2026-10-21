//! FILM-14: decoded keyframe spins track direct rotated film frames; SHAD-11: the studio
//! film matches the frozen CPU studio film.
use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn checked(output: Output, action: &str) -> Output {
    assert!(
        output.status.success(),
        "{action}: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    output
}

fn decoded_rgb(file: &Path) -> Vec<u8> {
    checked(
        Command::new("ffmpeg")
            .args(["-loglevel", "error", "-i"])
            .arg(file)
            .args(["-vsync", "0", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"])
            .output()
            .expect("ffmpeg is required for film tests"),
        "decode",
    )
    .stdout
}

#[test]
fn film_keyframes_default() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("fd-film18-{}-{stamp}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    for (name, flags, mode, fallback) in [
        ("default", vec![], "2", false),
        ("off", vec!["--keyframes", "off"], "null", false),
        ("explicit", vec!["--keyframes", "3"], "3", false),
        ("ss", vec!["--ss", "1"], "null", true),
        ("jet", vec!["--jet-aa", "on"], "null", true),
    ] {
        let movie = dir.join(format!("{name}.mp4"));
        let output = checked(
            Command::new(env!("CARGO_BIN_EXE_fd"))
                .args([
                    "film",
                    "-0.75",
                    "0.1",
                    "--from",
                    "4",
                    "--to",
                    "2",
                    "--seconds",
                    "1",
                    "--fps",
                    "2",
                    "--size",
                    "32x18",
                    "--iter",
                    "32",
                    "--threads",
                    "1",
                    "--x264",
                    "ultrafast",
                ])
                .args(flags)
                .arg("--mp4")
                .arg(&movie)
                .output()
                .unwrap(),
            name,
        );
        let report = String::from_utf8(output.stdout).unwrap();
        let log = String::from_utf8(output.stderr).unwrap();
        assert!(report.contains("\"schema\":\"fd-film/1\""), "{report}");
        assert!(
            report.contains(&format!("\"keyframes\":{mode},")),
            "{name}: {report}"
        );
        assert_eq!(log.contains("keyframes off ("), fallback, "{name}: {log}");
        assert!(!decoded_rgb(&movie).is_empty(), "{name}: empty film");
    }
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn film_spin_keyframes() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("fd-film14-{}-{stamp}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let speed = dir.join("speed.txt");
    let spins = dir.join("spins.txt");
    // Five frames at 4 fps: a quarter turn passes ~45 and ends at 90 degrees; a half
    // turn ends at 180 degrees, where the frame shows exactly the unrotated region.
    fs::write(&speed, format!("{:.16} 4\n", 4f64.log10())).unwrap();
    let frame_size = 320 * 180 * 3;
    let mut results = Vec::new();
    let mut direct_midpoints = Vec::new();
    let mut direct_last = Vec::new();
    for turns in ["0", "0.25", "0.5"] {
        fs::write(&spins, format!("0 1.2 {turns}\n")).unwrap();
        let mut images = Vec::new();
        for keyframes in [false, true] {
            let movie = dir.join(format!("{turns}-{keyframes}.mp4"));
            let mut cmd = Command::new(env!("CARGO_BIN_EXE_fd"));
            cmd.args(["film", "-0.75", "0.1", "--from", "4", "--to", "2"])
                .arg("--speed")
                .arg(&speed)
                .arg("--spins")
                .arg(&spins)
                .args([
                    "--fps",
                    "4",
                    "--size",
                    "320x180",
                    "--iter",
                    "128",
                    "--threads",
                    "2",
                    "--crf",
                    "0",
                    "--x264",
                    "ultrafast",
                    "--chroma",
                    "444",
                    "--gpu",
                    "off",
                ]);
            if keyframes {
                cmd.args(["--keyframes", "2"]);
            } else {
                cmd.args(["--keyframes", "off"]);
            }
            cmd.arg("--mp4").arg(&movie);
            checked(cmd.output().unwrap(), "film");
            images.push(decoded_rgb(&movie));
        }
        assert_eq!(images[0].len(), 5 * frame_size);
        assert_eq!(images[1].len(), images[0].len());
        direct_midpoints.push(images[0][2 * frame_size..3 * frame_size].to_vec());
        direct_last.push(images[0][4 * frame_size..].to_vec());
        let mut stats = Vec::new();
        for (i, (direct, key)) in images[0]
            .chunks_exact(frame_size)
            .zip(images[1].chunks_exact(frame_size))
            .enumerate()
        {
            let mut errors: Vec<u8> = direct
                .iter()
                .zip(key)
                .map(|(&a, &b)| a.abs_diff(b))
                .collect();
            let mean =
                errors.iter().map(|&v| u64::from(v)).sum::<u64>() as f64 / errors.len() as f64;
            errors.sort_unstable();
            let p99 = errors[errors.len() * 99 / 100];
            eprintln!("turns {turns}, frame {i}: mean {mean:.3}, p99 {p99}");
            stats.push((mean, p99));
        }
        results.push(stats);
    }
    // Reject a no-op rotation implementation: the direct 45-degree frame changes.
    assert_ne!(direct_midpoints[0], direct_midpoints[1]);
    // Keyframes lose no fidelity to rotation: mean error within 1.25x of the unrotated
    // film at every angle; p99 may grow by the tent's fixed grid orientation at edges.
    for (i, (&still, &spin)) in results[0].iter().zip(&results[1]).enumerate() {
        assert!(
            spin.0 <= still.0 * 1.25,
            "frame {i}: spin mean {:.3} exceeds 1.25x no-spin {:.3}",
            spin.0,
            still.0
        );
        assert!(
            u16::from(spin.1) <= u16::from(still.1) + 25,
            "frame {i}: spin p99 {} exceeds no-spin {} + 25",
            spin.1,
            still.1
        );
    }
    // At 180 degrees the content is identical, so fidelity must match the unrotated film.
    let (still, half) = (results[0][4].0, results[2][4].0);
    assert!(
        half <= still * 1.05,
        "180-degree mean {half:.3} vs unrotated {still:.3}"
    );
    // Light turns with the fractal in direct films too: the 180-degree direct frame is
    // the unrotated one turned upside down (owner, FILM-14).
    let flipped: Vec<u8> = direct_last[2]
        .chunks_exact(3)
        .rev()
        .flatten()
        .copied()
        .collect();
    let diff = flipped
        .iter()
        .zip(&direct_last[0])
        .map(|(&a, &b)| u64::from(a.abs_diff(b)))
        .sum::<u64>() as f64
        / flipped.len() as f64;
    assert!(
        diff < 2.0,
        "180-degree direct frame differs from unrotated by {diff:.3}"
    );
    fs::remove_dir_all(dir).unwrap();
}

/// SHAD-11: a rotating keyframed studio film (haze, terrain lines, breathing, flow and a
/// 40-stop palette) matches the CPU studio's film, frozen before that was deleted, in
/// every raw RGB24 channel within one level. tests/fixtures/film-full-look.rgb holds the
/// five frames of `fd film ... --gpu off` at fcab2b5.
#[test]
fn film_matches_frozen_reference() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("fd-film-frozen-{}-{stamp}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let speed = dir.join("speed.txt");
    let spins = dir.join("spins.txt");
    fs::write(&speed, format!("{:.16} 4\n", 4f64.log10())).unwrap();
    fs::write(&spins, "0 1.2 0.25\n").unwrap();
    let stops: Vec<String> = (0..40u32)
        .map(|i| {
            let r = (i * 97 + 30) % 256;
            let g = (i * 53 + 90) % 256;
            let b = (i * 151 + 10) % 256;
            format!("#{r:02x}{g:02x}{b:02x}")
        })
        .collect();
    let look = dir.join("full-look.look");
    fs::write(
        &look,
        format!(
            "stops {}\ninterior #05070d\ndensity 0.05\nterrace 0.5\nslope 1.1\nlight 135\n\
             lines 1\nline_px 2.2\nhaze #1e7fe0\nflow 0.11\nbreathe 0.08\nbrate 0.5\n",
            stops.join(" ")
        ),
    )
    .unwrap();
    let raw = dir.join("raw");
    let out = Command::new(env!("CARGO_BIN_EXE_fd"))
        .args(["film", "-0.75", "0.1", "--from", "4", "--to", "2"])
        .arg("--speed")
        .arg(&speed)
        .arg("--spins")
        .arg(&spins)
        .arg("--preset")
        .arg(&look)
        .args([
            "--fps",
            "4",
            "--size",
            "128x72",
            "--iter",
            "128",
            "--threads",
            "2",
            "--keyframes",
            "2",
            "--crf",
            "30",
            "--x264",
            "ultrafast",
        ])
        .arg("--raw-dir")
        .arg(&raw)
        .arg("--mp4")
        .arg(dir.join("film.mp4"))
        .output()
        .unwrap();
    let out = checked(out, "film");
    let log = String::from_utf8_lossy(&out.stderr);
    let device = log.lines().find(|l| l.contains("GPU shading on"));
    eprintln!("{}", device.expect("fd film names its wgpu adapter"));
    let mut names: Vec<_> = fs::read_dir(&raw)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "rgb"))
        .collect();
    names.sort();
    let frozen = include_bytes!("fixtures/film-full-look.rgb");
    let size = 128 * 72 * 3;
    assert_eq!(names.len(), 5);
    assert_eq!(frozen.len(), 5 * size);
    let mut worst = 0;
    for (i, (name, want)) in names.iter().zip(frozen.chunks_exact(size)).enumerate() {
        let got = fs::read(name).unwrap();
        assert_eq!(got.len(), size);
        for (k, (&a, &b)) in got.iter().zip(want).enumerate() {
            worst = worst.max(a.abs_diff(b));
            assert!(
                a.abs_diff(b) <= 1,
                "frame {i} pixel ({},{}) channel {}: WGSL {a}, frozen CPU {b}",
                k / 3 % 128,
                k / 3 / 128,
                k % 3
            );
        }
    }
    eprintln!("5 rotating frames within {worst} level(s) of the frozen CPU studio");
    fs::remove_dir_all(dir).unwrap();
}

/// GPU-21: GPU zone keyframes shaded on the zone kernel's device (resident) give the
/// same raw film as reading them back (FD_ZONE_READBACK=1), and only one validity word
/// per zone keyframe leaves the GPU. Skips where no wgpu FP64 Vulkan device exists.
#[test]
fn gpu_zone_resident() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("fd-gpu21-{}-{stamp}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let zone = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bench/zones/v0-core.zone");
    let run = |readback: bool| {
        let raw = dir.join(format!("raw-{readback}"));
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_fd"));
        cmd.args([
            "film",
            "-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502",
            "0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922",
            "--from", "1e-29", "--to", "1e-30", "--seconds", "1", "--fps", "8",
            "--size", "320x180", "--keyframes", "2", "--iter", "100000",
            "--preset", "master-ice", "--gpu-kernel", "mixed", "--crf", "30",
            "--x264", "ultrafast", "--zone",
        ])
        .arg(&zone)
        .arg("--raw-dir")
        .arg(&raw)
        .arg("--mp4")
        .arg(dir.join(format!("{readback}.mp4")));
        if readback {
            cmd.env("FD_ZONE_READBACK", "1");
        } else {
            cmd.env_remove("FD_ZONE_READBACK");
        }
        let out = checked(cmd.output().unwrap(), "film");
        let log = String::from_utf8_lossy(&out.stderr).into_owned();
        let report = String::from_utf8_lossy(&out.stdout).into_owned();
        let mut names: Vec<_> = fs::read_dir(&raw)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "rgb"))
            .collect();
        names.sort();
        let frames: Vec<u8> = names.iter().flat_map(|p| fs::read(p).unwrap()).collect();
        (log, report, frames)
    };
    let field = |report: &str, key: &str| -> u64 {
        let at = report.find(&format!("\"{key}\":")).unwrap() + key.len() + 3;
        report[at..]
            .split(|c: char| !c.is_ascii_digit())
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    let (log, report, resident) = run(false);
    if log.contains("using the perturbation kernel") {
        eprintln!("skipped: no wgpu zone device ({log})");
        return;
    }
    assert!(
        log.contains("(resident)"),
        "zone keyframes did not stay on the GPU: {log}"
    );
    let zone_keyframes = log.matches("(resident)").count() as u64;
    assert_eq!(
        field(&report, "zone_readback_bytes"),
        4 * zone_keyframes,
        "{report}"
    );
    let (log_b, report_b, readback) = run(true);
    assert!(!log_b.contains("(resident)"), "{log_b}");
    assert!(field(&report_b, "zone_readback_bytes") > 4 * zone_keyframes);
    assert_eq!(resident.len(), 8 * 320 * 180 * 3);
    assert!(resident == readback, "resident and readback films differ");
    eprintln!(
        "{zone_keyframes} resident zone keyframes, {} bytes read back (readback path {} bytes); raw films identical",
        field(&report, "zone_readback_bytes"),
        field(&report_b, "zone_readback_bytes")
    );
    fs::remove_dir_all(dir).unwrap();
}

/// Read a numeric member from the small report objects (same convention as bench.rs).
fn report_number(json: &str, key: &str) -> f64 {
    let prefix = format!("\"{key}\":");
    let rest = json
        .split_once(&prefix)
        .unwrap_or_else(|| panic!("missing {key}: {json}"))
        .1;
    rest.split([',', '}']).next().unwrap().parse().unwrap()
}

fn report_object<'a>(json: &'a str, key: &str) -> &'a str {
    let prefix = format!("\"{key}\":{{");
    let rest = json
        .split_once(&prefix)
        .unwrap_or_else(|| panic!("missing {key}: {json}"))
        .1;
    rest.split_once('}').unwrap().0
}

#[test]
fn film_report_kernel_breakdown() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("fd-prob28-{}-{stamp}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let zone = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bench/zones/v0-core.zone");
    let re = "-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502";
    let im = "0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922";
    for keys in [false, true] {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_fd"));
        cmd.args([
            "film",
            "--json",
            re,
            im,
            "--from",
            "1e-26",
            "--to",
            "1e-29",
            "--seconds",
            "0.2",
            "--fps",
            "30",
            "--size",
            "32x18",
            "--iter",
            "20000",
            "--threads",
            "1",
            "--x264",
            "ultrafast",
        ])
        .arg("--zone")
        .arg(&zone)
        .arg("--mp4")
        .arg(dir.join(format!("boundary-{keys}.mp4")));
        if keys {
            cmd.args(["--keyframes", "1"]);
        } else {
            cmd.args(["--ss", "1", "--keyframes", "off"]);
        }
        let out = checked(cmd.output().unwrap(), "film breakdown");
        let json = String::from_utf8(out.stdout).unwrap();
        assert!(json.starts_with("{\"schema\":\"fd-film/1\""));
        let routes = json.split_once("\"routes\":{").unwrap().1;
        let mut seconds = 0.0;
        let mut frames = 0.0;
        for route in [
            "zone",
            "zone-gpu",
            "mid",
            "bla",
            "perturbation",
            "scaled",
            "series-bla",
            "series-perturbation",
        ] {
            let object = report_object(routes, route);
            seconds += report_number(object, "seconds");
            frames += report_number(object, "frames");
        }
        assert_eq!(frames, 6.0);
        let render = report_number(&json, "total_seconds");
        assert!((seconds - render).abs() <= render * 0.05, "{json}");
        assert!(
            report_number(report_object(routes, "zone-gpu"), "kernel_frames") > 0.0,
            "{json}"
        );
        assert!(
            ["bla", "perturbation", "series-bla", "series-perturbation"]
                .iter()
                .map(|r| report_number(report_object(routes, r), "kernel_frames"))
                .sum::<f64>()
                > 0.0,
            "{json}"
        );
        for counter in ["reference_builds", "bla_builds"] {
            let object = report_object(&json, counter);
            assert!(report_number(object, "total") > 0.0);
            assert!(report_number(object, "repeated") <= report_number(object, "total"));
        }
        assert!(report_number(&json, "reference_seconds") > 0.0);
        assert!(report_number(&json, "bla_build_seconds") > 0.0);
    }
    // Direct and jet-AA films build once per output frame, not once per sample
    // mask. Identical inputs are counted as repeated, not silently cached.
    for jet in [false, true] {
        let out = checked(
            Command::new(env!("CARGO_BIN_EXE_fd"))
                .args([
                    "film",
                    "-0.75",
                    "0.1",
                    "--from",
                    "0.01",
                    "--to",
                    "0.01",
                    "--seconds",
                    "0.2",
                    "--fps",
                    "30",
                    "--size",
                    "16x10",
                    "--ss",
                    if jet { "2" } else { "1" },
                    "--jet-aa",
                    if jet { "on" } else { "off" },
                    "--keyframes",
                    "off",
                    "--iter",
                    "128",
                    "--threads",
                    "1",
                    "--x264",
                    "ultrafast",
                ])
                .arg("--mp4")
                .arg(dir.join(format!("repeated-{jet}.mp4")))
                .output()
                .unwrap(),
            "repeated builds",
        );
        let json = String::from_utf8(out.stdout).unwrap();
        for counter in ["reference_builds", "bla_builds"] {
            let object = report_object(&json, counter);
            assert_eq!(report_number(object, "total"), 6.0, "{json}");
            assert_eq!(report_number(object, "repeated"), 5.0, "{json}");
            let repeated = report_number(object, "repeated_seconds");
            let total = report_number(
                &json,
                if counter == "reference_builds" {
                    "reference_seconds"
                } else {
                    "bla_build_seconds"
                },
            );
            assert!(repeated > 0.0 && repeated <= total, "{json}");
        }
    }
    fs::remove_dir_all(dir).unwrap();
}
