//! `fd orbit put`: compute a view's reference orbit, store it as fixed-size orbit slab
//! chunks plus an orbit manifest binding them to the exact centre (docs/spec/ATLAS.md
//! "Orbit slabs"); `fd render --orbit` loads it back.
use crate::args::Args;
use crate::chunk::{report_target, store};
use crate::render::{job, FLAGS};
use fd_atlas::{ChunkId, OrbitManifest, OrbitSlab, Store};
use fd_fixed::Decimal;
use fd_kernel::Reference;
use fd_samples::View;

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let (Some("put"), Some(rest)) = (argv.first().map(String::as_str), argv.get(1..)) else {
        return Err("usage: fd orbit put --store DIR <render view flags> [--slab N]".into());
    };
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

/// Store `r` (computed at `view`'s centre with `bits`) as slabs of `size` points plus
/// its orbit manifest; returns the manifest id. `verbose` prints the slab report.
pub(crate) fn put(s: &Store, view: &View, r: &Reference, bits: u32, size: u32, verbose: bool) -> Result<ChunkId, String> {
    let slabs = OrbitSlab::split(&r.re, &r.im, size).map_err(|e| e.to_string())?;
    let (mut ids, mut total) = (Vec::new(), 0);
    for sl in &slabs {
        let chunk = sl.to_chunk(bits).map_err(|e| e.to_string())?;
        let (id, outcome) = s.put(&chunk).map_err(|e| e.to_string())?;
        if verbose {
            println!("slab {} {} {id} {} {}", sl.start, sl.re.len(), outcome.name(), chunk.bytes().len());
        }
        total += chunk.bytes().len();
        ids.push(id);
    }
    let m = OrbitManifest { center_re: view.center_re.clone(), center_im: view.center_im.clone(), precision_bits: bits, slabs: ids };
    let (id, _) = s.put(&m.to_chunk().map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    if verbose {
        println!("slabs {}\nslab_bytes {total}\nbytes_per_iter {:.3}", slabs.len(), total as f64 / r.len() as f64);
        println!("orbit {id}");
    }
    Ok(id)
}

/// Load the orbit named by its orbit manifest id, with its precision bits. Refuses an
/// orbit computed at any centre other than `view`'s (compared as exact decimals).
pub(crate) fn load(s: &Store, id: &str, view: &View) -> Result<(Reference, u32), String> {
    let id: ChunkId = id.parse()?;
    let m = OrbitManifest::from_chunk(&s.get(&id).map_err(|e| e.to_string())?).map_err(|e| format!("{id}: {e}"))?;
    if !same(&m.center_re, &view.center_re)? || !same(&m.center_im, &view.center_im)? {
        return Err(format!("orbit {id} is for centre {} {}, not this view's centre", m.center_re, m.center_im));
    }
    let mut slabs = Vec::new();
    for sid in &m.slabs {
        let chunk = s.get(sid).map_err(|e| e.to_string())?;
        if chunk.contract().precision_bits != m.precision_bits {
            return Err(format!("orbit slab {sid} disagrees with its manifest on precision"));
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
