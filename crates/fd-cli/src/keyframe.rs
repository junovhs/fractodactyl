//! Keyframe films (FILM-04): `fd film --keyframes M`. Keyframe `j` is the view at width
//! `w0 / 2^j`, rendered once at `M` times the output size (one sample per keyframe pixel)
//! and kept as samples: the cache holds fractal results, never pictures. Every frame
//! between widths `W_k` and `W_{k+1}` is built from keyframes `k` (outer) and `k + 1`
//! (inner): both are shaded at the frame's film time (looks stay late, DEC-07), then each
//! output pixel gathers their samples with a tent filter (half-width [`TENT`] output pixels,
//! never under one keyframe sample). With `M = 2` an output pixel always spans between 1
//! and 4 keyframe samples per axis, and because
//! samples are fixed to the fractal, not the screen, nothing re-rolls between frames.
//!
//! The inner keyframe fades in over the first quarter of each octave and across a thin
//! margin at its edge, so switching to the next keyframe pair is continuous.
//! Keyframe shading is dithered per sample, so the filtered result keeps sub-level
//! precision and the output needs no second dither. The studio look shades, and tent
//! resamples, only on wgpu ([`fd_shade::gpu::StudioGpu`], SHAD-11); other looks shade
//! whole keyframes on the CPU and resample here.
use crate::film::sig6_log;
use crate::film::{render_frame_profiled, BuildTimings, Used};
use fd_kernel::{zone_covers, Params, Zone};
use fd_samples::{Header, Samples, View};
use fd_shade::gpu::Resample;
use fd_shade::{Appearance, Look, Pass, Rgb8};

/// Tent half-width in output pixels. Probe (FILM-04, 2 s spiral-on-spiral clip vs direct
/// ss 3): 1.0 px is visibly soft (Laplacian 7.0 vs 11.2); 0.5 px matches ss 3 (11.2) and
/// changes between frames no more than ss 3 does (median ratio 0.99, max 1.10).
const TENT: f32 = 0.5;

pub(crate) struct Keyframes<'a> {
    re: String,
    im: String,
    l0: f64,
    /// Rendered keyframe footprint relative to the nominal zoom width.
    footprint: f64,
    /// Output size in pixels.
    out: (usize, usize),
    /// Keyframe render parameters (M x the output size, ss 1).
    p: Params,
    zone: Option<&'a Zone>,
    zone_renderer: Option<crate::zone_gpu::Renderer>,
    /// Rendered keyframes still needed: (index, log10 width, header, samples).
    cache: Vec<(usize, f64, Header, Samples)>,
    studio: Option<Look>,
    gpu: Option<fd_shade::gpu::StudioGpu>,
    /// The studio host shares the zone kernel's device: GPU zone keyframes stay there
    /// (GPU-21).
    resident: bool,
    /// Bytes of zone kernel output read back to the host (fd-film/1 report).
    pub zone_readback_bytes: u64,
    shaded: [Rgb8; 2],
    output: Rgb8,
    lut: [f32; 256],
    pub builds: BuildTimings,
    /// Outer contributing keyframe route for each output-frame count.
    pub frame_route: Used,
    /// Stage timings for the film JSON report. Prepare stays 0 since SHAD-11: the
    /// studio look prepares on the device, inside sample shade; the field keeps the
    /// fd-film/1 report's shape.
    pub prepare_seconds: f64,
    pub sample_shade_seconds: f64,
    pub resample_seconds: f64,
    /// Keyframes rendered so far and their seconds.
    pub rendered: Vec<(usize, f64, Used)>,
}

