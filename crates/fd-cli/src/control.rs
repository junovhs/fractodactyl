//! `fd control`: the independent-frame control renderer (VIDE-03, docs/spec/SAMPLES.md
//! "Independent-frame control"). Renders every frame of a camera path (PLAN.md format)
//! on its own, exactly as `fd render` would: its own reference orbit, computed fresh,
//! the best kernel `--kernel` allows, no stored orbit, BLA table or atlas, and nothing
//! carried from one frame to the next. It prints one fd-control/1 JSON line per frame
//! with the fd-bench/1 metric fields, then one totals line: the baseline the atlas
//! player is compared against (BENC-01).
//!
//! `--bla per-frame` is the stronger independent control (BENC-01 arm B): each frame
//! computes its own reference orbit and builds its own BLA table over it for its own
//! largest `|dc|` (the `fd compile --bla frame` contract: `eps` = `orbit::EPS`, every
//! level up to the first with no valid block), inside the frame's clock, then renders
//! with it; a table with no valid block is skipped as `fd play` skips one. Nothing is
//! kept for the next frame.
//!
//! `--zone FILE` (KERN-01): frames the zone covers (every sample within its `max_dc` of
//! the nucleus and, for a zone with `diag` lines, a bounded truncation shift: PROB-20),
//! decided before rendering (DEC-19), render with the minibrot-band fast path (`fd_kernel::render_zone`),
//! inside the frame's clock; every other frame renders as without it (with `--bla
//! per-frame` if given). Each frame's record says which (`"zone"`), the totals count
//! the zone frames and their seconds.
use crate::args::Args;
use crate::bench::{measure, peak_rss, q, reset_peak_rss, run_oracle, sample_bytes, Frame};
use crate::orbit::EPS;
use crate::plan::read_path;
use crate::render::{params, FLAGS};
use fd_kernel::{
    bla_dc_max, reference_bits, render_bla, render_with, render_zone, zone_covers, Bla, BlaStats,
    Params, Zone, ZoneStats,
};
use fd_samples::{write, Kind, View};
use std::fmt::Write as _;
use std::time::Instant;

