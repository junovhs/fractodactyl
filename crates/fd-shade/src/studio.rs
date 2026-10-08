//! The studio look (FX-02): Kalles-Fraktaler-style rendering modelled on the owner's
//! reference stills. Reads `Class`, `Nu`, `De` and `Normal`; no fractal math (DEC-07).
//! Prototyped and compared in tools/research/looks/proto.py.
//!
//! - Bands are linear in `nu`: palette index `t = density * nu + phase`. Bands in log
//!   `nu` (umber) are far too sparse at depth: `nu` spans ~0.24 log units across a
//!   2.7e-38 frame.
//! - A cyclic multi-stop palette, cosine-eased between stops in linear light.
//! - Terraces: a sawtooth darkening across each stop's band, so bands read as stacked
//!   scales with crisp edges.
//! - Slope light from the analytic gradient `grad nu = -(2 / (de ln 2)) * normal`
//!   (checked against finite differences on a stored frame: cos -1.000, ratio 1.000):
//!   bands whose uphill side faces the light are brighter, the far side darker, with a
//!   log-saturating steepness so it reads everywhere, not only next to the set.
//! - Terrain lines: a dark contour on every band edge at a constant pixel width (the
//!   distance to the nearest edge is `|c - round c|` over edges per pixel), anti-aliased,
//!   fading where edges crowd closer than a few pixels.
//! - Dark seams next to the set from `de`; `aa` fades the slope light within a sample of
//!   sub-sample filaments (their normals are noise).
//! - `aa` also band-limits the palette (FX-03): where bands are finer than a sample, the
//!   colour (and terrace ramp) fades toward its cycle mean with the Gaussian response
//!   `exp(-2 pi² σ² s²)` (`s` = cycles per sample, σ = [`BAND_SIGMA`] samples), so dense
//!   regions settle to a steady average instead of jumping to a random stop every frame.
//!
//! A look is data: [`Look`] reads and writes the `.look` text format (one `key values`
//! line each, `#` comments), so presets saved by the explorer render identically here.
use crate::resolve::{resolve, Rgb8};
use crate::{Appearance, Pass};
use fd_samples::{Column, ColumnSet, Header, Kind, Samples};

/// Steepness (palette cycles per pixel) at which the slope light is about half on.
const S0: f32 = 0.02;
/// Width (in samples) of the Gaussian that band-limits the palette under `aa`.
pub const BAND_SIGMA: f32 = 0.5;
/// Contour lines fade out where band edges are closer than this many pixels.
const LINE_SPACE: f32 = 3.0;
/// Dark-seam depth next to the set.
const CREVICE: f32 = 0.6;

/// Built-in looks, modelled on the owner's reference stills (2026-10-08).
pub const BUILTIN: [(&str, &str); 5] = [
    (
        "ice",
        "# navy, royal, sky, white, warm grey, slate: the blue scaled spiral, with terrain lines
stops #08152e #0b3d91 #1e7fe0 #7cc4ff #eef6fc #c9c3bc #7d7a78 #2a3140
interior #05070d
density 0.05
terrace 0.5
slope 1.1
light 135
lines 1
line_px 2.2
",
    ),
    (
        "coral",
        "# coral, salmon, sand, pale cyan, sky, teal, deep teal: the terrain-lines still
stops #e2553c #f2845a #f7d58c #bfe8e4 #7cc9dc #1f8fb3 #0f5e87 #c9483a
interior #0b1a26
density 0.05
terrace 0.4
slope 1
light 135
lines 1
line_px 2.2
",
    ),
    (
        "steel",
        "# black, grey and white ribs with indigo
stops #0a0a0c #2e2e34 #76767e #d9d9de #ffffff #aeb2d4 #4d5299 #191b3a
interior #05070d
density 0.025
terrace 0.45
slope 1.1
light 135
lines 0
line_px 2.2
",
    ),
    (
        "zebra",
        "# zebra white and black with blue and teal accents
stops #f4f8ff #0b0f19 #5b9dff #f4f8ff #12222c #22d1c3 #dfe9f7 #0a1220
interior #05070d
density 0.08
terrace 0
slope 1.1
light 135
lines 0
line_px 2.2
",
    ),
    (
        "smoke",
        "# smoky greys around a deep blue minibrot
stops #1e1e1f #4a4744 #8f8a84 #d9d4ce #f2eee9 #a39d96 #5c5854 #2a2928
interior #0a2a8a
density 0.03
terrace 0.5
slope 1.1
light 135
lines 0
line_px 2.2
",
    ),
];