impl<'a> Keyframes<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new_log(
        re: &str,
        im: &str,
        l0: f64,
        out: (u32, u32),
        m: u32,
        p: &Params,
        zone: Option<&'a Zone>,
        studio: Option<Look>,
    ) -> Self {
        let p = Params {
            nx: out.0 * m,
            ny: out.1 * m,
            ss: 1,
            ..*p
        };
        let (re, im) = (re.to_string(), im.to_string());
        Keyframes {
            re,
            im,
            l0,
            footprint: 1.0,
            out: (out.0 as usize, out.1 as usize),
            p,
            zone,
            zone_renderer: None,
            cache: Vec::new(),
            studio,
            gpu: None,
            resident: false,
            zone_readback_bytes: 0,
            shaded: std::array::from_fn(|_| Rgb8 {
                w: 0,
                h: 0,
                data: Vec::new(),
            }),
            output: Rgb8 {
                w: out.0 as usize,
                h: out.1 as usize,
                data: vec![0; out.0 as usize * out.1 as usize * 3],
            },
            lut: std::array::from_fn(|v| (v as f32 / 255.0).powf(2.2)),
            rendered: Vec::new(),
            builds: BuildTimings::default(),
            frame_route: Used::Plain,
            prepare_seconds: 0.0,
            sample_shade_seconds: 0.0,
            resample_seconds: 0.0,
        }
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        re: &str,
        im: &str,
        w0: f64,
        out: (u32, u32),
        m: u32,
        p: &Params,
        zone: Option<&'a Zone>,
        studio: Option<Look>,
    ) -> Self {
        Self::new_log(re, im, w0.log10(), out, m, p, zone, studio)
    }

    #[cfg(test)]
    fn octave(&self, w: f64) -> usize {
        self.octave_log(w.log10())
    }

    #[cfg(test)]
    pub fn frame(
        &mut self,
        w: f64,
        off: (f64, f64),
        rotation: f64,
        look: &dyn Pass,
        a: &Appearance,
    ) -> Result<&Rgb8, String> {
        self.frame_log(w.log10(), (off.0 / w, off.1 / w), rotation, look, a)
    }

    /// Enlarge rotated keyframes to at least `footprint` x the nominal width, keeping the
    /// unrotated sample pitch (so shading sees the same pixel scale).
    pub(crate) fn cover_rotations(&mut self, footprint: f64) -> Result<(), String> {
        let m = self.p.nx as f64 / self.out.0 as f64;
        let multiple = (m * footprint).ceil();
        if !(footprint.is_finite() && footprint >= 1.0 && multiple.is_finite()) {
            return Err("rotated keyframe footprint is out of range".into());
        }
        let (nx, ny) = (self.out.0 as f64 * multiple, self.out.1 as f64 * multiple);
        if nx > u32::MAX as f64 || ny > u32::MAX as f64 {
            return Err("rotated keyframe grid is too large".into());
        }
        let multiple = multiple as u32;
        self.p.nx = self.out.0 as u32 * multiple;
        self.p.ny = self.out.1 as u32 * multiple;
        // The grid's actual enlargement, so width / pixels stays the unrotated pitch.
        self.footprint = f64::from(multiple) / m;
        Ok(())
    }

    /// Open the studio look's wgpu device (SHAD-11: the only studio shading). With the
    /// wgpu zone kernel it is the kernel's device, so zone keyframes are shaded where
    /// they were rendered (GPU-21); FD_ZONE_READBACK=1 keeps them on the host instead.
    pub(crate) fn enable_gpu(&mut self, look: &Look, a: &Appearance) -> Result<String, String> {
        let multiple = self.p.nx as usize / self.out.0;
        let count = self.p.nx as usize * self.p.ny as usize;
        let shared = match self.zone_renderer.as_mut() {
            Some(r) if std::env::var_os("FD_ZONE_READBACK").is_none() => r.device(count).ok(),
            _ => None,
        };
        self.resident = shared.is_some();
        let gpu = match shared {
            Some((name, device, queue)) => fd_shade::gpu::StudioGpu::on_device(
                name, device, queue, look, a, self.out, multiple,
            )?,
            None => fd_shade::gpu::StudioGpu::new(look, a, self.out, multiple)?,
        };
        let name = gpu.name.clone();
        self.gpu = Some(gpu);
        Ok(name)
    }

    pub(crate) fn gpu_device(&self) -> Option<&str> {
        self.gpu.as_ref().map(|g| g.name.as_str())
    }

    /// Select wgpu zone samples; unsupported frames render with the perturbation kernel.
    pub(crate) fn enable_zone_kernel(&mut self, mode: crate::zone_gpu::Kernel) {
        self.zone_renderer = Some(crate::zone_gpu::Renderer::new(mode));
    }

    fn width_str(&self, j: usize) -> String {
        sig6_log(self.l0 + self.footprint.log10() - j as f64 * std::f64::consts::LOG10_2)
    }

    fn width_log(&self, j: usize) -> f64 {
        crate::film::width_log(&self.width_str(j)).expect("formatted width")
    }

    #[cfg(test)]
    fn width(&self, j: usize) -> f64 {
        10f64.powf(self.width_log(j))
    }

    /// Render keyframe `j` unless cached; drop keyframes shallower than `keep`.
    fn ensure(&mut self, j: usize, keep: usize) -> Result<(), String> {
        self.cache.retain(|c| c.0 >= keep);
        if self.cache.iter().any(|c| c.0 == j) {
            return Ok(());
        }
        let view = View {
            center_re: self.re.clone(),
            center_im: self.im.clone(),
            width: self.width_str(j),
            rotation: 0.0,
        };
        let t = std::time::Instant::now();
        let process_cpu = crate::process_cpu::now();
        let covered = match (self.zone, self.zone_renderer.as_ref()) {
            (Some(zone), Some(renderer)) if zone_covers(&view, &self.p, zone)? => {
                // A keyframe no GPU stage takes renders with the perturbation kernel; the
                // renderer stays on for the next one (CLEA-06).
                match renderer.declines(&view, &self.p, zone) {
                    Some(why) => {
                        eprintln!("fd film: the wgpu zone kernel declines keyframe {j} ({why}); using the perturbation kernel");
                        false
                    }
                    None => true,
                }
            }
            _ => false,
        };
        if covered && self.resident {
            let (zone, renderer) = (self.zone.unwrap(), self.zone_renderer.as_mut().unwrap());
            let gpu = self.gpu.as_mut().expect("resident studio host");
            match renderer.render_resident(&view, &self.p, zone, gpu, j) {
                Ok(Some(h)) => {
                    // Only the validity word came back; the samples stay on the device.
                    self.zone_readback_bytes += 4;
                    let cpu = crate::process_cpu::elapsed(process_cpu);
                    self.builds.process_cpu_frame(
                        &view,
                        &self.p,
                        Used::zone(&h),
                        t.elapsed().as_secs_f64(),
                        cpu,
                    );
                    self.rendered
                        .push((j, t.elapsed().as_secs_f64(), Used::zone(&h)));
                    self.cache
                        .push((j, self.width_log(j), h, Samples::default()));
                    return Ok(());
                }
                Ok(None) => {
                    eprintln!("fd film: wgpu zone keyframe {j} has invalid columns; using the perturbation kernel")
                }
                Err(e) => {
                    eprintln!("fd film: wgpu zone unavailable ({e}); using the perturbation kernel")
                }
            }
            self.zone_renderer = None;
            self.zone_readback_bytes += 4;
            let (h, s, used) =
                render_frame_profiled(&view, &self.p, self.zone, &mut None, &mut self.builds)?;
            let cpu = crate::process_cpu::elapsed(process_cpu);
            self.builds
                .process_cpu_frame(&view, &self.p, used, t.elapsed().as_secs_f64(), cpu);
            self.rendered.push((j, t.elapsed().as_secs_f64(), used));
            self.cache.push((j, self.width_log(j), h, s));
            return Ok(());
        }
        let gpu_result = match (self.zone, self.zone_renderer.as_mut()) {
            (Some(zone), Some(renderer)) if covered => Some(renderer.render(&view, &self.p, zone)),
            _ => None,
        };
        let (h, s, used) = match gpu_result {
            Some(Ok((h, s))) => {
                self.zone_readback_bytes += 16 * s.class.len() as u64;
                let used = Used::zone(&h);
                (h, s, used)
            }
            Some(Err(e)) => {
                eprintln!("fd film: wgpu zone unavailable ({e}); using the perturbation kernel");
                self.zone_renderer = None;
                render_frame_profiled(&view, &self.p, self.zone, &mut None, &mut self.builds)?
            }
            None => render_frame_profiled(&view, &self.p, self.zone, &mut None, &mut self.builds)?,
        };
        let cpu = crate::process_cpu::elapsed(process_cpu);
        self.builds
            .process_cpu_frame(&view, &self.p, used, t.elapsed().as_secs_f64(), cpu);
        self.rendered.push((j, t.elapsed().as_secs_f64(), used));
        self.cache.push((j, self.width_log(j), h, s));
        Ok(())
    }

    /// The keyframe pair for frame width `w`: `k` with `W_{k+1} < w <= W_k`.
    fn octave_log(&self, log: f64) -> usize {
        let mut k = ((self.l0 - log) * std::f64::consts::LOG2_10)
            .floor()
            .max(0.0) as usize;
        while k > 0 && self.width_log(k) < log {
            k -= 1;
        }
        while self.width_log(k + 1) >= log {
            k += 1;
        }
        k
    }

    /// The frame at log10 width `log`, centred `off` in frame-width units from the keyframe centre,
    /// rotated by `rotation` radians and shaded at appearance `a`.
    pub fn frame_log(
        &mut self,
        log: f64,
        off: (f64, f64),
        rotation: f64,
        look: &dyn Pass,
        a: &Appearance,
    ) -> Result<&Rgb8, String> {
        let req = coverage_rotated(1.0, off, (self.out.0 as u32, self.out.1 as u32), rotation);
        // Rotated keyframes are enlarged by `footprint`: pick the octave by the frame's
        // scale against the nominal widths, as an unrotated film would.
        let ramp_log = log
            + if self.footprint > 1.0 {
                self.footprint.log10()
            } else {
                req.log10()
            };
        let k = self.octave_log(ramp_log);
        self.ensure(k, k)?;
        self.ensure(k + 1, k)?;
        self.frame_route = self
            .rendered
            .iter()
            .find(|r| r.0 == k)
            .expect("rendered outer keyframe")
            .2;
        if let (Some(l), None) = (self.studio.clone(), &self.gpu) {
            self.enable_gpu(&l, a)
                .map_err(|e| format!("studio look needs a wgpu adapter: {e}"))?;
        }
        let cache = &self.cache;
        let get = |j: usize| cache.iter().find(|c| c.0 == j).expect("ensured");
        let (outer, inner) = (get(k), get(k + 1));
        let sa = Appearance { dither: true, ..*a };
        let (ow, oh) = self.out;
        let (sin, cos) = rotation.sin_cos();
        let (sin, cos) = (sin as f32, cos as f32);
        let u = ((outer.1 - ramp_log) * std::f64::consts::LOG2_10).clamp(0.0, 1.0) as f32;
        let ramp = (u / 0.25).min(1.0);
        let margin = 0.1 * (1.0 - u) + 1e-3;
        let (zo, zi) = (
            10f64.powf(log - outer.1) as f32,
            10f64.powf(log - inner.1) as f32,
        );
        // The frame centre in each keyframe, in its widths (x) and heights (y, down).
        let aspect64 = ow as f64 / oh as f64;
        let at = |lj: f64| {
            let ratio = 10f64.powf(log - lj);
            ((off.0 * ratio) as f32, (-off.1 * ratio * aspect64) as f32)
        };
        let ((oxo, oyo), (oxi, oyi)) = (at(outer.1), at(inner.1));
        let (owidth, oheight) = outer.2.pixels();
        let (iwidth, iheight) = inner.2.pixels();
        // The studio look shades only the texels the tent filter can read, with extra
        // edge padding; other looks shade whole keyframes.
        let studio = self.gpu.is_some();
        let ro = if studio {
            needed_rect((ow, oh), (owidth, oheight), zo, oxo, oyo, sin, cos)
        } else {
            (0, 0, owidth, oheight)
        };
        let ri = if studio {
            needed_rect((ow, oh), (iwidth, iheight), zi, oxi, oyi, sin, cos)
        } else {
            (0, 0, iwidth, iheight)
        };
        let r = Resample {
            out: self.out,
            sin,
            cos,
            zoom: (zo, zi),
            outer_at: (oxo, oyo),
            inner_at: (oxi, oyi),
            ramp,
            margin,
            outer: ((owidth, oheight), ro),
            inner: ((iwidth, iheight), ri),
        };
        if let Some(gpu) = &mut self.gpu {
            let t = std::time::Instant::now();
            let rgb = gpu
                .frame(
                    k,
                    (&outer.3, (owidth, oheight), ro),
                    k + 1,
                    (&inner.3, (iwidth, iheight), ri),
                    &sa,
                    &r,
                )
                .map_err(|e| format!("studio shading on wgpu: {e}"))?;
            self.output.data.copy_from_slice(rgb);
            self.sample_shade_seconds += t.elapsed().as_secs_f64();
            return Ok(&self.output);
        }
        let t_shade = std::time::Instant::now();
        self.shaded[0] = look.shade_with(&outer.2, &outer.3, &sa);
        self.shaded[1] = look.shade_with(&inner.2, &inner.3, &sa);
        self.sample_shade_seconds += t_shade.elapsed().as_secs_f64();
        let t_resample = std::time::Instant::now();
        resample(&r, &self.shaded, &self.lut, &mut self.output);
        self.resample_seconds += t_resample.elapsed().as_secs_f64();
        Ok(&self.output)
    }
}

