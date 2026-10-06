//! Appearance passes. Each declares the columns it reads and never recomputes
//! fractal math: recolouring costs one pass over stored samples (DEC-07).
//! These two passes are the BASE-01 proof; the real look pipeline is SHADE-01.
mod palette;
mod png;
mod relief;
mod resolve;

pub use png::encode as png;
pub use resolve::Rgb8;

use fd_samples::{ColumnSet, Header, Samples};

/// A late pass: which columns it needs, and how it turns samples into pixels.
pub trait Pass {
    /// Columns this pass reads; the reader loads nothing else.
    fn columns(&self) -> ColumnSet;
    /// Turn samples into an image. Must not recompute any fractal math.
    fn shade(&self, h: &Header, s: &Samples) -> Rgb8;
}

/// Look up a pass by name.
pub fn by_name(name: &str) -> Option<Box<dyn Pass>> {
    match name {
        "palette" => Some(Box::new(palette::Palette)),
        "relief" => Some(Box::new(relief::Relief)),
        _ => None,
    }
}

/// Names accepted by `by_name`.
pub const NAMES: [&str; 2] = ["palette", "relief"];
