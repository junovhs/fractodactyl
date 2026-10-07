//! Atlas storage (DEC-06): immutable semantic chunks named by the SHA-256 of their
//! canonical bytes, kept once each in a content-addressed directory store. Chunk
//! boundaries are semantic (one orbit slab, one operator, one certificate...), never
//! content-defined. Spec: docs/spec/ATLAS.md.
mod chunk;
mod manifest;
mod sha256;
mod store;

pub use chunk::{canonical_f64, Builder, Chunk, Contract, Formula, Kind, Rounding, CANONICAL_NAN, HEADER_LEN};
pub use manifest::{is_manifest, walk, Evidence, FrameManifest, TileManifest, Walk, MANIFEST_ENCODING};
pub use sha256::Sha256;
pub use store::{Budget, Put, Stats, Store};

use std::fmt;

/// A chunk's content address: SHA-256 of its canonical bytes. Text form is 64
/// lowercase hex digits; only that form parses.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChunkId(pub [u8; 32]);

impl ChunkId {
    /// The id of `bytes`: their SHA-256.
    pub fn of(bytes: &[u8]) -> ChunkId {
        ChunkId(Sha256::digest(bytes))
    }
}

impl fmt::Display for ChunkId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.iter().try_for_each(|b| write!(f, "{b:02x}"))
    }
}

impl fmt::Debug for ChunkId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ChunkId({self})")
    }
}

impl std::str::FromStr for ChunkId {
    type Err = String;
    fn from_str(s: &str) -> Result<ChunkId, String> {
        let digit = |c: u8| match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            _ => None,
        };
        let bad = || format!("bad chunk id {s:?}: expected 64 lowercase hex digits");
        if s.len() != 64 {
            return Err(bad());
        }
        let mut id = [0; 32];
        for (out, pair) in id.iter_mut().zip(s.as_bytes().chunks_exact(2)) {
            *out = digit(pair[0]).zip(digit(pair[1])).map(|(h, l)| (h << 4) | l).ok_or_else(bad)?;
        }
        Ok(ChunkId(id))
    }
}

/// Store and chunk errors. Corruption is always reported, never silently repaired on read.
#[derive(Debug)]
pub enum Error {
    /// No chunk with this id is stored.
    Missing(ChunkId),
    /// The stored bytes no longer hash to their id.
    Corrupt { id: ChunkId, actual: ChunkId },
    /// Bytes are not a canonical chunk (or the store directory is not a store).
    Malformed(String),
    /// Storing `need` more bytes on top of `used` would exceed the hard `cap` (DEC-03).
    OverBudget { need: u64, used: u64, cap: u64 },
    /// Filesystem failure.
    Io(std::io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Missing(id) => write!(f, "chunk {id} is not in the store"),
            Error::Corrupt { id, actual } => write!(f, "chunk {id} is corrupt: stored bytes hash to {actual}"),
            Error::Malformed(m) => write!(f, "malformed chunk: {m}"),
            Error::OverBudget { need, used, cap } => write!(
                f,
                "atlas over budget: {need} more bytes on top of {used} would exceed the hard cap of {cap} bytes (DEC-03); nothing written"
            ),
            Error::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::Io(e)
    }
}

#[cfg(test)]
mod tests {
    use super::ChunkId;

    #[test]
    fn id_text_round_trip() {
        let id = ChunkId::of(b"abc");
        let text = id.to_string();
        assert_eq!(text, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(text.parse::<ChunkId>().unwrap(), id);
        assert!(text.to_uppercase().parse::<ChunkId>().is_err());
        assert!(text[1..].parse::<ChunkId>().is_err());
    }
}
