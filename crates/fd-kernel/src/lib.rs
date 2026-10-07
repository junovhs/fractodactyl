//! Sample kernels: perturbation against one reference orbit, with rebasing, writing
//! palette-independent results straight into `fd_samples::Samples` columns.
//!
//! Three numeric tiers, cheapest first; `render` picks the cheapest one whose contract
//! holds for the view (see `view::Tier`):
//! - `pert-f64/1`: f64 reference at the f64-rounded centre, f64 deltas. Only while
//!   rounding the centre moves it by under 1/1024 of a sample.
//! - `pert-fx/1`: fixed-point reference at the exact decimal centre, f64 deltas, down
//!   to sample spacings of 2^-900.
//! - `pert-fx-scaled/1`: fixed-point reference, deltas and derivative carried as f64
//!   times an exact power of two: any depth.
//!
//! Results carry `Evidence::Heuristic`, except escaped samples of the f64/fx tiers when
//! the `Bound` column is requested (`Bounded`, docs/spec/LOD.md). No series/BLA skipping
//! (ACC-01). `render_refined` skips supersamples in blocks the screen-space error rule
//! accepts from one sample per pixel (LOD-02).
mod grid;
mod interior;
mod reference;
mod refine;
mod sample;
mod scaled;
mod store;
mod view;

pub use grid::{reference, reference_bits, render, render_stats, render_with, Params, Stats};
pub use reference::Reference;
pub use refine::{render_refined, Phase, Refinement, DENSE, FALLBACK, FINAL, PREVIEW, SPARSE};
pub use view::{Plane, Tier};
