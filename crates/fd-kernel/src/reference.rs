//! Reference orbit Z_0 = 0, Z_{n+1} = Z_n^2 + C, stored structure-of-arrays as f64.
//! Computed either in f64 (shallow) or in fixed point at the exact centre (deep).
use fd_fixed::Fixed;

/// Reference orbit points `Z_0..Z_len-1`.
pub struct Reference {
    /// Real parts.
    pub re: Vec<f64>,
    /// Imaginary parts.
    pub im: Vec<f64>,
}

impl Reference {
    /// Memory cap: 2^20 points = 16 MiB. Orbit slabs (REF-01) replace this.
    pub const MAX_LEN: usize = 1 << 20;

    /// Iterate until the orbit escapes `escape_radius`, `max_iter` steps are taken, or
    /// `MAX_LEN` points are stored. The final point is kept: a sample that reaches it
    /// rebases to Z_0, so a shorter reference is always valid, just less reused.
    pub fn new(c_re: f64, c_im: f64, max_iter: u64, escape_radius: f64) -> Reference {
        let r2 = escape_radius * escape_radius;
        let steps = max_iter.min(Self::MAX_LEN as u64 - 1);
        // Most references escape early; grow on demand rather than reserving `steps`.
        let cap = steps.min(1 << 16) as usize + 1;
        let (mut re, mut im) = (Vec::with_capacity(cap), Vec::with_capacity(cap));
        let (mut x, mut y) = (0.0f64, 0.0f64);
        re.push(x);
        im.push(y);
        for _ in 0..steps {
            let t = x * x - y * y + c_re;
            y = 2.0 * x * y + c_im;
            x = t;
            re.push(x);
            im.push(y);
            if x * x + y * y > r2 {
                break;
            }
        }
        Reference { re, im }
    }

    /// Same orbit computed in fixed point from the exact centre. Stops at `|Z| > 2^16`
    /// (samples rebase there; their own escape test uses the full radius).
    pub fn from_fixed(cr: &Fixed, ci: &Fixed, max_iter: u64) -> Reference {
        let steps = max_iter.min(Self::MAX_LEN as u64 - 1);
        let cap = steps.min(1 << 16) as usize + 1;
        let (mut re, mut im) = (Vec::with_capacity(cap), Vec::with_capacity(cap));
        re.push(0.0);
        im.push(0.0);
        fd_fixed::orbit(cr, ci, steps, 4294967296.0, |a, b| {
            re.push(a);
            im.push(b);
        });
        Reference { re, im }
    }

    /// Upper bounds `q[m] >= |A_m - Z_m|` on each stored point's distance from the
    /// exact orbit `A` of the exact view centre. `bits`: the fixed-point fraction bits
    /// the orbit was computed with, or `None` for an f64 orbit from `(c_re, c_im)`, the
    /// correctly rounded centre.
    pub(crate) fn error_radius(&self, bits: Option<u64>, c_re: f64, c_im: f64) -> Vec<f64> {
        const U: f64 = f64::EPSILON / 2.0;
        let c1 = c_re.abs() + c_im.abs();
        // Fixed point: three truncated squares per component plus the centre's own
        // truncation, each under one unit of 2^-bits.
        let fixed = bits.map(|b| fd_fixed::exp2i(4 - b as i64).max(f64::MIN_POSITIVE));
        let mut q = Vec::with_capacity(self.len());
        let mut a = 0.0f64; // |A_m - F_m|, F the orbit as computed before rounding to f64
        for m in 0..self.len() {
            let s = (self.re[m] * self.re[m] + self.im[m] * self.im[m]).sqrt() * (1.0 + 4.0 * U);
            let (round, defect) = match fixed {
                Some(d) => (2.0 * U * s + f64::MIN_POSITIVE, d),
                // f64: rounding of x^2 - y^2 + c and 2xy + c, plus the centre's rounding.
                None => (0.0, 8.0 * U * (s * s + c1) + 2.0 * U * c1 + f64::MIN_POSITIVE),
            };
            q.push(a + round);
            a = (a * (2.0 * (s + round) + a) + defect) * (1.0 + 8.0 * U);
        }
        q
    }

    /// Number of stored points (at least 2 unless `max_iter == 0`).
    #[inline]
    pub fn len(&self) -> usize {
        self.re.len()
    }

    /// Always false: `Z_0` is always stored.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.re.is_empty()
    }
}
