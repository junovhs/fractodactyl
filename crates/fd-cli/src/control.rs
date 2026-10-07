//! `fd control`: the independent-frame control renderer (VIDE-03, docs/spec/SAMPLES.md
//! "Independent-frame control"). Renders every frame of a camera path (PLAN.md format)
//! on its own, exactly as `fd render` would: its own reference orbit, computed fresh,
//! the best kernel `--kernel` allows, no stored orbit, BLA table or atlas, and nothing
//! carried from one frame to the next. It prints one fd-control/1 JSON line per frame
//! with the fd-bench/1 metric fields, then one totals line: the baseline the atlas
//! player is compared against (BENC-01).
use crate::args::Args;
use crate::bench::{measure, q, run_oracle};
use crate::plan::read_path;
use crate::render::{params, FLAGS};
use fd_kernel::{reference_bits, Params};
use fd_samples::View;
use std::fmt::Write as _;

const USAGE: &str = "usage: fd control PATH [render flags without --re/--im/--width/--rotation] [--runs N] [-o DIR] [--oracle tools/oracle.py [--k K] [--python P]]";

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let view_flags = ["re", "im", "width", "rotation"];
    let known: Vec<&str> =
        FLAGS.iter().copied().filter(|f| !view_flags.contains(f)).chain(["runs", "oracle", "k", "python"]).collect();
    let a = Args::parse(argv, &known)?;
    let [file] = a.positional.as_slice() else { return Err(USAGE.into()) };
    let p = params(&a)?;
    let runs: usize = a.num("runs", 1)?;
    if runs == 0 {
        return Err("--runs must be at least 1".into());
    }
    let dir = a.str("o");
    let oracle = a.str("oracle");
    if oracle.is_some() && dir.is_none() {
        return Err("--oracle needs -o DIR (the oracle reads the written .fds files)".into());
    }
    if let Some(d) = dir {
        std::fs::create_dir_all(d).map_err(|e| format!("{d}: {e}"))?;
    }
    let frames = read_path(file)?;

    // From here on a failure is a runtime one (exit 1), reported with the totals so far.
    let mut t = Totals::default();
    let mut error = None;
    for (f, (line, view)) in frames.iter().enumerate() {
        if let Err(e) = frame(&a, &p, runs, dir, f, *line, view, &mut t) {
            error = Some(format!("frame {f} (line {line}): {e}"));
            break;
        }
    }
    println!("{}", totals(file, runs, &t, error.as_deref()));
    if let Some(e) = &error {
        eprintln!("fd: {e}");
    }
    if error.is_some() || !t.ok {
        std::process::exit(1);
    }
    Ok(())
}

/// Render, record and print frame `f`, adding it to `t`.
#[allow(clippy::too_many_arguments)]
fn frame(a: &Args, p: &Params, runs: usize, dir: Option<&str>, f: usize, line: usize, view: &View, t: &mut Totals) -> Result<(), String> {
    let frame = measure(view, p, runs)?;
    let out = dir.map(|d| format!("{}/frame-{f:05}.fds", d.trim_end_matches('/')));
    if let Some(out) = &out {
        std::fs::write(out, &frame.bytes).map_err(|e| format!("{out}: {e}"))?;
    }
    let mut j = format!("{{\"schema\":\"fd-control/1\",\"record\":\"frame\",\"frame\":{f},\"line\":{line}");
    // Every run recomputes everything; later runs only check determinism, so they are
    // `repeat`, not `warm`: a control frame has no warm state.
    frame.body(&mut j, "repeat", None)?;
    let checked = runs > 1;
    let det = if checked { frame.deterministic.to_string() } else { "null".into() };
    let _ = write!(j, ",\"deterministic\":{det},\"fds\":{}", out.as_deref().map_or("null".into(), q));
    let mut ok = !checked || frame.deterministic;
    match (a.str("oracle"), &out) {
        (Some(script), Some(out)) => {
            let (report, passed, secs) = run_oracle(a, script, out)?;
            ok &= passed;
            t.oracle_frames += 1;
            t.oracle_failures += usize::from(!passed);
            let _ = write!(j, ",\"oracle\":{report},\"oracle_seconds\":{secs}");
        }
        _ => j.push_str(",\"oracle\":null"),
    }
    let _ = write!(j, ",\"ok\":{ok}}}");
    let log10_width = (frame.log2_spacing()? + f64::from(p.nx).log2()) * std::f64::consts::LOG10_2;
    let bits = reference_bits(view, p)?;
    println!("{j}");

    t.ok &= ok;
    t.frames += 1;
    t.cold += frame.times[0].0;
    t.reference += frame.times[0].1;
    t.iterations += frame.stats.iterations;
    t.pixels += frame.pixels();
    t.samples += frame.samples as u64;
    t.sample_bytes += frame.sample_bytes as u64;
    t.fds_bytes += frame.bytes.len() as u64;
    t.reference_len_max = t.reference_len_max.max(frame.stats.reference_len);
    t.peak_rss = match (t.peak_rss, frame.peak_rss) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    };
    for (sum, c) in t.classes.iter_mut().zip(frame.classes) {
        *sum += c as u64;
    }
    t.log10_width = (t.log10_width.0.min(log10_width), t.log10_width.1.max(log10_width));
    t.bits_max = t.bits_max.max(bits);
    if checked {
        t.deterministic &= frame.deterministic;
    }
    Ok(())
}

