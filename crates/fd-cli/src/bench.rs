//! `fd bench`: render one view `--runs` times in this process and print one JSON report
//! (schema `fd-bench/1`) of the SPEC.md metrics that apply to the baseline renderer:
//! timing (cold vs warm stated per run), memory, iterations, bytes, fallback and,
//! with `--oracle`, error vs the independent oracle (tools/oracle.py, DEC-09).
//!
//! Cache state: the baseline has no atlas and reuses nothing across frames, so the
//! report says `"atlas":"none"`. Run 1 is `cold` (first render in a fresh process);
//! later runs are `warm` (same process, allocator and CPU caches warm) but still
//! recompute the reference orbit and every sample. Every sample is computed by the
//! per-sample perturbation path, which is the fallback an atlas would replace, so the
//! fallback pixel fraction is 1 (DEC-10: the contract is stated, not implied).
use crate::args::Args;
use crate::render::{job, FLAGS};
use fd_kernel::{reference_bits, render_stats, BlaStats, Params, Plane, Stats};
use fd_samples::{write, Kind, Samples, View};
use std::fmt::Write as _;
use std::process::Command;
use std::time::Instant;

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let known: Vec<&str> = FLAGS.iter().copied().chain(["runs", "oracle", "k", "python"]).collect();
    let a = Args::parse(argv, &known)?;
    let (view, p) = job(&a)?;
    let runs: usize = a.num("runs", 3)?;
    if runs == 0 {
        return Err("--runs must be at least 1".into());
    }
    let out = a.str("o");
    let oracle = a.str("oracle");
    if oracle.is_some() && out.is_none() {
        return Err("--oracle needs -o (the oracle reads the written .fds file)".into());
    }

    let frame = measure(&view, &p, runs)?;
    if let Some(out) = out {
        std::fs::write(out, &frame.bytes).map_err(|e| format!("{out}: {e}"))?;
    }
    let warm: Vec<f64> = frame.times.iter().skip(1).map(|t| t.0).collect();
    let mut j = String::from("{\"schema\":\"fd-bench/1\"");
    frame.body(&mut j, "warm", median(warm))?;
    let deterministic = frame.deterministic;
    let _ = write!(j, ",\"deterministic\":{deterministic}");

    let mut ok = deterministic;
    match (oracle, out) {
        (Some(script), Some(out)) => {
            let (report, passed, secs) = run_oracle(&a, script, out)?;
            ok &= passed;
            let _ = write!(j, ",\"oracle\":{report},\"oracle_seconds\":{secs}");
        }
        _ => j.push_str(",\"oracle\":null"),
    }
    let _ = write!(j, ",\"ok\":{ok}}}");
    println!("{j}");
    if !ok {
        std::process::exit(1);
    }
    Ok(())
}

/// One view rendered `runs` times and what it cost: the source of the fd-bench/1 metric
/// fields that `fd bench` and `fd control` (fd-control/1) share. Only run 1's encoded
/// bytes and a summary of its samples are kept; later runs are compared against them.
pub(crate) struct Frame<'a> {
    pub(crate) view: &'a View,
    pub(crate) p: &'a Params,
    pub(crate) kernel: String,
    pub(crate) stats: Stats,
    /// Run 1's `.fds` encoding.
    pub(crate) bytes: Vec<u8>,
    /// (seconds, reference seconds) per run.
    pub(crate) times: Vec<(f64, f64)>,
    /// Every later run encoded to the same bytes with the same iteration count.
    pub(crate) deterministic: bool,
    pub(crate) samples: usize,
    pub(crate) sample_bytes: usize,
    /// Samples of each kind: escaped, interior, unresolved.
    pub(crate) classes: [usize; 3],
    /// Run 1's peak RSS and whether it covers run 1 alone (`run`) or the process so far.
    pub(crate) peak_rss: Option<u64>,
    pub(crate) peak_rss_scope: &'static str,
    /// Set when the frame was rendered from an atlas (`fd play`, fd-play/1): what it
    /// read and reused. `None` for the baseline (`fd bench`, `fd control`).
    pub(crate) atlas: Option<AtlasWork>,
}

/// What an atlas-played frame read and how much of its work BLA replaced (fd-play/1).
pub(crate) struct AtlasWork {
    /// `cold` when the frame read its orbit or BLA table from the store (first touch),
    /// `warm` when every math chunk was already loaded by an earlier frame.
    pub(crate) state: &'static str,
    /// Seconds reading, verifying and decoding chunks for this frame (cache misses).
    pub(crate) load_seconds: f64,
    /// Chunk bytes read from the store for this frame (misses only).
    pub(crate) bytes_read: u64,
    /// Chunk bytes this frame's manifests reach, whether read now or cached.
    pub(crate) bytes_referenced: u64,
    /// Tile manifests the frame names.
    pub(crate) tiles_touched: usize,
    /// BLA work when a table was applied; `None` when the frame rendered without one.
    pub(crate) bla: Option<BlaStats>,
}