const USAGE: &str = "usage: fd control PATH [render flags without --re/--im/--width/--rotation] [--bla none|per-frame] [--zone FILE] [--runs N] [-o DIR] [--oracle tools/oracle.py [--every N] [--k K] [--python P]]";

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let view_flags = ["re", "im", "width", "rotation"];
    let known: Vec<&str> = FLAGS
        .iter()
        .copied()
        .filter(|f| !view_flags.contains(f))
        .chain(["runs", "oracle", "every", "k", "python", "bla", "zone"])
        .collect();
    let a = Args::parse(argv, &known)?;
    let [file] = a.positional.as_slice() else {
        return Err(USAGE.into());
    };
    let p = params(&a)?;
    let runs: usize = a.num("runs", 1)?;
    if runs == 0 {
        return Err("--runs must be at least 1".into());
    }
    let dir = a.str("o");
    let per_frame = match a.str("bla").unwrap_or("none") {
        "none" => false,
        "per-frame" => true,
        b => {
            return Err(format!(
                "--bla: expected none or per-frame (a table built inside each frame), got {b:?}"
            ))
        }
    };
    let every: usize = a.num("every", 1)?;
    if every == 0 {
        return Err("--every must be at least 1".into());
    }
    let oracle = a.str("oracle");
    if oracle.is_some() && dir.is_none() {
        return Err("--oracle needs -o DIR (the oracle reads the written .fds files)".into());
    }
    if let Some(d) = dir {
        std::fs::create_dir_all(d).map_err(|e| format!("{d}: {e}"))?;
    }
    let zone = a.str("zone").map(Zone::load).transpose()?;
    let frames = read_path(file)?;

    // From here on a failure is a runtime one (exit 1), reported with the totals so far.
    let mut t = Totals::default();
    let mut error = None;
    for (f, (line, view)) in frames.iter().enumerate() {
        let check = oracle.filter(|_| f.is_multiple_of(every));
        if let Err(e) = frame(
            &a,
            &p,
            runs,
            per_frame,
            zone.as_ref(),
            dir,
            check,
            f,
            *line,
            view,
            &mut t,
        ) {
            error = Some(format!("frame {f} (line {line}): {e}"));
            break;
        }
    }
    println!("{}", totals(file, runs, per_frame, &t, error.as_deref()));
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
fn frame(
    a: &Args,
    p: &Params,
    runs: usize,
    per_frame: bool,
    zone: Option<&Zone>,
    dir: Option<&str>,
    oracle: Option<&str>,
    f: usize,
    line: usize,
    view: &View,
    t: &mut Totals,
) -> Result<(), String> {
    let covered = match zone {
        Some(z) => zone_covers(view, p, z)?,
        None => false,
    };
    let (frame, own, zst) = match zone {
        Some(z) if covered => {
            let (fr, st) = measure_zone(view, p, runs, z)?;
            (fr, None, Some(st))
        }
        _ if per_frame => {
            let (fr, own) = measure_per_frame_bla(view, p, runs)?;
            (fr, Some(own), None)
        }
        _ => (measure(view, p, runs)?, None, None),
    };
    let out = dir.map(|d| format!("{}/frame-{f:05}.fds", d.trim_end_matches('/')));
    if let Some(out) = &out {
        std::fs::write(out, &frame.bytes).map_err(|e| format!("{out}: {e}"))?;
    }
    let mut j =
        format!("{{\"schema\":\"fd-control/1\",\"record\":\"frame\",\"frame\":{f},\"line\":{line}");
    // Every run recomputes everything; later runs only check determinism, so they are
    // `repeat`, not `warm`: a control frame has no warm state.
    frame.body(&mut j, "repeat", None)?;
    let checked = runs > 1;
    let det = if checked {
        frame.deterministic.to_string()
    } else {
        "null".into()
    };
    let _ = write!(
        j,
        ",\"deterministic\":{det},\"fds\":{}",
        out.as_deref().map_or("null".into(), q)
    );
    if let Some(o) = &own {
        o.record(&mut j, frame.stats.iterations);
    }
    if zone.is_some() {
        let _ = write!(j, ",\"zone\":{{\"used\":{covered}");
        if let Some(z) = zst {
            let _ = write!(
                j,
                ",\"returns\":{},\"jumps\":{},\"patches\":{},\"plain_steps\":{}",
                z.returns, z.jumps, z.patches, z.plain
            );
        }
        j.push('}');
    }
    let mut ok = !checked || frame.deterministic;
    match (oracle, &out) {
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
    if covered {
        t.zone_frames += 1;
        t.zone_seconds += frame.times[0].0;
    }
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
    t.log10_width = (
        t.log10_width.0.min(log10_width),
        t.log10_width.1.max(log10_width),
    );
    t.bits_max = t.bits_max.max(bits);
    if checked {
        t.deterministic &= frame.deterministic;
    }
    if let Some(o) = own {
        t.bla_use[o.table_use as usize] += 1;
        t.operator += o.operator_seconds;
        t.render += o.render_seconds;
        t.table_bytes += o.table_bytes;
        t.table_bytes_max = t.table_bytes_max.max(o.table_bytes);
        let b = o.stats.unwrap_or_default();
        t.blocks += b.blocks;
        t.skipped += b.skipped;
        t.fallback_samples += o.stats.map_or(frame.samples as u64, |b| {
            b.fallback_samples + b.closed_form_samples
        });
        if let Some(px) = b.shift_px {
            t.shift_px_max = t.shift_px_max.max(px);
        }
    }
    Ok(())
}

/// How a `--bla per-frame` frame used the table it built (as `fd play`'s `bla_use`).
#[derive(Clone, Copy)]
enum TableUse {
    /// Scaled tier: BLA is unavailable, no table built.
    None,
    /// The table holds no valid block: skipped, the frame renders with its orbit alone.
    Empty,
    Used,
}

impl TableUse {
    fn name(self) -> &'static str {
        match self {
            TableUse::None => "none",
            TableUse::Empty => "skipped_empty",
            TableUse::Used => "used",
        }
    }
}

