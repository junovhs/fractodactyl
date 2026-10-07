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
use fd_kernel::{render_stats, Plane, Stats};
use fd_samples::{write, Kind, Samples};
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

    let mut times = Vec::with_capacity(runs);
    let mut first: Option<(Samples, Stats, Vec<u8>, String)> = None;
    let mut deterministic = true;
    for _ in 0..runs {
        let t = Instant::now();
        let (h, s, st) = render_stats(&view, &p)?;
        times.push((t.elapsed().as_secs_f64(), st.reference_seconds));
        let mut bytes = Vec::new();
        write(&mut bytes, &h, &s).map_err(|e| e.to_string())?;
        match &first {
            None => first = Some((s, st, bytes, h.kernel)),
            Some(f) => deterministic &= f.2 == bytes && f.1.iterations == st.iterations,
        }
    }
    let (s, st, bytes, kernel) = first.expect("runs >= 1");
    if let Some(out) = out {
        std::fs::write(out, &bytes).map_err(|e| format!("{out}: {e}"))?;
    }

    let n = s.class.len() as f64;
    let pixels = n / f64::from(p.ss * p.ss);
    let count = |k: Kind| s.class.iter().filter(|c| c.kind() == Some(k)).count();
    let (escaped, interior, unresolved) = (count(Kind::Escaped), count(Kind::Interior), count(Kind::Unresolved));
    let sample_bytes = s.class.len()
        + s.nu.as_ref().map_or(0, |v| v.len() * 8)
        + s.de.as_ref().map_or(0, |v| v.len() * 4)
        + s.normal.as_ref().map_or(0, |v| v.len() * 2);
    let plane = Plane::new(&view, p.nx, p.ny)?;
    let log2_h = plane.h_m.log2() + plane.h_e as f64;
    let warm: Vec<f64> = times.iter().skip(1).map(|t| t.0).collect();

    let mut j = String::from("{\"schema\":\"fd-bench/1\"");
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
        q(&kernel),
        p.threads
    );
    let _ = write!(
        j,
        ",\"depth\":{{\"log2_sample_spacing\":{log2_h},\"log10_width\":{}}}",
        (log2_h + f64::from(p.nx).log2()) * std::f64::consts::LOG10_2
    );
    j.push_str(",\"cache\":{\"atlas\":\"none\",\"cross_frame_reuse\":false},\"runs\":[");
    for (i, (secs, rsecs)) in times.iter().enumerate() {
        let state = if i == 0 { "cold" } else { "warm" };
        let sep = if i == 0 { "" } else { "," };
        let _ = write!(j, "{sep}{{\"run\":{},\"state\":\"{state}\",\"seconds\":{secs},\"reference_seconds\":{rsecs}}}", i + 1);
    }
    let _ = write!(
        j,
        "],\"timing\":{{\"cold_seconds\":{},\"warm_seconds\":{},\"warm_statistic\":\"median\"}}",
        times[0].0,
        median(warm).map_or("null".into(), |m| m.to_string())
    );
    let _ = write!(
        j,
        ",\"memory\":{{\"peak_rss_bytes\":{},\"sample_bytes\":{sample_bytes},\"reference_bytes\":{}}}",
        peak_rss().map_or("null".into(), |b| b.to_string()),
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
    let _ = write!(j, ",\"bytes\":{{\"fds_bytes\":{},\"atlas_bytes_read\":0}}", bytes.len());
    let _ = write!(
        j,
        ",\"fallback\":{{\"pixel_fraction\":1,\"iterations_per_pixel\":{},\"unresolved_fraction\":{}}}",
        st.iterations as f64 / pixels,
        unresolved as f64 / n
    );
    let _ = write!(
        j,
        ",\"classes\":{{\"escaped\":{escaped},\"interior\":{interior},\"unresolved\":{unresolved}}},\"deterministic\":{deterministic}"
    );

    let mut ok = deterministic;
    match (oracle, out) {
        (Some(script), Some(out)) => {
            let python = a.str("python").unwrap_or("python3");
            let k = a.str("k").unwrap_or("8");
            let t = Instant::now();
            let o = Command::new(python)
                .args([script, out, "--k", k])
                .output()
                .map_err(|e| format!("{python} {script}: {e}"))?;
            let report = String::from_utf8_lossy(&o.stdout);
            let report = report.trim();
            if !(report.starts_with('{') && report.ends_with('}')) {
                return Err(format!("oracle printed no JSON report: {}", String::from_utf8_lossy(&o.stderr)));
            }
            ok &= o.status.success();
            let _ = write!(j, ",\"oracle\":{report},\"oracle_seconds\":{}", t.elapsed().as_secs_f64());
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

/// JSON string literal.
fn q(s: &str) -> String {
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

/// Peak resident set size of this process (Linux `VmHWM`); `None` elsewhere.
fn peak_rss() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|l| l.starts_with("VmHWM:"))?;
    let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kib * 1024)
}