/// A studio look: palette and knobs, plus optional animation defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    /// Palette stops as sRGB `0xrrggbb`, cycled.
    pub stops: Vec<u32>,
    /// Interior colour (and unresolved, when drawn as interior).
    pub interior: u32,
    /// Palette cycles per unit of `nu`.
    pub density: f64,
    /// Terrace depth: 0 smooth bands, 1 deep steps.
    pub terrace: f32,
    /// Slope-light strength.
    pub slope: f32,
    /// Light direction, degrees counter-clockwise from screen right (135 = upper left).
    pub light: f32,
    /// Terrain-line darkness, 0 none to 1 black.
    pub lines: f32,
    /// Terrain-line width in output pixels.
    pub line_px: f32,
    /// Animation defaults (cycles/s, amplitude, Hz, cycles/s); flags override them.
    pub flow: f64,
    pub breathe: f64,
    pub brate: f64,
    pub drift: f64,
}

impl Look {
    /// A built-in look by name.
    pub fn builtin(name: &str) -> Option<Look> {
        BUILTIN.iter().find(|(n, _)| *n == name).map(|(_, t)| Look::parse(t).expect("built-in looks parse"))
    }

    /// Parse `.look` text. Every key is optional except `stops`; unknown keys are refused.
    pub fn parse(text: &str) -> Result<Look, String> {
        let mut l = Look {
            stops: Vec::new(),
            interior: 0x05070d,
            density: 0.05,
            terrace: 0.0,
            slope: 1.0,
            light: 135.0,
            lines: 0.0,
            line_px: 2.2,
            flow: 0.0,
            breathe: 0.0,
            brate: 0.0,
            drift: 0.0,
        };
        for (n, line) in text.lines().enumerate() {
            // A comment is a line starting with '#' (colour values contain '#' too).
            if line.trim_start().starts_with('#') {
                continue;
            }
            let mut f = line.split_whitespace();
            let Some(key) = f.next() else { continue };
            let vals: Vec<&str> = f.collect();
            let bad = |what: &str| format!("line {}: {key}: {what}", n + 1);
            let one = || -> Result<f64, String> {
                match vals.as_slice() {
                    [v] => v.parse::<f64>().ok().filter(|x| x.is_finite()).ok_or_else(|| bad("expected one finite number")),
                    _ => Err(bad("expected one value")),
                }
            };
            match key {
                "stops" => l.stops = vals.iter().map(|v| hex(v).ok_or_else(|| bad("expected #rrggbb colours"))).collect::<Result<_, _>>()?,
                "interior" => l.interior = vals.first().and_then(|v| hex(v)).filter(|_| vals.len() == 1).ok_or_else(|| bad("expected one #rrggbb"))?,
                "density" => l.density = one()?,
                "terrace" => l.terrace = one()? as f32,
                "slope" => l.slope = one()? as f32,
                "light" => l.light = one()? as f32,
                "lines" => l.lines = one()? as f32,
                "line_px" => l.line_px = one()? as f32,
                "flow" => l.flow = one()?,
                "breathe" => l.breathe = one()?,
                "brate" => l.brate = one()?,
                "drift" => l.drift = one()?,
                _ => return Err(bad("unknown key")),
            }
        }
        if !(2..=16).contains(&l.stops.len()) {
            return Err(format!("stops: expected 2 to 16 colours, got {}", l.stops.len()));
        }
        if l.density <= 0.0 || !(0.0..=1.0).contains(&l.terrace) || !(0.0..=1.0).contains(&l.lines) || l.line_px <= 0.0 {
            return Err("need density > 0, terrace and lines in 0..1, line_px > 0".into());
        }
        Ok(l)
    }

