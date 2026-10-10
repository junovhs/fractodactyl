//! Reader: parses the header once, then reads only the columns asked for.
use crate::class::Class;
use crate::column::{Column, ColumnSet};
use crate::header::{Header, MAX_HEADER};
use crate::samples::Samples;
use crate::wire::{align8, bad};
use std::fs::File;
use std::io::{ErrorKind, Read, Result, Seek, SeekFrom};
use std::path::Path;

const MAX_SAMPLE_FILE_BYTES: u64 = 1 << 34;

/// An open sample file whose header has been parsed and validated.
pub struct Reader {
    file: File,
    /// The parsed header.
    pub header: Header,
    body: usize,
}

impl Reader {
    /// Open and validate a sample file; no column data is read yet.
    pub fn open(path: &Path) -> Result<Reader> {
        let mut file = File::open(path)?;
        let size = file.metadata()?.len();
        // Headers are ~100 bytes; fall back to the format maximum only if needed.
        let (header, body) = match head(&mut file, size.min(4096) as usize) {
            Err(e) if e.kind() == ErrorKind::UnexpectedEof && size > 4096 => head(&mut file, size.min(MAX_HEADER as u64) as usize)?,
            r => r?,
        };
        let known = ColumnSet(header.columns.0 & ColumnSet::KNOWN);
        let n = u64::from(header.nx).checked_mul(u64::from(header.ny))
            .ok_or_else(|| bad("sample count overflow"))?;
        let mut need = body as u64;
        for c in known.iter() {
            let bytes = n.checked_mul(c.width() as u64)
                .and_then(|n| n.checked_add(7))
                .map(|n| n & !7)
                .ok_or_else(|| bad("sample column size overflow"))?;
            need = need.checked_add(bytes).ok_or_else(|| bad("sample file size overflow"))?;
        }
        if !header.columns.has(Column::Class) || need > MAX_SAMPLE_FILE_BYTES
            || need > size || usize::try_from(need).is_err()
        {
            return Err(bad("sample file is truncated, oversized, or lacks the class column"));
        }
        Ok(Reader { file, header, body })
    }

    /// Read the requested columns (plus `Class`, always). Errors if one is absent.
    pub fn read(&mut self, want: ColumnSet) -> Result<Samples> {
        let n = self.header.count();
        let mut s = Samples { class: self.col(Column::Class, |b| Class(b[0]))?, ..Samples::default() };
        if s.class.iter().any(|c| !c.is_valid()) {
            return Err(bad("invalid class byte"));
        }
        if want.has(Column::Nu) {
            s.nu = Some(self.col(Column::Nu, |b| f64::from_le_bytes(b.try_into().unwrap()))?);
        }
        if want.has(Column::De) {
            s.de = Some(self.col(Column::De, |b| f32::from_le_bytes(b.try_into().unwrap()))?);
        }
        if want.has(Column::Normal) {
            s.normal = Some(self.col(Column::Normal, |b| u16::from_le_bytes(b.try_into().unwrap()))?);
        }
        if want.has(Column::Bound) {
            s.bound = Some(self.col(Column::Bound, |b| f32::from_le_bytes(b.try_into().unwrap()))?);
        }
        debug_assert!(s.class.len() == n);
        Ok(s)
    }

    /// Seek straight to one column and decode it; other columns are never touched.
    fn col<T>(&mut self, c: Column, f: impl Fn(&[u8]) -> T) -> Result<Vec<T>> {
        let h = &self.header;
        if !h.columns.has(c) {
            return Err(bad(&format!("column {c:?} is not in this file")));
        }
        let n = h.count();
        let off = self.body
            + h.columns.iter().take_while(|&p| p != c).map(|p| align8(p.width() * n)).sum::<usize>();
        let mut raw = vec![0u8; c.width() * n];
        self.file.seek(SeekFrom::Start(off as u64))?;
        self.file.read_exact(&mut raw)?;
        Ok(raw.chunks_exact(c.width()).map(f).collect())
    }
}

fn head(file: &mut File, len: usize) -> Result<(Header, usize)> {
    let mut buf = vec![0u8; len];
    file.seek(SeekFrom::Start(0))?;
    file.read_exact(&mut buf)?;
    Header::decode(&buf)
}
