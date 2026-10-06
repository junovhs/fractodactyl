//! Parallel driver: rows are handed out on demand (fractal rows vary wildly in
//! cost), each thread writes its rows' column slices in place: no merge copy.
use crate::reference::Reference;
use crate::sample::{sample, Outcome};
use crate::view::Plane;
use crate::KERNEL;
use fd_samples::{Class, Column, ColumnSet, Evidence, Header, Kind, Samples, View, MINOR};
use std::sync::Mutex;

/// What to render (the view is passed separately).
#[derive(Clone, Copy, Debug)]
pub struct Params {
    /// Samples per row.
    pub nx: u32,
    /// Rows.
    pub ny: u32,
    /// Samples per output pixel along each axis.
    pub ss: u32,
    /// Iteration budget per sample.
    pub max_iter: u64,
    /// Escape radius.
    pub escape_radius: f64,
    /// Columns to produce. `Class` is implied; `Bound` is refused (no bounds in f64).
    pub columns: ColumnSet,
    /// Worker threads (bounded by the row count).
    pub threads: usize,
}

struct Row<'a> {
    j: usize,
    class: &'a mut [Class],
    nu: Option<&'a mut [f64]>,
    de: Option<&'a mut [f32]>,
    normal: Option<&'a mut [u16]>,
}

/// Compute every sample of `view` into fresh columns, with the matching header.
pub fn render(view: &View, p: &Params) -> Result<(Header, Samples), String> {
    if p.columns.has(Column::Bound) {
        return Err(format!("{KERNEL} produces heuristic results only: no Bound column"));
    }
    if p.ss == 0 || !p.nx.is_multiple_of(p.ss) || !p.ny.is_multiple_of(p.ss) {
        return Err("grid must be a whole number of pixels".into());
    }
    let plane = Plane::new(view, p.nx, p.ny)?;
    let cols = p.columns.with(Column::Class);
    let (nx, n) = (p.nx as usize, p.nx as usize * p.ny as usize);
    let reference = Reference::new(plane.c_re, plane.c_im, p.max_iter, p.escape_radius);
    let mut s = Samples::alloc(n, cols);
    {
        let mut nu = s.nu.as_deref_mut().map(|v| v.chunks_mut(nx));
        let mut de = s.de.as_deref_mut().map(|v| v.chunks_mut(nx));
        let mut normal = s.normal.as_deref_mut().map(|v| v.chunks_mut(nx));
        let rows: Vec<Row> = (s.class.chunks_mut(nx).enumerate())
            .map(|(j, class)| Row {
                j,
                class,
                nu: nu.as_mut().and_then(Iterator::next),
                de: de.as_mut().and_then(Iterator::next),
                normal: normal.as_mut().and_then(Iterator::next),
            })
            .collect();
        let queue = Mutex::new(rows.into_iter());
        let r2 = p.escape_radius * p.escape_radius;
        let job = Job { r: &reference, plane: &plane, px: plane.h * p.ss as f64, max_iter: p.max_iter, r2 };
        let deriv = cols.needs_derivative();
        std::thread::scope(|sc| {
            for _ in 0..p.threads.clamp(1, p.ny as usize) {
                sc.spawn(|| loop {
                    let Some(row) = queue.lock().unwrap().next() else { break };
                    if deriv {
                        job.fill::<true>(row)
                    } else {
                        job.fill::<false>(row)
                    }
                });
            }
        });
    }
    let h = Header {
        minor: MINOR,
        columns: cols,
        nx: p.nx,
        ny: p.ny,
        ss: p.ss,
        max_iter: p.max_iter,
        escape_radius: p.escape_radius,
        view: view.clone(),
        kernel: KERNEL.into(),
    };
    Ok((h, s))
}

struct Job<'a> {
    r: &'a Reference,
    plane: &'a Plane,
    /// Output pixel size in the complex plane.
    px: f64,
    max_iter: u64,
    r2: f64,
}

impl Job<'_> {
    fn fill<const D: bool>(&self, mut row: Row) {
        let pl = self.plane;
        for i in 0..row.class.len() {
            let (ar, ai) = pl.dc(i, row.j);
            let o = sample::<D>(self.r, (pl.c_re + ar, pl.c_im + ai), ar, ai, self.max_iter, self.r2);
            let kind = match o {
                Outcome::Escaped { .. } => Kind::Escaped,
                Outcome::Interior => Kind::Interior,
                Outcome::Unresolved => Kind::Unresolved,
            };
            row.class[i] = Class::new(kind, Evidence::Heuristic);
            let Outcome::Escaped { n, zr, zi, dr, di } = o else { continue };
            let z2 = zr * zr + zi * zi;
            let log2z = 0.5 * z2.log2();
            if let Some(nu) = row.nu.as_deref_mut() {
                nu[i] = n as f64 + 1.0 - log2z.log2();
            }
            if let Some(de) = row.de.as_deref_mut() {
                // 2 |z| ln|z| / |dz/dc|, in output pixels.
                let lnz = log2z * std::f64::consts::LN_2;
                de[i] = (2.0 * (z2 / (dr * dr + di * di)).sqrt() * lnz / self.px) as f32;
            }
            if let Some(nm) = row.normal.as_deref_mut() {
                // Direction of z / (dz/dc), i.e. z * conj(dz/dc).
                let (x, y) = pl.to_screen(zr * dr + zi * di, zi * dr - zr * di);
                nm[i] = Samples::angle(x, y);
            }
        }
    }
}
