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
/// Non-periodic DE structure where the nu palette is below Nyquist.
/// Invalid/zero DE declines to the original band-limited shading.
fn haze_terms(d: f32, cycles: f32, ss: f32) -> (f32, f32) {
    if !(d > 0.0 && d.is_finite() && cycles.is_finite()) {
        return (0.0, 0.0);
    }
    // Ease in from one to two cycles per sample, with zero slope at both ends.
    let t = (cycles / ss - 1.0).clamp(0.0, 1.0);
    let weight = t * t * (3.0 - 2.0 * t);
    let x = d * ss;
    (weight, 0.2 + 2.8 * x / (x + 0.02))
}

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
    /// Optional fine-band colour; None retains the FX-03 look.
    pub haze: Option<u32>,
    /// Animation defaults (cycles/s, amplitude, Hz, cycles/s); flags override them.
    pub flow: f64,
    pub breathe: f64,
    pub brate: f64,
    pub drift: f64,
}

impl Look {
    /// A built-in look by name.
    pub fn builtin(name: &str) -> Option<Look> {
        BUILTIN
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, t)| Look::parse(t).expect("built-in looks parse"))
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
            haze: None,
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
                    [v] => v
                        .parse::<f64>()
                        .ok()
                        .filter(|x| x.is_finite())
                        .ok_or_else(|| bad("expected one finite number")),
                    _ => Err(bad("expected one value")),
                }
            };
            match key {
                "stops" => {
                    l.stops = vals
                        .iter()
                        .map(|v| hex(v).ok_or_else(|| bad("expected #rrggbb colours")))
                        .collect::<Result<_, _>>()?
                }
                "interior" => {
                    l.interior = vals
                        .first()
                        .and_then(|v| hex(v))
                        .filter(|_| vals.len() == 1)
                        .ok_or_else(|| bad("expected one #rrggbb"))?
                }
                "density" => l.density = one()?,
                "terrace" => l.terrace = one()? as f32,
                "slope" => l.slope = one()? as f32,
                "light" => l.light = one()? as f32,
                "lines" => l.lines = one()? as f32,
                "line_px" => l.line_px = one()? as f32,
                "haze" => {
                    l.haze = Some(
                        vals.first()
                            .and_then(|v| hex(v))
                            .filter(|_| vals.len() == 1)
                            .ok_or_else(|| bad("expected one #rrggbb"))?,
                    )
                }
                "flow" => l.flow = one()?,
                "breathe" => l.breathe = one()?,
                "brate" => l.brate = one()?,
                "drift" => l.drift = one()?,
                _ => return Err(bad("unknown key")),
            }
        }
        if !(2..=16).contains(&l.stops.len()) {
            return Err(format!(
                "stops: expected 2 to 16 colours, got {}",
                l.stops.len()
            ));
        }
        if l.density <= 0.0
            || !(0.0..=1.0).contains(&l.terrace)
            || !(0.0..=1.0).contains(&l.lines)
            || l.line_px <= 0.0
        {
            return Err("need density > 0, terrace and lines in 0..1, line_px > 0".into());
        }
        Ok(l)
    }

    /// The `.look` text for this look ([`Look::parse`] reads it back unchanged).
    pub fn to_text(&self) -> String {
        let c = |x: u32| format!("#{x:06x}");
        let stops: Vec<String> = self.stops.iter().map(|&s| c(s)).collect();
        format!(
            "# fractodactyl look\nstops {}\ninterior {}\ndensity {}\nterrace {}\nslope {}\nlight {}\nlines {}\nline_px {}\n{}flow {}\nbreathe {}\nbrate {}\ndrift {}\n",
            stops.join(" "),
            c(self.interior),
            self.density,
            self.terrace,
            self.slope,
            self.light,
            self.lines,
            self.line_px,
            self.haze.map(|h| format!("haze {}\n", c(h))).unwrap_or_default(),
            self.flow,
            self.breathe,
            self.brate,
            self.drift
        )
    }
}

