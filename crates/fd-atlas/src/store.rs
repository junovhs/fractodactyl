//! Directory-backed content-addressed store. Layout (docs/spec/ATLAS.md):
//!
//! ```text
//! <root>/FDATLAS                  marker: "fd-atlas store 1\n"
//! <root>/chunks/<2 hex>/<62 hex>  one file per chunk, its canonical bytes, named by id
//! <root>/tmp/                     staging for atomic writes
//! ```
//!
//! A chunk file is written once (staged, then renamed into place) and never modified.
//! Every read re-hashes the bytes, so corruption is detected rather than returned.
use crate::{Chunk, ChunkId, Error};
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const MARKER: &str = "FDATLAS";
const MARKER_TEXT: &str = "fd-atlas store 1\n";

/// What [`Store::put`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Put {
    /// New chunk: its bytes were written.
    Stored,
    /// An intact copy was already stored; nothing was written.
    Deduplicated,
    /// A copy existed but failed verification; it was replaced with the correct bytes.
    Repaired,
}

impl Put {
    pub fn name(self) -> &'static str {
        match self {
            Put::Stored => "stored",
            Put::Deduplicated => "deduplicated",
            Put::Repaired => "repaired",
        }
    }
}

/// Unique chunks and the bytes they occupy (sum of chunk file sizes).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub chunks: u64,
    pub bytes: u64,
}

pub struct Store {
    root: PathBuf,
}

