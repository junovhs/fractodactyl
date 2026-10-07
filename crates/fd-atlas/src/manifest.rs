//! Tile and frame manifests (docs/spec/ATLAS.md "Manifests"): small chunks that name
//! other chunks by id, so the atlas is a Merkle DAG. A frame manifest names tile
//! manifests; a tile manifest names its children and the mathematical chunks of its
//! tile. Thousands of frames can share one orbit slab by naming the same id: chunk
//! bytes are stored once, only the 32-byte references repeat.
//!
//! Decoding re-encodes and compares, so a manifest only exists in canonical form.
use crate::{Builder, Chunk, ChunkId, Contract, Error, Formula, Kind, Rounding, Store};
use fd_addr::Tile;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Payload layout version of both manifest kinds.
pub const MANIFEST_ENCODING: u16 = 1;

/// Which evidence levels a tile's chunks provide (bit set; matches the sample classes
/// of SAMPLES.md).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Evidence(pub u8);

impl Evidence {
    /// Unproved samples are available.
    pub const HEURISTIC: Evidence = Evidence(1);
    /// Samples with error bounds are available.
    pub const BOUNDED: Evidence = Evidence(2);
    /// Certificates are available.
    pub const CERTIFIED: Evidence = Evidence(4);
    const NAMES: [(Evidence, &'static str); 3] =
        [(Evidence::HEURISTIC, "heuristic"), (Evidence::BOUNDED, "bounded"), (Evidence::CERTIFIED, "certified")];
    const KNOWN: u8 = 7;
}

impl std::str::FromStr for Evidence {
    type Err = String;
    /// Comma-separated names, e.g. `heuristic,certified`; empty or `none` for none.
    fn from_str(s: &str) -> Result<Evidence, String> {
        s.split(',').filter(|n| !n.is_empty() && *n != "none").try_fold(Evidence(0), |acc, n| {
            let (e, _) = Evidence::NAMES.iter().find(|(_, name)| *name == n).ok_or_else(|| format!("unknown evidence {n:?}"))?;
            Ok(Evidence(acc.0 | e.0))
        })
    }
}

impl fmt::Display for Evidence {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let names: Vec<&str> = Evidence::NAMES.iter().filter(|(e, _)| self.0 & e.0 != 0).map(|(_, n)| *n).collect();
        f.write_str(if names.is_empty() { "none".to_string() } else { names.join(",") }.as_str())
    }
}

/// One logical tile: its exact address, child tile manifests by quadrant, and the
/// chunks that hold its mathematics. `refs` is kept sorted by (kind, id) without
/// duplicates; [`TileManifest::to_chunk`] normalises it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TileManifest {
    /// Exact quadtree address (docs/spec/ADDRESS.md).
    pub tile: Tile,
    /// Evidence levels available for this tile.
    pub evidence: Evidence,
    /// Child tile manifest for quadrant `q` (fd-addr `Tile::child(q)`), if built.
    pub children: [Option<ChunkId>; 4],
    /// Referenced chunks with their kinds.
    pub refs: Vec<(Kind, ChunkId)>,
}

impl TileManifest {
    /// Contract of every v1 tile manifest: exact payload, no working precision.
    pub const CONTRACT: Contract = Contract {
        kind: Kind::TILE_MANIFEST,
        encoding: MANIFEST_ENCODING,
        formula: Formula::MANDELBROT,
        precision_bits: 0,
        rounding: Rounding::Exact,
    };

    /// Canonical chunk: refs sorted and deduplicated.
    pub fn to_chunk(&self) -> Chunk {
        let mut refs = self.refs.clone();
        refs.sort();
        refs.dedup();
        let mask = (0..4).filter(|&q| self.children[q].is_some()).fold(0u8, |m, q| m | (1 << q));
        let mut b = Builder::new(Self::CONTRACT);
        b.str(&self.tile.to_string()).u8(self.evidence.0).u8(mask);
        for id in self.children.iter().flatten() {
            b.raw(&id.0);
        }
        b.u64(refs.len() as u64);
        for (kind, id) in &refs {
            b.u16(kind.0).raw(&id.0);
        }
        b.finish()
    }

