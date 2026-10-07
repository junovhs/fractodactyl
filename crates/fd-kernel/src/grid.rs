//! Parallel driver: picks the cheapest valid tier, builds one reference, then hands
//! rows out on demand (fractal rows vary wildly in cost). Each thread writes its rows'
//! column slices in place: no merge copy. Output is independent of the thread count.
use crate::reference::Reference;
use crate::sample::sample;
use crate::scaled::scaled;
use crate::store::{Row, Store};
use crate::view::{Plane, Tier};
use fd_samples::{Column, ColumnSet, Header, Samples, View, MINOR};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

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

/// Work counters for one render (BASE-03). Independent of the thread count.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stats {
    /// Stored reference-orbit points.
    pub reference_len: usize,
    /// Wall time spent computing the reference orbit.
    pub reference_seconds: f64,
    /// Perturbation iterates summed over all samples (`max_iter` per unresolved one).
    pub iterations: u64,
}

/// Compute every sample of `view` into fresh columns, with the matching header.
pub fn render(view: &View, p: &Params) -> Result<(Header, Samples), String> {
    render_stats(view, p).map(|(h, s, _)| (h, s))
}

/// [`render`], also returning its work counters.
pub fn render_stats(view: &View, p: &Params) -> Result<(Header, Samples, Stats), String> {
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
    let t = Instant::now();
    let reference = match tier {
        Tier::F64 => Reference::new(plane.c_re, plane.c_im, p.max_iter, p.escape_radius),
        _ => {
            let (cr, ci) = Plane::center_fixed(view, plane.bits)?;
            Reference::from_fixed(&cr, &ci, p.max_iter)
        }
    };
    let reference_seconds = t.elapsed().as_secs_f64();
    let iterations = AtomicU64::new(0);
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
                sc.spawn(|| {
                    let mut its = 0;
                    loop {
                        // let-else drops the lock guard before the row is filled.
                        let Some(row) = queue.lock().unwrap().next() else { break };
                        its += if deriv { job.fill::<true>(row) } else { job.fill::<false>(row) };
                    }
                    iterations.fetch_add(its, Ordering::Relaxed);
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
    let stats = Stats { reference_len: reference.len(), reference_seconds, iterations: iterations.into_inner() };
    Ok((h, s, stats))
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
    /// Fill one row; returns the iterates spent.
    fn fill<const D: bool>(&self, mut row: Row) -> u64 {
        let pl = self.plane;
        let h = pl.h();
        let mut its = 0;
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
            its += o.iterations(self.max_iter);
            self.store.put(&mut row, i, o);
        }
        its
    }
}