/// Render `view` `runs` (>= 1) times. Before each run the peak-RSS high-water mark is
/// reset and the previous run's samples are already dropped, so run 1's peak is one
/// render's (plus whatever the process already holds).
pub(crate) fn measure<'a>(view: &'a View, p: &'a Params, runs: usize) -> Result<Frame<'a>, String> {
    let mut frame: Option<Frame> = None;
    let mut times = Vec::with_capacity(runs);
    for _ in 0..runs {
        let reset = reset_peak_rss();
        let t = Instant::now();
        let (h, s, st) = render_stats(view, p)?;
        times.push((t.elapsed().as_secs_f64(), st.reference_seconds));
        let mut bytes = Vec::new();
        write(&mut bytes, &h, &s).map_err(|e| e.to_string())?;
        match &mut frame {
            None => {
                let count = |k: Kind| s.class.iter().filter(|c| c.kind() == Some(k)).count();
                frame = Some(Frame {
                    view,
                    p,
                    kernel: h.kernel,
                    stats: st,
                    bytes,
                    times: Vec::new(),
                    deterministic: true,
                    samples: s.class.len(),
                    sample_bytes: sample_bytes(&s),
                    classes: [count(Kind::Escaped), count(Kind::Interior), count(Kind::Unresolved)],
                    peak_rss: peak_rss(),
                    peak_rss_scope: if reset { "run" } else { "process" },
                    atlas: None,
                });
            }
            Some(f) => f.deterministic &= f.bytes == bytes && f.stats.iterations == st.iterations,
        }
    }
    let mut frame = frame.expect("runs >= 1");
    frame.times = times;
    Ok(frame)
}

/// Bytes of the computed sample columns.
pub(crate) fn sample_bytes(s: &Samples) -> usize {
    s.class.len()
        + s.nu.as_ref().map_or(0, |v| v.len() * 8)
        + s.de.as_ref().map_or(0, |v| v.len() * 4)
        + s.normal.as_ref().map_or(0, |v| v.len() * 2)
        + s.bound.as_ref().map_or(0, |v| v.len() * 4)
}

