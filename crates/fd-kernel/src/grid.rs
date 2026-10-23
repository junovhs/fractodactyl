//! Parallel driver: picks the cheapest valid tier, builds one reference, then hands
//! rows out on demand (fractal rows vary wildly in cost). Each thread writes its rows'
//! column slices in place: no merge copy. Output is independent of the thread count.
use crate::bla::Bla;
use crate::reference::Reference;
use crate::sample::{iterate, perturb_from, Binary64Exponent, Outcome, Skip, Start};
use crate::series::{Series, SeriesConfig};
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

/// BLA work of one render (ACC-01); with BLA, [`Stats::iterations`] counts each applied
/// block as one iterate.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BlaStats {
    /// Blocks applied, summed over samples.
    pub blocks: u64,
    /// Perturbation steps those blocks replaced.
    pub skipped: u64,
    /// Samples that applied no block: computed entirely by the plain fallback. Excludes
    /// [`BlaStats::closed_form_samples`].
    pub fallback_samples: u64,
    /// Samples settled by the closed-form main-cardioid/period-2 test before any
    /// iteration (f64 tier): neither BLA nor fallback work.
    pub closed_form_samples: u64,
    /// Largest first-order shift estimate over the samples, in output pixels: the
    /// distance in `c` that moves a sample as much as the terms its blocks dropped
    /// (ATLAS.md "BLA tables"). `None` when `dz/dc` is not tracked (no `nu`-only
    /// estimate exists without it).
    pub shift_px: Option<f64>,
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
    run(view, p, supplied, None, None, None).map(|(h, s, st, _)| (h, s, st))
}

/// An escaped state supplied by an externally guarded renderer. The caller owns
/// its accuracy/escape proof; this type does not grant Bounded evidence (DEC-10).
#[derive(Clone, Copy, Debug)]
pub struct EscapedState {
    pub n: u64,
    pub z: (f64, f64),
    pub derivative: (f64, f64),
    pub derivative_exponent: i64,
}

/// Encode admitted escaped states through the CPU renderer's one column encoder.
/// Other samples retain the zero-start CPU fallback values already in `samples`.
/// Refused for Bound columns: these external states supply no certified radii.
pub fn overlay_escaped(
    view: &View,
    p: &Params,
    samples: &mut Samples,
    states: &[Option<EscapedState>],
) -> Result<(), String> {
    if p.columns.has(Column::Bound)
        || states.len() != samples.class.len()
        || states.len() != p.nx as usize * p.ny as usize
    {
        return Err("escaped overlay needs matching unbounded sample grid".into());
    }
    let (plane, _) = setup(view, p)?;
    let store = Store::new(&plane, p.ss);
    for mut row in Row::split(samples, p.nx as usize) {
        for i in 0..p.nx as usize {
            if let Some(s) = states[row.j * p.nx as usize + i] {
                if s.n > p.max_iter
                    || !s.z.0.is_finite()
                    || !s.z.1.is_finite()
                    || s.z.0.hypot(s.z.1) <= p.escape_radius
                    || !s.derivative.0.is_finite()
                    || !s.derivative.1.is_finite()
                    || s.derivative.0.hypot(s.derivative.1) == 0.0
                {
                    return Err("escaped overlay received an invalid state".into());
                }
                store.put(
                    &mut row,
                    i,
                    Outcome::Escaped {
                        n: s.n,
                        zr: s.z.0,
                        zi: s.z.1,
                        dr: s.derivative.0,
                        di: s.derivative.1,
                        dexp: s.derivative_exponent,
                        ez: f64::INFINITY,
                        ed: f64::INFINITY,
                    },
                );
            }
        }
    }
    Ok(())
}

/// [`render_with`] computing only the samples whose `mask` entry (row-major over the
/// sample grid) is true; the others are left unset (FILM-16 jet-AA passes).
pub fn render_with_masked(
    view: &View,
    p: &Params,
    supplied: Option<(Reference, u32)>,
    mask: &[bool],
) -> Result<(Header, Samples, Stats), String> {
    run(view, p, supplied, None, Some(mask), None).map(|(h, s, st, _)| (h, s, st))
}

