//! `fd orbit put`: compute a view's reference orbit and store it as fixed-size orbit
//! slab chunks (docs/spec/ATLAS.md "Orbit slabs"); `fd render --orbit` loads it back.
use crate::args::Args;
use crate::chunk::{report_target, store};
use crate::render::{job, FLAGS};
use fd_atlas::{ChunkId, OrbitSlab, Store};
use fd_kernel::Reference;

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
    let slabs = OrbitSlab::split(&r.re, &r.im, size).map_err(|e| e.to_string())?;
    println!("points {}\nprecision {bits}\nslab_size {size}\nslabs {}", r.len(), slabs.len());
    let (mut ids, mut total) = (Vec::new(), 0);
    for sl in &slabs {
        let chunk = sl.to_chunk(bits).map_err(|e| e.to_string())?;
        let (id, outcome) = s.put(&chunk).map_err(|e| e.to_string())?;
        println!("slab {} {} {id} {} {}", sl.start, sl.re.len(), outcome.name(), chunk.bytes().len());
        total += chunk.bytes().len();
        ids.push(id.to_string());
    }
    println!("slab_bytes {total}\nbytes_per_iter {:.3}", total as f64 / r.len() as f64);
    println!("orbit {}", ids.join(","));
    report_target(&s)
}

/// Load the orbit named by comma-separated slab ids, with its precision bits.
pub(crate) fn load(s: &Store, ids: &str) -> Result<(Reference, u32), String> {
    let mut slabs = Vec::new();
    let mut bits = None;
    for id in ids.split(',') {
        let chunk = s.get(&id.parse::<ChunkId>()?).map_err(|e| e.to_string())?;
        let b = chunk.contract().precision_bits;
        if *bits.get_or_insert(b) != b {
            return Err("orbit slabs disagree on precision".into());
        }
        slabs.push(OrbitSlab::from_chunk(&chunk).map_err(|e| format!("{id}: {e}"))?);
    }
    let (re, im) = OrbitSlab::join(&slabs).map_err(|e| e.to_string())?;
    Ok((Reference { re, im }, bits.unwrap_or(0)))
}
