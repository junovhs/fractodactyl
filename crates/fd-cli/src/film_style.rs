//! `fd film --style GRAPH` (STYL-10): films shaded by a style graph (fd style's engine,
//! DEC-33) instead of a studio look. Keyframe films shade each keyframe's samples with the
//! graph at the frame's film time, then tent-resample them on the CPU exactly as other
//! non-studio looks do (keyframe.rs); direct frames are shaded whole.
//!
//! The renderer for a sample set is built once on one shared device and kept while that
//! keyframe is in use, so a keyframe's samples upload once per octave, not once per frame.
//! A graph without `source time` shades the same keyframe identically every frame, so its
//! image is reused.
use crate::args::Args;
use fd_samples::{ColumnSet, Header, Samples};
use fd_shade::style::finishing::Finishing;
use fd_shade::style::render::{columns, StyleRenderer};
use fd_shade::style::{Compiled, Graph};
use fd_shade::{Appearance, Pass, Rgb8};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Flags `fd film` takes for style looks, besides `--style` itself.
pub(crate) const STYLE_FLAGS: [&str; 4] = ["style", "set", "bloom", "bloom-radius"];

/// One keyframe's (or direct frame's) renderer and its last image.
struct Live {
    /// The sample set it was built from: samples address, sample count and view width.
    key: (usize, usize, String),
    renderer: StyleRenderer,
    /// The last image and the output scale it was shaded at.
    image: Option<(f64, Rgb8)>,
    used: u64,
}

pub(crate) struct StyleLook {
    graph: Compiled,
    settings: Vec<(String, f32)>,
    finishing: Finishing,
    animated: bool,
    device: (String, wgpu::Device, wgpu::Queue),
    live: RefCell<(u64, Vec<Live>)>,
    /// Grid pixels per output pixel for the next shade (keyframes are finer).
    scale: Cell<f64>,
    /// The first shading error; the film loop stops on it (Pass has no error channel).
    error: RefCell<Option<String>>,
}

impl StyleLook {
    /// The graph named by `--style`, with `--set name=value,...` and `--bloom` applied.
    pub(crate) fn from_args(a: &Args, path: &str) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("--style {path}: {e}"))?;
        let graph = Graph::parse(&text)?.compile()?;
        let mut settings = Vec::new();
        if let Some(list) = a.str("set") {
            for setting in list.split(',') {
                let (name, value) = setting
                    .split_once('=')
                    .ok_or("--set expects name=value pairs")?;
                let value: f32 = value.parse().map_err(|_| "invalid --set number")?;
                if !graph.parameters.iter().any(|(n, _)| n == name) {
                    return Err(format!("--set: the style has no parameter {name}"));
                }
                settings.push((name.to_string(), value));
            }
        }
        let finishing = Finishing {
            bloom: a.num("bloom", 0.0)?,
            bloom_radius: a.num("bloom-radius", 12)?,
            ..Finishing::default()
        };
        finishing.validate()?;
        let animated = graph.sources.contains("time");
        let device = fd_shade::gpu::testkit::open_device("fd film style")?;
        Ok(StyleLook {
            graph,
            settings,
            finishing,
            animated,
            device,
            live: RefCell::new((0, Vec::new())),
            scale: Cell::new(1.0),
            error: RefCell::new(None),
        })
    }

    pub(crate) fn device_name(&self) -> &str {
        &self.device.0
    }

    /// The error that stopped shading, if any.
    pub(crate) fn error(&self) -> Option<String> {
        self.error.borrow().clone()
    }

    fn shade(&self, h: &Header, s: &Samples, a: &Appearance) -> Result<Rgb8, String> {
        let key = (
            s.class.as_ptr() as usize,
            s.class.len(),
            h.view.width.clone(),
        );
        let mut live = self.live.borrow_mut();
        let (generation, entries) = &mut *live;
        *generation += 1;
        // A keyframe pair is shaded every frame; anything unused for a few calls is gone.
        let now = *generation;
        entries.retain(|e| now - e.used <= 4);
        let index = match entries.iter().position(|e| e.key == key) {
            Some(i) => i,
            None => {
                let (name, device, queue) = &self.device;
                let mut renderer = StyleRenderer::with_device(
                    self.graph.clone(),
                    h,
                    s,
                    name.clone(),
                    device.clone(),
                    queue.clone(),
                )?;
                for (name, value) in &self.settings {
                    renderer.set_parameter(name, *value)?;
                }
                renderer.set_finishing(&Finishing {
                    dither: a.dither,
                    ..self.finishing.clone()
                })?;
                entries.push(Live {
                    key,
                    renderer,
                    image: None,
                    used: now,
                });
                entries.len() - 1
            }
        };
        let entry = &mut entries[index];
        entry.used = now;
        let scale = self.scale.get();
        if self.animated || entry.image.as_ref().is_none_or(|(at, _)| *at != scale) {
            entry.renderer.set_output_scale((1.0 / scale) as f32)?;
            let image = entry.renderer.frame(a.time as f32, a.unresolved_interior)?;
            entry.image = Some((scale, image));
        }
        let (_, image) = entry.image.as_ref().expect("shaded");
        Ok(Rgb8 {
            w: image.w,
            h: image.h,
            data: image.data.clone(),
        })
    }
}

/// The look handed to the film pipeline; the film keeps the other handle to read errors.
pub(crate) struct SharedStyle(pub Rc<StyleLook>);

impl Pass for SharedStyle {
    fn columns(&self) -> ColumnSet {
        columns(&self.0.graph)
    }

    fn set_output_scale(&self, grid_pixels_per_output_pixel: f64) {
        self.0.scale.set(grid_pixels_per_output_pixel);
    }

    fn shade_with(&self, h: &Header, s: &Samples, a: &Appearance) -> Rgb8 {
        self.0.shade(h, s, a).unwrap_or_else(|e| {
            self.0.error.borrow_mut().get_or_insert(e);
            let (w, h) = h.pixels();
            Rgb8 {
                w,
                h,
                data: vec![0; w * h * 3],
            }
        })
    }
}
