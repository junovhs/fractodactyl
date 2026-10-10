//! Columns: each is one dense little-endian array of `nx * ny` elements.
//!
//! Columns appear in the file in bit order. New columns always take higher bits,
//! so a reader that knows bits 0..k finds them at the same offsets regardless of
//! later additions (minor version bump only).

/// One per-sample quantity.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Column {
    /// `u8` packed `Class`. Always present.
    Class = 0,
    /// `f64` smooth escape value `nu = n + 1 - log2(log2|z_n|)`. Escaped samples only.
    Nu = 1,
    /// `f32` exterior distance estimate in output pixels. Escaped samples only.
    De = 2,
    /// `u16` screen-space normal angle, turns * 65536. Escaped samples only.
    Normal = 3,
    /// `f32` absolute error bound on `nu`. Present only when evidence >= Bounded.
    Bound = 4,
}

impl Column {
    /// Every v1 column, in file (bit) order.
    pub const ALL: [Column; 5] = [
        Column::Class,
        Column::Nu,
        Column::De,
        Column::Normal,
        Column::Bound,
    ];

    /// Bytes per element.
    #[inline]
    pub const fn width(self) -> usize {
        match self {
            Column::Class => 1,
            Column::Nu => 8,
            Column::De | Column::Bound => 4,
            Column::Normal => 2,
        }
    }

    /// This column's bit in a `ColumnSet`.
    #[inline]
    pub const fn bit(self) -> u32 {
        1 << self as u32
    }
}

/// A set of columns as a bitmask.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ColumnSet(pub u32);

impl ColumnSet {
    /// Mask of the columns this version understands.
    pub const KNOWN: u32 = (1 << Column::ALL.len()) - 1;

    /// The set containing exactly `cols`.
    pub fn of(cols: &[Column]) -> ColumnSet {
        ColumnSet(cols.iter().fold(0, |m, c| m | c.bit()))
    }

    /// Whether `c` is in the set.
    #[inline]
    pub fn has(self, c: Column) -> bool {
        self.0 & c.bit() != 0
    }

    /// The set plus `c`.
    pub fn with(self, c: Column) -> ColumnSet {
        ColumnSet(self.0 | c.bit())
    }

    /// Any derivative-based column requested (lets the kernel skip `dz/dc`).
    #[inline]
    pub fn needs_derivative(self) -> bool {
        self.has(Column::De) || self.has(Column::Normal)
    }

    /// Present known columns in file order.
    pub fn iter(self) -> impl Iterator<Item = Column> {
        Column::ALL.into_iter().filter(move |c| self.has(*c))
    }
}
