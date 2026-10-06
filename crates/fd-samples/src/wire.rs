//! Little-endian primitive encoding. No padding, no host layout ever reaches disk.
use std::io::{Error, ErrorKind, Result};

pub(crate) fn bad(msg: &str) -> Error {
    Error::new(ErrorKind::InvalidData, msg.to_string())
}

/// Round up to the 8-byte alignment every section starts on.
#[inline]
pub(crate) const fn align8(n: usize) -> usize {
    (n + 7) & !7
}

pub(crate) struct Out(pub Vec<u8>);

impl Out {
    pub(crate) fn bytes(&mut self, b: &[u8]) {
        self.0.extend_from_slice(b);
    }
    pub(crate) fn u16(&mut self, v: u16) {
        self.bytes(&v.to_le_bytes());
    }
    pub(crate) fn u32(&mut self, v: u32) {
        self.bytes(&v.to_le_bytes());
    }
    pub(crate) fn u64(&mut self, v: u64) {
        self.bytes(&v.to_le_bytes());
    }
    pub(crate) fn f64(&mut self, v: f64) {
        self.bytes(&v.to_le_bytes());
    }
    pub(crate) fn str(&mut self, s: &str) -> Result<()> {
        let n = u16::try_from(s.len()).map_err(|_| bad("string longer than 65535 bytes"))?;
        self.u16(n);
        self.bytes(s.as_bytes());
        Ok(())
    }
    pub(crate) fn pad8(&mut self) {
        self.0.resize(align8(self.0.len()), 0);
    }
}

pub(crate) struct In<'a> {
    buf: &'a [u8],
    pub(crate) pos: usize,
}

impl<'a> In<'a> {
    pub(crate) fn new(buf: &'a [u8]) -> Self {
        In { buf, pos: 0 }
    }
    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.pos.checked_add(n).filter(|&e| e <= self.buf.len());
        let end = end.ok_or_else(|| Error::new(ErrorKind::UnexpectedEof, "truncated header"))?;
        let s = &self.buf[self.pos..end];
        self.pos = end;
        Ok(s)
    }
    fn arr<const N: usize>(&mut self) -> Result<[u8; N]> {
        Ok(self.take(N)?.try_into().unwrap())
    }
    pub(crate) fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.arr()?))
    }
    pub(crate) fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.arr()?))
    }
    pub(crate) fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.arr()?))
    }
    pub(crate) fn f64(&mut self) -> Result<f64> {
        Ok(f64::from_le_bytes(self.arr()?))
    }
    pub(crate) fn str(&mut self) -> Result<String> {
        let n = self.u16()? as usize;
        String::from_utf8(self.take(n)?.to_vec()).map_err(|_| bad("string is not UTF-8"))
    }
}
