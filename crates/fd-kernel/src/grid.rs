//! Parallel driver: picks the cheapest valid tier, builds one reference, then hands
//! rows out on demand (fractal rows vary wildly in cost). Each thread writes its rows'
//! column slices in place: no merge copy. Output is independent of the thread count.
use crate::reference::Reference;
use crate::sample::{sample, Outcome};
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
    /// Columns to produce. `Class` is implied. `Bound` turns on error tracking: escaped
    /// samples whose error radii certify them are written `Bounded` (f64/fx tiers).
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
    render_with(view, p, None)
}

/// The reference orbit [`render`] computes for `view` and `p`, with its working
/// precision in bits (53 for the f64 tier, the fixed-point fraction bits otherwise).
pub fn reference(view: &View, p: &Params) -> Result<(Reference, u32), String> {
    let (plane, tier) = setup(view, p)?;
    Ok((compute(view, &plane, tier, p)?, precision(&plane, tier)))
}

/// The working precision [`reference`] would use for `view` and `p`, without computing it.
pub fn reference_bits(view: &View, p: &Params) -> Result<u32, String> {
    let (plane, tier) = setup(view, p)?;
    Ok(precision(&plane, tier))
}

/// [`render_stats`] with a supplied reference orbit and its precision bits (for example
/// loaded from atlas slabs) instead of computing it. The caller vouches that the orbit
/// is the one at this view's exact centre (any length, any `max_iter`). The f64 tier
/// needs exactly 53 bits; the deep tiers take any precision at least
/// [`reference_bits`] (REF-02: one deep orbit serves every shallower frame at its
/// centre), and the header records the precision used. With the precision
/// [`reference`] would use, output is identical to computing the orbit.
pub fn render_with(
    view: &View,
    p: &Params,
    supplied: Option<(Reference, u32)>,
) -> Result<(Header, Samples, Stats), String> {
    let (mut plane, tier) = setup(view, p)?;
    let t = Instant::now();
    let need = precision(&plane, tier);
    let reference = match supplied {
        Some((_, bits)) if bits != need && (tier == Tier::F64 || bits < need) => {
            let at_least = if tier == Tier::F64 { "" } else { "at least " };
            return Err(format!("supplied reference has {bits} bits; this view needs {at_least}{need}"));
        }
        Some((r, _)) if r.is_empty() || r.re.len() != r.im.len() || r.re[0] != 0.0 || r.im[0] != 0.0 => {
            return Err("supplied reference orbit must start at Z_0 = 0 with equal re/im lengths".into())
        }
        Some((r, bits)) => {
            plane.bits = bits.into();
            r
        }
        None => compute(view, &plane, tier, p)?,
    };
    let reference_seconds = t.elapsed().as_secs_f64();
    let iterations = AtomicU64::new(0);
    let cols = p.columns.with(Column::Class);
    let (nx, n) = (p.nx as usize, p.nx as usize * p.ny as usize);
    let mut s = Samples::alloc(n, cols);
    {
        let queue = Mutex::new(Row::split(&mut s, nx).into_iter());
        let job = Job::new(&reference, &plane, tier, p, cols);
        let (deriv, bounded) = (job.deriv, !job.q.is_empty());
        std::thread::scope(|sc| {
            for _ in 0..p.threads.clamp(1, p.ny as usize) {
                sc.spawn(|| {
                    let mut its = 0;
                    loop {
                        // let-else drops the lock guard before the row is filled.
                        let Some(row) = queue.lock().unwrap().next() else { break };
                        its += match (deriv, bounded) {
                            (_, true) => job.fill::<true, true>(row),
                            (true, false) => job.fill::<true, false>(row),
                            (false, false) => job.fill::<false, false>(row),
                        };
                    }
                    iterations.fetch_add(its, Ordering::Relaxed);
                });
            }
        });
    }
    let stats = Stats { reference_len: reference.len(), reference_seconds, iterations: iterations.into_inner() };
    Ok((header(view, p, cols, &plane, tier), s, stats))
}

/// The header of a render of `view` with `cols`.
pub(crate) fn header(view: &View, p: &Params, cols: ColumnSet, plane: &Plane, tier: Tier) -> Header {
    Header {
        minor: MINOR,
        columns: cols,
        nx: p.nx,
        ny: p.ny,
        ss: p.ss,
        max_iter: p.max_iter,
        escape_radius: p.escape_radius,
        view: view.clone(),
        kernel: plane.kernel(tier),
    }
}

