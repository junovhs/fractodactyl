//! `fd chunk`: put, read back and inspect chunks in a content-addressed atlas store
//! (docs/spec/ATLAS.md). Prints `name value` lines.
use crate::args::Args;
use fd_atlas::{Chunk, ChunkId, Contract, Store};

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let usage = || "usage: fd chunk <put|get|show|verify|stats> --store DIR ...".to_string();
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
            let (id, outcome) = store(&a)?.put(&chunk).map_err(|e| e.to_string())?;
            println!("id {id}\nresult {}\nbytes {}", outcome.name(), chunk.bytes().len());
            Ok(())
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
            let st = store(&a)?.stats().map_err(|e| e.to_string())?;
            println!("chunks {}\nbytes {}", st.chunks, st.bytes);
            Ok(())
        }
        _ => Err(usage()),
    }
}

pub(crate) fn store(a: &Args) -> Result<Store, String> {
    let dir = a.need("store")?;
    Store::open(dir).map_err(|e| format!("{dir}: {e}"))
}

/// The single positional chunk id.
fn id(a: &Args) -> Result<ChunkId, String> {
    match a.positional.as_slice() {
        [k] => k.parse(),
        _ => Err("expected one chunk id".into()),
    }
}
