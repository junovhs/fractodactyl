//! Screen-space error contract (LOD-01; DEC-05, DEC-10). Spec: `docs/spec/LOD.md`.
//!
//! Decides, per tile of output pixels, whether its samples may be accepted as visually
//! resolved or must be refined. The rule reads only mathematical columns (`class`, `de`,
//! `bound`), never colour, and it is conservative: anything without evidence refines.
use crate::class::{Evidence, Kind};
use crate::header::Header;
use crate::samples::Samples;

/// Initial final/video threshold on projected error, in output pixels. Experimental.
pub const FINAL_PX: f64 = 0.25;

/// What a tile's evidence establishes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    /// Every sample is certified interior: one value, no further sampling needed.
    CertifiedUniform,
    /// Every sample is exterior with a bounded value error and no set nearby.
    CertifiedApproximate,
    /// The set boundary may lie in the tile, or some sample lacks evidence. Always refines.
    UnresolvedBoundary,
}

impl State {
    /// Report name.
    pub fn name(self) -> &'static str {
        match self {
            State::CertifiedUniform => "certified_uniform",
            State::CertifiedApproximate => "certified_approximate",
            State::UnresolvedBoundary => "unresolved_boundary",
        }
    }
}

/// The rule's result for one tile.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Verdict {
    /// Tile state.
    pub state: State,
    /// Projected error `E_px = eps_T / sigma_f` in output pixels; infinite when unresolved.
    pub e_px: f64,
    /// True when the tile may skip further work at the threshold it was judged against.
    pub accept: bool,
    /// Why: `uniform`, `bounded`, `over-threshold`, or the first evidence gap found.
    pub reason: &'static str,
}

/// Judge the samples `idx` (indices into `s`) of one tile; `ss` is samples per pixel per
/// axis and `max_px` the accepted projected error in output pixels.
pub fn judge(s: &Samples, idx: impl IntoIterator<Item = usize>, ss: u32, max_px: f64) -> Verdict {
    // A sample cell is 1/ss px square; the set must stay farther than its half-diagonal.
    judge_reach(s, idx, std::f64::consts::SQRT_2 / (2.0 * ss.max(1) as f64), max_px)
}

/// [`judge`] where each sample stands for every point within `reach` output pixels of
/// it (the farthest point of the area it covers): the Koebe disk `de/4` must clear it.
pub fn judge_reach(s: &Samples, idx: impl IntoIterator<Item = usize>, reach: f64, max_px: f64) -> Verdict {
    let refine = |reason| Verdict { state: State::UnresolvedBoundary, e_px: f64::INFINITY, accept: false, reason };
    let (mut interior, mut escaped, mut e_px) = (false, false, 0.0f64);
    for k in idx {
        let c = s.class[k];
        let (kind, ev) = (c.kind(), c.evidence().unwrap_or(Evidence::Heuristic));
        match kind {
            Some(Kind::Escaped) if ev >= Evidence::Bounded => {
                let (Some(de), Some(bound)) = (&s.de, &s.bound) else { return refine("no-bound") };
                let (de, bound) = (de[k] as f64, bound[k] as f64);
                if de.is_nan() || de / 4.0 <= reach {
                    return refine("near-set"); // Koebe: distance to the set >= de/4
                }
                if !bound.is_finite() || bound < 0.0 {
                    return refine("no-bound");
                }
                escaped = true;
                e_px = e_px.max(bound * de * std::f64::consts::LN_2 / 2.0); // |dnu/dpx| = 2/(de ln 2)
            }
            Some(Kind::Interior) if ev == Evidence::Certified => interior = true,
            Some(Kind::Unresolved) => return refine("unresolved"),
            _ => return refine("uncertified"),
        }
        if interior && escaped {
            return refine("mixed");
        }
    }
    match (interior, escaped) {
        (true, _) => Verdict { state: State::CertifiedUniform, e_px: 0.0, accept: true, reason: "uniform" },
        (_, true) => Verdict {
            state: State::CertifiedApproximate,
            e_px,
            accept: e_px <= max_px,
            reason: if e_px <= max_px { "bounded" } else { "over-threshold" },
        },
        _ => refine("empty"),
    }
}

/// Judge every `tile_px x tile_px` tile of output pixels in row-major order (edge tiles
/// may be smaller). Returns `(tile column, tile row, verdict)`.
pub fn tiles(h: &Header, s: &Samples, tile_px: u32, max_px: f64) -> Vec<(usize, usize, Verdict)> {
    let t = (tile_px.max(1) * h.ss.max(1)) as usize; // tile side in samples
    let (nx, ny) = (h.nx as usize, h.ny as usize);
    let mut out = Vec::new();
    for ty in 0..ny.div_ceil(t) {
        for tx in 0..nx.div_ceil(t) {
            let rows = ty * t..((ty + 1) * t).min(ny);
            let idx = rows.flat_map(|j| (tx * t..((tx + 1) * t).min(nx)).map(move |i| j * nx + i));
            out.push((tx, ty, judge(s, idx, h.ss, max_px)));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::class::Class;
    use crate::column::{Column, ColumnSet};

    fn samples(classes: &[Class], de: f32, bound: f32) -> Samples {
        let mut s = Samples::alloc(classes.len(), ColumnSet::of(&[Column::Class, Column::De, Column::Bound]));
        s.class = classes.to_vec();
        s.de.as_mut().unwrap().fill(de);
        s.bound.as_mut().unwrap().fill(bound);
        s
    }

    #[test]
    fn states() {
        let esc = Class::new(Kind::Escaped, Evidence::Bounded);
        let int = Class::new(Kind::Interior, Evidence::Certified);
        let j = |s: &Samples| judge(s, 0..s.class.len(), 1, FINAL_PX);
        assert_eq!(j(&samples(&[int, int], 0.0, 0.0)).state, State::CertifiedUniform);
        // E_px = bound * de * ln2 / 2 = 0.1 * 4 * 0.693 / 2 = 0.139 px: accepted.
        let v = j(&samples(&[esc, esc], 4.0, 0.1));
        assert!(v.accept && v.state == State::CertifiedApproximate && (v.e_px - 0.1386).abs() < 1e-3, "{v:?}");
        assert_eq!(j(&samples(&[esc], 4.0, 1.0)).reason, "over-threshold");
        assert_eq!(j(&samples(&[esc], 1.0, 0.0)).reason, "near-set");
        assert_eq!(j(&samples(&[esc, int], 4.0, 0.0)).reason, "mixed");
        assert_eq!(j(&samples(&[Class::new(Kind::Escaped, Evidence::Heuristic)], 4.0, 0.0)).reason, "uncertified");
        assert_eq!(j(&samples(&[Class::new(Kind::Interior, Evidence::Bounded)], 0.0, 0.0)).reason, "uncertified");
        assert_eq!(j(&samples(&[Class::new(Kind::Unresolved, Evidence::Certified)], 0.0, 0.0)).reason, "unresolved");
    }
}
