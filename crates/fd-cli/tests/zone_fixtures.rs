//! CLEA-06: `fd render --zone` (the wgpu zone kernel by default) against frozen
//! CPU-zone frames, on compact fixture zones (tests/fixtures/zones: the v0 zone and the
//! k = 409 and k = 7676 ladder rungs, without their tail-patch trees, and the rungs
//! without the mid-band reference orbit they do not use). The frames are PROB-08's six
//! deep frames, KERN-02's ten mid-band frames and the rung frames at 5000/500/50/5
//! minibrot sizes, all 96x54. Strict bar (DEC-17): 0 class mismatches, every escaped
//! sample within 1e-3 px in nu position, and fd compare's DE (0.2%) and normal (0.2
//! degree) gates where the frozen DE exceeds 1e-3 px.
//!
//! `FD_REGENERATE_FIXTURES=1` rewrites the frozen frames with `--gpu-kernel cpu` first.
//! With `FD_REQUIRE_GPU` set (CI), a frame the GPU did not render fails.
use fd_samples::{Reader, Samples};
use std::path::{Path, PathBuf};
use std::process::Command;

/// v0's nucleus to 100 digits (bench/path-atlas-v0.txt).
const V0_RE: &str = "-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502";
const V0_IM: &str = "0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922";

struct Frame {
    name: String,
    zone: &'static str,
    re: String,
    im: String,
    width: String,
    iter: u64,
}

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/zones")
}

fn frames() -> Vec<Frame> {
    let mut v = Vec::new();
    // PROB-08's deep frames, centred at the nucleus.
    for width in ["1e-35", "1e-38", "1e-40", "1e-43", "1e-46", "2e-48"] {
        v.push(Frame {
            name: format!("v0-deep-{width}"),
            zone: "v0.zone",
            re: V0_RE.into(),
            im: V0_IM.into(),
            width: width.into(),
            iter: 100_000,
        });
    }
    // KERN-02's mid-band frames on v0 (path-atlas rows 100..400, GPU-22).
    for (row, width) in [
        (100, "1.04668e-6"),
        (175, "1.21095e-11"),
        (200, "2.73884e-13"),
        (225, "6.19447e-15"),
        (250, "1.40101e-16"),
        (275, "3.1687e-18"),
        (300, "7.1667e-20"),
        (325, "1.6209e-21"),
        (350, "3.66603e-23"),
        (400, "1.87531e-26"),
    ] {
        v.push(Frame {
            name: format!("v0-mid-{row}"),
            zone: "v0.zone",
            re: V0_RE.into(),
            im: V0_IM.into(),
            width: width.into(),
            iter: 100_000,
        });
    }
    // Ladder rungs (GPU-23) at PROB-19's widths, max_iter 40 periods.
    let rungs = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tools/research/misiurewicz/ladder_rungs.txt"),
    )
    .unwrap();
    for (k, zone, size) in [
        (409, "rung409.zone", "9.2e-101"),
        (7676, "rung7676.zone", "1.01e-1000"),
    ] {
        let line = rungs
            .lines()
            .find(|l| l.starts_with(&format!("{k} ")))
            .unwrap();
        let f: Vec<&str> = line.split_whitespace().collect();
        let period: u64 = f[1].parse().unwrap();
        let (m, e) = size.split_once('e').unwrap();
        for mult in [5000.0, 500.0, 50.0, 5.0] {
            v.push(Frame {
                name: format!("rung{k}-{mult}"),
                zone,
                re: f[2].into(),
                im: f[3].into(),
                width: format!("{}e{e}", m.parse::<f64>().unwrap() * mult),
                iter: 40 * period,
            });
        }
    }
    v
}

