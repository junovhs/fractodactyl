//! Directory-backed content-addressed store. Layout (docs/spec/ATLAS.md):
//!
//! ```text
//! <root>/FDATLAS                  marker: "fd-atlas store 1\n"
//! <root>/BUDGET                   optional: "target <bytes>\ncap <bytes>\n" (DEC-03)
//! <root>/chunks/<2 hex>/<62 hex>  one file per chunk, its canonical bytes, named by id
//! <root>/tmp/                     staging for atomic writes
//! ```
//!
//! A chunk file is written once (staged, then renamed into place) and never modified.
//! Every read re-hashes the bytes, so corruption is detected rather than returned.
//! A put that would take the store's chunk bytes over its hard cap is refused.
use crate::{Chunk, ChunkId, Error, Kind};
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

const MARKER: &str = "FDATLAS";
const MARKER_TEXT: &str = "fd-atlas store 1\n";
const BUDGET: &str = "BUDGET";

/// An atlas byte budget over stored chunk bytes (DEC-03). Exceeding `target` is reported;
/// a put that would exceed `cap` is refused and writes nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
    /// Planning target in bytes.
    pub target: u64,
    /// Hard cap in bytes.
    pub cap: u64,
}

impl Budget {
    /// DEC-03 default: 2.5 GiB target, 3 GiB hard cap.
    pub const DEFAULT: Budget = Budget { target: 5 << 29, cap: 3 << 30 };

    /// Parse the BUDGET file; anything but exactly the canonical two lines is refused.
    fn parse(text: &str) -> Option<Budget> {
        let mut lines = text.lines();
        let mut field = |name: &str| -> Option<u64> { lines.next()?.strip_prefix(name)?.strip_prefix(' ')?.parse().ok() };
        let b = Budget { target: field("target")?, cap: field("cap")? };
        let canonical = format!("target {}\ncap {}\n", b.target, b.cap);
        (text == canonical && b.target <= b.cap).then_some(b)
    }
}

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
    /// The outcome as printed by `fd chunk put`.
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
    /// Unique chunks stored.
    pub chunks: u64,
    /// Sum of their file sizes.
    pub bytes: u64,
}

