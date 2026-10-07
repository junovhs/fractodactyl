//! `fd plan`: known-path footprint planner (docs/spec/PLAN.md). For every frame of a
//! camera path it predicts the logical tiles (ADDRESS.md) the frame's samples fall in and
//! the frame where each tile is first used, then checks every sample of every frame
//! against the prediction using the renderer's own sample geometry.
use crate::args::Args;
use crate::render::size;
use fd_addr::{Nat, Tile, MAX_LEVEL};
use fd_kernel::Plane;
use fd_samples::View;
use std::collections::{BTreeMap, BTreeSet};

/// Sub-tile bits used to place a frame centre inside its tile (error <= 2^-33 tile).
const SUB: u32 = 32;
/// Padding, in tile sides, that the prediction adds to absorb f64 placement error.
const PAD: f64 = 1e-6;

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let a = Args::parse(argv, &["size", "ss", "tile-px"])?;
    let file = match a.positional.as_slice() {
        [f] => f,
        _ => return Err("usage: fd plan PATH --size WxH [--ss N] [--tile-px N]".into()),
    };
    let (w, h) = size(a.need("size")?)?;
    let ss: u32 = a.num("ss", 1)?;
    let tile_px: u32 = a.num("tile-px", 128)?;
    if ss == 0 || tile_px == 0 {
        return Err("--ss and --tile-px must be positive".into());
    }
    let mut uses: BTreeMap<String, (usize, usize, usize)> = BTreeMap::new(); // first, last, frames
    let (mut frames, mut demand, mut samples, mut unpredicted) = (Vec::new(), 0usize, 0u64, 0u64);
    for (n, view) in read_path(file)? {
        let f = frames.len();
        let fp = footprint(&view, w * ss, h * ss, tile_px * ss).map_err(|e| format!("{file}:{n}: {e}"))?;
        samples += fp.samples;
        unpredicted += fp.unpredicted;
        demand += fp.tiles.len();
        let mut new = 0;
        for t in &fp.tiles {
            let u = uses.entry(t.to_string()).or_insert_with(|| {
                new += 1;
                (f, f, 0)
            });
            u.1 = f;
            u.2 += 1;
        }
        frames.push((fp.level, fp.tiles.len(), new));
    }
    println!("frames {}\ntile_px {tile_px}\ntiles {}\ndemand {demand}", frames.len(), uses.len());
    for (f, (level, n, new)) in frames.iter().enumerate() {
        println!("frame {f} level {level} tiles {n} new {new}");
    }
    let mut order: Vec<_> = uses.iter().collect();
    order.sort_by_key(|(k, u)| (u.0, *k));
    for (k, (first, last, n)) in order {
        println!("tile {k} first {first} last {last} frames {n}");
    }
    println!("checked_samples {samples}\nunpredicted {unpredicted}");
    if unpredicted > 0 {
        return Err(format!("{unpredicted} samples fall in tiles the plan did not predict"));
    }
    Ok(())
}

/// The frames of a camera path file (docs/spec/PLAN.md "Path file") with their 1-based
/// line numbers; an error names the file and line. A path with no frame is an error.
pub(crate) fn read_path(file: &str) -> Result<Vec<(usize, View)>, String> {
    let text = std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
    let mut frames = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if !line.is_empty() {
            frames.push((n + 1, view(line).map_err(|e| format!("{file}:{}: {e}", n + 1))?));
        }
    }
    if frames.is_empty() {
        return Err(format!("{file}: no frames"));
    }
    Ok(frames)
}

/// One path line: `re im width [rotation]`.
fn view(line: &str) -> Result<View, String> {
    let f: Vec<&str> = line.split_whitespace().collect();
    let rotation = match f.len() {
        3 => 0.0,
        4 => f[3].parse().map_err(|_| format!("bad rotation {:?}", f[3]))?,
        _ => return Err("expected `re im width [rotation]`".into()),
    };
    Ok(View { center_re: f[0].into(), center_im: f[1].into(), width: f[2].into(), rotation })
}

/// A frame's predicted tiles and the result of checking every sample against them.
pub(crate) struct Footprint {
    /// Tile level `L` of the frame.
    pub(crate) level: u32,
    /// Level-`L` tiles the frame's samples fall in, sorted.
    pub(crate) tiles: Vec<Tile>,
    /// The level-`L` tile holding the frame centre (the frame manifest's anchor).
    pub(crate) anchor: Tile,
    /// The centre's position inside `anchor`, in anchor sides from its top-left corner
    /// (x right, y down), to within 2^-33.
    pub(crate) at: (f64, f64),
    pub(crate) samples: u64,
    pub(crate) unpredicted: u64,
}

