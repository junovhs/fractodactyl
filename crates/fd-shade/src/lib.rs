//! Looks: late appearance passes. Each declares the columns it reads and never
//! recomputes fractal math: recolouring costs one pass over stored samples (DEC-07).
//! This crate depends on `fd-samples` only, never on a kernel, so a look cannot
//! iterate (a test below holds that line).
mod palette;
mod png;
mod relief;
mod resolve;
mod umber;

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
        "umber" => Some(Box::new(umber::Umber)),
        "palette" => Some(Box::new(palette::Palette)),
        "relief" => Some(Box::new(relief::Relief)),
        _ => None,
    }
}

/// Names accepted by `by_name`; the first is the default look.
pub const NAMES: [&str; 3] = ["umber", "palette", "relief"];

#[cfg(test)]
mod tests {
    /// Looks are late: this crate links no kernel, so shading cannot iterate.
    #[test]
    fn depends_on_samples_only() {
        let toml = include_str!("../Cargo.toml");
        let deps = toml.split("[dependencies]").nth(1).unwrap_or("");
        let names: Vec<&str> =
            deps.lines().take_while(|l| !l.starts_with('[')).filter_map(|l| l.split('=').next()).map(str::trim).filter(|n| !n.is_empty()).collect();
        assert_eq!(names, ["fd-samples"], "fd-shade must not depend on anything that can iterate");
    }

    #[test]
    fn every_name_resolves() {
        for n in super::NAMES {
            assert!(super::by_name(n).is_some(), "{n}");
        }
    }
}