    /// Decode a tile manifest chunk, refusing anything not in canonical form.
    pub fn from_chunk(chunk: &Chunk) -> Result<TileManifest, Error> {
        expect(chunk, Self::CONTRACT)?;
        let mut r = Reader(chunk.payload());
        let tile: Tile = r.str()?.parse().map_err(Error::Malformed)?;
        let evidence = Evidence(r.u8()?);
        let mask = r.u8()?;
        if evidence.0 & !Evidence::KNOWN != 0 || mask >= 16 {
            return Err(malformed("unknown evidence or child bits"));
        }
        let mut children = [None; 4];
        for (q, child) in children.iter_mut().enumerate() {
            if mask & (1 << q) != 0 {
                *child = Some(r.id()?);
            }
        }
        let n = r.u64()?;
        let refs = (0..n).map(|_| Ok((Kind(r.u16()?), r.id()?))).collect::<Result<Vec<_>, Error>>()?;
        let m = TileManifest { tile, evidence, children, refs };
        canonical(chunk, &m.to_chunk(), r)?;
        Ok(m)
    }
}

/// One video frame: where the camera is, relative to an exact anchor tile (DEC-08
/// local coordinates), how to sample, and which tile manifests it needs. Holds no
/// mathematics itself. `tiles` is kept sorted without duplicates.
#[derive(Clone, Debug, PartialEq)]
pub struct FrameManifest {
    /// The chart the camera is expressed in.
    pub anchor: Tile,
    /// View centre minus anchor centre, in units of the anchor's side (re, im; im up).
    pub offset: (f64, f64),
    /// View width in units of the anchor's side.
    pub width: f64,
    /// View rotation in radians, as `fd render --rotation`.
    pub rotation: f64,
    /// Output size in pixels.
    pub size: (u32, u32),
    /// Supersampling factor per axis.
    pub ss: u32,
    /// Iteration limit.
    pub max_iter: u64,
    /// Sample column mask (SAMPLES.md `ColumnSet` bits).
    pub columns: u32,
    /// Tile manifests this frame reads.
    pub tiles: Vec<ChunkId>,
}

impl FrameManifest {
    /// Contract of every v1 frame manifest: f64 local camera, round to nearest.
    pub const CONTRACT: Contract = Contract {
        kind: Kind::FRAME_MANIFEST,
        encoding: MANIFEST_ENCODING,
        formula: Formula::MANDELBROT,
        precision_bits: 53,
        rounding: Rounding::Nearest,
    };

    /// Canonical chunk: tiles sorted and deduplicated. Refuses non-finite or
    /// non-positive camera values and empty sizes.
    pub fn to_chunk(&self) -> Result<Chunk, Error> {
        let finite = [self.offset.0, self.offset.1, self.width, self.rotation].iter().all(|v| v.is_finite());
        if !finite || self.width <= 0.0 || self.size.0 == 0 || self.size.1 == 0 || self.ss == 0 {
            return Err(malformed("frame camera must be finite with positive width, size and ss"));
        }
        let mut tiles = self.tiles.clone();
        tiles.sort();
        tiles.dedup();
        let mut b = Builder::new(Self::CONTRACT);
        b.str(&self.anchor.to_string()).f64(self.offset.0).f64(self.offset.1).f64(self.width).f64(self.rotation);
        b.u32(self.size.0).u32(self.size.1).u32(self.ss).u64(self.max_iter).u32(self.columns);
        b.u64(tiles.len() as u64);
        for id in &tiles {
            b.raw(&id.0);
        }
        Ok(b.finish())
    }

    /// Decode a frame manifest chunk, refusing anything not in canonical form.
    pub fn from_chunk(chunk: &Chunk) -> Result<FrameManifest, Error> {
        expect(chunk, Self::CONTRACT)?;
        let mut r = Reader(chunk.payload());
        let anchor: Tile = r.str()?.parse().map_err(Error::Malformed)?;
        let offset = (r.f64()?, r.f64()?);
        let (width, rotation) = (r.f64()?, r.f64()?);
        let size = (r.u32()?, r.u32()?);
        let (ss, max_iter, columns) = (r.u32()?, r.u64()?, r.u32()?);
        let n = r.u64()?;
        let tiles = (0..n).map(|_| r.id()).collect::<Result<Vec<_>, Error>>()?;
        let m = FrameManifest { anchor, offset, width, rotation, size, ss, max_iter, columns, tiles };
        canonical(chunk, &m.to_chunk()?, r)?;
        Ok(m)
    }
}

