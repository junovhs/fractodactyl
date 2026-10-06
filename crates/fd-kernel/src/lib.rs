//! Sample kernel: perturbation against one reference orbit, with rebasing, writing
//! palette-independent results straight into `fd_samples::Samples` columns.
//!
//! Numeric contract `pert-f64/1`: reference orbit, view offsets and per-sample deltas
//! are IEEE f64. Valid only while rounding the centre to f64 moves it by much less than
//! a sample (checked; see `view::Plane::new`). Deep zoom (arbitrary-precision reference,
//! extended-exponent deltas) is BASE-02. Results carry `Evidence::Heuristic`.
mod grid;
mod interior;
mod reference;
mod sample;
mod view;

pub use grid::{render, Params};
pub use reference::Reference;
pub use view::Plane;

/// Kernel identity written into the file header.
pub const KERNEL: &str = "pert-f64/1";