/// [`render_with`] a supplied orbit and a BLA table built over it (ACC-01): valid blocks
/// replace runs of perturbation steps; every other step is the plain kernel (the
/// fallback). Refused with the `Bound` column (BLA remainders are
/// not certified, ACC-02), for a table built over another orbit length, or when the
/// view's largest `|dc|` exceeds the table's `dc_max`. The header's kernel id gains
/// ` bla/1`.
pub fn render_bla(
    view: &View,
    p: &Params,
    orbit: (Reference, u32),
    bla: &Bla,
) -> Result<(Header, Samples, Stats, BlaStats), String> {
    render_bla_masked(view, p, orbit, bla, None)
}

/// [`render_bla`] computing only the samples whose `mask` entry is true (see
/// [`render_with_masked`]).
pub fn render_bla_masked(
    view: &View,
    p: &Params,
    orbit: (Reference, u32),
    bla: &Bla,
    mask: Option<&[bool]>,
) -> Result<(Header, Samples, Stats, BlaStats), String> {
    bla_checks(view, p, &orbit, bla)?;
    run(view, p, Some(orbit), Some(bla), mask, None)
}

/// PROB-41 research arm (not used by any fd command): [`render_with`] a supplied orbit,
/// starting every sample at the series' admitted iterate (`series.rs`), then continuing
/// with the plain kernel, or with BLA blocks when `bla` is given (as [`render_bla`]).
/// Refused on the scaled tier, with the `Bound` column, for a series built over another
/// orbit or for a smaller `|dc|` than the view reaches. The header's kernel id gains
/// ` series/0` (and ` bla/1`).
pub fn render_series(
    view: &View,
    p: &Params,
    orbit: (Reference, u32),
    series: &Series,
    bla: Option<&Bla>,
) -> Result<(Header, Samples, Stats, BlaStats), String> {
    series.render_masked(view, p, orbit, bla, None)
}

impl Series {
    /// Prepare a series start only when a small timing probe predicts a net win over
    /// the supplied BLA/plain renderer (DEC-19). Setup and probe costs count. Small,
    /// bounded and scaled frames decline before building. The caller may reuse the
    /// admitted start for masks of the same frame, without rebuilding it.
    /// Returns the initializer and whether its tail should use the supplied BLA.
    pub fn prepare(
        view: &View,
        p: &Params,
        orbit: &(Reference, u32),
        bla: Option<&Bla>,
        mask: Option<&[bool]>,
    ) -> Result<Option<(Series, bool)>, String> {
        let (plane, tier) = setup(view, p)?;
        let samples = p.nx as usize * p.ny as usize;
        if mask.is_some_and(|m| m.len() != samples) {
            return Err("sample mask must cover the sample grid".into());
        }
        let active = mask.map_or(samples, |m| m.iter().filter(|&&v| v).count());
        if active < 16_384
            || tier == Tier::Scaled
            || p.columns.has(Column::Bound)
            || p.max_iter < 256
        {
            return Ok(None);
        }
        if let Some(t) = bla {
            bla_checks(view, p, orbit, t)?;
        }
        let needed = precision(&plane, tier);
        if orbit.0.len() < 2
            || orbit.0.re.len() != orbit.0.im.len()
            || orbit.0.re[0] != 0.0
            || orbit.0.im[0] != 0.0
            || orbit.1 < needed
            || (tier == Tier::F64 && orbit.1 != needed)
        {
            return Err("series preparation needs the frame's valid reference orbit".into());
        }
        let started = Instant::now();
        // Keep an abstention cheap even on interior-heavy 100k-iteration frames.
        // Censored probe samples take the ordinary retry, making admission
        // conservative rather than treating an unfinished series tail as a win.
        let probe_params = Params {
            max_iter: p.max_iter.min(8192),
            ..*p
        };
        let series = Series::build(
            &orbit.0,
            dc_max(&plane, p),
            plane.h() * f64::from(p.ss),
            probe_params.max_iter,
            SeriesConfig::default(),
        );
        if series.n == 0 {
            return Ok(None);
        }
        let points: Vec<usize> = match mask {
            Some(m) => m
                .iter()
                .enumerate()
                .filter_map(|(i, &v)| v.then_some(i))
                .step_by((active / 9).max(1))
                .take(9)
                .collect(),
            None => [p.ny / 8, p.ny / 2, 7 * p.ny / 8]
                .into_iter()
                .flat_map(|j| {
                    [p.nx / 8, p.nx / 2, 7 * p.nx / 8]
                        .map(|i| j as usize * p.nx as usize + i as usize)
                })
                .collect(),
        };
        let mut job = Job::new(
            &orbit.0,
            &plane,
            tier,
            &probe_params,
            p.columns.with(Column::Class),
        );
        let mut probe = |with_series: bool, with_bla: bool| {
            job.series = with_series.then_some(&series);
            job.bla = if with_bla { bla } else { None };
            let t = Instant::now();
            for &index in &points {
                let mut skip = Skip::default();
                let (i, j) = (index % p.nx as usize, index / p.nx as usize);
                let result = match (job.deriv, job.bla.is_some()) {
                    (true, true) => job.one::<true, false, true>(i, j, &mut skip),
                    (true, false) => job.one::<true, false, false>(i, j, &mut skip),
                    (false, true) => job.one::<false, false, true>(i, j, &mut skip),
                    (false, false) => job.one::<false, false, false>(i, j, &mut skip),
                };
                std::hint::black_box(result);
            }
            t.elapsed().as_secs_f64()
        };
        let baseline = probe(false, bla.is_some());
        let plain_tail = probe(true, false);
        let bla_tail = if bla.is_some() {
            probe(true, true)
        } else {
            f64::INFINITY
        };
        let use_bla = bla_tail < plain_tail;
        let accelerated = plain_tail.min(bla_tail);
        // Pixel iteration is parallel; preparation/probing is serial. Charge setup
        // against the predicted parallel frame, with a 20% margin for sampling noise.
        let factor = active as f64 / points.len() as f64 / p.threads.clamp(1, p.ny as usize) as f64;
        let projected_baseline = baseline * factor;
        let projected_series = accelerated * factor + started.elapsed().as_secs_f64();
        Ok((projected_series < 0.8 * projected_baseline).then_some((series, use_bla)))
    }