/// Tent-resample the shaded outer and inner keyframe rectangles into `output`, blending
/// to the inner keyframe inside its frame, in linear light via `lut`.
pub(crate) fn resample(r: &Resample, shaded: &[Rgb8; 2], lut: &[f32], output: &mut Rgb8) {
    let (ow, oh) = r.out;
    let aspect = ow as f32 / oh as f32;
    let (so, si) = (&shaded[0], &shaded[1]);
    let (zo, zi) = r.zoom;
    let ((oxo, oyo), (oxi, oyi)) = (r.outer_at, r.inner_at);
    let (((owidth, oheight), ro), ((iwidth, iheight), ri)) = (r.outer, r.inner);
    let (sin, cos, ramp, margin) = (r.sin, r.cos, r.ramp, r.margin);
    let threads = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .clamp(1, oh);
    let rows_per = oh.div_ceil(threads);
    std::thread::scope(|sc| {
        for (c, chunk) in output.data.chunks_mut(rows_per * ow * 3).enumerate() {
            sc.spawn(move || {
                for (r, row) in chunk.chunks_mut(ow * 3).enumerate() {
                    let py = c * rows_per + r;
                    let yn = (py as f32 + 0.5) / oh as f32 - 0.5;
                    for px in 0..ow {
                        let xn = (px as f32 + 0.5) / ow as f32 - 0.5;
                        let (rx, ry) = rotated(xn, yn, sin, cos, aspect);
                        let (xi, yi) = (rx * zi + oxi, ry * zi + oyi);
                        let d = (2.0 * xi.abs()).max(2.0 * yi.abs());
                        let wi = ramp * ((1.0 - d) / margin).clamp(0.0, 1.0);
                        let mut col = [0f32; 3];
                        if wi < 1.0 {
                            let o = gather(
                                so,
                                lut,
                                rx * zo + oxo,
                                ry * zo + oyo,
                                TENT * zo * owidth as f32 / ow as f32,
                                ro,
                                (owidth, oheight),
                            );
                            col = o.map(|v| v * (1.0 - wi));
                        }
                        if wi > 0.0 {
                            let i = gather(
                                si,
                                lut,
                                xi,
                                yi,
                                TENT * zi * iwidth as f32 / ow as f32,
                                ri,
                                (iwidth, iheight),
                            );
                            col = [0, 1, 2].map(|j| col[j] + i[j] * wi);
                        }
                        for (o, v) in row[px * 3..px * 3 + 3].iter_mut().zip(col) {
                            let v = v.clamp(0.0, 1.0);
                            *o = (v.powf(1.0 / 2.2) * 255.0 + 0.5) as u8;
                        }
                    }
                }
            });
        }
    });
}

