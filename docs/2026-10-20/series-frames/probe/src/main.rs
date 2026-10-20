//! PROB-41 probe: every frame of a path rendered cold by each arm, setup inside the clock.
//!
//!   series-probe FRAMES OUT [--size WxH] [--iter N] [--threads T] [--runs R] [--arms LIST]
//!                [--shift-px P] [--derivative-rel R]   (series guard, defaults 1e-8 and 1e-6)
//!
//! Arms (all share nothing between frames or runs):
//! - `plain`: reference orbit + `render_with` (fd control's plain arm).
//! - `bla`: reference + BLA table for the frame's largest |dc| at fd's eps (2^-50) +
//!   `render_bla`, or plain when the table is empty (fd control --bla per-frame).
//! - `series`: reference + growing-order series (8..64) + `render_series` (plain tail).
//! - `series-bla`: `series`, plus the per-frame BLA table for the tail.
//! - `series-k64`: fixed order 64 (no growth), plain tail: the growth control.
//!
//! Writes OUT/ARM/frame-NNNNN.fds (first run) for `fd compare`, and OUT/frames.csv with the
//! median cold seconds over runs (arm order rotates per run) and each arm's setup split.
use fd_kernel::{
    bla_dc_max, reference, render_bla, render_series, render_with, Bla, BlaStats, Params, Series,
    SeriesConfig, Stats,
};
use fd_samples::{write, Column, ColumnSet, Header, Kind, Samples, View};
use std::fmt::Write as _;
use std::time::Instant;

/// fd's per-frame BLA eps (fd-cli orbit::EPS).
const EPS: f64 = 1.0 / 1125899906842624.0;

struct Run {
    seconds: f64,
    reference: f64,
    series: f64,
    bla: f64,
    render: f64,
    built: Option<Series>,
    bla_used: &'static str,
    stats: Stats,
    blas: BlaStats,
    out: (Header, Samples),
}