    /// Render an admitted series start on a whole frame or a sample mask. Unresolved
    /// samples retry from zero, retaining the ordinary nucleus-contraction heuristic.
    pub fn render_masked(
        &self,
        view: &View,
        p: &Params,
        orbit: (Reference, u32),
        bla: Option<&Bla>,
        mask: Option<&[bool]>,
    ) -> Result<(Header, Samples, Stats, BlaStats), String> {
        let series = self;
        let (plane, tier) = setup(view, p)?;
        if tier == Tier::Scaled || p.columns.has(Column::Bound) {
            return Err("a series start needs f64 deltas and no Bound column".into());
        }
        if series.points != orbit.0.len() as u64 {
            return Err("series built over another orbit".into());
        }
        let dc = dc_max(&plane, p);
        if dc.is_nan() || dc > series.rho {
            return Err(format!(
                "view reaches |dc| = {dc:e}, beyond the series radius {:e}",
                series.rho
            ));
        }
        if let Some(bla) = bla {
            bla_checks(view, p, &orbit, bla)?;
        }
        let (h, s, st, b) = run(view, p, Some(orbit), bla, mask, Some(series))?;
        let mut h = h;
        h.kernel.push_str(" series/0");
        Ok((h, s, st, b))
    }
}

/// [`render_bla`]'s refusals.
fn bla_checks(view: &View, p: &Params, orbit: &(Reference, u32), bla: &Bla) -> Result<(), String> {
    let (plane, tier) = setup(view, p)?;
    if p.columns.has(Column::Bound) {
        return Err(
            "--columns bound certifies every step; BLA remainders are not certified (ACC-02)"
                .into(),
        );
    }
    if bla.points != orbit.0.len() as u64 {
        return Err(format!(
            "BLA table covers a {}-point orbit, not this {}-point one",
            bla.points,
            orbit.0.len()
        ));
    }
    if p.escape_radius.is_nan() || p.escape_radius < 4.0 {
        return Err(format!(
            "escape radius {} is below 4: BLA blocks may only skip iterates with |z| <= 2 (1 + 2 eps), which needs escape radius >= 4",
            p.escape_radius
        ));
    }
    let (dc, e) = dc_bound(&plane, tier, p);
    // dc 2^e against dc_max 2^dc_exp, without leaving f64's range.
    let over = if e == bla.dc_exp {
        dc > bla.dc_max
    } else {
        dc.log2() + e as f64 > bla.dc_max.log2() + bla.dc_exp as f64
    };
    if dc.is_nan() || over {
        return Err(format!(
            "view reaches |dc| = {dc:e} 2^{e}, beyond the BLA table's dc_max {:e} 2^{}",
            bla.dc_max, bla.dc_exp
        ));
    }
    Ok(())
}

