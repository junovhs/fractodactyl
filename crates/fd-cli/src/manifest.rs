//! `fd manifest`: build, show and walk tile and frame manifests (docs/spec/ATLAS.md
//! "Manifests"). Every reference is checked against the store when a manifest is made.
//! Prints `name value` lines.
use crate::args::Args;
use crate::chunk::store;
use crate::render::{columns, size};
use fd_atlas::{is_manifest, walk, Chunk, ChunkId, FrameManifest, Kind, Store, TileManifest};

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let usage = || "usage: fd manifest <tile|frame|show|walk> --store DIR ...".to_string();
    let rest = argv.get(1..).ok_or_else(usage)?;
    match argv.first().map(String::as_str) {
        Some("tile") => {
            let a = Args::parse(rest, &["store", "tile", "evidence", "children", "refs"])?;
            let s = store(&a)?;
            let tile = a.need("tile")?.parse()?;
            let mut children = [None; 4];
            for item in list(a.str("children")) {
                let (q, id) = item.split_once(':').ok_or_else(|| format!("--children: expected Q:ID, got {item:?}"))?;
                let q: u8 = q.parse().ok().filter(|q| *q < 4).ok_or_else(|| format!("--children: bad quadrant {q:?}"))?;
                children[q as usize] = Some(id.parse()?);
            }
            let refs = list(a.str("refs"))
                .map(|r| {
                    let id: ChunkId = r.parse()?;
                    let kind = get(&s, &id)?.contract().kind;
                    if is_manifest(kind) {
                        return Err(format!("--refs: {id} is a {kind}, not a mathematical chunk"));
                    }
                    Ok((kind, id))
                })
                .collect::<Result<Vec<_>, String>>()?;
            let m = TileManifest { tile, evidence: a.str("evidence").unwrap_or("").parse()?, children, refs };
            for (q, id) in m.children.iter().enumerate() {
                if let Some(id) = id {
                    let child = TileManifest::from_chunk(&get(&s, id)?).map_err(|e| format!("child {id}: {e}"))?;
                    if child.tile != m.tile.child(q as u8) {
                        return Err(format!("child {q} of {} must be tile {}, not {}", m.tile, m.tile.child(q as u8), child.tile));
                    }
                }
            }
            put(&s, &m.to_chunk())
        }
        Some("frame") => {
            let flags = ["store", "anchor", "offset", "width", "rotation", "size", "ss", "iter", "columns", "tiles"];
            let a = Args::parse(rest, &flags)?;
            let s = store(&a)?;
            let offset = a.str("offset").unwrap_or("0,0");
            let (u, v) = offset.split_once(',').ok_or_else(|| format!("--offset: expected U,V, got {offset:?}"))?;
            let num = |t: &str| t.parse::<f64>().map_err(|_| format!("--offset: bad number {t:?}"));
            let tiles = list(a.str("tiles")).map(|t| t.parse()).collect::<Result<Vec<ChunkId>, String>>()?;
            for id in &tiles {
                TileManifest::from_chunk(&get(&s, id)?).map_err(|e| format!("--tiles {id}: {e}"))?;
            }
            let m = FrameManifest {
                anchor: a.need("anchor")?.parse()?,
                offset: (num(u)?, num(v)?),
                width: a.num("width", 1.0)?,
                rotation: a.num("rotation", 0.0)?,
                size: size(a.str("size").unwrap_or("640x360"))?,
                ss: a.num("ss", 1)?,
                max_iter: a.num("iter", 100_000)?,
                columns: columns(a.str("columns").unwrap_or("nu,de,normal"))?.0,
                tiles,
            };
            put(&s, &m.to_chunk().map_err(|e| e.to_string())?)
        }
        Some("show") => {
            let a = Args::parse(rest, &["store"])?;
            let s = store(&a)?;
            let [id] = a.positional.as_slice() else {
                return Err("usage: fd manifest show --store DIR <id>".into());
            };
            let id: ChunkId = id.parse()?;
            let chunk = get(&s, &id)?;
            println!("id {id}\nkind {}\nbytes {}", chunk.contract().kind, chunk.bytes().len());
            match chunk.contract().kind {
                Kind::TILE_MANIFEST => {
                    let m = TileManifest::from_chunk(&chunk).map_err(|e| e.to_string())?;
                    println!("tile {}\nevidence {}", m.tile, m.evidence);
                    for (q, c) in m.children.iter().enumerate() {
                        if let Some(c) = c {
                            println!("child {q} {c}");
                        }
                    }
                    println!("refs {}", m.refs.len());
                    for (kind, r) in &m.refs {
                        println!("ref {kind} {r}");
                    }
                }
                Kind::FRAME_MANIFEST => {
                    let m = FrameManifest::from_chunk(&chunk).map_err(|e| e.to_string())?;
                    println!("anchor {}\noffset {},{}\nwidth {}\nrotation {}", m.anchor, m.offset.0, m.offset.1, m.width, m.rotation);
                    println!("size {}x{}\nss {}\niter {}\ncolumns {}", m.size.0, m.size.1, m.ss, m.max_iter, m.columns);
                    println!("tiles {}", m.tiles.len());
                    for t in &m.tiles {
                        println!("tile-ref {t}");
                    }
                }
                k => return Err(format!("{id} is a {k} chunk, not a manifest")),
            }
            Ok(())
        }
        Some("walk") => {
            let a = Args::parse(rest, &["store"])?;
            let s = store(&a)?;
            let frames = if a.positional.is_empty() {
                let mut frames = Vec::new();
                for id in s.ids().map_err(|e| e.to_string())? {
                    if get(&s, &id)?.contract().kind == Kind::FRAME_MANIFEST {
                        frames.push(id);
                    }
                }
                frames
            } else {
                a.positional.iter().map(|p| p.parse()).collect::<Result<Vec<ChunkId>, String>>()?
            };
            let w = walk(&s, &frames).map_err(|e| e.to_string())?;
            let st = s.stats().map_err(|e| e.to_string())?;
            println!("frames {}\ntiles {}\nmath_chunks {}\nmath_bytes {}", w.frames, w.tiles, w.math_chunks, w.math_bytes);
            println!("math_refs {}\nmath_bytes_per_frame {}\nmanifest_bytes {}", w.math_refs, w.math_bytes_per_frame, w.manifest_bytes);
            let sharing = if w.math_bytes == 0 { 0.0 } else { w.math_bytes_per_frame as f64 / w.math_bytes as f64 };
            println!("sharing {sharing:.2}\nstore_chunks {}\nstore_bytes {}", st.chunks, st.bytes);
            Ok(())
        }
        _ => Err(usage()),
    }
}

/// Non-empty items of a comma-separated flag value.
fn list(v: Option<&str>) -> impl Iterator<Item = &str> {
    v.unwrap_or("").split(',').filter(|t| !t.is_empty())
}

fn get(s: &Store, id: &ChunkId) -> Result<Chunk, String> {
    s.get(id).map_err(|e| e.to_string())
}

fn put(s: &Store, chunk: &Chunk) -> Result<(), String> {
    let (id, outcome) = s.put(chunk).map_err(|e| e.to_string())?;
    println!("id {id}\nresult {}\nbytes {}", outcome.name(), chunk.bytes().len());
    Ok(())
}
