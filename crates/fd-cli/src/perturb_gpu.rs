//! Shared f32+exponent arithmetic and generic bounded perturbation checkpoints.
//! GPU-31 deliberately supplies no frame route: these states carry no accumulated
//! pixel-error guarantee, classification or derivative. GPU-11 must add those and
//! a CPU restart policy before a frame can use them (DEC-10/17/25).
//! Small relative delta error can exceed initial sample spacing by many orders;
//! checkpoint roundoff budgets are diagnostics, never DEC-17 output admission.
//! The shared arithmetic already serves the production zone kernels (DEC-24).

pub(crate) const CF: &str = include_str!("../shaders/cf.wgsl");
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const STEPPER: &str = include_str!("../shaders/perturb32.wgsl");

#[cfg(test)]
pub(crate) const CF_ADAPTER: &str = include_str!("../shaders/perturb_cf.wgsl");
pub(crate) const CE_ADAPTER: &str = include_str!("../shaders/perturb_ce.wgsl");
mod frame;
pub(crate) use frame::{enabled, render, render_bla};

#[cfg(test)]
mod tests;