/// The totals record; quantities over zero frames are `null`.
fn totals(file: &str, runs: usize, t: &Totals, error: Option<&str>) -> String {
    let n = t.frames;
    let num = |x: f64| if n == 0 { "null".to_string() } else { x.to_string() };
    let mut j = format!("{{\"schema\":\"fd-control/1\",\"record\":\"totals\",\"path\":{},\"frames\":{n}", q(file));
    j.push_str(",\"cache\":{\"atlas\":\"none\",\"cross_frame_reuse\":false}");
    let _ = write!(
        j,
        ",\"timing\":{{\"cold_seconds\":{},\"cold_seconds_per_frame\":{},\"warm_seconds\":null}}",
        t.cold,
        num(t.cold / n as f64)
    );
    let _ = write!(j, ",\"reference_seconds\":{{\"total\":{},\"per_frame\":{}}}", t.reference, num(t.reference / n as f64));
    let _ = write!(
        j,
        ",\"iterations\":{{\"total\":{},\"per_pixel\":{},\"per_sample\":{},\"reference_length_max\":{}}}",
        t.iterations,
        num(t.iterations as f64 / t.pixels),
        num(t.iterations as f64 / t.samples as f64),
        t.reference_len_max
    );
    let _ = write!(j, ",\"bytes\":{{\"fds_bytes\":{},\"atlas_bytes_read\":0}}", t.fds_bytes);
    j.push_str(",\"atlas_work\":{\"tiles_touched\":0,\"microblocks_touched\":0,\"macro_operators_per_pixel\":0}");
    let _ = write!(
        j,
        ",\"fallback\":{{\"pixel_fraction\":1,\"iterations_per_pixel\":{},\"unresolved_fraction\":{}}}",
        num(t.iterations as f64 / t.pixels),
        num(t.classes[2] as f64 / t.samples as f64)
    );
    let _ = write!(
        j,
        ",\"memory\":{{\"peak_rss_bytes\":{},\"sample_bytes\":{},\"device\":\"cpu\",\"peak_vram_bytes\":0}}",
        t.peak_rss.map_or("null".into(), |b| b.to_string()),
        t.sample_bytes
    );
    let _ = write!(j, ",\"classes\":{{\"escaped\":{},\"interior\":{},\"unresolved\":{}}}", t.classes[0], t.classes[1], t.classes[2]);
    let _ = write!(
        j,
        ",\"depth\":{{\"log10_width_min\":{},\"log10_width_max\":{},\"precision_bits_max\":{}}}",
        num(t.log10_width.0),
        num(t.log10_width.1),
        num(f64::from(t.bits_max))
    );
    let det = if runs > 1 && n > 0 { t.deterministic.to_string() } else { "null".into() };
    let ok = t.ok && error.is_none();
    let _ = write!(
        j,
        ",\"deterministic\":{det},\"oracle_frames\":{},\"oracle_failures\":{},\"error\":{},\"ok\":{ok}}}",
        t.oracle_frames,
        t.oracle_failures,
        error.map_or("null".into(), q)
    );
    j
}

/// Sums and extremes over the path's frames (first, `cold`, run of each).
struct Totals {
    frames: usize,
    ok: bool,
    cold: f64,
    reference: f64,
    iterations: u64,
    pixels: f64,
    samples: u64,
    sample_bytes: u64,
    fds_bytes: u64,
    reference_len_max: usize,
    peak_rss: Option<u64>,
    classes: [u64; 3],
    log10_width: (f64, f64),
    bits_max: u32,
    deterministic: bool,
    oracle_frames: usize,
    oracle_failures: usize,
}

impl Default for Totals {
    fn default() -> Totals {
        Totals {
            frames: 0,
            ok: true,
            cold: 0.0,
            reference: 0.0,
            iterations: 0,
            pixels: 0.0,
            samples: 0,
            sample_bytes: 0,
            fds_bytes: 0,
            reference_len_max: 0,
            peak_rss: None,
            classes: [0; 3],
            log10_width: (f64::INFINITY, f64::NEG_INFINITY),
            bits_max: 0,
            deterministic: true,
            oracle_frames: 0,
            oracle_failures: 0,
        }
    }
}