/// Tiles of side at most `tile_samples` sample spacings that the `nx x ny` grid of `v`
/// touches. Prediction intersects each candidate tile with the frame's sample rectangle
/// (separating axes, padded by `PAD`); the check then locates every sample.
pub(crate) fn footprint(v: &View, nx: u32, ny: u32, tile_samples: u32) -> Result<Footprint, String> {
    let p = Plane::new(v, nx, ny)?;
    // Coarsest level whose tile side 2^(2-L) is at most tile_samples * h.
    let lg = p.h_m.log2() + p.h_e as f64 + (tile_samples as f64).log2();
    let level = (2.0 - lg).ceil().max(0.0) as u32;
    if level + SUB > MAX_LEVEL {
        return Err(format!("tile level {level} exceeds {}", MAX_LEVEL - SUB));
    }
    // Centre tile and the centre's position inside it, from an exact deeper locate.
    let fine = Tile::locate(&v.center_re, &v.center_im, level + SUB)?;
    let (mid, i0, j0) = fine.owner(16)?;
    let (centre, i1, j1) = mid.owner(16)?;
    let frac = |hi: u32, lo: u32| ((((hi as u64) << 16) | lo as u64) as f64 + 0.5) / 2f64.powi(SUB as i32);
    let (cx, cy) = (frac(i1, i0), frac(j1, j0));
    // Sample spacing in tile sides: h / 2^(2-L), exact power-of-two scaling.
    let r = p.h_m * 2f64.powi((p.h_e + level as i64 - 2) as i32);
    let at = |i: usize, j: usize| {
        let (re, im) = p.unit_offset(i, j);
        (cx + r * re, cy - r * im) // tile y grows downward
    };
    let (mx, my) = (nx as usize - 1, ny as usize - 1);
    let rect = [at(0, 0), at(mx, 0), at(mx, my), at(0, my)];
    let lo = |f: fn(&(f64, f64)) -> f64| rect.iter().map(f).fold(f64::INFINITY, f64::min);
    let hi = |f: fn(&(f64, f64)) -> f64| rect.iter().map(f).fold(f64::NEG_INFINITY, f64::max);
    let (x0, x1) = ((lo(|q| q.0) - PAD).floor() as i64, (hi(|q| q.0) + PAD).floor() as i64);
    let (y0, y1) = ((lo(|q| q.1) - PAD).floor() as i64, (hi(|q| q.1) + PAD).floor() as i64);
    let mut predicted = BTreeSet::new();
    for dy in y0..=y1 {
        for dx in x0..=x1 {
            let (a, b) = (dx as f64, dy as f64);
            if overlaps(&rect, &[(a, b), (a + 1.0, b), (a + 1.0, b + 1.0), (a, b + 1.0)]) {
                predicted.insert((dx, dy));
            }
        }
    }
    // Check: every sample's containing tile must have been predicted.
    let mut unpredicted = 0;
    for j in 0..ny as usize {
        for i in 0..nx as usize {
            let (x, y) = at(i, j);
            if !predicted.contains(&(x.floor() as i64, y.floor() as i64)) {
                unpredicted += 1;
            }
        }
    }
    let tiles = predicted.iter().filter_map(|&(dx, dy)| neighbour(&centre, dx, dy)).collect();
    Ok(Footprint { level, tiles, anchor: centre, at: (cx, cy), samples: nx as u64 * ny as u64, unpredicted })
}

/// Separating-axis test for two convex quadrilaterals, padded by `PAD`.
fn overlaps(a: &[(f64, f64); 4], b: &[(f64, f64); 4]) -> bool {
    let axes = (0..4).flat_map(|k| [(a, k), (b, k)]).map(|(q, k)| {
        let (p, n) = (q[k], q[(k + 1) % 4]);
        (p.1 - n.1, n.0 - p.0)
    });
    let span = |q: &[(f64, f64); 4], ax: (f64, f64)| {
        q.iter().map(|p| p.0 * ax.0 + p.1 * ax.1).fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), v| (l.min(v), h.max(v)))
    };
    axes.filter(|ax| ax.0 != 0.0 || ax.1 != 0.0).all(|ax| {
        let pad = PAD * (ax.0.abs() + ax.1.abs());
        let ((al, ah), (bl, bh)) = (span(a, ax), span(b, ax));
        al <= bh + pad && bl <= ah + pad
    })
}

/// The tile `(dx, dy)` steps from `t` at the same level; `None` outside the root square.
fn neighbour(t: &Tile, dx: i64, dy: i64) -> Option<Tile> {
    let step = |v: &Nat, d: i64| {
        let m = Nat::small(d.unsigned_abs());
        if d >= 0 {
            Some(v.plus(&m))
        } else {
            v.minus(&m)
        }
    };
    Tile::new(t.level, step(&t.x, dx)?, step(&t.y, dy)?).ok()
}
