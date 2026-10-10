//! Exact dyadic tile addresses: a quadtree over the square `[-2, 2] x [-2, 2]`.
//!
//! A tile at `level` L has side `2^(2-L)` and integer indices `0 <= x, y < 2^L`, with x
//! growing to the right (real axis) and y growing downward (imaginary axis decreasing),
//! like sample rows. Indices are arbitrary-size integers, so every mapping here is exact
//! at any depth: no tile ever carries a rounded global coordinate. Spec:
//! docs/spec/ADDRESS.md.
mod nat;

pub use nat::Nat;

use fd_fixed::Decimal;
use std::fmt;

/// Deepest level accepted, to bound memory on malformed input (~2^-1e6, far past need).
pub const MAX_LEVEL: u32 = 1 << 20;
/// Largest per-axis sample-grid exponent: a tile holds at most `2^16 x 2^16` samples.
pub const MAX_GRID: u32 = 16;

/// A quadtree cell. Ordering and equality are exact.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Tile {
    pub level: u32,
    pub x: Nat,
    pub y: Nat,
}

impl Tile {
    /// The whole root square.
    pub fn root() -> Tile {
        Tile {
            level: 0,
            x: Nat::zero(),
            y: Nat::zero(),
        }
    }

    /// Checked constructor.
    pub fn new(level: u32, x: Nat, y: Nat) -> Result<Tile, String> {
        if level > MAX_LEVEL {
            return Err(format!("level {level} exceeds {MAX_LEVEL}"));
        }
        if x.bits() > level as u64 || y.bits() > level as u64 {
            return Err(format!("index outside the 2^{level} grid of level {level}"));
        }
        Ok(Tile { level, x, y })
    }

    /// The enclosing tile one level up; `None` for the root.
    pub fn parent(&self) -> Option<Tile> {
        (self.level > 0).then(|| Tile {
            level: self.level - 1,
            x: self.x.shr_bits(1),
            y: self.y.shr_bits(1),
        })
    }

    /// This tile's position in its parent: bit 0 = right half, bit 1 = lower half.
    pub fn quadrant(&self) -> u8 {
        (self.x.low(1) | (self.y.low(1) << 1)) as u8
    }

    /// Child `q` (0..4, same bit meaning as `quadrant`).
    pub fn child(&self, q: u8) -> Tile {
        assert!(q < 4, "quadrant {q} out of range");
        Tile {
            level: self.level + 1,
            x: self.x.shl_bits(1).or_small((q & 1) as u64),
            y: self.y.shl_bits(1).or_small((q >> 1) as u64),
        }
    }

    /// The ancestor `up` levels above (`up <= level`).
    pub fn ancestor(&self, up: u32) -> Tile {
        assert!(up <= self.level, "ancestor above the root");
        Tile {
            level: self.level - up,
            x: self.x.shr_bits(up as u64),
            y: self.y.shr_bits(up as u64),
        }
    }

    /// Sample `(i, j)` of this tile's `2^k x 2^k` sample grid, as the exact cell it covers:
    /// the descendant at level `level + k`. Sample positions are that cell's centre.
    pub fn sample(&self, k: u32, i: u32, j: u32) -> Result<Tile, String> {
        if k > MAX_GRID {
            return Err(format!("grid 2^{k} exceeds 2^{MAX_GRID}"));
        }
        if (i as u64) >> k != 0 || (j as u64) >> k != 0 {
            return Err(format!("sample ({i}, {j}) outside the 2^{k} grid"));
        }
        let x = self.x.shl_bits(k as u64).or_small(i as u64);
        let y = self.y.shl_bits(k as u64).or_small(j as u64);
        Tile::new(self.level + k, x, y)
    }

    /// Inverse of `sample`: the tile `k` levels up that owns this cell, and the cell's
    /// sample indices in that tile's `2^k` grid.
    pub fn owner(&self, k: u32) -> Result<(Tile, u32, u32), String> {
        if k > MAX_GRID || k > self.level {
            return Err(format!(
                "grid 2^{k} does not fit above level {}",
                self.level
            ));
        }
        Ok((self.ancestor(k), self.x.low(k) as u32, self.y.low(k) as u32))
    }

    /// The level-`level` tile containing the point `re + i·im` (decimal strings, exact).
    /// Tiles are half-open: `re_min <= re < re_max`, `im_min < im <= im_max`.
    pub fn locate(re: &str, im: &str, level: u32) -> Result<Tile, String> {
        let x = index(&Decimal::parse(re)?, false, level)
            .ok_or_else(|| format!("re {re} outside [-2, 2) or too long"))?;
        let y = index(&Decimal::parse(im)?, true, level)
            .ok_or_else(|| format!("im {im} outside (-2, 2] or too long"))?;
        Tile::new(level, x, y)
    }

