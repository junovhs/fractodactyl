//! Sample-grid geometry in the f64 contract: offsets from the reference at the centre.
use fd_samples::View;

/// Centre (reference point) and the transform from grid to complex offsets.
#[derive(Clone, Copy, Debug)]
pub struct Plane {
    /// Centre, real part, rounded to f64.
    pub c_re: f64,
    /// Centre, imaginary part, rounded to f64.
    pub c_im: f64,
    /// Sample spacing in the complex plane.
    pub h: f64,
    cos: f64,
    sin: f64,
    half_x: f64,
    half_y: f64,
}

impl Plane {
    /// Errors when the f64 contract cannot place samples genuinely: the centre's
    /// rounding error must stay below 1/1024 of a sample.
    pub fn new(v: &View, nx: u32, ny: u32) -> Result<Plane, String> {
        let num = |s: &str| s.trim().parse::<f64>().map_err(|e| format!("bad number {s:?}: {e}"));
        let (c_re, c_im, width) = (num(&v.center_re)?, num(&v.center_im)?, num(&v.width)?);
        if !(width.is_finite() && width > 0.0 && c_re.is_finite() && c_im.is_finite()) || nx == 0 || ny == 0 {
            return Err("view needs a finite centre, positive width and a non-empty grid".into());
        }
        let h = width / nx as f64;
        let rounding = c_re.abs().max(c_im.abs()) * f64::EPSILON;
        if rounding > h / 1024.0 {
            return Err(format!(
                "sample spacing {h:e} is too fine for the f64 kernel (centre rounding {rounding:e}); deep views need BASE-02"
            ));
        }
        let (sin, cos) = v.rotation.sin_cos();
        Ok(Plane { c_re, c_im, h, cos, sin, half_x: nx as f64 / 2.0, half_y: ny as f64 / 2.0 })
    }

    /// Offset `dc` of sample `(i, j)` from the centre (see `fd_samples::View`).
    #[inline]
    pub fn dc(&self, i: usize, j: usize) -> (f64, f64) {
        let x = (i as f64 + 0.5 - self.half_x) * self.h;
        let y = (j as f64 + 0.5 - self.half_y) * self.h;
        (self.cos * x + self.sin * y, self.sin * x - self.cos * y)
    }

    /// Complex-plane direction to screen direction (x right, y down).
    #[inline]
    pub fn to_screen(&self, re: f64, im: f64) -> (f64, f64) {
        (self.cos * re + self.sin * im, self.sin * re - self.cos * im)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(rot: f64) -> View {
        View { center_re: "-0.5".into(), center_im: "0.25".into(), width: "4".into(), rotation: rot }
    }

    #[test]
    fn screen_axes_match_sample_layout() {
        let p = Plane::new(&view(0.7), 8, 8).unwrap();
        // Moving one sample right/down in the grid is +x/+y on screen.
        let (a, b) = (p.dc(3, 3), p.dc(4, 3));
        let (x, y) = p.to_screen(b.0 - a.0, b.1 - a.1);
        assert!((x - p.h).abs() < 1e-12 && y.abs() < 1e-12);
        let b = p.dc(3, 4);
        let (x, y) = p.to_screen(b.0 - a.0, b.1 - a.1);
        assert!(x.abs() < 1e-12 && (y - p.h).abs() < 1e-12);
    }

    #[test]
    fn imaginary_axis_points_up() {
        let p = Plane::new(&view(0.0), 8, 8).unwrap();
        assert!(p.dc(0, 0).1 > 0.0 && p.dc(0, 7).1 < 0.0);
    }

    #[test]
    fn refuses_depth_beyond_f64() {
        let v = View { width: "1e-15".into(), ..view(0.0) };
        assert!(Plane::new(&v, 1000, 1000).is_err());
    }
}
