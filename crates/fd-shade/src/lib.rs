//! Looks: late appearance passes. Each declares the columns it reads and never
//! recomputes fractal math: recolouring costs one pass over stored samples (DEC-07).
//! This crate depends on `fd-samples` only, never on a kernel, so a look cannot
//! iterate (a test below holds that line).
mod palette;
mod png;
mod relief;
mod resolve;
mod umber;

pub use png::encode as png;
pub use resolve::Rgb8;

use fd_samples::{ColumnSet, Header, Samples};

/// A late pass: which columns it needs, and how it turns samples into pixels.
pub trait Pass {
    /// Columns this pass reads; the reader loads nothing else.
    fn columns(&self) -> ColumnSet;
    /// Turn samples into an image at appearance `a` (time, animation, anti-aliasing).
    /// Must not recompute any fractal math.
    fn shade_with(&self, h: &Header, s: &Samples, a: &Appearance) -> Rgb8;
    /// [`Pass::shade_with`] at [`Appearance::STILL`]: the look as it always was.
    fn shade(&self, h: &Header, s: &Samples) -> Rgb8 {
        self.shade_with(h, s, &Appearance::STILL)
    }
}

/// How a look is applied, beyond the samples (FX-01): absolute time for animation, the
/// animation rates (the explorer's knobs, same maths), band anti-aliasing and how
/// unresolved samples are drawn. [`Appearance::STILL`] reproduces every look exactly
/// as it was before appearance existed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Appearance {
    /// Seconds since the start of the film (frame / fps).
    pub time: f64,
    /// Band phase speed, cycles per second (bands move by `-flow * time`).
    pub flow: f64,
    /// Band density swing: density times `1 + breathe * sin(2 pi brate time)`.
    pub breathe: f64,
    /// Breathing rate, cycles per second.
    pub brate: f64,
    /// Hue drift, cycles per second (palette look only).
    pub drift: f64,
    /// Fade relief lighting where filaments are finer than the sample spacing.
    pub aa: bool,
    /// Draw unresolved samples as interior instead of marking them.
    pub unresolved_interior: bool,
}

impl Appearance {
    /// No animation, no anti-aliasing, unresolved samples marked.
    pub const STILL: Appearance =
        Appearance { time: 0.0, flow: 0.0, breathe: 0.0, brate: 0.0, drift: 0.0, aa: false, unresolved_interior: false };

    /// Band phase offset at this time, in cycles.
    pub fn phase(&self) -> f32 {
        (-self.flow * self.time).rem_euclid(1.0) as f32
    }

    /// Band density factor at this time.
    pub fn density(&self) -> f32 {
        (1.0 + self.breathe * (std::f64::consts::TAU * self.brate * self.time).sin()) as f32
    }

    /// Hue offset at this time, in cycles.
    pub fn hue(&self) -> f32 {
        (self.drift * self.time).rem_euclid(1.0) as f32
    }

    /// Weight of the relief lighting's contrast at a sample `de` output pixels from the
    /// boundary in a grid of `ss` samples per pixel (1 when `aa` is off). Within about a
    /// sample of a filament the normal turns over faster than the grid resolves, so its
    /// lighting is noise that shimmers as the camera moves; it fades to the flat-surface
    /// value there. Checked against ss 3 renders of v0 band frames (FX-01): the fade
    /// brought ss 1 renders 12-35% closer to them, where filtering the colour bands
    /// (which are wide at depth) made them worse.
    #[inline]
    pub fn relief_gain(&self, de: f32, ss: u32) -> f32 {
        if !self.aa {
            return 1.0;
        }
        let d = de * ss as f32 / AA_WIDTH;
        if d.is_nan() {
            0.0
        } else {
            d.max(0.0).tanh()
        }
    }
}

/// Distance from the boundary, in samples, over which `--aa` fades relief lighting.
const AA_WIDTH: f32 = 0.7;

/// Look up a pass by name.
pub fn by_name(name: &str) -> Option<Box<dyn Pass>> {
    match name {
        "umber" => Some(Box::new(umber::Umber)),
        "palette" => Some(Box::new(palette::Palette)),
        "relief" => Some(Box::new(relief::Relief)),
        _ => None,
    }
}

/// Names accepted by `by_name`; the first is the default look.
pub const NAMES: [&str; 3] = ["umber", "palette", "relief"];

#[cfg(test)]
mod tests {

    use crate::Appearance;
    use fd_samples::{Class, Column, ColumnSet, Evidence, Header, Kind, Samples, View, MINOR};

    /// A synthetic 64x36 ss-2 grid touching every branch of every look.
    pub(crate) fn fixture() -> (Header, Samples) {
        let (nx, ny) = (64u32, 36u32);
        let cols = ColumnSet::of(&[Column::Class, Column::Nu, Column::De, Column::Normal]);
        let n = (nx * ny) as usize;
        let mut s = Samples::alloc(n, cols);
        for i in 0..n {
            let (x, y) = ((i % nx as usize) as f64, (i / nx as usize) as f64);
            let kind = match i % 23 {
                0 => Kind::Interior,
                1 => Kind::Unresolved,
                _ => Kind::Escaped,
            };
            s.class[i] = Class::new(kind, Evidence::Heuristic);
            s.nu.as_mut().unwrap()[i] = 1.0 + (x * 0.37 + y * 1.3).exp() + 5000.0 * (i as f64 / n as f64);
            s.de.as_mut().unwrap()[i] = (1e-4 * (1.0 + x * y * 0.7)) as f32 * (1.0 + (i % 7) as f32 * 30.0);
            s.normal.as_mut().unwrap()[i] = Samples::angle((x * 0.3).cos(), (y * 0.2).sin());
        }
        let h = Header {
            minor: MINOR,
            columns: cols,
            nx,
            ny,
            ss: 2,
            max_iter: 1000,
            escape_radius: 1e10,
            view: View { center_re: "0".into(), center_im: "0".into(), width: "1".into(), rotation: 0.0 },
            kernel: "test".into(),
        };
        (h, s)
    }

