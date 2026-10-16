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

/// SHAD-07: a rotating keyframed studio film on the wgpu GPU path matches the CPU film.
#[test]
fn gpu_film_matches_cpu() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("fd-shad07-{}-{stamp}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let speed = dir.join("speed.txt");
    let spins = dir.join("spins.txt");
    fs::write(&speed, format!("{:.16} 4\n", 4f64.log10())).unwrap();
    fs::write(&spins, "0 1.2 0.25\n").unwrap();
    // The ice palette with terraces and relief but no lines (lines are SHAD-05's).
    let look = dir.join("ice-plain.look");
    fs::write(
        &look,
        "stops #08152e #0b3d91 #1e7fe0 #7cc4ff #eef6fc #c9c3bc #7d7a78 #2a3140\n\
         interior #05070d\ndensity 0.05\nterrace 0.5\nslope 1.1\nlight 135\nlines 0\n",
    )
    .unwrap();
    let mut frames = Vec::new();
    for gpu in ["on", "off"] {
        let raw = dir.join(format!("raw-{gpu}"));
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
                "320x180",
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
                "--gpu",
                gpu,
            ])
            .arg("--raw-dir")
            .arg(&raw)
            .arg("--mp4")
            .arg(dir.join(format!("{gpu}.mp4")))
            .output()
            .unwrap();
        let out = checked(out, "film");
        let log = String::from_utf8_lossy(&out.stderr);
        if gpu == "on" {
            if log.contains("no wgpu") {
                eprintln!("skipped: no wgpu adapter ({log})");
                return;
            }
            assert!(
                log.contains("GPU shading on") && !log.contains("using CPU"),
                "--gpu on did not shade on the GPU: {log}"
            );
            eprintln!(
                "{}",
                log.lines().find(|l| l.contains("GPU shading on")).unwrap()
            );
        }
        let mut names: Vec<_> = fs::read_dir(&raw)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "rgb"))
            .collect();
        names.sort();
        frames.push(
            names
                .iter()
                .map(|p| fs::read(p).unwrap())
                .collect::<Vec<_>>(),
        );
    }
    assert_eq!(frames[0].len(), 5);
    assert_eq!(frames[0].len(), frames[1].len());
    let mut worst = 0;
    for (i, (g, c)) in frames[0].iter().zip(&frames[1]).enumerate() {
        assert_eq!(g.len(), 320 * 180 * 3);
        for (k, (&a, &b)) in g.iter().zip(c).enumerate() {
            worst = worst.max(a.abs_diff(b));
            assert!(
                a.abs_diff(b) <= 1,
                "frame {i} pixel ({},{}) channel {}: GPU {a}, CPU {b}",
                k / 3 % 320,
                k / 3 / 320,
                k % 3
            );
        }
    }
    eprintln!("5 rotating frames: GPU within {worst} level(s) of CPU");
}