impl Frame<'_> {
    /// Output pixels.
    pub(crate) fn pixels(&self) -> f64 {
        self.samples as f64 / f64::from(self.p.ss * self.p.ss)
    }

    /// `log2` of the sample spacing.
    pub(crate) fn log2_spacing(&self) -> Result<f64, String> {
        let plane = Plane::new(self.view, self.p.nx, self.p.ny)?;
        Ok(plane.h_m.log2() + plane.h_e as f64)
    }

    /// Append the shared fields, from `"view"` through `"classes"`. Run 1 is `cold`,
    /// later runs are labelled `later`, and `warm` is the median reported for them
    /// (`warm_seconds` and `warm_statistic` are null when there is none).
    pub(crate) fn body(&self, j: &mut String, later: &str, warm: Option<f64>) -> Result<(), String> {
        let (view, p, st, times) = (self.view, self.p, &self.stats, &self.times);
        let n = self.samples as f64;
        let pixels = self.pixels();
        let [escaped, interior, unresolved] = self.classes;
        let log2_h = self.log2_spacing()?;
        let bits = reference_bits(view, p)?;
        let _ = write!(
            j,
            ",\"view\":{{\"re\":{},\"im\":{},\"width\":{},\"rotation\":{}}}",
            q(&view.center_re),
            q(&view.center_im),
            q(&view.width),
            view.rotation
        );
        let _ = write!(
            j,
            ",\"grid\":{{\"nx\":{},\"ny\":{},\"ss\":{},\"pixels\":{pixels},\"max_iter\":{}}},\"kernel\":{},\"threads\":{}",
            p.nx,
            p.ny,
            p.ss,
            p.max_iter,
            q(&self.kernel),
            p.threads
        );
        let _ = write!(
            j,
            ",\"depth\":{{\"log2_sample_spacing\":{log2_h},\"log10_width\":{},\"precision_bits\":{bits}}}",
            (log2_h + f64::from(p.nx).log2()) * std::f64::consts::LOG10_2
        );
        let a = self.atlas.as_ref();
        j.push_str(match a {
            None => ",\"cache\":{\"atlas\":\"none\",\"cross_frame_reuse\":false},\"runs\":[",
            Some(_) => ",\"cache\":{\"atlas\":\"store\",\"cross_frame_reuse\":true},\"runs\":[",
        });
        for (i, (secs, rsecs)) in times.iter().enumerate() {
            let state = match (i, a) {
                (0, Some(a)) => a.state,
                (0, None) => "cold",
                _ => later,
            };
            let sep = if i == 0 { "" } else { "," };
            let load = a.map_or(String::new(), |a| format!(",\"load_seconds\":{}", a.load_seconds));
            let _ = write!(j, "{sep}{{\"run\":{},\"state\":\"{state}\",\"seconds\":{secs},\"reference_seconds\":{rsecs}{load}}}", i + 1);
        }
        // An atlas frame is one run, either cold or warm: the other is null.
        let (cold, warm, stat) = match a {
            None => (Some(times[0].0), warm, warm.map(|_| "median")),
            Some(a) if a.state == "warm" => (None, Some(times[0].0), Some("frame")),
            Some(_) => (Some(times[0].0), None, None),
        };
        let _ = write!(
            j,
            "],\"timing\":{{\"cold_seconds\":{},\"warm_seconds\":{},\"warm_statistic\":{}}}",
            cold.map_or("null".into(), |m| m.to_string()),
            warm.map_or("null".into(), |m| m.to_string()),
            stat.map_or("null".into(), |s| format!("\"{s}\""))
        );
        let _ = write!(
            j,
            ",\"memory\":{{\"peak_rss_bytes\":{},\"peak_rss_scope\":\"{}\",\"sample_bytes\":{},\"reference_bytes\":{},\"device\":\"cpu\",\"peak_vram_bytes\":0}}",
            self.peak_rss.map_or("null".into(), |b| b.to_string()),
            self.peak_rss_scope,
            self.sample_bytes,
            st.reference_len * 16
        );
        let _ = write!(
            j,
            ",\"iterations\":{{\"total\":{},\"per_pixel\":{},\"per_sample\":{},\"reference_length\":{}}}",
            st.iterations,
            st.iterations as f64 / pixels,
            st.iterations as f64 / n,
            st.reference_len
        );
        let fds = self.bytes.len();
        match a {
            // No atlas: nothing is looked up, so these are zero by construction, not unmeasured.
            None => {
                let _ = write!(j, ",\"bytes\":{{\"fds_bytes\":{fds},\"atlas_bytes_read\":0}}");
                j.push_str(",\"atlas_work\":{\"tiles_touched\":0,\"microblocks_touched\":0,\"macro_operators_per_pixel\":0}");
                let _ = write!(
                    j,
                    ",\"fallback\":{{\"pixel_fraction\":1,\"iterations_per_pixel\":{},\"unresolved_fraction\":{}}}",
                    st.iterations as f64 / pixels,
                    unresolved as f64 / n
                );
            }
            Some(a) => {
                let _ = write!(
                    j,
                    ",\"bytes\":{{\"fds_bytes\":{fds},\"atlas_bytes_read\":{},\"atlas_bytes_referenced\":{}}}",
                    a.bytes_read, a.bytes_referenced
                );
                // Microblocks do not exist in the v0 atlas: zero by construction.
                let blocks = a.bla.map_or(0, |b| b.blocks);
                let _ = write!(
                    j,
                    ",\"atlas_work\":{{\"tiles_touched\":{},\"microblocks_touched\":0,\"macro_operators_per_pixel\":{}}}",
                    a.tiles_touched,
                    blocks as f64 / pixels
                );
                // Fallback pixels: samples that applied no block (without a table, all).
                let fb = a.bla.map_or(n, |b| (b.fallback_samples + b.closed_form_samples) as f64);
                let _ = write!(
                    j,
                    ",\"fallback\":{{\"pixel_fraction\":{},\"iterations_per_pixel\":{},\"unresolved_fraction\":{}}}",
                    fb / n,
                    (st.iterations - blocks) as f64 / pixels,
                    unresolved as f64 / n
                );
            }
        }
        let _ = write!(j, ",\"classes\":{{\"escaped\":{escaped},\"interior\":{interior},\"unresolved\":{unresolved}}}");
        Ok(())
    }
}

/// Run the oracle script on `out` (`--k`, `--python` from `a`): its JSON report, whether
/// it passed, and its wall time.
pub(crate) fn run_oracle(a: &Args, script: &str, out: &str) -> Result<(String, bool, f64), String> {
    let python = a.str("python").unwrap_or("python3");
    let k = a.str("k").unwrap_or("8");
    let t = Instant::now();
    let o = Command::new(python).args([script, out, "--k", k]).output().map_err(|e| format!("{python} {script}: {e}"))?;
    let report = String::from_utf8_lossy(&o.stdout).trim().to_string();
    if !(report.starts_with('{') && report.ends_with('}')) {
        return Err(format!("oracle printed no JSON report: {}", String::from_utf8_lossy(&o.stderr)));
    }
    Ok((report, o.status.success(), t.elapsed().as_secs_f64()))
}

/// JSON string literal.
pub(crate) fn q(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 => {
                let _ = write!(o, "\\u{:04x}", c as u32);
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn median(mut v: Vec<f64>) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    let m = v.len() / 2;
    Some(if v.len().is_multiple_of(2) { (v[m - 1] + v[m]) / 2.0 } else { v[m] })
}

/// Reset this process's peak RSS (`VmHWM`) to its current RSS, so the next reading
/// covers what follows. Linux `/proc/self/clear_refs` value 5; false where unavailable.
pub(crate) fn reset_peak_rss() -> bool {
    std::fs::write("/proc/self/clear_refs", "5").is_ok()
}

/// Peak resident set size of this process (Linux `VmHWM`); `None` elsewhere.
pub(crate) fn peak_rss() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|l| l.starts_with("VmHWM:"))?;
    let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kib * 1024)
}