pub(crate) struct Job<'a> {
    r: &'a Reference,
    plane: &'a Plane,
    tier: Tier,
    pub(crate) store: Store,
    pub(crate) max_iter: u64,
    r2: f64,
    /// Reference error radii; empty unless bounds are tracked.
    q: Vec<f64>,
    /// Whether `dz/dc` is tracked.
    deriv: bool,
}

impl<'a> Job<'a> {
    /// Per-render sample constants for columns `cols` (bounds tracked iff `Bound` is in it).
    pub(crate) fn new(r: &'a Reference, plane: &'a Plane, tier: Tier, p: &Params, cols: ColumnSet) -> Job<'a> {
        let q = match (cols.has(Column::Bound), tier) {
            (false, _) | (_, Tier::Scaled) => Vec::new(),
            (true, Tier::F64) => r.error_radius(None, plane.c_re, plane.c_im),
            (true, Tier::Fixed) => r.error_radius(Some(plane.bits), plane.c_re, plane.c_im),
        };
        let deriv = cols.needs_derivative() || !q.is_empty();
        let (store, r2) = (Store::new(plane, p.ss), p.escape_radius * p.escape_radius);
        Job { r, plane, tier, store, max_iter: p.max_iter, r2, q, deriv }
    }

    /// Outcome of sample `(i, j)`, with the same kernel choice as [`render`].
    pub(crate) fn outcome(&self, i: usize, j: usize) -> Outcome {
        match (self.deriv, !self.q.is_empty()) {
            (_, true) => self.one::<true, true>(i, j),
            (true, false) => self.one::<true, false>(i, j),
            (false, false) => self.one::<false, false>(i, j),
        }
    }

    #[inline]
    fn one<const D: bool, const B: bool>(&self, i: usize, j: usize) -> Outcome {
        let pl = self.plane;
        let h = pl.h();
        let (ux, uy) = pl.unit_offset(i, j);
        match self.tier {
            Tier::F64 => {
                let (ar, ai) = (ux * h, uy * h);
                sample::<D, B>(self.r, &self.q, Some((pl.c_re + ar, pl.c_im + ai)), ar, ai, self.max_iter, self.r2)
            }
            Tier::Fixed => sample::<D, B>(self.r, &self.q, None, ux * h, uy * h, self.max_iter, self.r2),
            Tier::Scaled => scaled::<D>(self.r, ux * pl.h_m, uy * pl.h_m, pl.h_e, self.max_iter, self.r2),
        }
    }

    /// Fill one row; returns the iterates spent.
    fn fill<const D: bool, const B: bool>(&self, mut row: Row) -> u64 {
        let mut its = 0;
        for i in 0..row.class.len() {
            let o = self.one::<D, B>(i, row.j);
            its += o.iterations(self.max_iter);
            self.store.put(&mut row, i, o);
        }
        its
    }
}

/// Validate `p` and choose the tier.
pub(crate) fn setup(view: &View, p: &Params) -> Result<(Plane, Tier), String> {
    if p.ss == 0 || !p.nx.is_multiple_of(p.ss) || !p.ny.is_multiple_of(p.ss) {
        return Err("grid must be a whole number of pixels".into());
    }
    let plane = Plane::new(view, p.nx, p.ny)?;
    let tier = p.tier.unwrap_or(plane.tier);
    if !plane.allows(tier) {
        return Err(format!("{tier:?} tier is not valid at this depth (cheapest valid: {:?})", plane.tier));
    }
    Ok((plane, tier))
}

/// Compute the reference orbit for `tier`.
pub(crate) fn compute(view: &View, plane: &Plane, tier: Tier, p: &Params) -> Result<Reference, String> {
    Ok(match tier {
        Tier::F64 => Reference::new(plane.c_re, plane.c_im, p.max_iter, p.escape_radius),
        _ => {
            let (cr, ci) = Plane::center_fixed(view, plane.bits)?;
            Reference::from_fixed(&cr, &ci, p.max_iter)
        }
    })
}

/// Working precision of the reference orbit for `tier`.
fn precision(plane: &Plane, tier: Tier) -> u32 {
    match tier {
        Tier::F64 => 53,
        _ => plane.bits as u32,
    }
}