fn arm(name: &str, view: &View, p: &Params, cfg: SeriesConfig) -> Result<Run, String> {
    let t = Instant::now();
    let orbit = reference(view, p)?;
    let reference_s = t.elapsed().as_secs_f64();
    let dc = bla_dc_max(view, p)?;
    let pixel = dc / (f64::from(p.nx) / 2.0).hypot(f64::from(p.ny) / 2.0) * f64::from(p.ss);
    let s = Instant::now();
    let series = match name {
        "series" | "series-bla" => Some(Series::build(&orbit.0, dc, pixel, p.max_iter, cfg)),
        "series-k64" => Some(Series::build(
            &orbit.0,
            dc,
            pixel,
            p.max_iter,
            SeriesConfig {
                start_order: 64,
                ..cfg
            },
        )),
        "plain" | "bla" => None,
        a => return Err(format!("unknown arm {a}")),
    };
    let series_s = s.elapsed().as_secs_f64();
    let b = Instant::now();
    let table = match name {
        "bla" | "series-bla" => Some(Bla::build(&orbit.0, EPS, dc)?),
        _ => None,
    };
    let bla_s = b.elapsed().as_secs_f64();
    let table = table.filter(|t| !t.levels.is_empty());
    let bla_used = match (name, &table) {
        ("bla" | "series-bla", Some(_)) => "used",
        ("bla" | "series-bla", None) => "skipped_empty",
        _ => "none",
    };
    let r = Instant::now();
    let (h, smp, stats, blas) = match (&series, &table) {
        (Some(s), t) => render_series(view, p, orbit, s, t.as_ref())?,
        (None, Some(t)) => render_bla(view, p, orbit, t)?,
        (None, None) => {
            let (h, s, st) = render_with(view, p, Some(orbit))?;
            (h, s, st, BlaStats::default())
        }
    };
    let render_s = r.elapsed().as_secs_f64();
    Ok(Run {
        seconds: t.elapsed().as_secs_f64(),
        reference: reference_s,
        series: series_s,
        bla: bla_s,
        render: render_s,
        built: series,
        bla_used,
        stats,
        blas,
        out: (h, smp),
    })
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |name: &str, default: &str| -> String {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1).cloned())
            .unwrap_or_else(|| default.to_string())
    };
    let [frames, out] = [args.first(), args.get(1)].map(|a| a.cloned());
    let (Some(frames), Some(out)) = (frames, out) else {
        return Err("usage: series-probe FRAMES OUT [--size WxH] [--iter N] [--threads T] [--runs R] [--arms LIST] [--shift-px P] [--derivative-rel R]".into());
    };
    let size = flag("--size", "480x270");
    let (nx, ny) = size.split_once('x').ok_or("--size WxH")?;
    let num = |s: String| s.parse::<u64>().map_err(|e| e.to_string());
    let p = Params {
        nx: nx.parse().map_err(|_| "--size")?,
        ny: ny.parse().map_err(|_| "--size")?,
        ss: 1,
        max_iter: num(flag("--iter", "100000"))?,
        escape_radius: 1e10,
        columns: ColumnSet::of(&[Column::Nu, Column::De]),
        threads: num(flag("--threads", "4"))? as usize,
        tier: None,
    };
    let runs = num(flag("--runs", "3"))?.max(1) as usize;
    let real = |s: String| s.parse::<f64>().map_err(|e| e.to_string());
    let cfg = SeriesConfig {
        shift_px: real(flag("--shift-px", "1e-8"))?,
        derivative_rel: real(flag("--derivative-rel", "1e-6"))?,
        ..SeriesConfig::default()
    };
    let arms: Vec<String> = flag("--arms", "plain,bla,series,series-bla,series-k64")
        .split(',')
        .map(str::to_string)
        .collect();
    let text = std::fs::read_to_string(&frames).map_err(|e| format!("{frames}: {e}"))?;
    let views: Vec<View> = text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            View {
                center_re: f[0].into(),
                center_im: f[1].into(),
                width: f[2].into(),
                rotation: 0.0,
            }
        })
        .collect();
    for a in &arms {
        std::fs::create_dir_all(format!("{out}/{a}")).map_err(|e| e.to_string())?;
    }
    let mut csv = String::from(
        "frame,width,arm,cold_s,cold_min_s,runs,reference_s,series_s,bla_s,render_s,series_n,series_order,series_built_steps,series_stop,series_pairs,series_growth,series_shift_px,series_state_bound,series_derivative_bound,bla_use,bla_blocks,bla_skipped,iterations,escaped,interior,unresolved\n",
    );
    let started = Instant::now();
    for (f, view) in views.iter().enumerate() {
        let mut times: Vec<Vec<f64>> = vec![Vec::new(); arms.len()];
        let mut first: Vec<Option<Run>> = arms.iter().map(|_| None).collect();
        for run in 0..runs {
            for k in 0..arms.len() {
                let i = (run + k) % arms.len();
                let r = arm(&arms[i], view, &p, cfg)?;
                times[i].push(r.seconds);
                if first[i].is_none() {
                    let mut bytes = Vec::new();
                    write(&mut bytes, &r.out.0, &r.out.1).map_err(|e| e.to_string())?;
                    std::fs::write(format!("{out}/{}/frame-{f:05}.fds", arms[i]), bytes)
                        .map_err(|e| e.to_string())?;
                    first[i] = Some(r);
                }
            }
        }
        for (i, name) in arms.iter().enumerate() {
            let r = first[i].as_ref().expect("runs >= 1");
            let t = &mut times[i];
            t.sort_by(f64::total_cmp);
            let median = t[t.len() / 2];
            let count = |k: Kind| r.out.1.class.iter().filter(|c| c.kind() == Some(k)).count();
            let s = r.built.as_ref();
            let _ = writeln!(
                csv,
                "{f},{},{name},{median:.6},{:.6},{},{:.6},{:.6},{:.6},{:.6},{},{},{},{},{},{},{:e},{:e},{:e},{},{},{},{},{},{},{}",
                view.width,
                t[0],
                t.len(),
                r.reference,
                r.series,
                r.bla,
                r.render,
                s.map_or(0, |s| s.n),
                s.map_or(0, |s| s.b.len()),
                s.map_or(0, |s| s.built_steps),
                s.map_or("", |s| s.stop),
                s.map_or(0, |s| s.coefficient_pairs),
                s.map_or(String::new(), |s| s.growth.iter().map(u64::to_string).collect::<Vec<_>>().join(" ")),
                s.map_or(0.0, |s| s.shift_px),
                s.map_or(0.0, |s| s.state_bound),
                s.map_or(0.0, |s| s.derivative_bound),
                r.bla_used,
                r.blas.blocks,
                r.blas.skipped,
                r.stats.iterations,
                count(Kind::Escaped),
                count(Kind::Interior),
                count(Kind::Unresolved),
            );
            eprintln!(
                "frame {f:2} width {:>14} {name:>10}: {median:8.3} s (series n {:>6}, render {:.3} s)",
                view.width,
                s.map_or(0, |s| s.n),
                r.render
            );
        }
    }
    std::fs::write(format!("{out}/frames.csv"), csv).map_err(|e| e.to_string())?;
    eprintln!(
        "{} frames x {} arms x {runs} runs in {:.1} s",
        views.len(),
        arms.len(),
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
