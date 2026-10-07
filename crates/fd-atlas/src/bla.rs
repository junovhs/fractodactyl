//! BLA operator tables (docs/spec/ATLAS.md "BLA tables", ACC-01): the bivariate linear
//! approximation blocks of one stored reference orbit, named by its orbit manifest, for
//! samples with `|dc| <= dc_max`. Stored levels are `j = 1, 2, ...` (single steps are
//! never stored); level `j` holds `(points - 2) >> j` blocks of `2^j` steps, each `A`,
//! `B`, validity radius `r` and remainder coefficients `alpha`, `beta`.
use crate::{Builder, Chunk, ChunkId, Contract, Error, Formula, Kind, Rounding};

/// Payload layout version of BLA tables: f64 structure-of-arrays per level.
pub const BLA_ENCODING: u16 = 1;

/// One BLA table, as stored.
#[derive(Clone, Debug, PartialEq)]
pub struct BlaTable {
    /// The orbit manifest (kind 9) of the reference orbit the blocks were built over.
    pub orbit: ChunkId,
    /// Relative tolerance of each dropped quadratic term.
    pub eps: f64,
    /// Largest `|dc|` the validity radii hold for.
    pub dc_max: f64,
    /// Points of that orbit.
    pub points: u64,
    /// `levels[i]` is level `i + 1`; per block: `a.re, a.im, b.re, b.im, r, alpha, beta`.
    pub levels: Vec<Vec<[f64; 7]>>,
}

impl BlaTable {
    /// Contract of every v1 BLA table: f64 coefficients rounded to nearest.
    pub const CONTRACT: Contract = Contract {
        kind: Kind::BLA,
        encoding: BLA_ENCODING,
        formula: Formula::MANDELBROT,
        precision_bits: 53,
        rounding: Rounding::Nearest,
    };

    /// Canonical chunk. Refuses a table whose shape does not fit `points` or that holds
    /// a non-finite value, a negative radius or a bad `eps`/`dc_max`.
    pub fn to_chunk(&self) -> Result<Chunk, Error> {
        let steps = self.points.saturating_sub(2);
        let shape = self.levels.len() < 63 && self.levels.iter().enumerate().all(|(i, l)| l.len() as u64 == steps >> (i + 1));
        // An invalid block (r = 0) is all zero, and every level holds a valid block (a
        // table ends before its first level without one): one canonical form per table.
        let values = self.levels.iter().flatten().all(|b| b.iter().all(|v| v.is_finite()) && (b[4] > 0.0 || b.iter().all(|&v| v == 0.0)));
        let used = self.levels.iter().all(|l| l.iter().any(|b| b[4] > 0.0));
        let scalars = self.eps > 0.0 && self.eps < 1.0 && self.dc_max > 0.0 && self.dc_max.is_finite();
        if !(shape && values && used && scalars) {
            return Err(malformed(
                "BLA table needs (points - 2) >> j finite blocks on level j = 1.., each level with a valid block, r > 0 or an all-zero block, 0 < eps < 1, dc_max > 0",
            ));
        }
        let mut b = Builder::new(Self::CONTRACT);
        b.raw(&self.orbit.0).f64(self.eps).f64(self.dc_max).u64(self.points).u64(self.levels.len() as u64);
        for level in &self.levels {
            for f in 0..7 {
                for block in level {
                    b.f64(block[f]);
                }
            }
        }
        Ok(b.finish())
    }

    /// Decode a BLA table chunk, refusing anything not in canonical form.
    pub fn from_chunk(chunk: &Chunk) -> Result<BlaTable, Error> {
        if *chunk.contract() != Self::CONTRACT || chunk.minor() != 0 {
            return Err(malformed("not a v1 BLA table chunk (kind, encoding, formula, precision or rounding)"));
        }
        let p = chunk.payload();
        let end = || malformed("BLA table payload ends early");
        let word = |i: usize| p.get(i..i + 8).map(|b| u64::from_le_bytes(b.try_into().unwrap())).ok_or_else(end);
        let orbit = ChunkId(p.get(..32).ok_or_else(end)?.try_into().unwrap());
        let (eps, dc_max) = (f64::from_bits(word(32)?), f64::from_bits(word(40)?));
        let (points, n) = (word(48)?, word(56)?);
        if n >= 63 {
            return Err(malformed("BLA table has too many levels"));
        }
        let steps = points.saturating_sub(2);
        let mut at = 64;
        let mut levels = Vec::with_capacity(n as usize);
        for j in 1..=n {
            let count = usize::try_from(steps >> j).map_err(|_| end())?;
            if p.len() < at + count.saturating_mul(56) {
                return Err(end());
            }
            let mut level = vec![[0.0; 7]; count];
            for f in 0..7 {
                for block in level.iter_mut() {
                    block[f] = f64::from_bits(word(at)?);
                    at += 8;
                }
            }
            levels.push(level);
        }
        let t = BlaTable { orbit, eps, dc_max, points, levels };
        if t.to_chunk()?.bytes() != chunk.bytes() {
            return Err(malformed("BLA table is not in canonical form"));
        }
        Ok(t)
    }
}

fn malformed(m: &str) -> Error {
    Error::Malformed(m.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> BlaTable {
        let block = |k: usize| [k as f64, -0.5, 1.0, 0.25, 1e-12 / (k + 1) as f64, 3e-13, 0.0];
        BlaTable {
            orbit: ChunkId([7; 32]),
            eps: 2f64.powi(-40),
            dc_max: 1e-20,
            points: 11,
            levels: vec![(0..4).map(block).collect(), (0..2).map(block).collect(), (0..1).map(block).collect()],
        }
    }

    #[test]
    fn round_trip_is_canonical() {
        let t = table();
        let c = t.to_chunk().unwrap();
        assert_eq!(c.contract().kind, Kind::BLA);
        assert_eq!(BlaTable::from_chunk(&c).unwrap(), t);
        assert_ne!(BlaTable { dc_max: 2e-20, ..t.clone() }.to_chunk().unwrap().id(), c.id());
        assert_ne!(BlaTable { orbit: ChunkId([8; 32]), ..t }.to_chunk().unwrap().id(), c.id());
    }

    #[test]
    fn bad_tables_are_refused() {
        let mut t = table();
        t.levels[1].pop();
        assert!(t.to_chunk().is_err());
        let mut t = table();
        t.levels[0][3][4] = -1.0;
        assert!(t.to_chunk().is_err());
        let mut t = table();
        t.levels[2][0][0] = f64::INFINITY;
        assert!(t.to_chunk().is_err());
        // An invalid block must be all zero; a level must hold a valid block.
        let mut t = table();
        t.levels[0][1] = [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        assert!(t.to_chunk().is_err());
        let mut t = table();
        t.levels[2][0] = [0.0; 7];
        assert!(t.to_chunk().is_err());
        // Claiming one more level than the payload holds.
        let mut bytes = table().to_chunk().unwrap().bytes().to_vec();
        bytes[32 + 56] = 4;
        assert!(Chunk::from_bytes(bytes).and_then(|c| BlaTable::from_chunk(&c)).is_err());
    }
}
