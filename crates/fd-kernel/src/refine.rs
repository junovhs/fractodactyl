//! Progressive refinement (LOD-02, docs/spec/LOD.md): parent preview, a sparse phase of
//! one sample per output pixel, a dense phase over the compacted list of blocks the
//! screen-space error rule did not accept, then every block final or fallback.
//! Only real samples are judged; the preview never certifies anything.
use crate::grid::{compute, header, setup, Job, Params, Stats};
use crate::sample::Outcome;
use crate::store::Row;
use fd_samples::lod::{judge, judge_reach};
use fd_samples::{Class, Column, Evidence, Header, Samples, View};
use std::sync::Mutex;
use std::time::Instant;

/// Block mask bit: the block's preview sample was computed.
pub const PREVIEW: u8 = 1;
/// Block mask bit: every pixel of the block has its sparse sample.
pub const SPARSE: u8 = 2;
/// Block mask bit: every sample of the block was computed.
pub const DENSE: u8 = 4;
/// Block mask bit: accepted by the screen-space error rule.
pub const FINAL: u8 = 8;
/// Block mask bit: not accepted after dense sampling; its exact per-sample results stand.
pub const FALLBACK: u8 = 16;

/// Work spent in one phase.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Phase {
    /// Blocks the phase worked on.
    pub blocks: usize,
    /// Samples computed.
    pub samples: u64,
    /// Perturbation iterates spent.
    pub iterations: u64,
}

/// What a progressive render did, block by block.
#[derive(Clone, Debug, PartialEq)]
pub struct Refinement {
    /// Block side in output pixels.
    pub block_px: u32,
    /// Blocks per block row.
    pub blocks_x: usize,
    /// Phase bits per block, row-major from the top-left.
    pub mask: Vec<u8>,
    /// One sample per block.
    pub preview: Phase,
    /// One sample per output pixel.
    pub sparse: Phase,
    /// The remaining samples of blocks the sparse phase did not accept.
    pub dense: Phase,
}

/// Render `view` progressively in `block_px` blocks, accepting at `max_px` (LOD.md).
/// `De` and `Bound` are always produced (the rule reads them). In a block accepted after
/// the sparse phase, each pixel's other samples are copies of its sparse sample marked
/// `Heuristic`; every other sample is computed exactly as [`crate::render`] would.
pub fn render_refined(
    view: &View,
    p: &Params,
    block_px: u32,
    max_px: f64,
) -> Result<(Header, Samples, Stats, Refinement), String> {
    if block_px == 0 || max_px.is_nan() || max_px < 0.0 {
        return Err("block size must be positive and max_px non-negative".into());
    }
    let (plane, tier) = setup(view, p)?;
    let t = Instant::now();
    let reference = compute(view, &plane, tier, p)?;
    let reference_seconds = t.elapsed().as_secs_f64();
    let cols = p.columns.with(Column::Class).with(Column::De).with(Column::Bound);
    let job = Job::new(&reference, &plane, tier, p, cols);
    let (nx, ss, b) = (p.nx as usize, p.ss as usize, block_px as usize);
    let (pw, ph) = (nx / ss, p.ny as usize / ss);
    let (bx, by) = (pw.div_ceil(b), ph.div_ceil(b));
    let mut s = Samples::alloc(nx * p.ny as usize, cols);
    let mut done = vec![false; s.class.len()];
    // A pixel's sparse sample is its sub-sample (ss/2, ss/2).
    let rep = |px: usize, py: usize| (py * ss + ss / 2) * nx + px * ss + ss / 2;
    let pixels = |k: usize| {
        let (x0, y0) = (k % bx * b, k / bx * b);
        (y0..(y0 + b).min(ph)).flat_map(move |py| (x0..(x0 + b).min(pw)).map(move |px| (px, py)))
    };
    let samples = |k: usize| {
        pixels(k).flat_map(move |(px, py)| (0..ss * ss).map(move |u| (py * ss + u / ss) * nx + px * ss + u % ss))
    };
    let blocks = bx * by;
    let mut mask = vec![PREVIEW | SPARSE; blocks];

    // Preview: the sparse sample of each block's centre pixel. Display only.
    let idx: Vec<usize> = (0..blocks).map(|k| rep((k % bx * b + b / 2).min(pw - 1), (k / bx * b + b / 2).min(ph - 1))).collect();
    let preview = run(&job, nx, &mut s, &mut done, &idx, blocks, p.threads);
    // Sparse: the rest of every pixel's sparse sample, then the rule per block with each
    // sample standing for its whole pixel (reach: its farthest pixel corner).
    let idx: Vec<usize> = (0..blocks).flat_map(|k| pixels(k).map(|(x, y)| rep(x, y))).filter(|&k| !done[k]).collect();
    let sparse = run(&job, nx, &mut s, &mut done, &idx, blocks, p.threads);
    let reach = std::f64::consts::SQRT_2 * ((ss / 2) as f64 + 0.5) / ss as f64;
    let mut open = Vec::new();
    for (k, m) in mask.iter_mut().enumerate() {
        if judge_reach(&s, pixels(k).map(|(x, y)| rep(x, y)), reach, max_px).accept {
            *m |= FINAL;
            for (px, py) in pixels(k) {
                fill(&mut s, rep(px, py), (0..ss * ss).map(|u| (py * ss + u / ss) * nx + px * ss + u % ss));
            }
        } else {
            open.push(k);
        }
    }
    // Dense: compact the open blocks and compute everything they still lack.
    let idx: Vec<usize> = open.iter().flat_map(|&k| samples(k)).filter(|&k| !done[k]).collect();
    let dense = run(&job, nx, &mut s, &mut done, &idx, open.len(), p.threads);
    for &k in &open {
        mask[k] |= DENSE | if judge(&s, samples(k), p.ss, max_px).accept { FINAL } else { FALLBACK };
    }

    let iterations = preview.iterations + sparse.iterations + dense.iterations;
    let stats = Stats { reference_len: reference.len(), reference_seconds, iterations };
    let r = Refinement { block_px, blocks_x: bx, mask, preview, sparse, dense };
    Ok((header(view, p, cols, &plane, tier), s, stats, r))
}