/// Run 1's per-frame BLA work: its own reference, its own table, the render.
struct OwnBla {
    table_use: TableUse,
    dc_max: Option<f64>,
    levels: usize,
    valid_blocks: usize,
    /// In-memory table size: 7 f64 per stored block (the `bla` chunk payload).
    table_bytes: u64,
    reference_seconds: f64,
    operator_seconds: f64,
    render_seconds: f64,
    stats: Option<BlaStats>,
}

impl OwnBla {
    fn record(&self, j: &mut String, iterations: u64) {
        let b = self.stats.unwrap_or_default();
        let _ = write!(
            j,
            ",\"bla\":{{\"mode\":\"per-frame\",\"use\":\"{}\",\"eps\":{EPS:e},\"dc_max\":{},\"levels\":{},\"valid_blocks\":{},\"table_bytes\":{},\
             \"reference_seconds\":{},\"operator_seconds\":{},\"render_seconds\":{},\"blocks\":{},\"skipped_steps\":{},\"iterations_equivalent\":{},\
             \"fallback_samples\":{},\"closed_form_samples\":{},\"shift_px_max\":{}}}",
            self.table_use.name(),
            self.dc_max.map_or("null".into(), |d| format!("{d:e}")),
            self.levels,
            self.valid_blocks,
            self.table_bytes,
            self.reference_seconds,
            self.operator_seconds,
            self.render_seconds,
            b.blocks,
            b.skipped,
            iterations - b.blocks + b.skipped,
            self.stats.map_or("null".into(), |b| b.fallback_samples.to_string()),
            self.stats.map_or("null".into(), |b| b.closed_form_samples.to_string()),
            b.shift_px.map_or("null".into(), |x| x.to_string())
        );
    }
}

/// [`measure`] for `--bla per-frame`: each run computes the frame's reference orbit,
/// builds a BLA table over it for the frame's largest `|dc|`, and renders with it (or
/// without it when it has no valid block, or on the scaled tier), all inside the run's
/// clock. Runs share nothing.
fn measure_per_frame_bla<'a>(
    view: &'a View,
    p: &'a Params,
    runs: usize,
) -> Result<(Frame<'a>, OwnBla), String> {
    let mut first: Option<(Frame, OwnBla)> = None;
    let mut times = Vec::with_capacity(runs);
    for _ in 0..runs {
        let reset = reset_peak_rss();
        let t = Instant::now();
        let (orbit, bits) = fd_kernel::reference(view, p)?;
        let reference_seconds = t.elapsed().as_secs_f64();
        let op = Instant::now();
        // The scaled tier has no BLA (f64 deltas only): no table, as `fd compile`.
        let table = match bla_dc_max(view, p) {
            Ok(dc) => Some((dc, Bla::build(&orbit, EPS, dc)?)),
            Err(_) => None,
        };
        let operator_seconds = op.elapsed().as_secs_f64();
        let r = Instant::now();
        let (table_use, (h, s, st, bla)) = match &table {
            Some((_, bla)) if !bla.levels.is_empty() => {
                let (h, s, st, b) = render_bla(view, p, (orbit, bits), bla)?;
                (TableUse::Used, (h, s, st, Some(b)))
            }
            other => {
                let (h, s, st) = render_with(view, p, Some((orbit, bits)))?;
                (
                    if other.is_some() {
                        TableUse::Empty
                    } else {
                        TableUse::None
                    },
                    (h, s, st, None),
                )
            }
        };
        let render_seconds = r.elapsed().as_secs_f64();
        let seconds = t.elapsed().as_secs_f64();
        times.push((seconds, reference_seconds));
        let mut bytes = Vec::new();
        write(&mut bytes, &h, &s).map_err(|e| e.to_string())?;
        match &mut first {
            None => {
                let count = |k: Kind| s.class.iter().filter(|c| c.kind() == Some(k)).count();
                let (all, valid) = table.as_ref().map_or((0, 0), |(_, b)| b.counts());
                let own = OwnBla {
                    table_use,
                    dc_max: table.as_ref().map(|(d, _)| *d),
                    levels: table.as_ref().map_or(0, |(_, b)| b.levels.len()),
                    valid_blocks: valid,
                    table_bytes: all as u64 * 56,
                    reference_seconds,
                    operator_seconds,
                    render_seconds,
                    stats: bla,
                };
                let fr = Frame {
                    view,
                    p,
                    kernel: h.kernel,
                    stats: st,
                    bytes,
                    times: Vec::new(),
                    deterministic: true,
                    samples: s.class.len(),
                    sample_bytes: sample_bytes(&s),
                    classes: [
                        count(Kind::Escaped),
                        count(Kind::Interior),
                        count(Kind::Unresolved),
                    ],
                    peak_rss: peak_rss(),
                    peak_rss_scope: if reset { "run" } else { "process" },
                    atlas: None,
                    own_bla: bla,
                };
                first = Some((fr, own));
            }
            Some((f, _)) => {
                f.deterministic &= f.bytes == bytes && f.stats.iterations == st.iterations
            }
        }
    }
    let (mut frame, own) = first.expect("runs >= 1");
    frame.times = times;
    Ok((frame, own))
}