    /// FNV-1a of an image.
    pub(crate) fn fnv(b: &[u8]) -> u64 {
        b.iter().fold(0xcbf29ce484222325u64, |h, &x| (h ^ u64::from(x)).wrapping_mul(0x100000001b3))
    }

    /// Image fingerprints of the looks before appearance existed (FX-01): computed with
    /// the pre-FX-01 fd-shade on this fixture.
    const GOLDEN: [(&str, u64); 3] =
        [("umber", 0x4ed011cfed07d299), ("palette", 0xb4f7c8b22c6325f6), ("relief", 0x35c254217df5b0a1)];

    #[test]
    fn still_appearance_is_byte_identical_to_the_old_looks() {
        let (h, s) = fixture();
        for (name, want) in GOLDEN {
            let pass = super::by_name(name).unwrap();
            assert_eq!(fnv(&pass.shade(&h, &s).data), want, "{name}");
            let still = Appearance { time: 12.5, ..Appearance::STILL };
            assert_eq!(fnv(&pass.shade_with(&h, &s, &still).data), want, "{name}: time alone must not change a still look");
        }
    }

    #[test]
    fn flow_moves_the_bands_and_relief_ignores_time() {
        let (h, s) = fixture();
        let at = |t: f64| Appearance { time: t, flow: 0.25, breathe: 0.2, brate: 0.1, drift: 0.05, ..Appearance::STILL };
        for name in ["umber", "palette"] {
            let p = super::by_name(name).unwrap();
            let (a, b) = (p.shade_with(&h, &s, &at(0.0)).data, p.shade_with(&h, &s, &at(1.0)).data);
            assert_ne!(a, b, "{name}: bands should move");
            // Exactly one cycle of flow (4 s at 0.25 c/s), with breathing and drift
            // switched off, returns to the same image.
            let cyc = |t: f64| Appearance { time: t, flow: 0.25, ..Appearance::STILL };
            assert_eq!(p.shade_with(&h, &s, &cyc(0.0)).data, p.shade_with(&h, &s, &cyc(4.0)).data, "{name}");
        }
        let r = super::by_name("relief").unwrap();
        assert_eq!(fnv(&r.shade_with(&h, &s, &at(3.0)).data), GOLDEN[2].1);
    }

    #[test]
    fn aa_fades_relief_only_next_to_sub_sample_filaments() {
        let a = Appearance { aa: true, ..Appearance::STILL };
        assert_eq!(Appearance::STILL.relief_gain(1e-6, 1), 1.0);
        assert!(a.relief_gain(10.0, 1) > 0.999, "far from the boundary the relief is untouched");
        assert!(a.relief_gain(1e-3, 1) < 0.01, "on a filament the lighting is flat");
        assert!(a.relief_gain(0.1, 4) > a.relief_gain(0.1, 1), "supersampling resolves more of it");
        assert_eq!(a.relief_gain(f32::NAN, 1), 0.0);
        // On the fixture (de down to 1e-4 px), AA changes the lit looks, never palette.
        let (h, s) = fixture();
        for (name, want) in GOLDEN {
            let p = super::by_name(name).unwrap();
            let changed = fnv(&p.shade_with(&h, &s, &a).data) != want;
            assert_eq!(changed, name != "palette", "{name}");
        }
    }

    #[test]
    fn unresolved_can_be_drawn_as_interior() {
        let (h, mut s) = fixture();
        let inside = Appearance { unresolved_interior: true, ..Appearance::STILL };
        for name in super::NAMES {
            let p = super::by_name(name).unwrap();
            let marked = p.shade_with(&h, &s, &inside).data;
            // The same image as when those samples really are interior.
            for c in s.class.iter_mut() {
                if c.kind() == Some(Kind::Unresolved) {
                    *c = Class::new(Kind::Interior, Evidence::Heuristic);
                }
            }
            assert_eq!(p.shade(&h, &s).data, marked, "{name}");
            s = fixture().1;
        }
    }
    /// Looks are late: this crate links no kernel, so shading cannot iterate.
    #[test]
    fn depends_on_samples_only() {
        let toml = include_str!("../Cargo.toml");
        let deps = toml.split("[dependencies]").nth(1).unwrap_or("");
        let names: Vec<&str> =
            deps.lines().take_while(|l| !l.starts_with('[')).filter_map(|l| l.split('=').next()).map(str::trim).filter(|n| !n.is_empty()).collect();
        assert_eq!(names, ["fd-samples"], "fd-shade must not depend on anything that can iterate");
    }

    #[test]
    fn every_name_resolves() {
        for n in super::NAMES {
            assert!(super::by_name(n).is_some(), "{n}");
        }
    }
}