/// Compute samples `idx` on `threads` workers and store them.
fn run(job: &Job, nx: usize, s: &mut Samples, done: &mut [bool], idx: &[usize], blocks: usize, threads: usize) -> Phase {
    let queue = Mutex::new(idx.chunks(256));
    let parts: Vec<Vec<(usize, Outcome)>> = std::thread::scope(|sc| {
        let workers: Vec<_> = (0..threads.max(1))
            .map(|_| {
                sc.spawn(|| {
                    let mut v = Vec::new();
                    loop {
                        // let-else drops the lock guard before the chunk is computed.
                        let Some(c) = queue.lock().unwrap().next() else { break };
                        v.extend(c.iter().map(|&k| (k, job.outcome(k % nx, k / nx))));
                    }
                    v
                })
            })
            .collect();
        workers.into_iter().map(|w| w.join().unwrap()).collect()
    });
    let n = s.class.len();
    let mut row = Row::split(s, n).pop().expect("non-empty grid");
    let mut ph = Phase { blocks, ..Phase::default() };
    for (k, o) in parts.into_iter().flatten() {
        ph.samples += 1;
        ph.iterations += o.iterations(job.max_iter);
        job.store.put(&mut row, k, o);
        done[k] = true;
    }
    ph
}

/// Copy sample `from` into the others of `to`, as `Heuristic`: a copy carries no
/// evidence for its own position.
fn fill(s: &mut Samples, from: usize, to: impl Iterator<Item = usize>) {
    let c = s.class[from];
    let copy = c.kind().map_or(c, |kind| Class::new(kind, Evidence::Heuristic));
    for k in to.filter(|&k| k != from) {
        s.class[k] = copy;
        if let Some(v) = s.nu.as_mut() {
            v[k] = v[from];
        }
        if let Some(v) = s.de.as_mut() {
            v[k] = v[from];
        }
        if let Some(v) = s.normal.as_mut() {
            v[k] = v[from];
        }
        if let Some(v) = s.bound.as_mut() {
            v[k] = v[from];
        }
    }
}