/// What walking a set of frame manifests found: how much the frames share.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Walk {
    /// Frame manifests walked.
    pub frames: u64,
    /// Distinct tile manifests reached.
    pub tiles: u64,
    /// Distinct non-manifest chunks reached.
    pub math_chunks: u64,
    /// Bytes of those distinct chunks: what the store holds for them.
    pub math_bytes: u64,
    /// Per-frame sum of reachable math chunks (one frame's sharing counts once).
    pub math_refs: u64,
    /// Bytes if every frame owned a private copy of everything it reaches.
    pub math_bytes_per_frame: u64,
    /// Bytes of all distinct manifests reached (frames and tiles).
    pub manifest_bytes: u64,
}

/// Walk the DAG below `frames`, verifying every chunk read (hash and canonical form),
/// that references have the right kind, and that each child tile manifest is for the
/// matching child address. Fails on the first missing, corrupt or inconsistent chunk.
pub fn walk(store: &Store, frames: &[ChunkId]) -> Result<Walk, Error> {
    let mut w = Walk::default();
    // Tile id -> its tile and the distinct math chunks below it (inclusive).
    let mut tiles: BTreeMap<ChunkId, (Tile, BTreeSet<ChunkId>)> = BTreeMap::new();
    let mut sizes: BTreeMap<ChunkId, u64> = BTreeMap::new();
    let mut seen_frames = BTreeSet::new();
    for fid in frames {
        if !seen_frames.insert(*fid) {
            continue;
        }
        let chunk = store.get(fid)?;
        let frame = FrameManifest::from_chunk(&chunk)?;
        w.frames += 1;
        w.manifest_bytes += chunk.bytes().len() as u64;
        let mut reach = BTreeSet::new();
        for tid in &frame.tiles {
            reach.extend(tile_closure(store, tid, &mut tiles, &mut sizes, &mut w)?.1);
        }
        w.math_refs += reach.len() as u64;
        w.math_bytes_per_frame += reach.iter().map(|id| sizes[id]).sum::<u64>();
    }
    w.tiles = tiles.len() as u64;
    w.math_chunks = sizes.len() as u64;
    w.math_bytes = sizes.values().sum();
    Ok(w)
}

/// Load (once) a tile manifest and everything below it.
fn tile_closure(
    store: &Store,
    id: &ChunkId,
    tiles: &mut BTreeMap<ChunkId, (Tile, BTreeSet<ChunkId>)>,
    sizes: &mut BTreeMap<ChunkId, u64>,
    w: &mut Walk,
) -> Result<(Tile, BTreeSet<ChunkId>), Error> {
    if let Some(done) = tiles.get(id) {
        return Ok(done.clone());
    }
    let chunk = store.get(id)?;
    let m = TileManifest::from_chunk(&chunk)?;
    w.manifest_bytes += chunk.bytes().len() as u64;
    let mut reach = BTreeSet::new();
    for (kind, rid) in &m.refs {
        if !sizes.contains_key(rid) {
            let c = store.get(rid)?;
            if c.contract().kind != *kind || is_manifest(*kind) {
                return Err(Error::Malformed(format!("tile {} ref {rid}: kind {} does not match", m.tile, c.contract().kind)));
            }
            sizes.insert(*rid, c.bytes().len() as u64);
        }
        reach.insert(*rid);
    }
    for (q, child) in m.children.iter().enumerate() {
        if let Some(cid) = child {
            let (ctile, creach) = tile_closure(store, cid, tiles, sizes, w)?;
            if ctile != m.tile.child(q as u8) {
                return Err(Error::Malformed(format!("tile {} child {q} is tile {ctile}", m.tile)));
            }
            reach.extend(creach);
        }
    }
    let out = (m.tile, reach);
    tiles.insert(*id, out.clone());
    Ok(out)
}

/// Whether chunks of `kind` are manifests rather than mathematics.
pub fn is_manifest(kind: Kind) -> bool {
    kind == Kind::TILE_MANIFEST || kind == Kind::FRAME_MANIFEST
}

