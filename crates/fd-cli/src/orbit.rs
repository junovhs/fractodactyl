//! `fd orbit put`: compute a view's reference orbit, store it as fixed-size orbit slab
//! chunks plus an orbit manifest binding them to the exact centre (docs/spec/ATLAS.md
//! "Orbit slabs"); `fd render --orbit` loads it back. `fd orbit bla` builds a BLA table
//! over a stored orbit and stores it (ATLAS.md "BLA tables"); `fd render --bla` uses it.
use crate::args::Args;
use crate::chunk::{report_target, store};
use crate::render::{job, FLAGS};
use fd_atlas::{BlaTable, ChunkId, OrbitManifest, OrbitSlab, Store};
use fd_fixed::Decimal;
use fd_kernel::{Bla, Block, Reference};
use fd_samples::View;
use std::time::Instant;

/// Default BLA tolerance: each dropped quadratic term is at most 2^-50 of the kept term,
/// a few f64 roundings, so BLA adds error of the order the plain kernel's own rounding.
pub(crate) const EPS: f64 = 1.0 / 1125899906842624.0;

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let rest = argv.get(1..).unwrap_or_default();
    match argv.first().map(String::as_str) {
        Some("put") => run_put(rest),
        Some("bla") => run_bla(rest),
        _ => Err(
            "usage: fd orbit put --store DIR <render view flags> [--slab N]
       fd orbit bla --store DIR <render view flags> [--orbit ID] [--eps E] [--slab N]"
                .into(),
        ),
    }
}

fn run_put(rest: &[String]) -> Result<(), String> {
    let known: Vec<&str> = FLAGS.iter().copied().chain(["store", "slab"]).collect();
    let a = Args::parse(rest, &known)?;
    let (view, p) = job(&a)?;
    let size: u32 = a.num("slab", 4096)?;
    let s = store(&a)?;
    let (r, bits) = fd_kernel::reference(&view, &p)?;
    println!("points {}\nprecision {bits}\nslab_size {size}", r.len());
    put(&s, &view, &r, bits, size, true)?;
    report_target(&s)
}

/// `fd orbit bla`: build the BLA table of a stored orbit (`--orbit`, else computed and
/// stored now) for the view's largest `|dc|`, and store it as one `bla` chunk.
fn run_bla(rest: &[String]) -> Result<(), String> {
    let known: Vec<&str> = FLAGS
        .iter()
        .copied()
        .chain(["store", "slab", "orbit", "eps"])
        .collect();
    let a = Args::parse(rest, &known)?;
    let (view, p) = job(&a)?;
    let s = store(&a)?;
    let eps: f64 = a.num("eps", EPS)?;
    let dc_max = fd_kernel::bla_dc_max(&view, &p)?;
    let (r, bits, orbit) = match a.str("orbit") {
        Some(id) => {
            let (r, bits) = load(&s, id, &view)?;
            (r, bits, id.parse::<ChunkId>()?)
        }
        None => {
            let (r, bits) = fd_kernel::reference(&view, &p)?;
            let id = put(&s, &view, &r, bits, a.num("slab", 4096)?, false)?;
            (r, bits, id)
        }
    };
    let t = Instant::now();
    let bla = Bla::build(&r, eps, dc_max)?;
    let secs = t.elapsed().as_secs_f64();
    let levels = bla
        .levels
        .iter()
        .map(|l| l.iter().map(Block::to_array).collect())
        .collect();
    let table = BlaTable {
        orbit,
        eps,
        dc_max,
        points: bla.points,
        levels,
    };
    let chunk = table.to_chunk().map_err(|e| e.to_string())?;
    let (id, outcome) = s.put(&chunk).map_err(|e| e.to_string())?;
    let (blocks, valid) = bla.counts();
    let (bytes, orbit_bytes) = (chunk.bytes().len(), slab_bytes(&s, &orbit)?);
    println!("points {}", r.len());
    println!("precision {bits}");
    println!("eps {eps:e}");
    println!("dc_max {dc_max:e}");
    println!("levels {}", bla.levels.len());
    println!("blocks {blocks}");
    println!("valid_blocks {valid}");
    println!("build_seconds {secs:.6}");
    println!("bytes {bytes}");
    println!("bytes_per_iter {:.3}", bytes as f64 / r.len() as f64);
    println!("orbit_slab_bytes {orbit_bytes}");
    println!("bytes_over_orbit {:.3}", bytes as f64 / orbit_bytes as f64);
    println!("result {}", outcome.name());
    println!("orbit {orbit}");
    println!("bla {id}");
    report_target(&s)
}

/// Stored bytes of the slab chunks an orbit manifest names.
fn slab_bytes(s: &Store, orbit: &ChunkId) -> Result<usize, String> {
    let m = OrbitManifest::from_chunk(&s.get(orbit).map_err(|e| e.to_string())?)
        .map_err(|e| format!("{orbit}: {e}"))?;
    m.slabs
        .iter()
        .map(|id| {
            s.get(id)
                .map(|c| c.bytes().len())
                .map_err(|e| e.to_string())
        })
        .sum()
}