    /// The `.look` text for this look ([`Look::parse`] reads it back unchanged).
    pub fn to_text(&self) -> String {
        let c = |x: u32| format!("#{x:06x}");
        let stops: Vec<String> = self.stops.iter().map(|&s| c(s)).collect();
        format!(
            "# fractadactyl look\nstops {}\ninterior {}\ndensity {}\nterrace {}\nslope {}\nlight {}\nlines {}\nline_px {}\nflow {}\nbreathe {}\nbrate {}\ndrift {}\n",
            stops.join(" "),
            c(self.interior),
            self.density,
            self.terrace,
            self.slope,
            self.light,
            self.lines,
            self.line_px,
            self.flow,
            self.breathe,
            self.brate,
            self.drift
        )
    }
}

fn hex(s: &str) -> Option<u32> {
    let h = s.strip_prefix('#')?;
    (h.len() == 6).then(|| u32::from_str_radix(h, 16).ok()).flatten()
}

fn linear(rgb: u32) -> [f32; 3] {
    [16, 8, 0].map(|sh| {
        let c = ((rgb >> sh) & 0xff) as f32 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    })
}

/// The studio pass for `look`.
pub struct Studio(pub Look);

impl Pass for Studio {
    fn columns(&self) -> ColumnSet {
        ColumnSet::of(&[Column::Class, Column::Nu, Column::De, Column::Normal])
    }

