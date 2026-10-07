//! Canonical chunk encoding (docs/spec/ATLAS.md): a fixed 32-byte header that states the
//! numeric contract, then the payload, zero-padded to 8 bytes. Every field is written
//! explicitly in little-endian order; no host struct layout is ever copied. A [`Chunk`]
//! only exists in canonical form, so equal semantic content always has equal bytes.
use crate::{ChunkId, Error};
use std::fmt;

/// Chunk magic.
pub const MAGIC: [u8; 8] = *b"FDCHUNK\0";
/// Bumped for any change in the meaning or layout of the header.
pub const MAJOR: u16 = 1;
/// Bumped for additive header changes; part of the bytes, so part of the identity.
pub const MINOR: u16 = 0;
/// Header bytes before the payload.
pub const HEADER_LEN: usize = 32;
/// The canonical NaN: every NaN is written with this one bit pattern.
pub const CANONICAL_NAN: u64 = 0x7ff8_0000_0000_0000;
const CANONICAL_NAN_F32: u32 = 0x7fc0_0000;

/// What a chunk holds (ARCHITECTURE.md "Candidate semantic chunks"). Payloads are
/// mathematical data, never raster imagery (DEC-04).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Kind(pub u16);

impl Kind {
    /// Reference orbit range.
    pub const ORBIT_SLAB: Kind = Kind(1);
    /// Bilinear-approximation operators.
    pub const BLA: Kind = Kind(2);
    /// Experimental return-map operators.
    pub const RETURN_MAP: Kind = Kind(3);
    /// Proved claims over an exact region.
    pub const CERTIFICATE: Kind = Kind(4);
    /// Exact-coordinate samples.
    pub const EXACT_SAMPLE: Kind = Kind(5);
    /// A whole `.fds` sample file (docs/spec/SAMPLES.md), stored byte for byte.
    pub const SAMPLES: Kind = Kind(6);
    /// A tile manifest (`fd_atlas::TileManifest`).
    pub const TILE_MANIFEST: Kind = Kind(7);
    /// A frame manifest (`fd_atlas::FrameManifest`).
    pub const FRAME_MANIFEST: Kind = Kind(8);
    /// A reference orbit bound to its exact centre (`fd_atlas::OrbitManifest`).
    pub const ORBIT_MANIFEST: Kind = Kind(9);

    const NAMES: [(Kind, &'static str); 9] = [
        (Kind::ORBIT_SLAB, "orbit-slab"),
        (Kind::BLA, "bla"),
        (Kind::RETURN_MAP, "return-map"),
        (Kind::CERTIFICATE, "certificate"),
        (Kind::EXACT_SAMPLE, "exact-sample"),
        (Kind::SAMPLES, "samples"),
        (Kind::TILE_MANIFEST, "tile-manifest"),
        (Kind::FRAME_MANIFEST, "frame-manifest"),
        (Kind::ORBIT_MANIFEST, "orbit-manifest"),
    ];

    /// The registered name, if this code has one.
    pub fn name(self) -> Option<&'static str> {
        Kind::NAMES.iter().find(|(k, _)| *k == self).map(|(_, n)| *n)
    }
}

impl std::str::FromStr for Kind {
    type Err = String;
    fn from_str(s: &str) -> Result<Kind, String> {
        if let Some((k, _)) = Kind::NAMES.iter().find(|(_, n)| *n == s) {
            return Ok(*k);
        }
        match s.parse::<u16>() {
            Ok(0) | Err(_) => Err(format!("unknown chunk kind {s:?}")),
            Ok(n) => Ok(Kind(n)),
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.name() {
            Some(n) => f.write_str(n),
            None => write!(f, "{}", self.0),
        }
    }
}

/// The iterated formula the payload describes; 0 means formula-independent data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Formula(pub u16);

impl Formula {
    /// Formula-independent data.
    pub const NONE: Formula = Formula(0);
    /// `z -> z^2 + c`.
    pub const MANDELBROT: Formula = Formula(1);
}

impl std::str::FromStr for Formula {
    type Err = String;
    fn from_str(s: &str) -> Result<Formula, String> {
        match s {
            "none" => Ok(Formula::NONE),
            "mandelbrot" => Ok(Formula::MANDELBROT),
            _ => s.parse().map(Formula).map_err(|_| format!("unknown formula {s:?}")),
        }
    }
}

impl fmt::Display for Formula {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Formula::NONE => f.write_str("none"),
            Formula::MANDELBROT => f.write_str("mandelbrot"),
            Formula(n) => write!(f, "{n}"),
        }
    }
}

/// How the payload's numbers relate to the true values they stand for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Rounding {
    /// No rounding: integers, exact dyadics, exact decimals.
    Exact = 0,
    /// Round to nearest, ties to even (IEEE 754 default).
    Nearest = 1,
    /// Directed outward: every stored bound encloses the true value.
    Outward = 2,
}