/// [`measure`] for a frame the zone covers: each run renders it with the zone fast
/// path inside the run's clock (the zone's constants are loaded once, before the clock,
/// as `--bla ID` tables are).
fn measure_zone<'a>(
    view: &'a View,
    p: &'a Params,
    runs: usize,
    zone: &Zone,
) -> Result<(Frame<'a>, ZoneStats), String> {
    let mut first: Option<(Frame, ZoneStats)> = None;
    let mut times = Vec::with_capacity(runs);
    for _ in 0..runs {
        let reset = reset_peak_rss();
        let t = Instant::now();
        let (h, s, st, zst) = render_zone(view, p, zone)?;
        times.push((t.elapsed().as_secs_f64(), 0.0));
        let mut bytes = Vec::new();
        write(&mut bytes, &h, &s).map_err(|e| e.to_string())?;
        match &mut first {
            None => {
                let count = |k: Kind| s.class.iter().filter(|c| c.kind() == Some(k)).count();
                let fr = Frame {
                    view,
                    p,
                    kernel: h.kernel,
                    stats: st,
                    bytes,
                    times: Vec::new(),
                    deterministic: true,
                    samples: s.class.len(),
                    sample_bytes: sample_bytes(&s),
                    classes: [
                        count(Kind::Escaped),
                        count(Kind::Interior),
                        count(Kind::Unresolved),
                    ],
                    peak_rss: peak_rss(),
                    peak_rss_scope: if reset { "run" } else { "process" },
                    atlas: None,
                    own_bla: None,
                };
                first = Some((fr, zst));
            }
            Some((f, _)) => {
                f.deterministic &= f.bytes == bytes && f.stats.iterations == st.iterations
            }
        }
    }
    let (mut frame, zst) = first.expect("runs >= 1");
    frame.times = times;
    Ok((frame, zst))
}