/// `fd render` of `frame` into `out`; returns stderr.
fn render(frame: &Frame, out: &Path, kernel: Option<&str>) -> String {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_fd"));
    cmd.args([
        "render",
        "--re",
        &frame.re,
        "--im",
        &frame.im,
        "--width",
        &frame.width,
    ])
    .args(["--size", "96x54", "--iter", &frame.iter.to_string()])
    .args(["--columns", "nu,de,normal", "--zone"])
    .arg(dir().join(frame.zone))
    .arg("-o")
    .arg(out);
    if let Some(k) = kernel {
        cmd.args(["--gpu-kernel", k]);
    }
    let o = cmd.output().unwrap();
    let err = String::from_utf8_lossy(&o.stderr).into_owned();
    assert!(o.status.success(), "{}: {err}", frame.name);
    err
}

fn read(path: &Path) -> (String, Samples) {
    let mut r = Reader::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let columns = r.header.columns;
    let s = r.read(columns).unwrap();
    (r.header.kernel.clone(), s)
}

#[test]
fn zone_fixtures() {
    let frozen = dir().join("frames");
    let tmp = std::env::temp_dir().join(format!("fd-zone-fixtures-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let regenerate = std::env::var_os("FD_REGENERATE_FIXTURES").is_some();
    if regenerate {
        std::fs::create_dir_all(&frozen).unwrap();
    }
    let require_gpu = std::env::var_os("FD_REQUIRE_GPU").is_some();
    // FD_ZONE_KERNEL=K runs `--gpu-kernel K` instead of fd's own choice.
    let kernel = std::env::var("FD_ZONE_KERNEL").ok();
    let mut failures = Vec::new();
    for frame in frames() {
        let want = frozen.join(format!("{}.fds", frame.name));
        if regenerate {
            render(&frame, &want, Some("cpu"));
        }
        let got = tmp.join(format!("{}.fds", frame.name));
        let log = render(&frame, &got, kernel.as_deref());
        let (_, a) = read(&want);
        let (used, b) = read(&got);
        let (an, bn) = (a.nu.as_ref().unwrap(), b.nu.as_ref().unwrap());
        let (ad, bd) = (a.de.as_ref().unwrap(), b.de.as_ref().unwrap());
        let (aw, bw) = (a.normal.as_ref().unwrap(), b.normal.as_ref().unwrap());
        let (mut classes, mut nu_off, mut de_off, mut normal_off) = (0, 0, 0, 0);
        let mut worst = [0.0f64; 3];
        for i in 0..a.class.len() {
            if a.class[i].kind() != b.class[i].kind() {
                classes += 1;
                continue;
            }
            if !a.class[i].escaped() {
                continue;
            }
            let de = f64::from(ad[i]);
            let px = (an[i] - bn[i]).abs() * std::f64::consts::LN_2 * de / 2.0;
            worst[0] = worst[0].max(px);
            nu_off += usize::from(px.is_nan() || px > 1e-3);
            if de > 1e-3 {
                let rel = (f64::from(bd[i]) / de - 1.0).abs();
                let deg = f64::from(aw[i].wrapping_sub(bw[i]).min(bw[i].wrapping_sub(aw[i])))
                    * 360.0
                    / 65536.0;
                worst[1] = worst[1].max(rel);
                worst[2] = worst[2].max(deg);
                de_off += usize::from(rel.is_nan() || rel > 2e-3);
                normal_off += usize::from(deg > 0.2);
            }
        }
        eprintln!(
            "CLEA-06 {} ({}): class {classes}, nu > 1e-3 px {nu_off}, de > 0.2% {de_off}, normal > 0.2 deg {normal_off}; worst nu {:.1e} px, de {:.1e}, normal {:.1e} deg",
            frame.name, used, worst[0], worst[1], worst[2]
        );
        if (classes, nu_off, de_off, normal_off) != (0, 0, 0, 0) {
            failures.push(frame.name.clone());
        }
        if require_gpu && !used.contains("wgpu") {
            failures.push(format!("{} not on the GPU ({used}): {log}", frame.name));
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
    assert!(failures.is_empty(), "{failures:?}");
}