impl Rounding {
    fn from_u8(v: u8) -> Option<Rounding> {
        [Rounding::Exact, Rounding::Nearest, Rounding::Outward].into_iter().find(|r| *r as u8 == v)
    }
}

impl std::str::FromStr for Rounding {
    type Err = String;
    fn from_str(s: &str) -> Result<Rounding, String> {
        match s {
            "exact" => Ok(Rounding::Exact),
            "nearest" => Ok(Rounding::Nearest),
            "outward" => Ok(Rounding::Outward),
            _ => Err(format!("unknown rounding {s:?} (exact|nearest|outward)")),
        }
    }
}

impl fmt::Display for Rounding {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Rounding::Exact => "exact",
            Rounding::Nearest => "nearest",
            Rounding::Outward => "outward",
        })
    }
}

/// Everything a reader must know to interpret the payload. Part of the chunk bytes, so
/// the same numbers under a different contract are a different chunk.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Contract {
    /// What the payload holds.
    pub kind: Kind,
    /// Payload layout version for this kind.
    pub encoding: u16,
    /// Iterated formula the payload describes.
    pub formula: Formula,
    /// Significand bits of the payload's working precision (53 for f64; 0 if exact).
    pub precision_bits: u32,
    /// How payload numbers relate to true values.
    pub rounding: Rounding,
}

/// An immutable chunk in canonical byte form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chunk {
    contract: Contract,
    bytes: Vec<u8>,
}

impl Chunk {
    /// Wrap already-canonical payload bytes (for example a `.fds` file, whose own format
    /// is canonical). Numeric payloads should be built with [`Builder`] instead.
    pub fn new(contract: Contract, payload: &[u8]) -> Chunk {
        let mut b = Builder::new(contract);
        b.raw(payload);
        b.finish()
    }

    /// Parse and validate canonical bytes; anything non-canonical is refused.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Chunk, Error> {
        let bad = |m: &str| Err(Error::Malformed(m.to_string()));
        if bytes.len() < HEADER_LEN {
            return bad("shorter than the chunk header");
        }
        let u16_at = |o: usize| u16::from_le_bytes([bytes[o], bytes[o + 1]]);
        let u32_at = |o: usize| u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
        if bytes[..8] != MAGIC {
            return bad("bad magic: not a chunk");
        }
        if u16_at(8) != MAJOR {
            return Err(Error::Malformed(format!("unsupported chunk major version {}", u16_at(8))));
        }
        let kind = Kind(u16_at(12));
        if kind.0 == 0 {
            return bad("chunk kind 0 is invalid");
        }
        let Some(rounding) = Rounding::from_u8(bytes[18]) else {
            return bad("unknown rounding contract");
        };
        if bytes[19] != 0 {
            return bad("reserved header byte is not zero");
        }
        let len = u64::from_le_bytes(bytes[24..32].try_into().unwrap());
        let padded = len.checked_add(7).map(|l| l & !7);
        if padded != Some((bytes.len() - HEADER_LEN) as u64) {
            return bad("payload length does not match the chunk size");
        }
        if bytes[HEADER_LEN + len as usize..].iter().any(|&b| b != 0) {
            return bad("payload padding is not zero");
        }
        let contract =
            Contract { kind, encoding: u16_at(14), formula: Formula(u16_at(16)), precision_bits: u32_at(20), rounding };
        Ok(Chunk { contract, bytes })
    }

    /// The numeric contract from the header.
    pub fn contract(&self) -> &Contract {
        &self.contract
    }

    /// Header minor version.
    pub fn minor(&self) -> u16 {
        u16::from_le_bytes([self.bytes[10], self.bytes[11]])
    }

    /// The payload, without header or padding.
    pub fn payload(&self) -> &[u8] {
        let len = u64::from_le_bytes(self.bytes[24..32].try_into().unwrap()) as usize;
        &self.bytes[HEADER_LEN..HEADER_LEN + len]
    }

    /// The canonical bytes: exactly what the store keeps and hashes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Content address: SHA-256 of [`Chunk::bytes`].
    pub fn id(&self) -> ChunkId {
        ChunkId::of(&self.bytes)
    }
}

/// Appends payload fields in canonical form. Floats are normalised: `-0.0` becomes
/// `+0.0` and every NaN becomes [`CANONICAL_NAN`], so equal values give equal bytes.
pub struct Builder {
    contract: Contract,
    payload: Vec<u8>,
}

impl Builder {
    /// Start an empty payload under `contract`.
    pub fn new(contract: Contract) -> Builder {
        Builder { contract, payload: Vec::new() }
    }