/// A content-addressed chunk store rooted at a directory.
pub struct Store {
    root: PathBuf,
    budget: Budget,
    /// Stored chunk bytes, counted on first put and kept current; the lock also
    /// serialises the cap check with the write within this process.
    used: Mutex<Option<u64>>,
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
        let budget = match fs::read_to_string(root.join(BUDGET)) {
            Ok(text) => Budget::parse(&text)
                .ok_or_else(|| Error::Malformed(format!("{}: bad budget file", root.join(BUDGET).display())))?,
            Err(e) if e.kind() == ErrorKind::NotFound => Budget::DEFAULT,
            Err(e) => return Err(e.into()),
        };
        Ok(Store { root, budget, used: Mutex::new(None) })
    }

    /// The byte budget in force: the BUDGET file, else [`Budget::DEFAULT`].
    pub fn budget(&self) -> Budget {
        self.budget
    }

    /// Persist a new budget for this store; `target` must not exceed `cap`. A cap below
    /// current usage is allowed: existing chunks stay, new ones are refused.
    pub fn set_budget(&mut self, budget: Budget) -> Result<(), Error> {
        if budget.target > budget.cap {
            return Err(Error::Malformed(format!("budget target {} exceeds cap {}", budget.target, budget.cap)));
        }
        let text = format!("target {}\ncap {}\n", budget.target, budget.cap);
        let tmp = self.root.join("tmp").join(format!("{BUDGET}.{}", std::process::id()));
        write_then_rename(&tmp, &self.root.join(BUDGET), text.as_bytes())?;
        self.budget = budget;
        Ok(())
    }

    /// The store directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Where the chunk with this id lives (whether or not it exists).
    pub fn path(&self, id: &ChunkId) -> PathBuf {
        let hex = id.to_string();
        self.root.join("chunks").join(&hex[..2]).join(&hex[2..])
    }

    /// Whether a file exists for `id` (not verified).
    pub fn contains(&self, id: &ChunkId) -> bool {
        self.path(id).is_file()
    }

    /// Store `chunk` under its content address. Identical chunks are kept once: a second
    /// put of the same bytes verifies the stored copy and writes nothing. A new chunk that
    /// would take stored bytes over the budget's cap is refused with [`Error::OverBudget`].
    /// Repairs are always allowed: they restore bytes that were already admitted.
    pub fn put(&self, chunk: &Chunk) -> Result<(ChunkId, Put), Error> {
        let id = chunk.id();
        let mut used = self.used.lock().unwrap_or_else(|e| e.into_inner());
        let outcome = match self.get(&id) {
            Ok(_) => return Ok((id, Put::Deduplicated)),
            Err(Error::Missing(_)) => Put::Stored,
            Err(Error::Corrupt { .. } | Error::Malformed(_)) => Put::Repaired,
            Err(e) => return Err(e),
        };
        let total = match *used {
            Some(u) => u,
            None => self.stats()?.bytes,
        };
        let len = chunk.bytes().len() as u64;
        if outcome == Put::Stored && total.saturating_add(len) > self.budget.cap {
            return Err(Error::OverBudget { need: len, used: total, cap: self.budget.cap });
        }
        let dest = self.path(&id);
        fs::create_dir_all(dest.parent().expect("chunk path has a parent"))?;
        let tmp = self.staging(&id);
        let staged = write_then_rename(&tmp, &dest, chunk.bytes());
        if staged.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        staged?;
        // A repaired file's old length is unknown here; recount on the next put.
        *used = (outcome == Put::Stored).then_some(total + len);
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

    /// Unique chunk count and bytes on disk.
    pub fn stats(&self) -> Result<Stats, Error> {
        let files = self.files()?;
        Ok(Stats { chunks: files.len() as u64, bytes: files.iter().map(|(_, len)| len).sum() })
    }

    /// Unique chunks and bytes per kind, sorted by kind code. The kind is read from each
    /// file's header without verifying it; a file too short to have one counts as kind 0.
    pub fn stats_by_kind(&self) -> Result<Vec<(Kind, Stats)>, Error> {
        let mut out: Vec<(Kind, Stats)> = Vec::new();
        for (id, len) in self.files()? {
            let mut head = [0u8; 14];
            let kind = match fs::File::open(self.path(&id))?.read_exact(&mut head) {
                Ok(()) => Kind(u16::from_le_bytes([head[12], head[13]])),
                Err(e) if e.kind() == ErrorKind::UnexpectedEof => Kind(0),
                Err(e) => return Err(e.into()),
            };
            let i = match out.binary_search_by_key(&kind.0, |(k, _)| k.0) {
                Ok(i) => i,
                Err(i) => {
                    out.insert(i, (kind, Stats::default()));
                    i
                }
            };
            out[i].1.chunks += 1;
            out[i].1.bytes += len;
        }
        Ok(out)
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
    fn hard_cap_refuses_new_chunks_only() {
        let dir = scratch("budget");
        let mut s = Store::open(&dir).unwrap();
        assert_eq!(s.budget(), Budget::DEFAULT);
        let one = chunk(b"one").bytes().len() as u64;
        s.set_budget(Budget { target: one, cap: one * 2 }).unwrap();
        s.put(&chunk(b"one")).unwrap();
        s.put(&chunk(b"two")).unwrap();
        assert!(matches!(s.put(&chunk(b"six")), Err(Error::OverBudget { .. })));
        assert_eq!(s.put(&chunk(b"one")).unwrap().1, Put::Deduplicated);
        assert_eq!(s.stats().unwrap().bytes, one * 2);
        let reopened = Store::open(&dir).unwrap();
        assert_eq!(reopened.budget().cap, one * 2);
        assert_eq!(reopened.stats_by_kind().unwrap(), vec![(Kind::CERTIFICATE, Stats { chunks: 2, bytes: one * 2 })]);
        assert!(s.set_budget(Budget { target: 3, cap: 2 }).is_err());
        fs::write(dir.join(BUDGET), "target 1\ncap 2\nextra\n").unwrap();
        assert!(matches!(Store::open(&dir), Err(Error::Malformed(_))));
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
