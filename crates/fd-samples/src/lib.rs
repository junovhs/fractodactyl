//! Palette-independent sample file (`.fds`), format v1. Spec: `docs/spec/SAMPLES.md`.
//!
//! Mathematical results only (DEC-07): classification, smooth escape value, distance
//! estimate, normal, optional error bound. Colour is a later pass that reads these.
//! Columns are stored structure-of-arrays so a consumer reads only what it uses.
mod class;
mod column;
mod header;
mod read;
mod samples;
mod wire;
mod write;

pub use class::{Class, Evidence, Kind};
pub use column::{Column, ColumnSet};
pub use header::{Header, View};
pub use read::Reader;
pub use samples::Samples;
pub use write::write;

/// File magic: identifies a Fractadactyl sample file.
pub const MAGIC: [u8; 8] = *b"FDSAMPLE";
/// Bumped for any change in the meaning or layout of existing fields.
pub const MAJOR: u16 = 1;
/// Bumped for additive changes (new optional columns) that old readers may ignore.
pub const MINOR: u16 = 0;