/// The width needed to cover every rotated output corner, including any centre offset.
pub(crate) fn coverage_rotated(w: f64, off: (f64, f64), out: (u32, u32), rotation: f64) -> f64 {
    let aspect = out.0 as f64 / out.1 as f64;
    let (sin, cos) = rotation.sin_cos();
    let x = w * (cos.abs() + sin.abs() / aspect) + 2.0 * off.0.abs();
    let y = w * (cos.abs() + sin.abs() * aspect) + 2.0 * off.1.abs() * aspect;
    x.max(y)
}

/// Width of an unrotated frame, including its complex centre offset.
pub(crate) fn coverage(w: f64, off: (f64, f64), out: (u32, u32)) -> f64 {
    coverage_rotated(w, off, out, 0.0)
}

/// Screen coordinates of an output pixel in the unrotated sample-grid axes.
fn rotated(x: f32, y: f32, sin: f32, cos: f32, aspect: f32) -> (f32, f32) {
    (cos * x + sin * y / aspect, cos * y - sin * x * aspect)
}

/// Conservative bounds on source samples touched by any rotated output tent.
pub(crate) fn needed_rect(
    out: (usize, usize),
    image: (usize, usize),
    z: f32,
    ox: f32,
    oy: f32,
    sin: f32,
    cos: f32,
) -> (usize, usize, usize, usize) {
    let ex = 0.5 - 0.5 / out.0 as f32;
    let ey = 0.5 - 0.5 / out.1 as f32;
    let aspect = out.0 as f32 / out.1 as f32;
    let dx = z * (ex * cos.abs() + ey * sin.abs() / aspect);
    let dy = z * (ey * cos.abs() + ex * sin.abs() * aspect);
    let f = (TENT * z * image.0 as f32 / out.0 as f32).max(1.0);
    let bounds = |n: usize, offset: f32, extent: f32| {
        let c0 = (offset - extent) * n as f32 + n as f32 / 2.0;
        let c1 = (offset + extent) * n as f32 + n as f32 / 2.0;
        let lo = ((c0.min(c1) - f).floor() as i64 - 3).clamp(0, n as i64 - 1) as usize;
        let hi = ((c0.max(c1) + f).ceil() as i64 + 4).clamp(lo as i64 + 1, n as i64) as usize;
        (lo, hi - lo)
    };
    let (x, w) = bounds(image.0, ox, dx);
    let (y, h) = bounds(image.1, oy, dy);
    (x, y, w, h)
}