fn malformed(m: &str) -> Error {
    Error::Malformed(m.to_string())
}

fn expect(chunk: &Chunk, want: Contract) -> Result<(), Error> {
    let c = chunk.contract();
    if *c != want || chunk.minor() != 0 {
        return Err(Error::Malformed(format!("expected a {} v{} chunk, got {} v{}", want.kind, want.encoding, c.kind, c.encoding)));
    }
    Ok(())
}

/// The decoded manifest must re-encode to exactly the chunk it came from.
fn canonical(chunk: &Chunk, again: &Chunk, rest: Reader) -> Result<(), Error> {
    if !rest.0.is_empty() || again.bytes() != chunk.bytes() {
        return Err(malformed("manifest is not in canonical form"));
    }
    Ok(())
}

/// Little-endian payload reader.
struct Reader<'a>(&'a [u8]);

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], Error> {
        if self.0.len() < n {
            return Err(malformed("manifest payload ends early"));
        }
        let (head, tail) = self.0.split_at(n);
        self.0 = tail;
        Ok(head)
    }
    fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, Error> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, Error> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn f64(&mut self) -> Result<f64, Error> {
        Ok(f64::from_bits(self.u64()?))
    }
    fn id(&mut self) -> Result<ChunkId, Error> {
        Ok(ChunkId(self.take(32)?.try_into().unwrap()))
    }
    fn str(&mut self) -> Result<&str, Error> {
        let n = usize::try_from(self.u64()?).map_err(|_| malformed("string too long"))?;
        std::str::from_utf8(self.take(n)?).map_err(|_| malformed("string is not UTF-8"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u8) -> ChunkId {
        ChunkId([n; 32])
    }

    fn tile() -> TileManifest {
        TileManifest {
            tile: "3/1/6".parse().unwrap(),
            evidence: Evidence(Evidence::HEURISTIC.0 | Evidence::CERTIFIED.0),
            children: [None, Some(id(9)), None, None],
            refs: vec![(Kind::BLA, id(2)), (Kind::ORBIT_SLAB, id(1)), (Kind::BLA, id(2))],
        }
    }

    #[test]
    fn tile_round_trip_is_canonical() {
        let m = tile();
        let c = m.to_chunk();
        let back = TileManifest::from_chunk(&c).unwrap();
        assert_eq!(back.refs, vec![(Kind::ORBIT_SLAB, id(1)), (Kind::BLA, id(2))]);
        assert_eq!(back.to_chunk(), c);
        let reordered = TileManifest { refs: vec![(Kind::ORBIT_SLAB, id(1)), (Kind::BLA, id(2))], ..m };
        assert_eq!(reordered.to_chunk().id(), c.id());
        assert_eq!(back.evidence.to_string(), "heuristic,certified");
        assert_eq!("heuristic,certified".parse::<Evidence>().unwrap(), back.evidence);
    }

    #[test]
    fn frame_round_trip_and_refusals() {
        let f = FrameManifest {
            anchor: "40/3/5".parse().unwrap(),
            offset: (0.25, -0.0),
            width: 0.5,
            rotation: 0.0,
            size: (64, 36),
            ss: 1,
            max_iter: 1000,
            columns: 3,
            tiles: vec![id(5), id(4), id(5)],
        };
        let c = f.to_chunk().unwrap();
        let back = FrameManifest::from_chunk(&c).unwrap();
        assert_eq!(back.tiles, vec![id(4), id(5)]);
        assert_eq!(back.offset, (0.25, 0.0));
        assert!(TileManifest::from_chunk(&c).is_err());
        assert!(FrameManifest { width: 0.0, ..f.clone() }.to_chunk().is_err());
        // Unsorted tile refs written by hand are not canonical.
        let mut b = Builder::new(FrameManifest::CONTRACT);
        b.str("40/3/5").f64(0.25).f64(0.0).f64(0.5).f64(0.0).u32(64).u32(36).u32(1).u64(1000).u32(3);
        b.u64(2).raw(&id(5).0).raw(&id(4).0);
        assert!(FrameManifest::from_chunk(&b.finish()).is_err());
    }
}