/// Upper bound `m 2^e` on `|dc|` over the samples of a view, on every tier (`e = 0`
/// except on the scaled tier, whose `|dc|` lies below f64's range): what
/// [`fd_kernel::Bla::build_scaled`] takes (FIX-03).
pub fn bla_dc_bound(view: &View, p: &Params) -> Result<(f64, i64), String> {
    let (plane, tier) = setup(view, p)?;
    Ok(dc_bound(&plane, tier, p))
}

fn dc_bound(plane: &Plane, tier: Tier, p: &Params) -> (f64, i64) {
    let half = (f64::from(p.nx) / 2.0).hypot(f64::from(p.ny) / 2.0);
    if tier == Tier::Scaled {
        (plane.h_m * half, plane.h_e)
    } else {
        (dc_max(plane, p), 0)
    }
}

/// Upper bound on `|dc|` over the samples of a view; refused on the scaled tier, which
/// BLA does not support.
pub fn bla_dc_max(view: &View, p: &Params) -> Result<f64, String> {
    let (plane, tier) = setup(view, p)?;
    if tier == Tier::Scaled {
        return Err("BLA tables need f64 deltas: not available on the scaled tier".into());
    }
    Ok(dc_max(&plane, p))
}

fn dc_max(plane: &Plane, p: &Params) -> f64 {
    plane.h() * (f64::from(p.nx) / 2.0).hypot(f64::from(p.ny) / 2.0)
}

