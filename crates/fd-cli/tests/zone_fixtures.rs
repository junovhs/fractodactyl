//! CLEA-06: `fd render --zone` (the wgpu zone kernel by default) against frozen
//! CPU-zone frames, on compact fixture zones (tests/fixtures/zones: the v0 zone and the
//! k = 409 and k = 7676 ladder rungs, without their tail-patch trees, and the rungs
//! without the mid-band reference orbit they do not use). The frames are PROB-08's six
//! deep frames, KERN-02's ten mid-band frames and the rung frames at 5000/500/50/5
//! minibrot sizes, all 96x54. Strict bar (DEC-17): 0 class mismatches, every escaped
//! sample within 1e-3 px in nu position, and fd compare's DE (0.2%) and normal (0.2
//! degree) gates where the frozen DE exceeds 1e-3 px.
//!
//! The frames were frozen from the CPU zone, which CLEA-04 deleted: they cannot be
//! regenerated, and a change that moves them needs its own evidence. With
//! `FD_REQUIRE_GPU` set (CI), a frame the GPU did not render fails.
//!
//! `zone_fixtures_f32_kernels` holds the f32 and mixed kernels (f64-capable Vulkan GPUs
//! only) to GPU-02's guarded bar on the v0 frames.
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

/// Differences of one rendered frame from its frozen frame.
#[derive(Default)]
struct Diff {
    classes: usize,
    nu_off: usize,
    de_off: usize,
    normal_off: usize,
    /// Samples with any of nu, DE or normal over 1e-3 px (GPU-02's strict count).
    strict: usize,
    samples: usize,
    /// Worst nu px, DE relative error, normal degrees, and worst error of the three in px.
    worst: [f64; 4],
}

fn diff(a: &Samples, b: &Samples) -> Diff {
    let (an, bn) = (a.nu.as_ref().unwrap(), b.nu.as_ref().unwrap());
    let (ad, bd) = (a.de.as_ref().unwrap(), b.de.as_ref().unwrap());
    let (aw, bw) = (a.normal.as_ref().unwrap(), b.normal.as_ref().unwrap());
    let mut d = Diff {
        samples: a.class.len(),
        ..Diff::default()
    };
    for i in 0..a.class.len() {
        if a.class[i].kind() != b.class[i].kind() {
            d.classes += 1;
            continue;
        }
        if !a.class[i].escaped() {
            continue;
        }
        let de = f64::from(ad[i]);
        let px = (an[i] - bn[i]).abs() * std::f64::consts::LN_2 * de / 2.0;
        d.worst[0] = d.worst[0].max(px);
        d.nu_off += usize::from(px.is_nan() || px > 1e-3);
        let turn = f64::from(aw[i].wrapping_sub(bw[i]).min(bw[i].wrapping_sub(aw[i])));
        let any = px
            .max((de - f64::from(bd[i])).abs())
            .max(turn * std::f64::consts::TAU / 65536.0 * de);
        d.worst[3] = d.worst[3].max(any);
        d.strict += usize::from(any.is_nan() || any > 1e-3);
        if de > 1e-3 {
            let rel = (f64::from(bd[i]) / de - 1.0).abs();
            let deg = turn * 360.0 / 65536.0;
            d.worst[1] = d.worst[1].max(rel);
            d.worst[2] = d.worst[2].max(deg);
            d.de_off += usize::from(rel.is_nan() || rel > 2e-3);
            d.normal_off += usize::from(deg > 0.2);
        }
    }
    d
}

/// Render `frame` with `kernel` (fd's own choice when `None`) and compare it with its
/// frozen frame: the kernel tag used, the log and the differences.
fn check(frame: &Frame, tmp: &Path, kernel: Option<&str>) -> (String, String, Diff) {
    let want = dir().join("frames").join(format!("{}.fds", frame.name));
    let got = tmp.join(format!("{}.fds", frame.name));
    let log = render(frame, &got, kernel);
    let (_, a) = read(&want);
    let (used, b) = read(&got);
    (used, log, diff(&a, &b))
}

fn scratch(tag: &str) -> PathBuf {
    let tmp = std::env::temp_dir().join(format!("fd-zone-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

#[test]
fn zone_fixtures() {
    let tmp = scratch("fixtures");
    let require_gpu = std::env::var_os("FD_REQUIRE_GPU").is_some();
    // FD_ZONE_KERNEL=K runs `--gpu-kernel K` instead of fd's own choice.
    let kernel = std::env::var("FD_ZONE_KERNEL").ok();
    let mut failures = Vec::new();
    for frame in frames() {
        let (used, log, d) = check(&frame, &tmp, kernel.as_deref());
        eprintln!(
            "CLEA-06 {} ({}): class {}, nu > 1e-3 px {}, de > 0.2% {}, normal > 0.2 deg {}; worst nu {:.1e} px, de {:.1e}, normal {:.1e} deg",
            frame.name, used, d.classes, d.nu_off, d.de_off, d.normal_off, d.worst[0], d.worst[1], d.worst[2]
        );
        if (d.classes, d.nu_off, d.de_off, d.normal_off) != (0, 0, 0, 0) {
            failures.push(frame.name.clone());
        }
        if require_gpu && !used.contains("wgpu") {
            failures.push(format!("{} not on the GPU ({used}): {log}", frame.name));
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
    assert!(failures.is_empty(), "{failures:?}");
}

/// GPU-02's bar for the f32 and mixed kernels, which replaced their parity tests against
/// the CPU zone: at most one class mismatch, every escaped sample's nu, DE and normal
/// within 0.05 px, and at most 0.5% of samples over 1e-3 px. A kernel whose first frame
/// renders elsewhere (f32only on a GPU without f64, as on lavapipe and WARP) is skipped.
#[test]
fn zone_fixtures_f32_kernels() {
    let tmp = scratch("f32-kernels");
    let mut failures = Vec::new();
    for kernel in ["mixed", "f32"] {
        for frame in frames().iter().filter(|f| f.zone == "v0.zone") {
            let (used, _, d) = check(frame, &tmp, Some(kernel));
            if !used.contains(&format!("wgpu-{kernel}/")) {
                eprintln!("GPU-02 {kernel}: SKIP, {} rendered by {used}", frame.name);
                break;
            }
            eprintln!(
                "GPU-02 {kernel} {}: class {}, > 1e-3 px {}, worst {:.2e} px",
                frame.name, d.classes, d.strict, d.worst[3]
            );
            if d.classes > 1 || d.worst[3] > 0.05 || d.strict * 200 > d.samples {
                failures.push(format!("{kernel} {}", frame.name));
            }
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
    assert!(failures.is_empty(), "{failures:?}");
}
