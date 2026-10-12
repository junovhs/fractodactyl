//! CPU kernel contract check. It exercises the kernels used by fd's render routes.
use fd_kernel::{Params, Tier, Zone};
use fd_samples::{Column, ColumnSet, View};
use std::fs;
use std::path::Path;
use std::time::Instant;

struct Case {
    name: String,
    view: View,
    params: Params,
    zone: bool,
}

fn params(nx: u32, ny: u32, max_iter: u64, tier: Option<Tier>) -> Params {
    Params {
        nx,
        ny,
        ss: 1,
        max_iter,
        escape_radius: 1e10,
        columns: ColumnSet::of(&[Column::Nu, Column::De, Column::Normal]),
        threads: 1,
        tier,
    }
}

fn cases() -> Result<Vec<Case>, String> {
    let mut out = Vec::new();
    for line in include_str!("../../../bench/locations.txt").lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let f: Vec<_> = line.split_whitespace().collect();
        if f.len() != 9 {
            return Err(format!("bad location: {line}"));
        }
        let (nx, ny) = f[5].split_once('x').ok_or("bad size")?;
        let tier = match f[7] {
            "auto" => None,
            "f64" => Some(Tier::F64),
            "fx" => Some(Tier::Fixed),
            "scaled" => Some(Tier::Scaled),
            other => return Err(format!("unknown tier {other}")),
        };
        out.push(Case {
            name: f[0].into(),
            view: View {
                center_re: f[1].into(),
                center_im: f[2].into(),
                width: f[3].into(),
                rotation: f[6].parse().map_err(|e| format!("{e}"))?,
            },
            params: params(
                nx.parse().map_err(|e| format!("{e}"))?,
                ny.parse().map_err(|e| format!("{e}"))?,
                f[4].parse().map_err(|e| format!("{e}"))?,
                tier,
            ),
            zone: false,
        });
    }
    let rung = include_str!("../../../tools/research/misiurewicz/ladder_rungs.txt")
        .lines()
        .find(|line| line.starts_with("7676 "))
        .ok_or("missing rung 7676")?;
    let f: Vec<_> = rung.split_whitespace().collect();
    out.push(Case {
        name: "rung-7676".into(),
        view: View {
            center_re: f[2].into(),
            center_im: f[3].into(),
            width: "5.05e-997".into(),
            rotation: 0.0,
        },
        params: params(96, 54, 150_000, None),
        zone: false,
    });
    for width in ["1e-35", "1e-38", "1e-40", "1e-43", "1e-46", "2e-48"] {
        out.push(Case {
            name: format!("zone-{width}"),
            view: View {
                center_re: "-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502".into(),
                center_im: "0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922".into(),
                width: width.into(), rotation: 0.0,
            },
            params: params(96, 54, 20_000, None), zone: true,
        });
    }
    Ok(out)
}

fn bytes(case: &Case, zone: &Zone) -> Result<Vec<u8>, String> {
    let (header, samples) = if case.zone {
        let (h, s, _, _) = fd_kernel::render_zone(&case.view, &case.params, zone)?;
        (h, s)
    } else {
        fd_kernel::render(&case.view, &case.params)?
    };
    let mut bytes = Vec::new();
    fd_samples::write(&mut bytes, &header, &samples).map_err(|e| e.to_string())?;
    Ok(bytes)
}

fn append(out: &mut Vec<u8>, name: &str, bytes: &[u8]) {
    out.extend_from_slice(&(name.len() as u64).to_le_bytes());
    out.extend_from_slice(name.as_bytes());
    out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    out.extend_from_slice(bytes);
}

fn timing() -> Result<f64, String> {
    let view = View {
        center_re: "0".into(),
        center_im: "1".into(),
        width: "1e-1000".into(),
        rotation: 0.0,
    };
    let p = params(640, 360, 20_000, None);
    let start = Instant::now();
    let (_, samples) = fd_kernel::render(&view, &p)?;
    std::hint::black_box(samples);
    Ok(start.elapsed().as_secs_f64())
}

fn run(mode: &str, bundle: &Path, report: &Path) -> Result<(), String> {
    if mode == "time-capture" || mode == "time-compare" {
        return check_timing(mode == "time-capture", report);
    }
    let zone = Zone::parse(include_str!("../../../bench/zones/v0-core.zone"))?;
    let mut actual = b"KERN05-FDS-1\0".to_vec();
    for case in cases()? {
        let data = bytes(&case, &zone)?;
        append(&mut actual, &case.name, &data);
        println!("fixture {}: {} bytes", case.name, data.len());
    }
    match mode {
        "capture" => fs::write(bundle, &actual).map_err(|e| e.to_string())?,
        "compare" | "fixtures-compare" => {
            let expected = fs::read(bundle).map_err(|e| e.to_string())?;
            if actual != expected {
                let offset = actual
                    .iter()
                    .zip(&expected)
                    .position(|(a, b)| a != b)
                    .unwrap_or(actual.len().min(expected.len()));
                return Err(format!("fixture bundle differs at byte {offset}"));
            }
            println!("all fixture bytes identical");
        }
        _ => return Err("unknown check mode".into()),
    }
    if mode == "fixtures-compare" {
        return Ok(());
    }
    check_timing(mode == "capture", report)
}

fn check_timing(capture: bool, report: &Path) -> Result<(), String> {
    let seconds = timing()?;
    if capture {
        fs::write(report, format!("{seconds:.9}\n")).map_err(|e| e.to_string())?;
        println!("baseline 640x360 seconds: {seconds:.9}");
    } else {
        let baseline: f64 = fs::read_to_string(report)
            .map_err(|e| e.to_string())?
            .trim()
            .parse()
            .map_err(|e| format!("{e}"))?;
        let ratio = seconds / baseline;
        println!(
            "candidate 640x360 seconds: {seconds:.9}; baseline: {baseline:.9}; ratio: {ratio:.6}"
        );
        if ratio > 1.02 {
            return Err(format!(
                "candidate is {:.2}% slower; limit is 2%",
                (ratio - 1.0) * 100.0
            ));
        }
    }
    Ok(())
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!("usage: kernel_contract capture|compare FIXTURE_BUNDLE BASELINE_TIMING");
        std::process::exit(2);
    }
    if let Err(e) = run(&args[1], Path::new(&args[2]), Path::new(&args[3])) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