fn run(
    view: &View,
    p: &Params,
    supplied: Option<(Reference, u32)>,
    bla: Option<&Bla>,
    mask: Option<&[bool]>,
    series: Option<&Series>,
) -> Result<(Header, Samples, Stats, BlaStats), String> {
    if mask.is_some_and(|m| m.len() != p.nx as usize * p.ny as usize) {
        return Err("sample mask must cover the sample grid".into());
    }
    let (mut plane, tier) = setup(view, p)?;
    let t = Instant::now();
    let need = precision(&plane, tier);
    let reference = match supplied {
        Some((_, bits)) if bits != need && (tier == Tier::F64 || bits < need) => {
            let at_least = if tier == Tier::F64 { "" } else { "at least " };
            return Err(format!(
                "supplied reference has {bits} bits; this view needs {at_least}{need}"
            ));
        }
        Some((r, _))
            if r.is_empty() || r.re.len() != r.im.len() || r.re[0] != 0.0 || r.im[0] != 0.0 =>
        {
            return Err(
                "supplied reference orbit must start at Z_0 = 0 with equal re/im lengths".into(),
            )
        }
        Some((r, bits)) => {
            plane.bits = bits.into();
            r
        }
        None => compute(view, &plane, tier, p)?,
    };
    let reference_seconds = t.elapsed().as_secs_f64();
    let iterations = AtomicU64::new(0);
    let total = Mutex::new((BlaStats::default(), 0.0f64));
    let cols = p.columns.with(Column::Class);
    let (nx, n) = (p.nx as usize, p.nx as usize * p.ny as usize);
    let mut s = Samples::alloc(n, cols);
    {
        let queue = Mutex::new(Row::split(&mut s, nx).into_iter());
        let mut job = Job::new(&reference, &plane, tier, p, cols);
        job.bla = bla;
        job.mask = mask;
        job.series = series.filter(|s| s.n > 0);
        let (deriv, bounded, blas) = (job.deriv, !job.q.is_empty(), job.bla.is_some());
        std::thread::scope(|sc| {
            for _ in 0..p.threads.clamp(1, p.ny as usize) {
                sc.spawn(|| {
                    let (mut its, mut b, mut shift) = (0, BlaStats::default(), 0.0f64);
                    loop {
                        // let-else drops the lock guard before the row is filled.
                        let Some(row) = queue.lock().unwrap().next() else {
                            break;
                        };
                        let sh = &mut shift;
                        its += match (deriv, bounded, blas) {
                            (_, true, _) => job.fill::<true, true, false>(row, &mut b, sh),
                            (true, false, false) => job.fill::<true, false, false>(row, &mut b, sh),
                            (false, false, false) => {
                                job.fill::<false, false, false>(row, &mut b, sh)
                            }
                            (true, false, true) => job.fill::<true, false, true>(row, &mut b, sh),
                            (false, false, true) => job.fill::<false, false, true>(row, &mut b, sh),
                        };
                    }
                    iterations.fetch_add(its, Ordering::Relaxed);
                    let mut sum = total.lock().unwrap();
                    sum.0.blocks += b.blocks;
                    sum.0.skipped += b.skipped;
                    sum.0.fallback_samples += b.fallback_samples;
                    sum.0.closed_form_samples += b.closed_form_samples;
                    sum.1 = sum.1.max(shift);
                });
            }
        });
    }
    let stats = Stats {
        reference_len: reference.len(),
        reference_seconds,
        iterations: iterations.into_inner(),
    };
    let mut h = header(view, p, cols, &plane, tier);
    if bla.is_some() {
        h.kernel.push_str(" bla/1");
    }
    let (mut b, shift) = total.into_inner().unwrap();
    // Output pixel = ss samples of spacing h.
    b.shift_px =
        (bla.is_some() && cols.needs_derivative()).then(|| shift / (plane.h() * f64::from(p.ss)));
    Ok((h, s, stats, b))
}

/// The header of a render of `view` with `cols`.
pub(crate) fn header(
    view: &View,
    p: &Params,
    cols: ColumnSet,
    plane: &Plane,
    tier: Tier,
) -> Header {
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
    nucleus: Option<usize>,
    /// Reference error radii; empty unless bounds are tracked.
    q: Vec<f64>,
    /// Whether `dz/dc` is tracked.
    deriv: bool,
    /// BLA table over `r`, if blocks may replace steps (never with bounds).
    bla: Option<&'a Bla>,
    /// Samples to compute (row-major over the grid of width `nx`); `None` computes all.
    mask: Option<&'a [bool]>,
    /// Series start for every sample (PROB-41 research arm; never with bounds).
    series: Option<&'a Series>,
    nx: usize,
}