/// The totals record; quantities over zero frames are `null`.
fn totals(file: &str, runs: usize, per_frame: bool, t: &Totals, error: Option<&str>) -> String {
    let n = t.frames;
    let num = |x: f64| {
        if n == 0 {
            "null".to_string()
        } else {
            x.to_string()
        }
    };
    let mut j = format!(
        "{{\"schema\":\"fd-control/1\",\"record\":\"totals\",\"path\":{},\"frames\":{n}",
        q(file)
    );
    j.push_str(if per_frame {
        ",\"cache\":{\"atlas\":\"none\",\"cross_frame_reuse\":false,\"bla\":\"per-frame\"}"
    } else {
        ",\"cache\":{\"atlas\":\"none\",\"cross_frame_reuse\":false}"
    });
    let _ = write!(
        j,
        ",\"timing\":{{\"cold_seconds\":{},\"cold_seconds_per_frame\":{},\"warm_seconds\":null}}",
        t.cold,
        num(t.cold / n as f64)
    );
    let _ = write!(
        j,
        ",\"reference_seconds\":{{\"total\":{},\"per_frame\":{}}}",
        t.reference,
        num(t.reference / n as f64)
    );
    let _ = write!(
        j,
        ",\"iterations\":{{\"total\":{},\"per_pixel\":{},\"per_sample\":{},\"reference_length_max\":{}}}",
        t.iterations,
        num(t.iterations as f64 / t.pixels),
        num(t.iterations as f64 / t.samples as f64),
        t.reference_len_max
    );
    let _ = write!(
        j,
        ",\"bytes\":{{\"fds_bytes\":{},\"atlas_bytes_read\":0}}",
        t.fds_bytes
    );
    let _ = write!(
        j,
        ",\"atlas_work\":{{\"tiles_touched\":0,\"microblocks_touched\":0,\"macro_operators_per_pixel\":{}}}",
        num(t.blocks as f64 / t.pixels)
    );
    let fallback = if per_frame {
        t.fallback_samples as f64 / t.samples as f64
    } else {
        1.0
    };
    let _ = write!(
        j,
        ",\"fallback\":{{\"pixel_fraction\":{},\"iterations_per_pixel\":{},\"unresolved_fraction\":{}}}",
        num(fallback),
        num((t.iterations - t.blocks) as f64 / t.pixels),
        num(t.classes[2] as f64 / t.samples as f64)
    );
    if per_frame {
        let _ = write!(
            j,
            ",\"bla\":{{\"mode\":\"per-frame\",\"eps\":{EPS:e},\"frames_used\":{},\"frames_empty_table_skipped\":{},\"frames_without_table\":{},\
             \"operator_seconds\":{{\"total\":{},\"per_frame\":{}}},\"render_seconds\":{},\"table_bytes\":{{\"total\":{},\"max\":{}}},\
             \"blocks\":{},\"skipped_steps\":{},\"iterations_equivalent\":{},\"shift_px_max\":{}}}",
            t.bla_use[TableUse::Used as usize],
            t.bla_use[TableUse::Empty as usize],
            t.bla_use[TableUse::None as usize],
            t.operator,
            num(t.operator / n as f64),
            t.render,
            t.table_bytes,
            t.table_bytes_max,
            t.blocks,
            t.skipped,
            t.iterations - t.blocks + t.skipped,
            t.shift_px_max
        );
    }
    let _ = write!(
        j,
        ",\"zone\":{{\"frames\":{},\"seconds\":{}}}",
        t.zone_frames, t.zone_seconds
    );
    let _ = write!(
        j,
        ",\"memory\":{{\"peak_rss_bytes\":{},\"sample_bytes\":{},\"device\":\"cpu\",\"peak_vram_bytes\":0}}",
        t.peak_rss.map_or("null".into(), |b| b.to_string()),
        t.sample_bytes
    );
    let _ = write!(
        j,
        ",\"classes\":{{\"escaped\":{},\"interior\":{},\"unresolved\":{}}}",
        t.classes[0], t.classes[1], t.classes[2]
    );
    let _ = write!(
        j,
        ",\"depth\":{{\"log10_width_min\":{},\"log10_width_max\":{},\"precision_bits_max\":{}}}",
        num(t.log10_width.0),
        num(t.log10_width.1),
        num(f64::from(t.bits_max))
    );
    let det = if runs > 1 && n > 0 {
        t.deterministic.to_string()
    } else {
        "null".into()
    };
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
    /// `--bla per-frame` only: frames per [`TableUse`], operator (table build) and
    /// render seconds, table bytes, BLA work, fallback samples.
    bla_use: [usize; 3],
    operator: f64,
    render: f64,
    table_bytes: u64,
    table_bytes_max: u64,
    blocks: u64,
    skipped: u64,
    fallback_samples: u64,
    shift_px_max: f64,
    /// `--zone` only: frames rendered by the zone fast path and their run-1 seconds.
    zone_frames: usize,
    zone_seconds: f64,
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
            bla_use: [0; 3],
            operator: 0.0,
            render: 0.0,
            table_bytes: 0,
            table_bytes_max: 0,
            blocks: 0,
            skipped: 0,
            fallback_samples: 0,
            shift_px_max: 0.0,
            zone_frames: 0,
            zone_seconds: 0.0,
        }
    }
}
