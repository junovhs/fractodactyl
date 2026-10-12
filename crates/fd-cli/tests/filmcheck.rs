//! FILM-08: full-resolution shaded film equivalence, including the raw-dir path.
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

fn render(dir: &Path, name: &str, iter: &str) {
    let frames = dir.join(name);
    let movie = dir.join(format!("{name}.mp4"));
    checked(
        Command::new(env!("CARGO_BIN_EXE_fd"))
            .args([
                "film",
                "-0.75",
                "0.1",
                "--from",
                "0.5",
                "--to",
                "0.45",
                "--seconds",
                "0.75",
                "--fps",
                "4",
                "--size",
                "320x180",
                "--ss",
                "1",
                "--iter",
                iter,
                "--threads",
                "2",
                "--gpu",
                "off",
                "--crf",
                "30",
                "--x264",
                "ultrafast",
            ])
            .arg("--raw-dir")
            .arg(frames)
            .arg("--mp4")
            .arg(movie)
            .output()
            .expect("fd film"),
        "film",
    );
}

#[test]
fn filmcheck() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("fd-film08-{}-{stamp}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    render(&dir, "reference", "400");
    render(&dir, "identical", "400");
    render(&dir, "degraded", "40");
    let ref_dir = dir.join("reference");
    let good = Command::new(env!("CARGO_BIN_EXE_fd"))
        .arg("filmcheck")
        .arg(&ref_dir)
        .arg(dir.join("identical"))
        .arg("--heatmap")
        .arg(dir.join("identical.ppm"))
        .output()
        .unwrap();
    let good = checked(good, "identical filmcheck");
    let text = String::from_utf8_lossy(&good.stdout);
    assert!(text.contains("PASS fd-filmcheck/1 frames=3"), "{text}");
    assert_eq!(text.matches(": within_1=100.0000%").count(), 3, "{text}");
    assert_eq!(
        text.matches("outside_noise_within_1=100.0000%").count(),
        3,
        "{text}"
    );

    let bad_map = dir.join("degraded.ppm");
    let bad = Command::new(env!("CARGO_BIN_EXE_fd"))
        .arg("filmcheck")
        .arg(&ref_dir)
        .arg(dir.join("degraded"))
        .arg("--heatmap")
        .arg(&bad_map)
        .output()
        .unwrap();
    assert_eq!(
        bad.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&bad.stdout)
    );
    let text = String::from_utf8_lossy(&bad.stdout);
    assert!(text.contains("FAIL fd-filmcheck/1 frames=3"), "{text}");
    assert!(text.contains("outside_noise_within_1="), "{text}");
    assert!(fs::read(&bad_map)
        .unwrap()
        .starts_with(b"P6\n320 180\n255\n"));
    assert!(fs::metadata(&bad_map).unwrap().len() > 320 * 180 * 3);

    // Also check the encoded input decoding path, without treating x264 as reference truth.
    let decoded = Command::new(env!("CARGO_BIN_EXE_fd"))
        .arg("filmcheck")
        .arg(dir.join("reference.mp4"))
        .arg(&ref_dir)
        .arg("--heatmap")
        .arg(dir.join("encoded.ppm"))
        .output()
        .unwrap();
    assert!(
        decoded.status.code().is_some(),
        "filmcheck hung decoding mp4"
    );
    assert!(String::from_utf8_lossy(&decoded.stdout).contains("fd-filmcheck/1"));

    fs::remove_dir_all(dir).unwrap();
}
