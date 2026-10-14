//! FILM-14: decoded CPU keyframe spins track direct rotated film frames.
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
            cmd.args(["--ss", "1"]);
        }
        let out = checked(cmd.output().unwrap(), "film breakdown");
        let json = String::from_utf8(out.stdout).unwrap();
        assert!(json.starts_with("{\"schema\":\"fd-film/1\""));
        let routes = json.split_once("\"routes\":{").unwrap().1;
        let mut seconds = 0.0;
        let mut frames = 0.0;
        for route in ["zone", "zone-gpu", "mid", "bla", "perturbation", "scaled"] {
            let object = report_object(routes, route);
            seconds += report_number(object, "seconds");
            frames += report_number(object, "frames");
        }
        assert_eq!(frames, 6.0);
        let render = report_number(&json, "total_seconds");
        assert!((seconds - render).abs() <= render * 0.05, "{json}");
        assert!(
            report_number(report_object(routes, "zone"), "kernel_frames") > 0.0,
            "{json}"
        );
        assert!(
            report_number(report_object(routes, "bla"), "kernel_frames") > 0.0,
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
    // Identical inputs across direct output frames must be counted as repeated,
    // rather than cached; every frame still builds both mathematical products.
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
                "1",
                "--iter",
                "128",
                "--threads",
                "1",
                "--x264",
                "ultrafast",
            ])
            .arg("--mp4")
            .arg(dir.join("repeated.mp4"))
            .output()
            .unwrap(),
        "repeated builds",
    );
    let json = String::from_utf8(out.stdout).unwrap();
    for counter in ["reference_builds", "bla_builds"] {
        let object = report_object(&json, counter);
        assert_eq!(report_number(object, "total"), 6.0, "{json}");
        assert_eq!(report_number(object, "repeated"), 5.0, "{json}");
    }
    fs::remove_dir_all(dir).unwrap();
}