/// Tent-filtered linear colour; the stored image may be a cropped source rectangle.
fn gather(
    img: &Rgb8,
    lut: &[f32],
    x: f32,
    y: f32,
    f: f32,
    rect: (usize, usize, usize, usize),
    full: (usize, usize),
) -> [f32; 3] {
    let f = f.max(1.0);
    let (iw, ih) = (img.w as i64, img.h as i64);
    let cx = x * full.0 as f32 + full.0 as f32 / 2.0;
    let cy = y * full.1 as f32 + full.1 as f32 / 2.0;
    let (x0, x1) = ((cx - f).floor() as i64, (cx + f).ceil() as i64);
    let (y0, y1) = ((cy - f).floor() as i64, (cy + f).ceil() as i64);
    let (mut acc, mut wsum) = ([0f32; 3], 0f32);
    for sy in y0..=y1 {
        let wy = 1.0 - ((sy as f32 + 0.5 - cy).abs() / f);
        if wy <= 0.0 {
            continue;
        }
        let row = (sy - rect.1 as i64).clamp(0, ih - 1) as usize * img.w;
        for sx in x0..=x1 {
            let wx = 1.0 - ((sx as f32 + 0.5 - cx).abs() / f);
            if wx <= 0.0 {
                continue;
            }
            let k = (row + (sx - rect.0 as i64).clamp(0, iw - 1) as usize) * 3;
            let wgt = wx * wy;
            for (a, &v) in acc.iter_mut().zip(&img.data[k..k + 3]) {
                *a += lut[v as usize] * wgt;
            }
            wsum += wgt;
        }
    }
    acc.map(|a| a / wsum.max(1e-12))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gather_keeps_flat_colour_and_weights_by_distance() {
        let flat = Rgb8 {
            w: 8,
            h: 4,
            data: [10u8, 128, 250].repeat(32),
        };
        let lut: Vec<f32> = (0..256).map(|v| v as f32).collect();
        for (x, y, f) in [(0.0, 0.0, 1.0), (0.49, -0.49, 3.0), (-0.3, 0.2, 0.2)] {
            let g = gather(&flat, &lut, x, y, f, (0, 0, 8, 4), (8, 4));
            assert!(
                g.iter()
                    .zip([10.0, 128.0, 250.0])
                    .all(|(a, b)| (a - b).abs() < 1e-3),
                "{g:?}"
            );
        }
        // Left half 0, right half 200: a point a quarter pixel right of the edge, with a
        // one-pixel tent, sees both sides, more of the right.
        let mut img = Rgb8 {
            w: 8,
            h: 4,
            data: vec![0; 96],
        };
        for r in 0..4 {
            for c in 4..8 {
                img.data[(r * 8 + c) * 3..(r * 8 + c) * 3 + 3].copy_from_slice(&[200; 3]);
            }
        }
        let v = gather(&img, &lut, 0.25 / 8.0, 0.0, 1.0, (0, 0, 8, 4), (8, 4))[0];
        assert!(v > 100.0 && v < 200.0, "{v}");
    }

    #[test]
    fn cropped_tent_matches_full_source_on_whole_output() {
        let full = Rgb8 {
            w: 128,
            h: 72,
            data: (0..128 * 72 * 3).map(|i| (i * 37 % 256) as u8).collect(),
        };
        let lut: [f32; 256] = std::array::from_fn(|i| (i as f32 / 255.0).powf(2.2));
        let out = (64, 36);
        for (z, ox, oy) in [(0.55, 0.0, 0.0), (0.81, 0.2, -0.13), (1.7, -0.28, 0.23)] {
            for angle in [0.0f32, 0.3, std::f32::consts::FRAC_PI_4, -1.2] {
                let (sin, cos) = angle.sin_cos();
                let rect = needed_rect(out, (full.w, full.h), z, ox, oy, sin, cos);
                let mut data = Vec::new();
                for y in rect.1..rect.1 + rect.3 {
                    let i = (y * full.w + rect.0) * 3;
                    data.extend_from_slice(&full.data[i..i + rect.2 * 3]);
                }
                let cropped = Rgb8 {
                    w: rect.2,
                    h: rect.3,
                    data,
                };
                for py in 0..out.1 {
                    let yn = (py as f32 + 0.5) / out.1 as f32 - 0.5;
                    for px in 0..out.0 {
                        let xn = (px as f32 + 0.5) / out.0 as f32 - 0.5;
                        let (rx, ry) = rotated(xn, yn, sin, cos, out.0 as f32 / out.1 as f32);
                        let x = rx * z + ox;
                        let y = ry * z + oy;
                        let f = TENT * z * full.w as f32 / out.0 as f32;
                        let a = gather(
                            &full,
                            &lut,
                            x,
                            y,
                            f,
                            (0, 0, full.w, full.h),
                            (full.w, full.h),
                        );
                        let b = gather(&cropped, &lut, x, y, f, rect, (full.w, full.h));
                        assert!(
                            a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-5),
                            "z {z}, angle {angle} at {px},{py}: {a:?} vs {b:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn rotated_footprint_preserves_keyframe_sample_pitch() {
        let p = Params {
            nx: 640,
            ny: 360,
            ss: 1,
            max_iter: 10,
            escape_radius: 1e10,
            columns: crate::render::columns("nu").unwrap(),
            threads: 1,
            tier: None,
        };
        let mut k = Keyframes::new("0", "0", 4.0, (320, 180), 2, &p, None, None);
        k.cover_rotations(2.05).unwrap();
        assert_eq!((k.p.nx, k.p.ny), (1600, 900));
        // A larger world-space footprint must not make sample spacing coarser.
        assert!((k.width(0) / f64::from(k.p.nx) - 4.0 / 640.0).abs() < 1e-12);
        assert!((k.width(1) / f64::from(k.p.nx) - 2.0 / 640.0).abs() < 1e-12);
        assert_eq!(k.octave(2.0 * k.footprint), 1);
        assert!(
            k.width(1)
                >= coverage_rotated(2.0, (0.0, 0.0), (320, 180), std::f64::consts::FRAC_PI_2,)
        );
    }

    #[test]
    fn octave_picks_the_pair_around_the_frame_width() {
        let p = Params {
            nx: 2,
            ny: 2,
            ss: 1,
            max_iter: 10,
            escape_radius: 1e10,
            columns: crate::render::columns("nu").unwrap(),
            threads: 1,
            tier: None,
        };
        let k = Keyframes::new("0", "0", 4.0, (2, 2), 2, &p, None, None);
        assert_eq!(k.octave(4.0), 0);
        assert_eq!(k.octave(2.0001), 0);
        assert_eq!(k.octave(2.0), 1);
        assert_eq!(k.octave(1e-80), (4e80f64).log2().floor() as usize);
        let j = k.octave(3e-50);
        assert!(k.width(j) >= 3e-50 && k.width(j + 1) < 3e-50);
        // Off-centre frames need wider keyframes: a frame 1 wide, 0.3 right of centre,
        // needs 1.6; 0.1 above centre in a 2:1 output needs 1 + 0.4.
        assert!((coverage(1.0, (0.3, 0.0), (2, 1)) - 1.6).abs() < 1e-12);
        assert!((coverage(1.0, (0.0, 0.1), (2, 1)) - 1.4).abs() < 1e-12);
        assert_eq!(coverage(2.0, (0.0, 0.0), (16, 9)), 2.0);
        for angle in [0.2f64, std::f64::consts::FRAC_PI_4, -1.0] {
            let req = coverage_rotated(2.0, (0.0, 0.0), (16, 9), angle);
            let (sin, cos) = angle.sin_cos();
            for (x, y) in [(-0.5, -0.5), (0.5, -0.5), (-0.5, 0.5), (0.5, 0.5)] {
                let (re, down) = (
                    2.0 * (cos * x + sin * y * 9.0 / 16.0),
                    2.0 * (cos * y - sin * x * 16.0 / 9.0),
                );
                assert!(re.abs() <= req / 2.0 + 1e-12);
                assert!(down.abs() <= req / 2.0 + 1e-12);
            }
        }
    }
}
