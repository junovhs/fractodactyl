//! Drives the real `fd play` (VIDE-02) on a compiled cut of the Atlas v0 path: every
//! frame renders from the atlas alone (no reference, no operator built), its camera
//! comes from the frame manifest, empty BLA tables are skipped, loaded chunks are reused
//! by later frames (warm), and every played frame matches an independent `fd render` of
//! the same view: identical classes everywhere, identical bytes where no table applies
//! (given the same stored orbit), and `nu` within the BLA shift contract where one does.
use fd_samples::{Column, ColumnSet, Kind, Reader};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fd(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fd")).args(args).output().unwrap()
}

fn ok(o: &Output) -> String {
    let text = String::from_utf8(o.stdout.clone()).unwrap();
    assert!(o.status.success(), "{text}\n{}", String::from_utf8_lossy(&o.stderr));
    text
}

/// The number after the first `"key":` in a flat scan of one JSON line.
fn num(json: &str, key: &str) -> f64 {
    let at = json.find(&format!("\"{key}\":")).unwrap_or_else(|| panic!("no {key} in {json}"));
    let rest = &json[at + key.len() + 3..];
    let end = rest.find([',', '}', ']']).unwrap();
    rest[..end].parse().unwrap_or_else(|_| panic!("{key}: {}", &rest[..end]))
}

/// The string after the first `"key":"`.
fn string<'a>(json: &'a str, key: &str) -> &'a str {
    let at = json.find(&format!("\"{key}\":\"")).unwrap_or_else(|| panic!("no {key} in {json}"));
    let rest = &json[at + key.len() + 4..];
    &rest[..rest.find('"').unwrap()]
}

fn committed() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bench/path-atlas-v0.txt")
}

fn samples(p: &Path) -> fd_samples::Samples {
    Reader::open(p).unwrap().read(ColumnSet::of(&[Column::Class, Column::Nu, Column::De])).unwrap()
}