fn hex(s: &str) -> Option<u32> {
    let h = s.strip_prefix('#')?;
    (h.len() == 6)
        .then(|| u32::from_str_radix(h, 16).ok())
        .flatten()
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
        let nm = s
            .normal
            .as_deref()
            .expect("studio look needs the Normal column");
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
        let haze = l.haze.map(linear);
        resolve(h, a.dither, |i| match s.class[i].kind() {
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
                if let Some(tint) = haze {
                    let (weight, tone) = haze_terms(d, cycles, ss);
                    if weight > 0.0 {
                        col = [0, 1, 2].map(|j| (1.0 - weight) * col[j] + weight * tint[j] * tone);
                    }
                }
                let steep = (0.6 * (cycles / S0).ln_1p()).tanh();
                let fade = if a.aa { a.relief_gain(d, h.ss) } else { 1.0 };
                let mut v = (1.0 + l.slope * facing * steep * fade)
                    * (1.0 - CREVICE * (-d * ss * 1.5).exp());
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

/// Static per-sample studio terms for flowing, non-breathing looks without lines.
/// Those looks change only the palette phase; geometry and lighting do not.
pub struct PreparedStudio {
    w: usize,
    h: usize,
    stops: Vec<[f32; 3]>,
    inside: [f32; 3],
    mean: [f32; 3],
    haze: Option<[f32; 3]>,
    ramp_mean: f32,
    terrace: f32,
    aa: bool,
    unresolved_interior: bool,
    points: Vec<PreparedPoint>,
    static_rgb: Vec<[u8; 4]>,
}

struct PreparedPoint {
    t: f64,
    light: f32,
    band: f32,
    haze_weight: f32,
    haze_tone: f32,
    kind: u8,
}

impl PreparedStudio {
    /// Decline when time changes anything but phase, or the source is not one sample/pixel.
    pub fn new(l: &Look, h: &Header, s: &Samples, a: &Appearance) -> Option<Self> {
        if h.ss != 1 || l.lines != 0.0 || a.breathe != 0.0 {
            return None;
        }
        let nu = s.nu.as_deref()?;
        let de = s.de.as_deref()?;
        let nm = s.normal.as_deref()?;
        let stops: Vec<[f32; 3]> = l.stops.iter().map(|&c| linear(c)).collect();
        let n = stops.len();
        let mean = [0, 1, 2].map(|j| stops.iter().map(|c| c[j]).sum::<f32>() / n as f32);
        let ramp_mean = 1.0 - l.terrace.max(0.0) * 0.4;
        let density = l.density * f64::from(a.density());
        let (ls, lc) = l.light.to_radians().sin_cos();
        let (lx, ly) = (lc, -ls);
        let ss = h.ss as f32;
        let band_k = -2.0 * (std::f32::consts::PI * BAND_SIGMA).powi(2);
        let haze = l.haze.map(linear);
        let points: Vec<PreparedPoint> = s
            .class
            .iter()
            .enumerate()
            .map(|(i, c)| {
                if c.kind() != Some(Kind::Escaped) {
                    return PreparedPoint {
                        t: 0.0,
                        light: 0.0,
                        band: 0.0,
                        haze_weight: 0.0,
                        haze_tone: 0.0,
                        kind: if c.kind() == Some(Kind::Interior) {
                            1
                        } else {
                            2
                        },
                    };
                }
                let d = de[i].max(0.0);
                let (ux, uy) = Samples::unit(nm[i]);
                let facing = -(ux * lx + uy * ly);
                let cycles = density as f32 * 2.0 / (d.max(1e-30) * std::f32::consts::LN_2);
                let band = if a.aa {
                    let s = cycles / ss;
                    (band_k * s * s).exp()
                } else {
                    1.0
                };
                let steep = (0.6 * (cycles / S0).ln_1p()).tanh();
                let fade = if a.aa { a.relief_gain(d, h.ss) } else { 1.0 };
                let light = (1.0 + l.slope * facing * steep * fade)
                    * (1.0 - CREVICE * (-d * ss * 1.5).exp());
                let (haze_weight, haze_tone) = if haze.is_some() {
                    haze_terms(d, cycles, ss)
                } else {
                    (0.0, 0.0)
                };
                PreparedPoint {
                    t: density * nu[i],
                    light,
                    band,
                    haze_weight,
                    haze_tone,
                    kind: 0,
                }
            })
            .collect();
        let (w, h) = h.pixels();
        // Reuse already encoded bytes only for samples whose palette is
        // provably phase-independent. Film always dithers source samples.
        let inside = linear(l.interior);
        let static_rgb: Vec<[u8; 4]> = points
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let col = match p.kind {
                    0 if a.aa && p.band == 0.0 && p.t.is_finite() && p.light.is_finite() => {
                        let m = if l.terrace > 0.0 { ramp_mean } else { 1.0 };
                        [0, 1, 2].map(|j| {
                            let base = mean[j] * m;
                            let v = if p.haze_weight > 0.0 {
                                if let Some(tint) = haze {
                                    (1.0 - p.haze_weight) * base
                                        + p.haze_weight * tint[j] * p.haze_tone
                                } else {
                                    base
                                }
                            } else {
                                base
                            };
                            v * p.light
                        })
                    }
                    1 => inside,
                    2 if a.unresolved_interior => inside,
                    2 => [0.25, 0.0, 0.25],
                    _ => return [0; 4],
                };
                let px = i % w;
                let py = i / w;
                let mut rgb = [0u8; 4];
                rgb[3] = 1;
                for j in 0..3 {
                    let d = crate::resolve::DITHER_LEVELS
                        * crate::resolve::tpdf((py as u32) << 16 ^ (px as u32) << 2 ^ j as u32);
                    rgb[j] = (col[j].clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0 + 0.5 + d)
                        .clamp(0.0, 255.0) as u8;
                }
                rgb
            })
            .collect();
        Some(Self {
            w,
            h,
            stops,
            inside,
            mean,
            haze,
            ramp_mean,
            terrace: l.terrace,
            aa: a.aa,
            unresolved_interior: a.unresolved_interior,
            points,
            static_rgb,
        })
    }

    /// Shade just the requested sample rectangle, retaining global dither coordinates.
    /// Caller must keep the non-time appearance flags used in new().
    pub fn shade_rect_into(
        &self,
        rect: (usize, usize, usize, usize),
        a: &Appearance,
        out: &mut Rgb8,
    ) {
        self.shade_rect(rect, a, out);
    }

    /// Film source shading shares the exact arithmetic with all studio looks.
    pub fn shade_rect_film_into(
        &self,
        rect: (usize, usize, usize, usize),
        a: &Appearance,
        out: &mut Rgb8,
    ) {
        self.shade_rect(rect, a, out);
    }

    fn shade_rect(&self, rect: (usize, usize, usize, usize), a: &Appearance, out: &mut Rgb8) {
        let (x0, y0, w, h) = rect;
        assert!(x0 + w <= self.w && y0 + h <= self.h);
        assert_eq!(self.aa, a.aa);
        assert_eq!(self.unresolved_interior, a.unresolved_interior);
        out.w = w;
        out.h = h;
        out.data.resize(w * h * 3, 0);
        let phase = f64::from(a.phase());
        let n = self.stops.len();
        let threads = std::thread::available_parallelism()
            .map_or(1, |n| n.get())
            .clamp(1, h);
        let rows_per = h.div_ceil(threads);
        std::thread::scope(|sc| {
            for (c, chunk) in out.data.chunks_mut(rows_per * w * 3).enumerate() {
                sc.spawn(move || {
                    for (r, row) in chunk.chunks_mut(w * 3).enumerate() {
                        let py = y0 + c * rows_per + r;
                        for (dx, rgb) in row.chunks_mut(3).enumerate() {
                            let px = x0 + dx;
                            let i = py * self.w + px;
                            if a.dither && self.static_rgb[i][3] != 0 {
                                rgb.copy_from_slice(&self.static_rgb[i][..3]);
                                continue;
                            }
                            let p = &self.points[i];
                            let col = if p.kind == 0 {
                                let x = ((p.t + phase).rem_euclid(1.0) * n as f64) as f32;
                                let k = (x as usize).min(n - 1);
                                let fr = x - k as f32;
                                let f = 0.5 - 0.5 * (std::f32::consts::PI * fr).cos();
                                let (c0, c1) = (self.stops[k], self.stops[(k + 1) % n]);
                                let mut col = [0, 1, 2].map(|j| c0[j] + (c1[j] - c0[j]) * f);
                                if self.terrace > 0.0 {
                                    let ramp = 1.0 - self.terrace * (1.0 - fr).powf(1.5);
                                    col = col.map(|c| c * ramp);
                                }
                                if self.aa {
                                    let m = if self.terrace > 0.0 {
                                        self.ramp_mean
                                    } else {
                                        1.0
                                    };
                                    col = [0, 1, 2].map(|j| {
                                        p.band * col[j] + (1.0 - p.band) * self.mean[j] * m
                                    });
                                }
                                if p.haze_weight > 0.0 {
                                    if let Some(tint) = self.haze {
                                        col = [0, 1, 2].map(|j| {
                                            (1.0 - p.haze_weight) * col[j]
                                                + p.haze_weight * tint[j] * p.haze_tone
                                        });
                                    }
                                }
                                col.map(|c| c * p.light)
                            } else if p.kind == 1 || self.unresolved_interior {
                                self.inside
                            } else {
                                [0.25, 0.0, 0.25]
                            };
                            for (channel, (&v, dst)) in col.iter().zip(rgb.iter_mut()).enumerate() {
                                let t = if a.dither {
                                    crate::resolve::DITHER_LEVELS
                                        * crate::resolve::tpdf(
                                            (py as u32) << 16 ^ (px as u32) << 2 ^ channel as u32,
                                        )
                                } else {
                                    0.0
                                };
                                let v = v.clamp(0.0, 1.0);
                                *dst =
                                    (v.powf(1.0 / 2.2) * 255.0 + 0.5 + t).clamp(0.0, 255.0) as u8;
                            }
                        }
                    }
                });
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepared_rect_matches_full_studio_at_every_pixel() {
        let (mut h, s) = crate::tests::fixture();
        h.ss = 1;
        let mut l = Look::builtin("ice").unwrap();
        l.lines = 0.0;
        for aa in [false, true] {
            let a = Appearance {
                flow: 0.11,
                aa,
                dither: true,
                unresolved_interior: true,
                ..Appearance::STILL
            };
            let prepared = PreparedStudio::new(&l, &h, &s, &a).unwrap();
            for time in [0.0, 0.033333, 3.7, 22.0] {
                let at = Appearance { time, ..a };
                let full = Studio(l.clone()).shade_with(&h, &s, &at);
                for rect in [(0, 0, full.w, full.h), (7, 5, 25, 17)] {
                    let mut got = Rgb8 {
                        w: 0,
                        h: 0,
                        data: Vec::new(),
                    };
                    prepared.shade_rect_into(rect, &at, &mut got);
                    for y in 0..rect.3 {
                        let a = &got.data[y * rect.2 * 3..(y + 1) * rect.2 * 3];
                        let i = ((rect.1 + y) * full.w + rect.0) * 3;
                        assert_eq!(
                            a,
                            &full.data[i..i + rect.2 * 3],
                            "time {time} rect {rect:?} row {y}"
                        );
                    }
                }
            }
        }
        assert!(
            PreparedStudio::new(&Look::builtin("ice").unwrap(), &h, &s, &Appearance::STILL)
                .is_none(),
            "lines need full shader"
        );
        let a = Appearance {
            breathe: 0.1,
            ..Appearance::STILL
        };
        assert!(
            PreparedStudio::new(&l, &h, &s, &a).is_none(),
            "breathing needs full shader"
        );
    }

    #[test]
    fn film_tables_match_every_source_pixel_of_master_ice() {
        let (mut h, s) = crate::tests::fixture();
        h.ss = 1;
        let l = Look::parse(include_str!("../../../looks/master-ice.look")).unwrap();
        let a = Appearance {
            flow: l.flow,
            aa: true,
            dither: true,
            unresolved_interior: true,
            ..Appearance::STILL
        };
        let prepared = PreparedStudio::new(&l, &h, &s, &a).unwrap();
        for time in [0.0, 1.0 / 30.0, 3.7, 22.0] {
            let at = Appearance { time, ..a };
            let exact = Studio(l.clone()).shade_with(&h, &s, &at);
            let mut fast = Rgb8 {
                w: 0,
                h: 0,
                data: Vec::new(),
            };
            prepared.shade_rect_film_into((0, 0, exact.w, exact.h), &at, &mut fast);
            assert_eq!(fast.data, exact.data, "time {time}: byte drift");
        }
    }

    #[test]
    fn haze_is_per_sample_continuous_and_matches_prepared_film() {
        use fd_samples::{Class, Evidence};
        let (mut h, mut s) = crate::tests::fixture();
        h.ss = 1;
        let look = Look::parse(include_str!("../../../looks/master-ice.look")).unwrap();
        let mut old = look.clone();
        old.haze = None;
        assert_eq!(haze_terms(0.1, 1.0, 1.0).0, 0.0);
        assert!(haze_terms(0.1, 1.001, 1.0).0 < 0.00001);
        assert_eq!(haze_terms(0.1, 2.0, 1.0).0, 1.0);
        for c in &mut s.class {
            *c = Class::new(Kind::Escaped, Evidence::Heuristic);
        }
        // Just one crowded pixel in an otherwise ordinary frame still gets colour.
        for d in s.de.as_mut().unwrap() {
            *d = 2.0;
        }
        s.de.as_mut().unwrap()[2] = 1e-5;
        s.normal.as_mut().unwrap()[2] = Samples::angle(-1.0, -1.0);
        for aa in [false, true] {
            let a = Appearance {
                aa,
                dither: true,
                unresolved_interior: true,
                ..Appearance::STILL
            };
            let baseline = Studio(old.clone()).shade_with(&h, &s, &a);
            let updated = Studio(look.clone()).shade_with(&h, &s, &a);
            assert_ne!(baseline.data[6..9], updated.data[6..9]);
            for i in 0..h.count() {
                if i != 2 {
                    assert_eq!(
                        &baseline.data[3 * i..3 * i + 3],
                        &updated.data[3 * i..3 * i + 3],
                        "ordinary pixel {i}"
                    );
                }
            }
        }
        // A mixed dense frame must change only fine-band samples, never neighbours.
        for (i, d) in s.de.as_mut().unwrap().iter_mut().enumerate() {
            *d = 0.0002 + (i % 64) as f32 * 0.005;
        }
        for aa in [false, true] {
            let a = Appearance {
                aa,
                dither: true,
                unresolved_interior: true,
                ..Appearance::STILL
            };
            let updated = Studio(look.clone()).shade_with(&h, &s, &a);
            let baseline = Studio(old.clone()).shade_with(&h, &s, &a);
            let mut changed = 0;
            let mut unchanged = 0;
            for (i, &d) in s.de.as_ref().unwrap().iter().enumerate() {
                let cycles = look.density as f32 * 2.0 / (d * std::f32::consts::LN_2);
                let old_rgb = &baseline.data[3 * i..3 * i + 3];
                let new_rgb = &updated.data[3 * i..3 * i + 3];
                if haze_terms(d, cycles, 1.0).0 == 0.0 {
                    assert_eq!(old_rgb, new_rgb, "outside haze {i}");
                    unchanged += 1;
                } else if old_rgb != new_rgb {
                    changed += 1;
                }
            }
            assert!(changed > 0 && unchanged > 0);
            let prepared = PreparedStudio::new(&look, &h, &s, &a).unwrap();
            let mut fast = Rgb8 {
                w: 0,
                h: 0,
                data: Vec::new(),
            };
            prepared.shade_rect_film_into((0, 0, updated.w, updated.h), &a, &mut fast);
            assert_eq!(fast.data, updated.data, "film parity aa {aa}");
        }
    }

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
        assert_eq!(
            (l.stops.len(), l.interior, l.density, l.lines, l.flow),
            (3, 0x010203, 0.1, 0.5, 0.2)
        );
        for bad in [
            "stops #ff0000\n",                     // one stop
            "stops #ff0000 #00ff00\nsparkle 1\n",  // unknown key
            "stops #ff0000 #00ff00\ndensity -1\n", // density <= 0
            "stops #ff0000 #00ff0\n",              // bad colour
            "stops #ff0000 #00ff00\nterrace 2\n",  // out of range
            "stops #ff0000 #00ff00\nlines 1 2\n",  // two values
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
        let view = View {
            center_re: "0".into(),
            center_im: "0".into(),
            width: "1".into(),
            rotation: 0.0,
        };
        let h = Header {
            minor: MINOR,
            columns: cols,
            nx: 2,
            ny: 1,
            ss: 1,
            max_iter: 1000,
            escape_radius: 1e10,
            view,
            kernel: "test".into(),
        };
        let look = Look::parse("stops #ff0000 #00ff00 #0000ff\nslope 0\n").unwrap();
        let shade = |aa| {
            Studio(look.clone())
                .shade_with(
                    &h,
                    &s,
                    &Appearance {
                        aa,
                        ..Appearance::STILL
                    },
                )
                .data
        };
        let (on, off) = (shade(true), shade(false));
        assert_eq!(on[3..], off[3..], "coarse bands are untouched");
        // Mean of pure R, G, B is 1/3 each in linear light, times the crevice darkening.
        let want = ((1.0f32 / 3.0 * (1.0 - CREVICE * (-1.5e-6f32).exp())).powf(1.0 / 2.2) * 255.0
            + 0.5) as u8;
        assert_eq!(on[..3], [want; 3]);
        assert_ne!(off[..3], [want; 3]);
    }
}