impl Store {
    /// Open the store at `root`, creating it if `root` is missing or empty. A non-empty
    /// directory without the marker is refused, so a typo never scatters chunks.
    pub fn open(root: impl Into<PathBuf>) -> Result<Store, Error> {
        let root = root.into();
        let marker = root.join(MARKER);
        match fs::read_to_string(&marker) {
            Ok(text) if text == MARKER_TEXT => {}
            Ok(_) => return Err(Error::Malformed(format!("{}: unsupported store marker", marker.display()))),
            Err(e) if e.kind() == ErrorKind::NotFound => {
                if fs::read_dir(&root).is_ok_and(|mut d| d.next().is_some()) {
                    return Err(Error::Malformed(format!("{}: not empty and not an atlas store", root.display())));
                }
                fs::create_dir_all(&root)?;
                fs::write(&marker, MARKER_TEXT)?;
            }
            Err(e) => return Err(e.into()),
        }
        fs::create_dir_all(root.join("chunks"))?;
        fs::create_dir_all(root.join("tmp"))?;
        Ok(Store { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Where the chunk with this id lives (whether or not it exists).
    pub fn path(&self, id: &ChunkId) -> PathBuf {
        let hex = id.to_string();
        self.root.join("chunks").join(&hex[..2]).join(&hex[2..])
    }

    pub fn contains(&self, id: &ChunkId) -> bool {
        self.path(id).is_file()
    }

    /// Store `chunk` under its content address. Identical chunks are kept once: a second
    /// put of the same bytes verifies the stored copy and writes nothing.
    pub fn put(&self, chunk: &Chunk) -> Result<(ChunkId, Put), Error> {
        let id = chunk.id();
        let outcome = match self.get(&id) {
            Ok(_) => return Ok((id, Put::Deduplicated)),
            Err(Error::Missing(_)) => Put::Stored,
            Err(Error::Corrupt { .. } | Error::Malformed(_)) => Put::Repaired,
            Err(e) => return Err(e),
        };
        let dest = self.path(&id);
        fs::create_dir_all(dest.parent().expect("chunk path has a parent"))?;
        let tmp = self.staging(&id);
        let staged = write_then_rename(&tmp, &dest, chunk.bytes());
        if staged.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        staged?;
        Ok((id, outcome))
    }

    /// Read and verify one chunk: the bytes must hash to `id` and be canonical.
    pub fn get(&self, id: &ChunkId) -> Result<Chunk, Error> {
        let bytes = match fs::read(self.path(id)) {
            Ok(b) => b,
            Err(e) if e.kind() == ErrorKind::NotFound => return Err(Error::Missing(*id)),
            Err(e) => return Err(e.into()),
        };
        let actual = ChunkId::of(&bytes);
        if actual != *id {
            return Err(Error::Corrupt { id: *id, actual });
        }
        Chunk::from_bytes(bytes)
    }

    /// Every stored chunk id, sorted. Files whose names are not ids are ignored.
    pub fn ids(&self) -> Result<Vec<ChunkId>, Error> {
        Ok(self.files()?.into_iter().map(|(id, _)| id).collect())
    }

    pub fn stats(&self) -> Result<Stats, Error> {
        let files = self.files()?;
        Ok(Stats { chunks: files.len() as u64, bytes: files.iter().map(|(_, len)| len).sum() })
    }

    /// Re-hash every stored chunk; returns the ones that fail, with the reason.
    pub fn verify(&self) -> Result<Vec<(ChunkId, Error)>, Error> {
        let mut bad = Vec::new();
        for id in self.ids()? {
            match self.get(&id) {
                Ok(_) => {}
                Err(Error::Io(e)) => return Err(Error::Io(e)),
                Err(e) => bad.push((id, e)),
            }
        }
        Ok(bad)
    }

    /// `(id, file length)` for every chunk file, sorted by id.
    fn files(&self) -> Result<Vec<(ChunkId, u64)>, Error> {
        let mut out = Vec::new();
        for dir in fs::read_dir(self.root.join("chunks"))? {
            let dir = dir?;
            let prefix = dir.file_name().to_string_lossy().into_owned();
            if prefix.len() != 2 || !dir.file_type()?.is_dir() {
                continue;
            }
            for file in fs::read_dir(dir.path())? {
                let file = file?;
                let name = file.file_name().to_string_lossy().into_owned();
                if let (Ok(id), true) = (format!("{prefix}{name}").parse::<ChunkId>(), file.file_type()?.is_file()) {
                    out.push((id, file.metadata()?.len()));
                }
            }
        }
        out.sort();
        Ok(out)
    }

    fn staging(&self, id: &ChunkId) -> PathBuf {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        self.root.join("tmp").join(format!("{id}.{}.{n}", std::process::id()))
    }
}

/// Write `bytes` to `tmp`, flush to disk, then move it into place. Concurrent writers of
/// one id write identical bytes, so replacing an existing file is safe.
fn write_then_rename(tmp: &Path, dest: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut f = fs::File::create(tmp)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    drop(f);
    fs::rename(tmp, dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Contract, Formula, Kind, Rounding};

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fd-atlas-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn chunk(payload: &[u8]) -> Chunk {
        let c = Contract {
            kind: Kind::CERTIFICATE,
            encoding: 1,
            formula: Formula::MANDELBROT,
            precision_bits: 0,
            rounding: Rounding::Outward,
        };
        Chunk::new(c, payload)
    }

    #[test]
    fn identical_chunks_stored_once() {
        let dir = scratch("dedup");
        let s = Store::open(&dir).unwrap();
        let (a, first) = s.put(&chunk(b"one")).unwrap();
        let (b, second) = s.put(&chunk(b"one")).unwrap();
        assert_eq!((a, first, second), (b, Put::Stored, Put::Deduplicated));
        s.put(&chunk(b"two")).unwrap();
        let st = s.stats().unwrap();
        assert_eq!(st.chunks, 2);
        assert_eq!(st.bytes, (chunk(b"one").bytes().len() + chunk(b"two").bytes().len()) as u64);
        assert_eq!(s.get(&a).unwrap().payload(), b"one");
        assert!(fs::read_dir(dir.join("tmp")).unwrap().next().is_none());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn corruption_detected_then_repaired() {
        let dir = scratch("corrupt");
        let s = Store::open(&dir).unwrap();
        let (id, _) = s.put(&chunk(b"payload")).unwrap();
        let mut bytes = fs::read(s.path(&id)).unwrap();
        bytes[33] ^= 1;
        fs::write(s.path(&id), bytes).unwrap();
        assert!(matches!(s.get(&id), Err(Error::Corrupt { .. })));
        assert_eq!(s.verify().unwrap().len(), 1);
        assert_eq!(s.put(&chunk(b"payload")).unwrap().1, Put::Repaired);
        assert!(s.verify().unwrap().is_empty());
        let missing = ChunkId::of(b"nothing");
        assert!(matches!(s.get(&missing), Err(Error::Missing(_))));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn refuses_foreign_directory() {
        let dir = scratch("foreign");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("notes.txt"), "hi").unwrap();
        assert!(matches!(Store::open(&dir), Err(Error::Malformed(_))));
        fs::remove_dir_all(&dir).unwrap();
    }
}
