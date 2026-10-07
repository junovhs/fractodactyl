//! Reference orbit slabs (docs/spec/ATLAS.md "Orbit slabs"): fixed semantic ranges
//! `Z_start..Z_start+len` of one reference orbit, `start` a multiple of the slab size.
//! Boundaries depend only on the iteration index, never on content, so the same orbit
//! prefix always cuts into the same chunks.
use crate::{Builder, Chunk, Contract, Error, Formula, Kind, Rounding};

/// Payload layout version of orbit slabs: f64 structure-of-arrays.
pub const SLAB_ENCODING: u16 = 1;

/// Points `Z_start..Z_start+re.len()` of one reference orbit, as f64.
#[derive(Clone, Debug, PartialEq)]
pub struct OrbitSlab {
    /// Points per full slab; `start` is a multiple of it.
    pub size: u32,
    /// Iteration index of the first point.
    pub start: u64,
    /// Real parts.
    pub re: Vec<f64>,
    /// Imaginary parts.
    pub im: Vec<f64>,
}

impl OrbitSlab {
    /// Cut an orbit into slabs of `size` points; only the last may be shorter.
    pub fn split(re: &[f64], im: &[f64], size: u32) -> Result<Vec<OrbitSlab>, Error> {
        if size == 0 || re.len() != im.len() {
            return Err(malformed("orbit slabs need a positive size and equal re/im lengths"));
        }
        let n = size as usize;
        let slabs = re.chunks(n).zip(im.chunks(n)).enumerate();
        Ok(slabs.map(|(i, (r, m))| OrbitSlab { size, start: (i * n) as u64, re: r.to_vec(), im: m.to_vec() }).collect())
    }

    /// Reassemble an orbit from its slabs in order: one size, contiguous from `Z_0`,
    /// only the last slab short.
    pub fn join(slabs: &[OrbitSlab]) -> Result<(Vec<f64>, Vec<f64>), Error> {
        let (mut re, mut im) = (Vec::new(), Vec::new());
        for (i, s) in slabs.iter().enumerate() {
            let last = i + 1 == slabs.len();
            if s.size != slabs[0].size || s.start != re.len() as u64 || (!last && s.re.len() != s.size as usize) {
                return Err(malformed("orbit slabs are not one contiguous orbit from Z_0"));
            }
            re.extend_from_slice(&s.re);
            im.extend_from_slice(&s.im);
        }
        Ok((re, im))
    }

    /// Canonical chunk. `precision_bits` is the working precision of the orbit (53 for
    /// f64, the fixed-point fraction bits otherwise); points are rounded to nearest f64.
    pub fn to_chunk(&self, precision_bits: u32) -> Result<Chunk, Error> {
        let len = self.re.len();
        let ok = self.size > 0
            && (1..=self.size as usize).contains(&len)
            && self.im.len() == len
            && self.start.is_multiple_of(self.size as u64)
            && self.re.iter().chain(&self.im).all(|v| v.is_finite());
        if !ok {
            return Err(malformed("orbit slab needs 1..=size finite points at a multiple of size"));
        }
        let mut b = Builder::new(contract(precision_bits));
        b.u32(self.size).u32(len as u32).u64(self.start);
        for &v in self.re.iter().chain(&self.im) {
            b.f64(v);
        }
        Ok(b.finish())
    }

    /// Decode an orbit slab chunk, refusing anything not in canonical form.
    pub fn from_chunk(chunk: &Chunk) -> Result<OrbitSlab, Error> {
        let c = chunk.contract();
        if *c != contract(c.precision_bits) {
            return Err(malformed("not an orbit slab chunk (kind, encoding, formula or rounding)"));
        }
        let p = chunk.payload();
        let word = |i: usize| p.get(i..i + 8).map(|b| u64::from_le_bytes(b.try_into().unwrap()));
        let head = word(0).ok_or_else(|| malformed("orbit slab payload ends early"))?;
        let (size, len) = (head as u32, (head >> 32) as usize);
        let start = word(8).ok_or_else(|| malformed("orbit slab payload ends early"))?;
        let vals = (0..2 * len)
            .map(|k| word(16 + 8 * k).map(f64::from_bits))
            .collect::<Option<Vec<f64>>>()
            .ok_or_else(|| malformed("orbit slab payload ends early"))?;
        let (re, im) = vals.split_at(len);
        let s = OrbitSlab { size, start, re: re.to_vec(), im: im.to_vec() };
        if s.to_chunk(c.precision_bits)?.bytes() != chunk.bytes() {
            return Err(malformed("orbit slab is not in canonical form"));
        }
        Ok(s)
    }
}

fn contract(precision_bits: u32) -> Contract {
    Contract {
        kind: Kind::ORBIT_SLAB,
        encoding: SLAB_ENCODING,
        formula: Formula::MANDELBROT,
        precision_bits,
        rounding: Rounding::Nearest,
    }
}

fn malformed(m: &str) -> Error {
    Error::Malformed(m.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_round_trips_through_chunks() {
        let re: Vec<f64> = (0..5000).map(|i| (i as f64).sin()).collect();
        let im: Vec<f64> = (0..5000).map(|i| (i as f64).cos()).collect();
        for size in [2048, 4096, 8192] {
            let slabs = OrbitSlab::split(&re, &im, size).unwrap();
            assert_eq!(slabs.len(), 5000usize.div_ceil(size as usize));
            let back: Vec<OrbitSlab> =
                slabs.iter().map(|s| OrbitSlab::from_chunk(&s.to_chunk(200).unwrap()).unwrap()).collect();
            assert_eq!(OrbitSlab::join(&back).unwrap(), (re.clone(), im.clone()));
        }
    }

    #[test]
    fn bad_slabs_are_refused() {
        let s = OrbitSlab::split(&[0.0, 1.0, 2.0], &[0.0; 3], 2).unwrap();
        assert!(OrbitSlab::join(&[s[1].clone(), s[0].clone()]).is_err());
        let mut odd = s[0].clone();
        odd.start = 1;
        assert!(odd.to_chunk(53).is_err());
        let mut bytes = s[0].to_chunk(53).unwrap().bytes().to_vec();
        bytes[32 + 4] = 3; // claims 3 points
        assert!(Chunk::from_bytes(bytes).and_then(|c| OrbitSlab::from_chunk(&c)).is_err());
    }
}
