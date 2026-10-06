//! `fd render`: compute samples and write a `.fds` file. No colour happens here.
use crate::args::Args;
use fd_kernel::{render, Params};
use fd_samples::{write, Column, ColumnSet, View};
use std::io::BufWriter;
use std::time::Instant;

const FLAGS: [&str; 10] = ["re", "im", "width", "size", "ss", "iter", "columns", "threads", "rotation", "o"];

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let a = Args::parse(argv, &FLAGS)?;
    let out = a.need("o")?;
    let view = View {
        center_re: a.need("re")?.into(),
        center_im: a.need("im")?.into(),
        width: a.need("width")?.into(),
        rotation: a.num("rotation", 0.0)?,
    };
    let (w, h) = size(a.str("size").unwrap_or("640x360"))?;
    let ss: u32 = a.num("ss", 1)?;
    let threads = a.num("threads", std::thread::available_parallelism().map_or(1, |n| n.get()))?;
    let p = Params {
        nx: w * ss,
        ny: h * ss,
        ss,
        max_iter: a.num("iter", 100_000)?,
        escape_radius: 1e10,
        columns: columns(a.str("columns").unwrap_or("nu,de,normal"))?,
        threads,
    };
    let t = Instant::now();
    let (header, samples) = render(&view, &p)?;
    let secs = t.elapsed().as_secs_f64();
    let file = std::fs::File::create(out).map_err(|e| format!("{out}: {e}"))?;
    write(BufWriter::new(file), &header, &samples).map_err(|e| format!("{out}: {e}"))?;
    let bytes = std::fs::metadata(out).map(|m| m.len()).unwrap_or(0);
    println!("{out}: {}x{} samples, {threads} threads, {secs:.3} s, {bytes} bytes", p.nx, p.ny);
    Ok(())
}

fn size(s: &str) -> Result<(u32, u32), String> {
    let bad = || format!("--size: expected WxH, got {s:?}");
    let (w, h) = s.split_once('x').ok_or_else(bad)?;
    match (w.parse(), h.parse()) {
        (Ok(w), Ok(h)) if w > 0 && h > 0 => Ok((w, h)),
        _ => Err(bad()),
    }
}

fn columns(s: &str) -> Result<ColumnSet, String> {
    s.split(',').filter(|c| !c.is_empty()).try_fold(ColumnSet::of(&[Column::Class]), |set, c| {
        let col = match c {
            "nu" => Column::Nu,
            "de" => Column::De,
            "normal" => Column::Normal,
            _ => return Err(format!("--columns: unknown column {c:?} (nu, de, normal)")),
        };
        Ok(set.with(col))
    })
}
