//! `fd render`: compute samples and write a `.fds` file. No colour happens here.
use crate::args::Args;
use fd_kernel::{render_refined, render_with, Params, Refinement, Tier, FINAL};
use fd_samples::{write, Column, ColumnSet, View};
use std::io::BufWriter;
use std::time::Instant;

/// Flags shared by `fd render` and `fd bench`.
pub(crate) const FLAGS: [&str; 11] = ["re", "im", "width", "size", "ss", "iter", "columns", "threads", "rotation", "kernel", "o"];

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let known: Vec<&str> = FLAGS.iter().copied().chain(["store", "orbit", "refine", "max-px"]).collect();
    let a = Args::parse(argv, &known)?;
    let out = a.need("o")?;
    let (view, p) = job(&a)?;
    // `--orbit ID`: a stored reference orbit for this exact centre instead of computing it.
    let orbit = match a.str("orbit") {
        Some(id) => Some(crate::orbit::load(&crate::chunk::store(&a)?, id, &view)?),
        None => None,
    };
    let threads = p.threads;
    let t = Instant::now();
    // `--refine B`: progressive refinement in B x B pixel blocks (docs/spec/LOD.md).
    let (header, samples, refined) = match a.str("refine") {
        None => render_with(&view, &p, orbit).map(|(h, s, _)| (h, s, None))?,
        Some(_) if orbit.is_some() => return Err("--refine does not take --orbit".into()),
        Some(_) => {
            let max_px = a.num("max-px", fd_samples::lod::FINAL_PX)?;
            let (h, s, st, r) = render_refined(&view, &p, a.num("refine", 0)?, max_px)?;
            (h, s, Some((st.iterations, max_px, r)))
        }
    };
    let secs = t.elapsed().as_secs_f64();
    let file = std::fs::File::create(out).map_err(|e| format!("{out}: {e}"))?;
    write(BufWriter::new(file), &header, &samples).map_err(|e| format!("{out}: {e}"))?;
    let bytes = std::fs::metadata(out).map(|m| m.len()).unwrap_or(0);
    println!("{out}: {}x{} samples, {}, {threads} threads, {secs:.3} s, {bytes} bytes", p.nx, p.ny, header.kernel);
    if let Some((iterations, max_px, r)) = refined {
        report(&r, max_px, iterations, samples.class.len() as u64);
    }
    Ok(())
}

/// Work per phase and each block's phase mask (final or fallback).
fn report(r: &Refinement, max_px: f64, iterations: u64, full: u64) {
    let finals = r.mask.iter().filter(|&&m| m & FINAL != 0).count();
    println!("refine block_px {} max_px {max_px} blocks {}", r.block_px, r.mask.len());
    for (name, ph) in [("preview", r.preview), ("sparse", r.sparse), ("dense", r.dense)] {
        println!("phase {name} blocks {} samples {} iterations {}", ph.blocks, ph.samples, ph.iterations);
    }
    let samples = r.preview.samples + r.sparse.samples + r.dense.samples;
    println!("blocks.final {finals}
blocks.fallback {}", r.mask.len() - finals);
    println!("samples {samples}
full_samples {full}
skipped_samples {}
iterations {iterations}", full - samples);
    for (k, m) in r.mask.iter().enumerate() {
        let end = if m & FINAL != 0 { "final" } else { "fallback" };
        println!("block {} {} mask {m} {end}", k % r.blocks_x, k / r.blocks_x);
    }
}

/// The view and render parameters named by the shared flags.
pub(crate) fn job(a: &Args) -> Result<(View, Params), String> {
    let view = View {
        center_re: a.need("re")?.into(),
        center_im: a.need("im")?.into(),
        width: a.need("width")?.into(),
        rotation: a.num("rotation", 0.0)?,
    };
    Ok((view, params(a)?))
}

/// The render parameters named by the shared flags other than the view's.
pub(crate) fn params(a: &Args) -> Result<Params, String> {
    let (w, h) = size(a.str("size").unwrap_or("640x360"))?;
    let ss: u32 = a.num("ss", 1)?;
    let threads = a.num("threads", std::thread::available_parallelism().map_or(1, |n| n.get()))?;
    Ok(Params {
        nx: w * ss,
        ny: h * ss,
        ss,
        max_iter: a.num("iter", 100_000)?,
        escape_radius: 1e10,
        columns: columns(a.str("columns").unwrap_or("nu,de,normal"))?,
        threads,
        tier: tier(a.str("kernel").unwrap_or("auto"))?,
    })
}

pub(crate) fn size(s: &str) -> Result<(u32, u32), String> {
    let bad = || format!("--size: expected WxH, got {s:?}");
    let (w, h) = s.split_once('x').ok_or_else(bad)?;
    match (w.parse(), h.parse()) {
        (Ok(w), Ok(h)) if w > 0 && h > 0 => Ok((w, h)),
        _ => Err(bad()),
    }
}

fn tier(s: &str) -> Result<Option<Tier>, String> {
    match s {
        "auto" => Ok(None),
        "f64" => Ok(Some(Tier::F64)),
        "fx" => Ok(Some(Tier::Fixed)),
        "scaled" => Ok(Some(Tier::Scaled)),
        _ => Err(format!("--kernel: expected auto, f64, fx or scaled, got {s:?}")),
    }
}

pub(crate) fn columns(s: &str) -> Result<ColumnSet, String> {
    s.split(',').filter(|c| !c.is_empty()).try_fold(ColumnSet::of(&[Column::Class]), |set, c| {
        let col = match c {
            "nu" => Column::Nu,
            "de" => Column::De,
            "normal" => Column::Normal,
            "bound" => Column::Bound,
            _ => return Err(format!("--columns: unknown column {c:?} (nu, de, normal, bound)")),
        };
        Ok(set.with(col))
    })
}
