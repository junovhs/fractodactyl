//! Writer: header, then each present column in bit order, each 8-byte aligned.
use crate::header::Header;
use crate::samples::Samples;
use crate::wire::{align8, bad};
use std::io::{Result, Write};

/// Write `s` described by `h`. `h.columns` must match the columns present in `s`.
pub fn write<W: Write>(mut w: W, h: &Header, s: &Samples) -> Result<()> {
    if h.columns != s.columns() {
        return Err(bad("header column set does not match the sample columns"));
    }
    let n = h.count();
    let lens = [
        Some(s.class.len()),
        s.nu.as_ref().map(Vec::len),
        s.de.as_ref().map(Vec::len),
        s.normal.as_ref().map(Vec::len),
        s.bound.as_ref().map(Vec::len),
    ];
    if lens.iter().flatten().any(|&l| l != n) {
        return Err(bad("column length does not match nx * ny"));
    }
    w.write_all(&h.encode()?)?;
    let mut buf = Vec::with_capacity(align8(n * 8));
    put(&mut w, &mut buf, s.class.iter().map(|c| [c.0]))?;
    if let Some(v) = &s.nu {
        put(&mut w, &mut buf, v.iter().map(|x| x.to_le_bytes()))?;
    }
    if let Some(v) = &s.de {
        put(&mut w, &mut buf, v.iter().map(|x| x.to_le_bytes()))?;
    }
    if let Some(v) = &s.normal {
        put(&mut w, &mut buf, v.iter().map(|x| x.to_le_bytes()))?;
    }
    if let Some(v) = &s.bound {
        put(&mut w, &mut buf, v.iter().map(|x| x.to_le_bytes()))?;
    }
    w.flush()
}

/// Encode one column into `buf` (reused across columns) and write it padded.
fn put<W: Write, const N: usize>(w: &mut W, buf: &mut Vec<u8>, it: impl Iterator<Item = [u8; N]>) -> Result<()> {
    buf.clear();
    for b in it {
        buf.extend_from_slice(&b);
    }
    buf.resize(align8(buf.len()), 0);
    w.write_all(buf)
}
