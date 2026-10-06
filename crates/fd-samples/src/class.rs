//! One byte per sample: what the sample is, and how strongly that is known.
//!
//! Bits 0-1 `Kind`, bits 2-3 `Evidence`, bits 4-7 reserved (written as 0).

/// What the computation concluded about the sample's orbit.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Orbit left the escape radius; `nu`, `de` and `normal` are meaningful.
    Escaped = 0,
    /// Orbit was judged bounded (e.g. an attracting cycle was found).
    Interior = 1,
    /// The iteration budget ran out first. This is uncertainty, not "inside".
    Unresolved = 2,
}

/// How strong the claim behind `Kind` (and the scalars) is. Mirrors the atlas
/// certification hierarchy (docs/research/02, "Certification hierarchy").
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum Evidence {
    /// Ordinary floating-point evaluation; no error bound attached.
    Heuristic = 0,
    /// Numerical error is bounded; see the `bound` column.
    Bounded = 1,
    /// Classification proven (ball/interval or rigorous interior proof).
    Certified = 2,
}

/// Packed classification byte.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Class(pub u8);

impl Class {
    /// Pack a kind and its evidence level.
    #[inline]
    pub const fn new(kind: Kind, ev: Evidence) -> Class {
        Class(kind as u8 | (ev as u8) << 2)
    }

    /// The kind, or `None` for the invalid encoding 3.
    #[inline]
    pub fn kind(self) -> Option<Kind> {
        match self.0 & 3 {
            0 => Some(Kind::Escaped),
            1 => Some(Kind::Interior),
            2 => Some(Kind::Unresolved),
            _ => None,
        }
    }

    /// The evidence level, or `None` for the invalid encoding 3.
    #[inline]
    pub fn evidence(self) -> Option<Evidence> {
        match (self.0 >> 2) & 3 {
            0 => Some(Evidence::Heuristic),
            1 => Some(Evidence::Bounded),
            2 => Some(Evidence::Certified),
            _ => None,
        }
    }

    /// True when the byte is a valid v1 encoding.
    #[inline]
    pub fn is_valid(self) -> bool {
        self.0 >> 4 == 0 && self.kind().is_some() && self.evidence().is_some()
    }

    /// Fast test for `Kind::Escaped`, the only kind with meaningful scalars.
    #[inline]
    pub fn escaped(self) -> bool {
        self.0 & 3 == Kind::Escaped as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        for k in [Kind::Escaped, Kind::Interior, Kind::Unresolved] {
            for e in [Evidence::Heuristic, Evidence::Bounded, Evidence::Certified] {
                let c = Class::new(k, e);
                assert!(c.is_valid());
                assert_eq!((c.kind(), c.evidence()), (Some(k), Some(e)));
            }
        }
        assert!(!Class(3).is_valid());
        assert!(!Class(0x10).is_valid());
    }
}