/// Load the BLA table `id` and the orbit it was built over (centre-checked against
/// `view` like [`load`]), with the orbit manifest's id.
pub(crate) fn load_bla(
    s: &Store,
    id: &str,
    view: &View,
) -> Result<(Reference, u32, Bla, ChunkId), String> {
    let id: ChunkId = id.parse()?;
    let t = BlaTable::from_chunk(&s.get(&id).map_err(|e| e.to_string())?)
        .map_err(|e| format!("{id}: {e}"))?;
    let (r, bits) = load(s, &t.orbit.to_string(), view)?;
    let levels = t
        .levels
        .into_iter()
        .map(|l| l.into_iter().map(Block::from_array).collect())
        .collect();
    let bla = Bla::new(t.eps, t.dc_max, t.points, levels).map_err(|e| format!("{id}: {e}"))?;
    Ok((r, bits, bla, t.orbit))
}

/// Store `r` (computed at `view`'s centre with `bits`) as slabs of `size` points plus
/// its orbit manifest; returns the manifest id. `verbose` prints the slab report.
pub(crate) fn put(
    s: &Store,
    view: &View,
    r: &Reference,
    bits: u32,
    size: u32,
    verbose: bool,
) -> Result<ChunkId, String> {
    put_sized(s, view, r, bits, size, verbose).map(|(id, _)| id)
}

/// [`put`], also returning the encoded bytes of the slabs plus the manifest (stored now
/// or already present).
pub(crate) fn put_sized(
    s: &Store,
    view: &View,
    r: &Reference,
    bits: u32,
    size: u32,
    verbose: bool,
) -> Result<(ChunkId, usize), String> {
    let slabs = OrbitSlab::split(&r.re, &r.im, size).map_err(|e| e.to_string())?;
    let (mut ids, mut total) = (Vec::new(), 0);
    for sl in &slabs {
        let chunk = sl.to_chunk(bits).map_err(|e| e.to_string())?;
        let (id, outcome) = s.put(&chunk).map_err(|e| e.to_string())?;
        if verbose {
            println!(
                "slab {} {} {id} {} {}",
                sl.start,
                sl.re.len(),
                outcome.name(),
                chunk.bytes().len()
            );
        }
        total += chunk.bytes().len();
        ids.push(id);
    }
    let m = OrbitManifest {
        center_re: view.center_re.clone(),
        center_im: view.center_im.clone(),
        precision_bits: bits,
        slabs: ids,
    };
    let manifest = m.to_chunk().map_err(|e| e.to_string())?;
    let (id, _) = s.put(&manifest).map_err(|e| e.to_string())?;
    if verbose {
        println!(
            "slabs {}\nslab_bytes {total}\nbytes_per_iter {:.3}",
            slabs.len(),
            total as f64 / r.len() as f64
        );
        println!("orbit {id}");
    }
    Ok((id, total + manifest.bytes().len()))
}

/// Load the orbit named by its orbit manifest id, with its precision bits. Refuses an
/// orbit computed at any centre other than `view`'s (compared as exact decimals).
pub(crate) fn load(s: &Store, id: &str, view: &View) -> Result<(Reference, u32), String> {
    let id: ChunkId = id.parse()?;
    let m = OrbitManifest::from_chunk(&s.get(&id).map_err(|e| e.to_string())?)
        .map_err(|e| format!("{id}: {e}"))?;
    if !same(&m.center_re, &view.center_re)? || !same(&m.center_im, &view.center_im)? {
        return Err(format!(
            "orbit {id} is for centre {} {}, not this view's centre",
            m.center_re, m.center_im
        ));
    }
    let mut slabs = Vec::new();
    for sid in &m.slabs {
        let chunk = s.get(sid).map_err(|e| e.to_string())?;
        if chunk.contract().precision_bits != m.precision_bits {
            return Err(format!(
                "orbit slab {sid} disagrees with its manifest on precision"
            ));
        }
        slabs.push(OrbitSlab::from_chunk(&chunk).map_err(|e| format!("{sid}: {e}"))?);
    }
    let (re, im) = OrbitSlab::join(&slabs).map_err(|e| e.to_string())?;
    Ok((Reference { re, im }, m.precision_bits))
}

/// Whether two decimal strings name the same number.
pub(crate) fn same(a: &str, b: &str) -> Result<bool, String> {
    Ok(canonical(a)? == canonical(b)?)
}

/// Exact decimal without leading or trailing zero digits (`1.50` and `15e-1` agree).
fn canonical(s: &str) -> Result<Decimal, String> {
    let mut d = Decimal::parse(s)?;
    let lead = d.digits.iter().take_while(|&&x| x == 0).count();
    d.digits.drain(..lead);
    while d.digits.last() == Some(&0) {
        d.digits.pop();
        d.exp10 += 1;
    }
    if d.digits.is_empty() {
        d.exp10 = 0;
    }
    Ok(d)
}