#[test]
fn plays_a_compiled_cut_from_the_atlas() {
    let dir = std::env::temp_dir().join(format!("fd-play-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // 12 frames over the whole depth range of the v0 path, one centre.
    let text = std::fs::read_to_string(committed()).unwrap();
    let first = text.lines().find(|l| !l.starts_with('#')).unwrap();
    let (re, im) = (first.split(' ').next().unwrap(), first.split(' ').nth(1).unwrap());
    let path = dir.join("cut.txt");
    let cut = ok(&fd(&["path", "zoom", "--re", re, "--im", im, "--from", "4", "--to", "2e-49", "--seconds", "12", "--fps", "1"]));
    std::fs::write(&path, &cut).unwrap();
    let widths: Vec<String> =
        cut.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()).map(|l| l.split(' ').nth(2).unwrap().to_string()).collect();
    assert_eq!(widths.len(), 12);
    let render = ["--size", "32x18", "--iter", "20000", "--columns", "nu,de,normal"];
    let st = dir.join("atlas");
    let s = st.to_str().unwrap();
    // Wide BLA level runs, so neighbouring frames share a table (warm reuse).
    let log = ok(&fd(&[&["compile", path.to_str().unwrap(), "--store", s, "--workers", "2", "--bla-levels", "64"][..], &render].concat()));
    let log_file = dir.join("compile.log");
    std::fs::write(&log_file, &log).unwrap();
    let lf = log_file.to_str().unwrap();

    let frames = dir.join("frames");
    let fr = frames.to_str().unwrap();
    let out = ok(&fd(&["play", lf, "--store", s, "--threads", "2", "-o", fr, "--look", "umber"]));
    let records: Vec<&str> = out.lines().collect();
    assert_eq!(records.len(), 13, "{out}");
    let (mut warm, mut empty, mut used) = (0, 0, 0);
    for (f, rec) in records[..12].iter().enumerate() {
        assert!(rec.starts_with(&format!("{{\"schema\":\"fd-play/1\",\"record\":\"frame\",\"frame\":{f},")), "{rec}");
        for k in ["\"atlas\":\"store\",\"cross_frame_reuse\":true", "\"reference_seconds\":0,\"operator_seconds\":0", "\"microblocks_touched\":0", "\"ok\":true}"] {
            assert!(rec.contains(k), "missing {k} in {rec}");
        }
        assert_eq!(num(rec, "reference_seconds"), 0.0, "{rec}");
        assert!(num(rec, "tiles_touched") > 0.0 && num(rec, "atlas_bytes_referenced") > 0.0, "{rec}");
        let state = string(rec, "state");
        warm += usize::from(state == "warm");
        if f == 0 {
            assert_eq!(state, "cold");
        }
        // The camera came from the manifest: the exact centre and the path's width.
        assert!(rec.contains(&format!("\"re\":\"{re}\"")), "{rec}");
        let w: f64 = string(rec, "width").parse().unwrap();
        let path_w: f64 = widths[f].parse().unwrap();
        assert!((w / path_w - 1.0).abs() < 1e-5, "frame {f}: {w} vs {path_w}");
        assert!(frames.join(format!("frame-{f:05}.umber.png")).is_file());

        // An independent render of the same view, the cold way.
        let width = string(rec, "width");
        let view = ["--re", re, "--im", im, "--width", width];
        let plain = dir.join(format!("plain-{f}.fds"));
        ok(&fd(&[&["render"][..], &view, &render, &["--threads", "2", "-o", plain.to_str().unwrap()]].concat()));
        let played = frames.join(format!("frame-{f:05}.fds"));
        let (a, b) = (samples(&plain), samples(&played));
        match string(rec, "bla_use") {
            "used" => {
                used += 1;
                // BLA judges periodicity at the plain kernel's iterates (blocks end at the
                // Brent save points, FIX-02): every class matches fd render exactly.
                let mismatched = a.class.iter().zip(&b.class).filter(|(x, y)| x != y).count();
                assert_eq!(mismatched, 0, "frame {f}: {mismatched} classes differ from fd render");
                let shift = num(rec, "shift_px_max");
                assert!(num(rec, "bla_blocks") > 0.0 && shift < 0.25, "{rec}");
                assert!(num(rec, "macro_operators_per_pixel") > 0.0, "{rec}");
                let (an, bn, de) = (a.nu.unwrap(), b.nu.unwrap(), a.de.unwrap());
                for i in 0..a.class.len() {
                    if a.class[i].kind() == Some(Kind::Escaped) {
                        // Displacement in output pixels; first-order estimate plus rounding.
                        let px = (an[i] - bn[i]).abs() * f64::from(de[i]) * std::f64::consts::LN_2 / 2.0;
                        assert!(px <= 2.0 * shift + 1e-6, "frame {f} sample {i}: {px} px, shift {shift}");
                    }
                }
            }
            u => {
                assert_eq!(a.class, b.class, "frame {f}: classes differ from fd render");
                empty += usize::from(u == "skipped_empty");
                assert_eq!(num(rec, "pixel_fraction"), 1.0, "{rec}");
                // Without a table the play is fd render with that stored orbit, bit for bit.
                let same = dir.join(format!("orbit-{f}.fds"));
                let orbit = string(rec, "orbit");
                ok(&fd(&[&["render"][..], &view, &render, &["--threads", "2", "--store", s, "--orbit", orbit, "-o", same.to_str().unwrap()]].concat()));
                assert_eq!(std::fs::read(&same).unwrap(), std::fs::read(&played).unwrap(), "frame {f}");
            }
        }
    }
    assert!(warm > 0 && used > 0 && empty > 0, "warm {warm} used {used} empty {empty}\n{out}");
    let t = records[12];
    assert!(t.starts_with("{\"schema\":\"fd-play/1\",\"record\":\"totals\""), "{t}");
    assert_eq!(num(t, "frames"), 12.0);
    assert_eq!(num(t, "warm_frames"), warm as f64);
    assert_eq!(num(t, "frames_empty_table_skipped"), empty as f64);
    assert_eq!(num(t, "frames_used"), used as f64);
    assert_eq!(num(t, "pngs"), 12.0);
    assert!(t.contains("\"error\":null,\"ok\":true}"), "{t}");

    // A range plays just those frames; usage errors exit 2.
    let part = ok(&fd(&["play", lf, "--store", s, "--frames", "10..12", "--threads", "2"]));
    assert_eq!(part.lines().count(), 3);
    assert!(part.starts_with("{\"schema\":\"fd-play/1\",\"record\":\"frame\",\"frame\":10,"), "{part}");
    for bad in [&["--frames", "5..99"][..], &["--look", "umber"], &["--mp4", "x.mp4", "-o", fr], &["--every", "0"]] {
        let o = fd(&[&["play", lf, "--store", s][..], bad].concat());
        assert_eq!(o.status.code(), Some(2), "{bad:?}");
    }
    assert_eq!(fd(&["play", path.to_str().unwrap(), "--store", s]).status.code(), Some(2));
    let _ = std::fs::remove_dir_all(&dir);
}