impl<'a> Job<'a> {
    /// Per-render sample constants for columns `cols` (bounds tracked iff `Bound` is in it).
    pub(crate) fn new(
        r: &'a Reference,
        plane: &'a Plane,
        tier: Tier,
        p: &Params,
        cols: ColumnSet,
    ) -> Job<'a> {
        let q = match (cols.has(Column::Bound), tier) {
            (false, _) | (_, Tier::Scaled) => Vec::new(),
            (true, Tier::F64) => r.error_radius(None, plane.c_re, plane.c_im),
            (true, Tier::Fixed) => r.error_radius(Some(plane.bits), plane.c_re, plane.c_im),
        };
        let deriv = cols.needs_derivative() || !q.is_empty();
        let (store, r2) = (Store::new(plane, p.ss), p.escape_radius * p.escape_radius);
        Job {
            r,
            plane,
            tier,
            store,
            max_iter: p.max_iter,
            r2,
            nucleus: r.nucleus_period(),
            q,
            deriv,
            bla: None,
            mask: None,
            series: None,
            nx: p.nx as usize,
        }
    }

    /// Outcome of sample `(i, j)`, with the same kernel choice as [`render`] (no BLA).
    pub(crate) fn outcome(&self, i: usize, j: usize) -> Outcome {
        let skip = &mut Skip::default();
        match (self.deriv, !self.q.is_empty()) {
            (_, true) => self.one::<true, true, false>(i, j, skip).0,
            (true, false) => self.one::<true, false, false>(i, j, skip).0,
            (false, false) => self.one::<false, false, false>(i, j, skip).0,
        }
    }

    #[inline]
    fn one<const D: bool, const B: bool, const L: bool>(
        &self,
        i: usize,
        j: usize,
        skip: &mut Skip,
    ) -> (Outcome, u64) {
        let first = self.one_from::<D, B, L>(i, j, skip, true);
        let prefix = self
            .series
            .filter(|_| !B && first != Outcome::Interior { n: 0 })
            .map_or(0, |s| s.n);
        let mut spent = first.iterations(self.max_iter) - prefix;
        let result = if prefix > 0 && first == Outcome::Unresolved {
            let ordinary = self.one_from::<D, B, L>(i, j, skip, false);
            spent += ordinary.iterations(self.max_iter);
            ordinary
        } else {
            first
        };
        if L {
            spent = spent - skip.skipped + skip.blocks;
        }
        (result, spent)
    }

    #[inline]
    fn one_from<const D: bool, const B: bool, const L: bool>(
        &self,
        i: usize,
        j: usize,
        skip: &mut Skip,
        use_series: bool,
    ) -> Outcome {
        let pl = self.plane;
        let h = pl.h();
        let (ux, uy) = pl.unit_offset(i, j);
        let start = |ar: f64, ai: f64| match self.series {
            Some(s) if !B && use_series => {
                let (xr, xi, dr, di) = s.eval::<D>(ar, ai);
                Start {
                    n: s.n,
                    xr,
                    xi,
                    dr,
                    di,
                }
            }
            _ => Start::default(),
        };
        match self.tier {
            Tier::F64 => {
                let (ar, ai) = (ux * h, uy * h);
                let c = Some((pl.c_re + ar, pl.c_im + ai));
                perturb_from::<D, B, L>(
                    self.r,
                    &self.q,
                    self.bla,
                    skip,
                    c,
                    ar,
                    ai,
                    self.max_iter,
                    self.r2,
                    self.nucleus,
                    start(ar, ai),
                )
            }
            Tier::Fixed => perturb_from::<D, B, L>(
                self.r,
                &self.q,
                self.bla,
                skip,
                None,
                ux * h,
                uy * h,
                self.max_iter,
                self.r2,
                self.nucleus,
                start(ux * h, uy * h),
            ),
            Tier::Scaled => iterate::<Binary64Exponent, D, false, L>(
                self.r,
                &[],
                self.bla,
                skip,
                None,
                ux * pl.h_m,
                uy * pl.h_m,
                pl.h_e,
                self.max_iter,
                self.r2,
                self.nucleus,
                Start::default(),
            ),
        }
    }

    /// Fill one row, adding its BLA work to `b` and raising `shift` (in `c` units) to
    /// the row's largest shift estimate; returns the iterates spent (one per applied
    /// block).
    fn fill<const D: bool, const B: bool, const L: bool>(
        &self,
        mut row: Row,
        b: &mut BlaStats,
        shift: &mut f64,
    ) -> u64 {
        let mut its = 0;
        for i in 0..row.class.len() {
            if self.mask.is_some_and(|m| !m[row.j * self.nx + i]) {
                continue;
            }
            let mut skip = Skip::default();
            let (o, spent) = self.one::<D, B, L>(i, row.j, &mut skip);
            its += spent;
            if L {
                b.blocks += skip.blocks;
                b.skipped += skip.skipped;
                let closed = matches!(o, Outcome::Interior { n: 0 });
                b.closed_form_samples += u64::from(closed);
                b.fallback_samples += u64::from(skip.blocks == 0 && !closed);
                *shift = shift.max(skip.shift);
            }
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
        return Err(format!(
            "{tier:?} tier is not valid at this depth (cheapest valid: {:?})",
            plane.tier
        ));
    }
    Ok((plane, tier))
}

/// Compute the reference orbit for `tier`.
pub(crate) fn compute(
    view: &View,
    plane: &Plane,
    tier: Tier,
    p: &Params,
) -> Result<Reference, String> {
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