    /// Exact decimal real part of the centre.
    pub fn center_re(&self) -> String {
        // -2 + (2x+1)·2^(1-L) = ((2x+1) - 2^L)·2^(1-L)
        signed_dyadic(
            &self.x.shl_bits(1).or_small(1),
            &Nat::pow2(self.level as u64),
            self.level as i64 - 1,
        )
    }

    /// Exact decimal imaginary part of the centre.
    pub fn center_im(&self) -> String {
        // 2 - (2y+1)·2^(1-L) = (2^L - (2y+1))·2^(1-L)
        signed_dyadic(
            &Nat::pow2(self.level as u64),
            &self.y.shl_bits(1).or_small(1),
            self.level as i64 - 1,
        )
    }

    /// Exact decimal side length, `2^(2-L)`.
    pub fn side(&self) -> String {
        dyadic(&Nat::small(1), self.level as i64 - 2)
    }
}

/// Canonical text key `level/xhex/yhex` (lowercase, no leading zeros).
impl fmt::Display for Tile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}/{}", self.level, self.x.to_hex(), self.y.to_hex())
    }
}

impl std::str::FromStr for Tile {
    type Err = String;

    /// Accepts exactly the canonical form `Display` produces.
    fn from_str(s: &str) -> Result<Tile, String> {
        let bad = || format!("not a tile key (level/xhex/yhex): {s:?}");
        let mut parts = s.split('/');
        let (Some(l), Some(x), Some(y), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(bad());
        };
        let canonical = !l.is_empty()
            && l.bytes().all(|b| b.is_ascii_digit())
            && (l == "0" || !l.starts_with('0'));
        let level = l
            .parse::<u32>()
            .ok()
            .filter(|_| canonical)
            .ok_or_else(bad)?;
        Tile::new(
            level,
            Nat::from_hex(x).ok_or_else(bad)?,
            Nat::from_hex(y).ok_or_else(bad)?,
        )
    }
}

/// `floor((2 ± v)·2^(L-2))` for x (`flip` false: `v + 2`) or y (`flip` true: `2 - v`),
/// or `None` when the result is outside `[0, 2^L)`.
fn index(d: &Decimal, flip: bool, level: u32) -> Option<Nat> {
    // Refuse absurd exponents rather than build enormous integers (|v| > 2 or finer
    // than any supported level either way).
    if d.exp10.unsigned_abs() > 4 * MAX_LEVEL as u64 {
        return None;
    }
    // v = n / 10^e exactly, with n and e integers, e >= 0.
    let n = Nat::from_digits(&d.digits);
    let (n, e) = if d.exp10 >= 0 {
        (n.mul_pow10(d.exp10 as u64), 0)
    } else {
        (n, d.exp10.unsigned_abs())
    };
    let two = Nat::small(2).mul_pow10(e);
    let num = if d.neg != flip {
        two.minus(&n)?
    } else {
        two.plus(&n)
    };
    // floor(num·2^(L-2) / (2^e·5^e)): exact as successive floors of non-negative integers.
    let l = level as i64 - 2;
    let num = if l > 0 { num.shl_bits(l as u64) } else { num };
    let idx = num
        .shr_bits(e + if l < 0 { l.unsigned_abs() } else { 0 })
        .div_pow5(e);
    (idx.bits() <= level as u64).then_some(idx)
}

/// Exact decimal of `(a - b)·2^-f`.
fn signed_dyadic(a: &Nat, b: &Nat, f: i64) -> String {
    match a.minus(b) {
        Some(m) => dyadic(&m, f),
        None => format!("-{}", dyadic(&b.minus(a).unwrap(), f)),
    }
}

