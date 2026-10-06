//! File header: what was sampled, where, and with which kernel.
use crate::column::ColumnSet;
use crate::wire::{align8, bad, In, Out};
use crate::{MAGIC, MAJOR, MINOR};
use std::io::Result;

/// The sampled region. Coordinates are decimal strings exactly as given, so no
/// precision is lost to a float round-trip (the reference may need thousands of bits).
///
/// Sample `(i, j)`, `0 <= i < nx`, `0 <= j < ny`, sits at
/// `c = center + e^{i*rotation} * h * (x - i*y)` with
/// `x = i + 0.5 - nx/2`, `y = j + 0.5 - ny/2`, `h = width / nx`
/// (row `j` grows downward on screen; imaginary axis points up).
#[derive(Clone, Debug, PartialEq)]
pub struct View {
    /// Real part of the centre, decimal.
    pub center_re: String,
    /// Imaginary part of the centre, decimal.
    pub center_im: String,
    /// Width of the whole sample grid in the complex plane.
    pub width: String,
    /// Radians, counter-clockwise.
    pub rotation: f64,
}

/// Everything a consumer needs to interpret the columns.
#[derive(Clone, Debug, PartialEq)]
pub struct Header {
    /// Format version of the file this header was read from (writers use MAJOR.MINOR).
    pub minor: u16,
    /// Columns present in the file.
    pub columns: ColumnSet,
    /// Sample grid size (output pixels times `ss`).
    pub nx: u32,
    pub ny: u32,
    /// Samples per output pixel along each axis.
    pub ss: u32,
    /// Iteration budget; samples that reach it are `Unresolved`.
    pub max_iter: u64,
    /// Escape radius used for `nu`, `de` and `normal`.
    pub escape_radius: f64,
    /// Where the samples are.
    pub view: View,
    /// Producing kernel and its numeric contract, e.g. `pert-f64/1`.
    pub kernel: String,
}

impl Header {
    /// Number of samples (`nx * ny`).
    #[inline]
    pub fn count(&self) -> usize {
        self.nx as usize * self.ny as usize
    }

    /// Output raster size in pixels.
    pub fn pixels(&self) -> (usize, usize) {
        let s = self.ss.max(1) as usize;
        (self.nx as usize / s, self.ny as usize / s)
    }

    /// Canonical header bytes, padded to 8.
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut o = Out(Vec::with_capacity(128));
        o.bytes(&MAGIC);
        o.u16(MAJOR);
        o.u16(MINOR);
        o.u32(self.columns.0);
        o.u32(self.nx);
        o.u32(self.ny);
        o.u32(self.ss);
        o.u32(0); // reserved
        o.u64(self.max_iter);
        o.f64(self.escape_radius);
        o.f64(self.view.rotation);
        o.str(&self.view.center_re)?;
        o.str(&self.view.center_im)?;
        o.str(&self.view.width)?;
        o.str(&self.kernel)?;
        o.pad8();
        Ok(o.0)
    }

    /// Decode from the start of `buf`; returns the header and its padded length.
    pub fn decode(buf: &[u8]) -> Result<(Header, usize)> {
        let mut i = In::new(buf);
        if i.take(8)? != MAGIC {
            return Err(bad("not a Fractadactyl sample file"));
        }
        if i.u16()? != MAJOR {
            return Err(bad("unsupported sample format major version"));
        }
        let minor = i.u16()?;
        let columns = ColumnSet(i.u32()?);
        let (nx, ny, ss) = (i.u32()?, i.u32()?, i.u32()?);
        i.u32()?;
        let (max_iter, escape_radius, rotation) = (i.u64()?, i.f64()?, i.f64()?);
        let view = View { center_re: i.str()?, center_im: i.str()?, width: i.str()?, rotation };
        let kernel = i.str()?;
        if ss == 0 || !nx.is_multiple_of(ss) || !ny.is_multiple_of(ss) {
            return Err(bad("grid is not a whole number of pixels"));
        }
        let h = Header { minor, columns, nx, ny, ss, max_iter, escape_radius, view, kernel };
        Ok((h, align8(i.pos)))
    }
}

/// Upper bound on a v1 header: fixed part plus four strings of at most 65535 bytes.
pub const MAX_HEADER: usize = 56 + 4 * (2 + 65535) + 8;
