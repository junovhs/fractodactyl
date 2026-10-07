//! Parallel driver: picks the cheapest valid tier, builds one reference, then hands
//! rows out on demand (fractal rows vary wildly in cost). Each thread writes its rows'
//! column slices in place: no merge copy. Output is independent of the thread count.
use crate::reference::Reference;
use crate::sample::sample;
use crate::scaled::scaled;
use crate::store::{Row, Store};
use crate::view::{Plane, Tier};
use fd_samples::{Column, ColumnSet, Header, Samples, View, MINOR};
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
    /// Columns to produce. `Class` is implied; `Bound` is refused (no bounds yet).
    pub columns: ColumnSet,
    /// Worker threads (bounded by the row count).
    pub threads: usize,
    /// Force a tier instead of the cheapest valid one (refused where it is invalid).
    pub tier: Option<Tier>,
}

/// Compute every sample of `view` into fresh columns, with the matching header.
pub fn render(view: &View, p: &Params) -> Result<(Header, Samples), String> {
    if p.columns.has(Column::Bound) {
        return Err("kernels produce heuristic results only: no Bound column".into());
    }
    if p.ss == 0 || !p.nx.is_multiple_of(p.ss) || !p.ny.is_multiple_of(p.ss) {
        return Err("grid must be a whole number of pixels".into());
    }
    let plane = Plane::new(view, p.nx, p.ny)?;
    let tier = p.tier.unwrap_or(plane.tier);
    if !plane.allows(tier) {
        return Err(format!("{tier:?} tier is not valid at this depth (cheapest valid: {:?})", plane.tier));
    }
    let reference = match tier {
        Tier::F64 => Reference::new(plane.c_re, plane.c_im, p.max_iter, p.escape_radius),
        _ => {
            let (cr, ci) = Plane::center_fixed(view, plane.bits)?;
            Reference::from_fixed(&cr, &ci, p.max_iter)
        }
    };
    let cols = p.columns.with(Column::Class);
    let (nx, n) = (p.nx as usize, p.nx as usize * p.ny as usize);
    let mut s = Samples::alloc(n, cols);
    {
        let queue = Mutex::new(Row::split(&mut s, nx).into_iter());
        let job = Job {
            r: &reference,
            plane: &plane,
            tier,
            store: Store::new(&plane, p.ss),
            max_iter: p.max_iter,
            r2: p.escape_radius * p.escape_radius,
        };
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
        kernel: plane.kernel(tier),
    };
    Ok((h, s))
}

struct Job<'a> {
    r: &'a Reference,
    plane: &'a Plane,
    tier: Tier,
    store: Store,
    max_iter: u64,
    r2: f64,
}

impl Job<'_> {
    fn fill<const D: bool>(&self, mut row: Row) {
        let pl = self.plane;
        let h = pl.h();
        for i in 0..row.class.len() {
            let (ux, uy) = pl.unit_offset(i, row.j);
            let o = match self.tier {
                Tier::F64 => {
                    let (ar, ai) = (ux * h, uy * h);
                    sample::<D>(self.r, Some((pl.c_re + ar, pl.c_im + ai)), ar, ai, self.max_iter, self.r2)
                }
                Tier::Fixed => sample::<D>(self.r, None, ux * h, uy * h, self.max_iter, self.r2),
                Tier::Scaled => scaled::<D>(self.r, ux * pl.h_m, uy * pl.h_m, pl.h_e, self.max_iter, self.r2),
            };
            self.store.put(&mut row, i, o);
        }
    }
}