/// Exact decimal of `m·2^-f`: `m·5^f / 10^f` for f > 0, so exactly f fraction digits
/// before trailing zeros are trimmed.
fn dyadic(m: &Nat, f: i64) -> String {
    if f <= 0 {
        return m.shl_bits(f.unsigned_abs()).to_decimal();
    }
    let f = f as u64;
    let digits = m.mul_pow5(f).to_decimal();
    let digits = format!("{digits:0>width$}", width = f as usize + 1);
    let (int, frac) = digits.split_at(digits.len() - f as usize);
    let frac = frac.trim_end_matches('0');
    if frac.is_empty() {
        int.to_string()
    } else {
        format!("{int}.{frac}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(s: &str) -> Tile {
        s.parse().unwrap()
    }

    #[test]
    fn shallow_hand_checked() {
        let r = Tile::root();
        assert_eq!(
            (r.center_re(), r.center_im(), r.side()),
            ("0".into(), "0".into(), "4".into())
        );
        // Level 1, lower-right quadrant: [0,2] x [-2,0], centre 1 - 1i.
        let t = r.child(3);
        assert_eq!(t.to_string(), "1/1/1");
        assert_eq!(
            (t.center_re(), t.center_im(), t.side()),
            ("1".into(), "-1".into(), "2".into())
        );
        // Level 3, x=1, y=6: re [-1.5,-1), im [-1.5,-1); centre -1.25 - 1.25i.
        let t = key("3/1/6");
        assert_eq!(
            (t.center_re(), t.center_im(), t.side()),
            ("-1.25".into(), "-1.25".into(), "0.5".into())
        );
        assert_eq!(Tile::locate("-1.25", "-1.25", 3).unwrap(), t);
        // Half-open edges: re = -1.5 belongs to x=1; im = -1 belongs to y=6 (the upper edge).
        assert_eq!(Tile::locate("-1.5", "-1", 3).unwrap(), t);
        assert_eq!(Tile::locate("-1.0000001", "-1.4999999", 3).unwrap(), t);
        assert_eq!(Tile::locate("-1", "-1.25", 3).unwrap().to_string(), "3/2/6");
        assert_eq!(Tile::locate("-2", "2", 0).unwrap(), r);
        assert!(Tile::locate("2", "0", 4).is_err());
        assert!(Tile::locate("0", "-2", 4).is_err());
        assert!(Tile::locate("-2.0000001", "0", 4).is_err());
        assert_eq!(
            Tile::locate("1.5e0", "25e-2", 2).unwrap().to_string(),
            "2/3/1"
        );
    }

    #[test]
    fn family_round_trips() {
        let t = key("5/13/1e");
        for q in 0..4 {
            let c = t.child(q);
            assert_eq!(c.quadrant(), q);
            assert_eq!(c.parent().unwrap(), t);
        }
        assert_eq!(Tile::root().parent(), None);
        assert_eq!(t.ancestor(5), Tile::root());
    }

    #[test]
    fn keys_are_canonical() {
        for s in ["0/0/0", "1/1/0", "70/3fffffffffffffffff/0"] {
            assert_eq!(key(s).to_string(), s);
        }
        for s in [
            "", "1/2/0", "01/0/0", "1/0", "1/0/0/0", "2/01/0", "x/0/0", "1/A/0", "+1/0/0",
        ] {
            assert!(s.parse::<Tile>().is_err(), "{s}");
        }
    }

    #[test]
    fn samples_are_descendant_cells() {
        let t = key("4/9/2");
        let s = t.sample(7, 100, 3).unwrap();
        assert_eq!(s.level, 11);
        assert_eq!(s.owner(7).unwrap(), (t.clone(), 100, 3));
        // The same cell in the parent's 2^8 grid: offset by half a grid on odd x (9), not
        // on even y (2).
        let (p, i, j) = s.owner(8).unwrap();
        assert_eq!((p, i, j), (t.parent().unwrap(), 128 + 100, 3));
        assert_eq!(t.parent().unwrap().sample(8, 228, 3).unwrap(), s);
        assert!(t.sample(7, 128, 0).is_err());
        assert!(t.sample(17, 0, 0).is_err());
        assert!(t.owner(5).is_err());
    }

    /// A point near the 1e-1000-deep location in bench/locations.txt style.
    const RE: &str =
        "-0.74364388703715870475219150611477100000000000000000000000000001234567890123e0";
    const IM: &str = "0.131825904205311970493132056385139000000000000000000000000000000987654321";

    #[test]
    fn extreme_depth_round_trips() {
        // 2^-3400 ~ 1e-1023: deeper than a 1e-1000 view.
        let level = 3400;
        let t = Tile::locate(RE, IM, level).unwrap();
        assert_eq!(t.level, level);
        assert!(t.x.bits() > 3390 && t.y.bits() > 3390);
        // Key text, centre and parent/child are exact at this depth.
        assert_eq!(t.to_string().parse::<Tile>().unwrap(), t);
        assert_eq!(
            Tile::locate(&t.center_re(), &t.center_im(), level).unwrap(),
            t
        );
        assert_eq!(t.parent().unwrap().child(t.quadrant()), t);
        // Coarser locates are exactly the ancestors (floor is consistent across levels).
        for up in [1, 63, 64, 65, 1000, 3400] {
            assert_eq!(
                Tile::locate(RE, IM, level - up).unwrap(),
                t.ancestor(up),
                "{up}"
            );
        }
        // Sample cells at this depth and back.
        let s = t.sample(16, 65535, 1).unwrap();
        assert_eq!(s.owner(16).unwrap(), (t.clone(), 65535, 1));
        assert_eq!(
            Tile::locate(&s.center_re(), &s.center_im(), s.level).unwrap(),
            s
        );
        // Side is exact: 2^-3398 has 3398 fraction digits.
        assert_eq!(t.side().len(), 2 + 3398);
    }

    #[test]
    fn centre_lies_inside_its_tile() {
        // Every child centre locates to that child, at shallow and deep levels.
        let deep = Tile::locate(RE, IM, 3333).unwrap();
        for t in [Tile::root(), key("2/1/3"), deep] {
            for q in 0..4 {
                let c = t.child(q);
                assert_eq!(
                    Tile::locate(&c.center_re(), &c.center_im(), c.level).unwrap(),
                    c
                );
            }
        }
    }
}