    /// Append a byte.
    pub fn u8(&mut self, v: u8) -> &mut Self {
        self.raw(&[v])
    }
    /// Append a little-endian `u16`.
    pub fn u16(&mut self, v: u16) -> &mut Self {
        self.raw(&v.to_le_bytes())
    }
    /// Append a little-endian `u32`.
    pub fn u32(&mut self, v: u32) -> &mut Self {
        self.raw(&v.to_le_bytes())
    }
    /// Append a little-endian `u64`.
    pub fn u64(&mut self, v: u64) -> &mut Self {
        self.raw(&v.to_le_bytes())
    }
    /// Append a little-endian `i64`.
    pub fn i64(&mut self, v: i64) -> &mut Self {
        self.raw(&v.to_le_bytes())
    }
    /// Append an `f64` in canonical bits.
    pub fn f64(&mut self, v: f64) -> &mut Self {
        self.u64(canonical_f64(v))
    }
    /// Append an `f32` in canonical bits.
    pub fn f32(&mut self, v: f32) -> &mut Self {
        let bits = if v == 0.0 { 0 } else if v.is_nan() { CANONICAL_NAN_F32 } else { v.to_bits() };
        self.u32(bits)
    }
    /// A `u64` length followed by the bytes.
    pub fn bytes(&mut self, v: &[u8]) -> &mut Self {
        self.u64(v.len() as u64).raw(v)
    }
    /// A UTF-8 string, length-prefixed like [`Builder::bytes`].
    pub fn str(&mut self, v: &str) -> &mut Self {
        self.bytes(v.as_bytes())
    }
    /// Bytes the caller guarantees are already canonical.
    pub fn raw(&mut self, v: &[u8]) -> &mut Self {
        self.payload.extend_from_slice(v);
        self
    }

    /// Write the header and padding; the builder is left empty.
    pub fn finish(&mut self) -> Chunk {
        let c = self.contract;
        let payload = std::mem::take(&mut self.payload);
        let mut bytes = Vec::with_capacity(HEADER_LEN + payload.len() + 7);
        bytes.extend_from_slice(&MAGIC);
        bytes.extend_from_slice(&MAJOR.to_le_bytes());
        bytes.extend_from_slice(&MINOR.to_le_bytes());
        bytes.extend_from_slice(&c.kind.0.to_le_bytes());
        bytes.extend_from_slice(&c.encoding.to_le_bytes());
        bytes.extend_from_slice(&c.formula.0.to_le_bytes());
        bytes.push(c.rounding as u8);
        bytes.push(0);
        bytes.extend_from_slice(&c.precision_bits.to_le_bytes());
        bytes.extend_from_slice(&(payload.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&payload);
        bytes.resize(bytes.len().next_multiple_of(8), 0);
        Chunk { contract: c, bytes }
    }
}

/// Bits of `v` with signed zero and NaN normalised.
pub fn canonical_f64(v: f64) -> u64 {
    if v == 0.0 {
        0
    } else if v.is_nan() {
        CANONICAL_NAN
    } else {
        v.to_bits()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract() -> Contract {
        Contract {
            kind: Kind::ORBIT_SLAB,
            encoding: 1,
            formula: Formula::MANDELBROT,
            precision_bits: 53,
            rounding: Rounding::Nearest,
        }
    }

    #[test]
    fn signed_zero_and_nan_are_normalised() {
        let a = Builder::new(contract()).f64(0.0).f64(f64::NAN).f32(-0.0).finish();
        let b = Builder::new(contract()).f64(-0.0).f64(-f64::NAN).f32(0.0).finish();
        assert_eq!(a.bytes(), b.bytes());
        assert_eq!(a.id(), b.id());
    }

    #[test]
    fn round_trip_and_layout() {
        let c = Builder::new(contract()).u8(7).str("ab").f64(1.5).finish();
        assert_eq!(c.bytes().len() % 8, 0);
        assert_eq!(c.payload().len(), 1 + 8 + 2 + 8);
        let back = Chunk::from_bytes(c.bytes().to_vec()).unwrap();
        assert_eq!(back, c);
        assert_eq!(back.contract(), &contract());
    }

    #[test]
    fn contract_is_part_of_identity() {
        let a = Chunk::new(contract(), b"same");
        let b = Chunk::new(Contract { precision_bits: 128, ..contract() }, b"same");
        assert_ne!(a.id(), b.id());
    }

    #[test]
    fn non_canonical_bytes_are_refused() {
        let good = Chunk::new(contract(), b"xyz").bytes().to_vec();
        let mut pad = good.clone();
        *pad.last_mut().unwrap() = 1;
        let mut reserved = good.clone();
        reserved[19] = 1;
        let mut long = good.clone();
        long.extend_from_slice(&[0; 8]);
        let mut rounding = good.clone();
        rounding[18] = 9;
        for bytes in [pad, reserved, long, rounding, good[..20].to_vec()] {
            assert!(matches!(Chunk::from_bytes(bytes), Err(Error::Malformed(_))));
        }
    }
}
