//! Reference orbit Z_0 = 0, Z_{n+1} = Z_n^2 + C, stored structure-of-arrays.

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
