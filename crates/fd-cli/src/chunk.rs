//! `fd chunk`: put, read back and inspect chunks in a content-addressed atlas store
//! (docs/spec/ATLAS.md). Prints `name value` lines.
use crate::args::Args;
use fd_atlas::{Budget, Chunk, ChunkId, Contract, Kind, Store};

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let usage = || "usage: fd chunk <put|get|show|verify|stats|budget> --store DIR ...".to_string();
    let rest = argv.get(1..).ok_or_else(usage)?;
    match argv.first().map(String::as_str) {
        Some("put") => {
            let a = Args::parse(rest, &["store", "kind", "encoding", "formula", "precision", "rounding"])?;
            let [file] = a.positional.as_slice() else {
                return Err("usage: fd chunk put --store DIR --kind K [contract flags] <file>".into());
            };
            let contract = Contract {
                kind: a.need("kind")?.parse()?,
                encoding: a.num("encoding", 1)?,
                formula: a.str("formula").unwrap_or("mandelbrot").parse()?,
                precision_bits: a.num("precision", 53)?,
                rounding: a.str("rounding").unwrap_or("nearest").parse()?,
            };
            let payload = std::fs::read(file).map_err(|e| format!("{file}: {e}"))?;
            let chunk = Chunk::new(contract, &payload);
            let s = store(&a)?;
            let (id, outcome) = s.put(&chunk).map_err(|e| e.to_string())?;
            println!("id {id}\nresult {}\nbytes {}", outcome.name(), chunk.bytes().len());
            report_target(&s)
        }
        Some("get") => {
            let a = Args::parse(rest, &["store", "o"])?;
            let out = a.need("o")?;
            let chunk = store(&a)?.get(&id(&a)?).map_err(|e| e.to_string())?;
            std::fs::write(out, chunk.payload()).map_err(|e| format!("{out}: {e}"))?;
            println!("payload {}", chunk.payload().len());
            Ok(())
        }
        Some("show") => {
            let a = Args::parse(rest, &["store"])?;
            let (s, id) = (store(&a)?, id(&a)?);
            let chunk = s.get(&id).map_err(|e| e.to_string())?;
            let c = chunk.contract();
            println!("id {id}\nformat 1.{}\nkind {}\nencoding {}", chunk.minor(), c.kind, c.encoding);
            println!("formula {}\nprecision {}\nrounding {}", c.formula, c.precision_bits, c.rounding);
            println!("payload {}\nbytes {}\npath {}", chunk.payload().len(), chunk.bytes().len(), s.path(&id).display());
            Ok(())
        }
        Some("verify") => {
            let a = Args::parse(rest, &["store"])?;
            let s = store(&a)?;
            let checked = s.ids().map_err(|e| e.to_string())?.len();
            let bad = s.verify().map_err(|e| e.to_string())?;
            println!("checked {checked}\nbad {}", bad.len());
            for (id, e) in &bad {
                println!("corrupt {id} {e}");
            }
            if bad.is_empty() {
                Ok(())
            } else {
                Err(format!("{} of {checked} chunks failed verification", bad.len()))
            }
        }
        Some("stats") => {
            let a = Args::parse(rest, &["store"])?;
            let s = store(&a)?;
            let st = s.stats().map_err(|e| e.to_string())?;
            let b = s.budget();
            println!("chunks {}\nbytes {}\ntarget {}\ncap {}", st.chunks, st.bytes, b.target, b.cap);
            println!("headroom {}\nover_target {}", b.cap.saturating_sub(st.bytes), u8::from(st.bytes > b.target));
            let mut classes = [("operators", 0u64), ("evidence", 0), ("manifests", 0), ("other", 0)];
            for (kind, k) in s.stats_by_kind().map_err(|e| e.to_string())? {
                println!("kind.{kind} {} {}", k.chunks, k.bytes);
                classes[class(kind)].1 += k.bytes;
            }
            for (name, bytes) in classes {
                println!("class.{name} {bytes}");
            }
            Ok(())
        }
        Some("budget") => {
            let a = Args::parse(rest, &["store", "target", "cap"])?;
            let mut s = store(&a)?;
            if a.str("target").is_some() || a.str("cap").is_some() {
                let old = s.budget();
                let byte_flag = |k: &str, default: u64| a.str(k).map_or(Ok(default), |v| bytes(k, v));
                let new = Budget { target: byte_flag("target", old.target)?, cap: byte_flag("cap", old.cap)? };
                s.set_budget(new).map_err(|e| e.to_string())?;
            }
            println!("target {}\ncap {}", s.budget().target, s.budget().cap);
            Ok(())
        }
        _ => Err(usage()),
    }
}

pub(crate) fn store(a: &Args) -> Result<Store, String> {
    let dir = a.need("store")?;
    Store::open(dir).map_err(|e| format!("{dir}: {e}"))
}

/// After a successful put: print stored bytes and warn (without failing) past the target.
pub(crate) fn report_target(s: &Store) -> Result<(), String> {
    let used = s.stats().map_err(|e| e.to_string())?.bytes;
    let b = s.budget();
    println!("atlas_bytes {used}\nover_target {}", u8::from(used > b.target));
    if used > b.target {
        eprintln!("warning: atlas holds {used} bytes, over its {} byte target (cap {})", b.target, b.cap);
    }
    Ok(())
}

/// Budget allocation class of a kind (docs/spec/ATLAS.md "Byte budget"): index into
/// operators, evidence, manifests, other.
fn class(kind: Kind) -> usize {
    match kind {
        Kind::ORBIT_SLAB | Kind::BLA | Kind::RETURN_MAP => 0,
        Kind::CERTIFICATE | Kind::EXACT_SAMPLE | Kind::SAMPLES => 1,
        Kind::TILE_MANIFEST | Kind::FRAME_MANIFEST | Kind::ORBIT_MANIFEST => 2,
        _ => 3,
    }
}

/// A byte count: an integer with an optional KiB, MiB or GiB suffix.
fn bytes(flag: &str, v: &str) -> Result<u64, String> {
    let (num, unit) = [("GiB", 1u64 << 30), ("MiB", 1 << 20), ("KiB", 1 << 10)]
        .iter()
        .find_map(|&(sfx, unit)| v.strip_suffix(sfx).map(|n| (n, unit)))
        .unwrap_or((v, 1));
    num.parse::<u64>().ok().and_then(|n| n.checked_mul(unit)).ok_or_else(|| format!("--{flag}: bad byte count {v:?}"))
}

/// The single positional chunk id.
fn id(a: &Args) -> Result<ChunkId, String> {
    match a.positional.as_slice() {
        [k] => k.parse(),
        _ => Err("expected one chunk id".into()),
    }
}