    fn shade_with(&self, h: &Header, s: &Samples, a: &Appearance) -> Rgb8 {
        let nu = s.nu.as_deref().expect("studio look needs the Nu column");
        let de = s.de.as_deref().expect("studio look needs the De column");
        let nm = s.normal.as_deref().expect("studio look needs the Normal column");
        let l = &self.0;
        let stops: Vec<[f32; 3]> = l.stops.iter().map(|&c| linear(c)).collect();
        let n = stops.len();
        let inside = linear(l.interior);
        let density = l.density * f64::from(a.density());
        let phase = f64::from(a.phase());
        let (ls, lc) = l.light.to_radians().sin_cos();
        let (lx, ly) = (lc, -ls); // screen y points down
        let ss = h.ss as f32;
        // Cycle means: the cosine-eased palette averages to the mean stop, the terrace
        // ramp 1 - terrace (1 - fr)^1.5 to 1 - terrace / 2.5.
        let mean = [0, 1, 2].map(|j| stops.iter().map(|c| c[j]).sum::<f32>() / n as f32);
        let ramp_mean = 1.0 - l.terrace.max(0.0) * 0.4;
        let band_k = -2.0 * (std::f32::consts::PI * BAND_SIGMA).powi(2);
        resolve(h, |i| match s.class[i].kind() {
            Some(Kind::Escaped) => {
                let t = density * nu[i] + phase;
                let x = (t.rem_euclid(1.0) * n as f64) as f32;
                let k = (x as usize).min(n - 1);
                let fr = x - k as f32;
                let f = 0.5 - 0.5 * (std::f32::consts::PI * fr).cos();
                let (c0, c1) = (stops[k], stops[(k + 1) % n]);
                let mut col = [0, 1, 2].map(|j| c0[j] + (c1[j] - c0[j]) * f);
                if l.terrace > 0.0 {
                    let ramp = 1.0 - l.terrace * (1.0 - fr).powf(1.5);
                    col = col.map(|c| c * ramp);
                }
                let d = de[i].max(0.0);
                let (ux, uy) = Samples::unit(nm[i]);
                let facing = -(ux * lx + uy * ly); // uphill (increasing nu) is -normal
                let cycles = density as f32 * 2.0 / (d.max(1e-30) * std::f32::consts::LN_2);
                if a.aa {
                    let s = cycles / ss;
                    let g = (band_k * s * s).exp();
                    let m = if l.terrace > 0.0 { ramp_mean } else { 1.0 };
                    col = [0, 1, 2].map(|j| g * col[j] + (1.0 - g) * mean[j] * m);
                }
                let steep = (0.6 * (cycles / S0).ln_1p()).tanh();
                let fade = if a.aa { a.relief_gain(d, h.ss) } else { 1.0 };
                let mut v = (1.0 + l.slope * facing * steep * fade) * (1.0 - CREVICE * (-d * ss * 1.5).exp());
                if l.lines > 0.0 {
                    let c = t * n as f64;
                    let per_px = (cycles * n as f32).max(1e-30);
                    let dpx = (c - c.round()).abs() as f32 / per_px;
                    let line = (l.line_px / 2.0 + 0.5 - dpx).clamp(0.0, 1.0);
                    let vis = ((1.0 / per_px - LINE_SPACE) / LINE_SPACE).clamp(0.0, 1.0);
                    v *= 1.0 - l.lines * line * vis;
                }
                col.map(|c| c * v)
            }
            Some(Kind::Interior) => inside,
            _ if a.unresolved_interior => inside,
            _ => [0.25, 0.0, 0.25],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_parse_and_round_trip_through_text() {
        for (name, _) in BUILTIN {
            let l = Look::builtin(name).unwrap();
            assert_eq!(Look::parse(&l.to_text()).unwrap(), l, "{name}");
        }
        assert!(Look::builtin("nope").is_none());
    }

    #[test]
    fn parse_reads_keys_and_refuses_bad_input() {
        let l = Look::parse("# mine\nstops #ff0000 #00ff00 #0000ff\ninterior #010203\ndensity 0.1\nlines 0.5\nflow 0.2\n").unwrap();
        assert_eq!((l.stops.len(), l.interior, l.density, l.lines, l.flow), (3, 0x010203, 0.1, 0.5, 0.2));
        for bad in [
            "stops #ff0000\n",                       // one stop
            "stops #ff0000 #00ff00\nsparkle 1\n",    // unknown key
            "stops #ff0000 #00ff00\ndensity -1\n",   // density <= 0
            "stops #ff0000 #00ff0\n",                // bad colour
            "stops #ff0000 #00ff00\nterrace 2\n",    // out of range
            "stops #ff0000 #00ff00\nlines 1 2\n",    // two values
        ] {
            assert!(Look::parse(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn aa_fades_sub_sample_bands_to_the_palette_mean_and_keeps_coarse_ones() {
        use fd_samples::{Class, Evidence, View, MINOR};
        let cols = ColumnSet::of(&[Column::Class, Column::Nu, Column::De, Column::Normal]);
        let mut s = Samples::alloc(2, cols);
        for i in 0..2 {
            s.class[i] = Class::new(Kind::Escaped, Evidence::Heuristic);
            s.nu.as_mut().unwrap()[i] = 3.3;
            s.normal.as_mut().unwrap()[i] = Samples::angle(1.0, 0.0);
        }
        // Sample 0: bands ~1e5 per sample (noise); sample 1: far below one per sample.
        s.de.as_mut().unwrap().copy_from_slice(&[1e-6, 1e6]);
        let view = View { center_re: "0".into(), center_im: "0".into(), width: "1".into(), rotation: 0.0 };
        let h = Header { minor: MINOR, columns: cols, nx: 2, ny: 1, ss: 1, max_iter: 1000, escape_radius: 1e10, view, kernel: "test".into() };
        let look = Look::parse("stops #ff0000 #00ff00 #0000ff\nslope 0\n").unwrap();
        let shade = |aa| Studio(look.clone()).shade_with(&h, &s, &Appearance { aa, ..Appearance::STILL }).data;
        let (on, off) = (shade(true), shade(false));
        assert_eq!(on[3..], off[3..], "coarse bands are untouched");
        // Mean of pure R, G, B is 1/3 each in linear light, times the crevice darkening.
        let want = ((1.0f32 / 3.0 * (1.0 - CREVICE * (-1.5e-6f32).exp())).powf(1.0 / 2.2) * 255.0 + 0.5) as u8;
        assert_eq!(on[..3], [want; 3]);
        assert_ne!(off[..3], [want; 3]);
    }
}
