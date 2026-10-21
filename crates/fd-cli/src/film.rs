//! `fd film` (FILM-01): one command from a location to an mp4. It generates the camera
//! path (constant or curve-driven zoom speed, optional ease and twist), renders every frame with the
//! cheapest valid kernel (the zone fast path where `--zone` covers the frame, otherwise
//! fd's own reference plus a per-frame BLA table, as `fd control --bla per-frame`),
//! shades it in memory with the chosen look at film time `frame / fps`, and pipes raw
//! RGB straight into ffmpeg: no `.fds` or PNG files touch the disk.
//!
//! Film defaults differ from `fd shade`'s on purpose: `--look studio --preset ice`,
//! `--ss 2`, `--aa on` and `--unresolved interior`, because a film should look its best,
//! not shimmer and not show magenta.
//! Progress (with an ETA) goes to stderr; one fd-film/1 JSON report goes to stdout at
//! the end. `--compare-every K` also times fd's per-frame BLA render on every K-th
//! zone frame, so the report can state the speed-up against it.
use crate::args::Args;
use crate::orbit::EPS;
use crate::path::sig6;
use crate::render::{columns, size};
use crate::shade::{appearance, passes, APPEARANCE_FLAGS, LOOK_FLAGS};
use fd_kernel::{
    bla_dc_max, render_bla, render_bla_masked, render_with, render_with_masked, zone_covers, Bla,
    Params, Series, Zone,
};
use fd_samples::{Header, Samples, View};
use fd_shade::Appearance;
use std::io::Write as _;
use std::process::{Command, Stdio};
use std::time::Instant;

const USAGE: &str = "usage: fd film (RE IM --to WIDTH | --place NAME [--to WIDTH (the place's)]) --mp4 FILE [--from W0 (4)] [--fps F (60)] \
[--json] [--seconds S | --rate DECADES_PER_S (0.15) | --speed FILE] [--twist TURNS (0) | --spin PEAK,ON,OFF | --spins FILE] [--ease on|off (off) | --ease-in S (0)] \
[--size WxH (1920x1080)] [--ss N (2)] [--iter N (100000)] [--threads N] [--zone FILE] \
[--look L (studio)] [--preset LOOK (ice)] [look knobs as fd shade] [--crf N (16)] [--x264 P (slow)] [--chroma 420|444 (420)] [--compare-every K (0)] \
[--frames A..B] [--raw-dir DIR (save lossless RGB24 frames)] [--keyframes M|off (2; M x oversized keyframe per 2x zoom; off renders every frame)] [--gpu on|off (accepted, ignored: the studio look always shades on wgpu)] \
[--ease-out S] [--offset DRE,DIM (keyframes only)] [--t0 SECONDS (0)] [--jet-aa on|off (off; ss 2 from ~1-3 samples/px, FILM-16)] [fd shade's appearance flags; film defaults --aa on --unresolved interior --dither on]";

/// Which kernel rendered a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // Keep retired CPU routes explicit as zero entries in the report schema.
pub(crate) enum Used {
    Zone,
    Bla,
    Plain,
    ZoneGpu,
    Mid,
    Scaled,
    SeriesBla,
    SeriesPlain,
}

impl Used {
    const ALL: [Self; 8] = [
        Self::Zone,
        Self::Bla,
        Self::Plain,
        Self::ZoneGpu,
        Self::Mid,
        Self::Scaled,
        Self::SeriesBla,
        Self::SeriesPlain,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::Zone => "zone",
            Self::Bla => "bla",
            Self::Plain => "perturbation",
            Self::ZoneGpu => "zone-gpu",
            Self::Mid => "mid",
            Self::Scaled => "scaled",
            Self::SeriesBla => "series-bla",
            Self::SeriesPlain => "series-perturbation",
        }
    }

    pub(crate) fn zone(header: &Header) -> Self {
        if header.kernel.starts_with("zone-mid-") {
            Self::Mid
        } else {
            Self::ZoneGpu
        }
    }

    fn legacy(self) -> usize {
        match self {
            Self::Zone | Self::ZoneGpu | Self::Mid => 0,
            Self::Bla | Self::SeriesBla => 1,
            Self::Plain | Self::Scaled | Self::SeriesPlain => 2,
        }
    }
}

/// Observes builds without retaining or reusing mathematical products.
#[derive(Default)]
pub(crate) struct BuildTimings {
    reference_seconds: f64,
    bla_seconds: f64,
    reference_builds: usize,
    repeated_references: usize,
    repeated_reference_seconds: f64,
    bla_builds: usize,
    repeated_blas: usize,
    repeated_bla_seconds: f64,
    references: std::collections::HashSet<String>,
    blas: std::collections::HashSet<String>,
}

impl BuildTimings {
    fn reference_key(view: &View, p: &Params, bits: u32) -> String {
        format!(
            "{}|{}|{bits}|{}|{}",
            view.center_re,
            view.center_im,
            p.max_iter,
            p.escape_radius.to_bits()
        )
    }

    fn reference(&mut self, key: &str, seconds: f64) {
        self.reference_seconds += seconds;
        self.reference_builds += 1;
        if !self.references.insert(key.to_owned()) {
            self.repeated_references += 1;
            self.repeated_reference_seconds += seconds;
        }
    }

    fn bla(&mut self, key: &str, dc: f64, seconds: f64) {
        self.bla_seconds += seconds;
        self.bla_builds += 1;
        // No rounding/bucketing is used by today's builder: its exact dc_max bits
        // and EPS are part of the build input, not merely a potential reuse radius.
        let key = format!("{key}|{}|{}", EPS.to_bits(), dc.to_bits());
        if !self.blas.insert(key) {
            self.repeated_blas += 1;
            self.repeated_bla_seconds += seconds;
        }
    }
}

// Reproducible before/after measurements and an explicit diagnostic fallback.
// The normal film path always chooses before rendering; it never renders truth
// as part of production authorization.
fn prepare_cpu_series(
    view: &View,
    p: &Params,
    orbit: &(fd_kernel::Reference, u32),
    bla: Option<&Bla>,
    mask: Option<&[bool]>,
) -> Result<Option<(Series, bool)>, String> {
    if std::env::var_os("FD_CPU_SERIES").is_some_and(|v| v == "off") {
        return Ok(None);
    }
    Series::prepare(view, p, orbit, bla, mask)
}

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let mut known = vec![
        "to",
        "mp4",
        "from",
        "fps",
        "seconds",
        "rate",
        "speed",
        "twist",
        "ease",
        "size",
        "ss",
        "iter",
        "threads",
        "zone",
        "look",
        "crf",
        "x264",
        "chroma",
        "compare-every",
        "frames",
        "place",
        "ease-in",
        "spin",
        "spins",
        "keyframes",
        "ease-out",
        "offset",
        "t0",
        "gpu",
        "gpu-kernel",
        "raw-dir",
        "jet-aa",
    ];
    known.extend(APPEARANCE_FLAGS);
    known.extend(LOOK_FLAGS);
    // Film always emits fd-film/1; accept an explicit --json for report consumers.
    let argv: Vec<_> = argv
        .iter()
        .filter(|a| a.as_str() != "--json")
        .cloned()
        .collect();
    let a = Args::parse(&argv, &known)?;
    // Zone frames and keyframes run on the wgpu zone kernel (CLEA-04); zone_gpu::choose
    // picks it when the flag is absent.
    let kernel_flag = a.str("gpu-kernel");
    if let Some(value) = kernel_flag {
        crate::zone_gpu::Kernel::parse(value)?;
    }
    if kernel_flag.is_some() && a.str("zone").is_none() {
        return Err("--gpu-kernel requires --zone".into());
    }
    // The studio look shades only on wgpu (SHAD-11); --gpu stays accepted for old
    // command lines.
    if let Some(v) = a.str("gpu").filter(|v| !matches!(*v, "on" | "off")) {
        return Err(format!("--gpu: expected on or off, got {v:?}"));
    }
    // `--place NAME` (EXPL-08): a place saved by fd explore gives RE IM and the default --to.
    let place = a.str("place").map(crate::place::load).transpose()?;
    let (re, im, to) = match (&place, a.positional.as_slice()) {
        (Some(p), []) => (&p.re, &p.im, a.str("to").unwrap_or(&p.width)),
        (None, [re, im]) => (re, im, a.need("to")?),
        _ => return Err(USAGE.into()),
    };
    let mp4 = a.need("mp4")?;
    let fps: f64 = a.num("fps", 60.0)?;
    let speed_file = a.str("speed");
    let from = a.str("from").unwrap_or("4");
    let width_log = |name: &str, value: &str| -> Result<f64, String> {
        let d = fd_fixed::Decimal::parse(value)?;
        let log = d.log2_abs() * std::f64::consts::LOG10_2;
        if d.neg || !log.is_finite() {
            return Err(format!(
                "--{name}: expected a positive width, got {value:?}"
            ));
        }
        Ok(log)
    };
    let (l0, l1) = (width_log("from", from)?, width_log("to", to)?);
    // Existing constant-rate films are f64-limited; curve films can keep deep widths
    // as decimal strings without ever converting their magnitude to f64 (DEC-21).
    let (w0, mut w1): (f64, f64) = if speed_file.is_some() {
        (from.parse().unwrap_or(0.0), to.parse().unwrap_or(0.0))
    } else {
        (
            a.num("from", 4.0)?,
            to.parse()
                .ok()
                .filter(|w: &f64| *w > 0.0)
                .ok_or_else(|| format!("--to: bad or out-of-range width {to:?}"))?,
        )
    };
    let ease = match a.str("ease").unwrap_or("off") {
        "on" => true,
        "off" => false,
        v => return Err(format!("--ease: expected on or off, got {v:?}")),
    };
    // `--ease-in S` (FILM-02): the zoom speed ramps from zero to --rate over S seconds,
    // then holds. With --seconds the film stops wherever that leaves it, short of --to.
    let ease_in: f64 = a.num("ease-in", 0.0)?;
    if !(ease_in.is_finite() && ease_in >= 0.0) || (ease_in > 0.0 && ease) {
        return Err("--ease-in: expected seconds >= 0, and not with --ease on".into());
    }
    // `--ease-out S` (FILM-05): the speed holds --rate, then eases to rest over the last
    // S seconds (an ending; the mirror of --ease-in).
    let ease_out: f64 = a.num("ease-out", 0.0)?;
    if !(ease_out.is_finite() && ease_out >= 0.0) || (ease_out > 0.0 && (ease || ease_in > 0.0)) {
        return Err(
            "--ease-out: expected seconds >= 0, and not with --ease on or --ease-in".into(),
        );
    }
    if speed_file.is_some()
        && ["seconds", "rate", "ease", "ease-in", "ease-out"]
            .iter()
            .any(|flag| a.str(flag).is_some())
    {
        return Err("--speed cannot be combined with --seconds, --rate or --ease flags".into());
    }
    let speed_curve = speed_file
        .map(|file| {
            let text =
                std::fs::read_to_string(file).map_err(|e| format!("--speed {file:?}: {e}"))?;
            SpeedCurve::parse(&text, l0, l1)
        })
        .transpose()?;
    let rate: f64 = a.num("rate", 0.15)?;
    let seconds = if let Some(curve) = &speed_curve {
        curve.total_seconds
    } else {
        match a.str("seconds") {
            Some(_) => a.num("seconds", 0.0)?,
            None if ease_out > 0.0 => (w0 / w1).log10().abs() / rate + ease_out / 2.0,
            None if ease_in > 0.0 => {
                let t = (w0 / w1).log10().abs() / rate + ease_in / 2.0;
                if t < ease_in {
                    return Err(format!("--ease-in {ease_in}: the zoom reaches --to before full speed; shorten --ease-in"));
                }
                t
            }
            None => (w0 / w1).log10().abs() / rate,
        }
    };
    if ease_in > 0.0 && a.str("seconds").is_some() {
        let end = w0 / 10f64.powf(rate * eased_depth(seconds, ease_in));
        if end < w1 {
            return Err(format!(
                "--ease-in: {seconds} s at --rate {rate} would pass --to ({end:e} < {w1:e})"
            ));
        }
        w1 = end;
    }
    if ease_out > 0.0 && a.str("seconds").is_some() {
        if seconds < ease_out {
            return Err(format!(
                "--ease-out {ease_out} is longer than --seconds {seconds}"
            ));
        }
        let end = w0 / 10f64.powf(rate * out_depth(seconds, seconds, ease_out));
        if end < w1 {
            return Err(format!(
                "--ease-out: {seconds} s at --rate {rate} would pass --to ({end:e} < {w1:e})"
            ));
        }
        w1 = end;
    }
    // `--offset DRE,DIM` (FILM-05): the camera centre starts at the target plus this
    // complex offset; the target then eases to the middle of the screen (see veer).
    let offset = match a.str("offset") {
        None => (0.0, 0.0),
        Some(v) => match v
            .split_once(',')
            .map(|(x, y)| (x.trim().parse::<f64>(), y.trim().parse::<f64>()))
        {
            Some((Ok(x), Ok(y))) if x.is_finite() && y.is_finite() => (x, y),
            _ => return Err(format!("--offset: expected DRE,DIM, got {v:?}")),
        },
    };
    let t0: f64 = a.num("t0", 0.0)?;
    // `--spin PEAK,ON,OFF` (FILM-03): occasional spins in place of a constant --twist.
    let spin = match a.str("spin") {
        None => None,
        Some(v) => {
            let bad = || {
                format!(
                    "--spin: expected PEAK_TURNS_PER_S,ON_S,OFF_S (all >= 0, ON > 0), got {v:?}"
                )
            };
            let x: Vec<f64> = v
                .split(',')
                .map(|t| t.trim().parse::<f64>().map_err(|_| bad()))
                .collect::<Result<_, _>>()?;
            match x[..] {
                [p, on, off]
                    if [p, on, off].iter().all(|t| t.is_finite() && *t >= 0.0) && on > 0.0 =>
                {
                    Some((p, on, off))
                }
                _ => return Err(bad()),
            }
        }
    };
    if [a.str("spin"), a.str("spins"), a.str("twist")]
        .iter()
        .filter(|v| v.is_some())
        .count()
        > 1
    {
        return Err("--spin, --spins and --twist are exclusive".into());
    }
    let twist = a.num("twist", 0.0)?;
    let mut views = if let Some(curve) = &speed_curve {
        curve.path(re, im, fps, twist, spin)?
    } else {
        path(
            re, im, w0, w1, seconds, fps, twist, ease, ease_in, spin, ease_out,
        )?
    };
    // Placed spins are additive turns; each has zero angular speed at both ends.
    if let Some(file) = a.str("spins") {
        let text = std::fs::read_to_string(file).map_err(|e| format!("--spins {file:?}: {e}"))?;
        let spins = parse_spins(&text, seconds)?;
        let last = (views.len() - 1) as f64;
        for (i, view) in views.iter_mut().enumerate() {
            view.rotation =
                std::f64::consts::TAU * placed_spin_turns(seconds * i as f64 / last, &spins);
        }
    }
    if views.iter().any(|v| !v.rotation.is_finite()) {
        return Err("film rotation is out of range".into());
    }
    let (w, h) = size(a.str("size").unwrap_or("1920x1080"))?;
    let ss: u32 = a.num("ss", 2)?;
    let p = Params {
        nx: w * ss,
        ny: h * ss,
        ss,
        max_iter: a.num("iter", 100_000)?,
        escape_radius: 1e10,
        columns: columns("nu,de,normal")?,
        threads: a.num(
            "threads",
            std::thread::available_parallelism().map_or(1, |n| n.get()),
        )?,
        tier: None,
    };
    let look_name = a.str("look").unwrap_or("studio");
    let mut base = appearance(&a)?;
    let look = match passes(look_name, &a, &mut base)?.pop() {
        Some((_, p)) if !look_name.contains(',') => p,
        _ => return Err("fd film takes one --look".into()),
    };
    base.aa = a.str("aa").is_none_or(|v| v == "on");
    base.unresolved_interior = a.str("unresolved").is_none_or(|v| v == "interior");
    base.dither = a.str("dither").is_none_or(|v| v == "on");
    let zone = a.str("zone").map(Zone::load).transpose()?;
    // `--jet-aa on` (FILM-16): ss 2 frames from one computed sample per pixel plus the
    // samples the jet rebuild rejects (jetaa.rs). Films only; stills stay exact.
    let jet_aa = match a.str("jet-aa").unwrap_or("off") {
        "off" => false,
        "on" => true,
        v => return Err(format!("--jet-aa: expected on or off, got {v:?}")),
    };
    if jet_aa && (ss != 2 || kernel_flag.is_some()) {
        return Err("--jet-aa needs --ss 2 and no --gpu-kernel".into());
    }
    let zone_kernel = zone
        .as_ref()
        .map(|z| crate::zone_gpu::choose(kernel_flag, z))
        .transpose()?;
    // FILM-18 / DEC-29: films use the validated M=2 keyframe route by default.
    // Unsupported settings retain direct rendering rather than failing a film.
    let requested_keys = match a.str("keyframes").unwrap_or("2") {
        "off" => None,
        value => {
            let m = value
                .parse::<u32>()
                .ok()
                .filter(|m| (1..=4).contains(m))
                .ok_or("--keyframes takes M in 1..=4 or off")?;
            Some(m)
        }
    };
    let fallback = if a.str("ss").is_some() {
        Some("explicit --ss")
    } else if jet_aa {
        Some("--jet-aa on")
    } else if !(w0.is_normal() && w1.is_normal() && w0 > 0.0 && w1 > 0.0) {
        Some("widths outside the normal f64 range")
    } else {
        None
    };
    let keyframes = match (requested_keys, fallback) {
        (Some(_), Some(why)) => {
            eprintln!("fd film: keyframes off ({why}); rendering every frame directly");
            None
        }
        (mode, _) => mode,
    };
    let mut keys = match keyframes {
        None => None,
        Some(m) => {
            // Render the diagonal footprint at the original per-pixel sample pitch.
            let rotating = views.iter().any(|v| v.rotation != 0.0);
            let footprint = if rotating {
                let scale = views.iter().enumerate().try_fold(
                    1.0f64,
                    |scale, (f, view)| -> Result<f64, String> {
                        let width: f64 = view
                            .width
                            .parse()
                            .map_err(|_| "bad keyframe width".to_string())?;
                        let off = veer(offset, f as f64 / (views.len() - 1) as f64, width / w0);
                        Ok(scale.max(
                            crate::keyframe::coverage_rotated(width, off, (w, h), view.rotation)
                                / width,
                        ))
                    },
                )?;
                // One output pixel of extra clearance for the tent at the corners.
                scale * (1.0 + 2.0 / w.min(h) as f64)
            } else {
                1.0
            };
            let base = if rotating {
                w0
            } else {
                crate::keyframe::coverage(w0, offset, (w, h))
            };
            let mut keys = crate::keyframe::Keyframes::new(
                re,
                im,
                base,
                (w, h),
                m,
                &p,
                zone.as_ref(),
                if look_name == "studio" {
                    Some(crate::shade::studio_look(&a)?)
                } else {
                    None
                },
            );
            if rotating {
                keys.cover_rotations(footprint)?;
            }
            Some(keys)
        }
    };
    if let (Some(mode), Some(k)) = (zone_kernel, keys.as_mut()) {
        k.enable_zone_kernel(mode);
    }
    // Direct frames (--keyframes off or a fallback) share one zone renderer.
    let mut frame_gpu = zone_kernel
        .filter(|_| keys.is_none())
        .map(crate::zone_gpu::Renderer::new);
    if look_name == "studio" {
        let device = match keys.as_mut() {
            Some(k) => k.enable_gpu(
                &crate::shade::studio_look(&a)?,
                &Appearance { time: t0, ..base },
            ),
            None => fd_shade::gpu::shared_adapter(),
        }
        .map_err(|e| format!("studio look needs a wgpu adapter: {e}"))?;
        eprintln!("fd film: wgpu GPU shading on {device}");
    }
    if keys.is_none() && offset != (0.0, 0.0) {
        return Err("--offset needs --keyframes".into());
    }
    let compare_every: usize = a.num("compare-every", 0)?;
    let (from, to) = match a.str("frames") {
        None => (0, views.len()),
        Some(r) => {
            let bad = || {
                format!(
                    "--frames: expected A..B within 0..{}, got {r:?}",
                    views.len()
                )
            };
            let (x, y) = r.split_once("..").ok_or_else(bad)?;
            let (x, y): (usize, usize) =
                (x.parse().map_err(|_| bad())?, y.parse().map_err(|_| bad())?);
            if x >= y || y > views.len() {
                return Err(bad());
            }
            (x, y)
        }
    };
    let raw_dir = a.str("raw-dir");
    if let Some(dir) = raw_dir {
        std::fs::create_dir_all(dir).map_err(|e| format!("--raw-dir {dir}: {e}"))?;
        std::fs::write(format!("{dir}/size.txt"), format!("{w}x{h}\n"))
            .map_err(|e| format!("--raw-dir metadata: {e}"))?;
    }
    let crf: u32 = a.num("crf", 16)?;
    let preset = a.str("x264").unwrap_or("slow");
    // 4:2:0 halves colour resolution, which softens thin coloured lines (terrain lines
    // lose ~7 levels on average); 4:4:4 keeps them exact but some players cannot play it.
    let pix_fmt = match a.str("chroma").unwrap_or("420") {
        "420" => "yuv420p",
        "444" => "yuv444p",
        v => return Err(format!("--chroma: expected 420 or 444, got {v:?}")),
    };

    eprintln!(
        "fd film: {} frames ({:.1} s at {fps} fps), {w}x{h} ss {ss}, look {look_name}{}, zone {}",
        to - from,
        (to - from) as f64 / fps,
        if look_name == "studio" {
            format!(" preset {}", a.str("preset").unwrap_or("ice"))
        } else {
            String::new()
        },
        zone.as_ref().map_or("none".to_string(), |z| format!(
            "P={} within {:e}",
            z.period, z.max_dc
        ))
    );
    let mut enc = Command::new("ffmpeg")
        .args([
            "-y",
            "-loglevel",
            "error",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "-s",
            &format!("{w}x{h}"),
            "-r",
            &fps.to_string(),
        ])
        .args([
            "-i",
            "-",
            "-c:v",
            "libx264",
            "-crf",
            &crf.to_string(),
            "-preset",
            preset,
            "-pix_fmt",
            pix_fmt,
        ])
        // With dither on, keep it through the encode: grain tuning and dark-biased
        // adaptive quantisation stop x264 smoothing slow gradients back into rings (FX-04).
        .args(if base.dither {
            &["-tune", "grain", "-aq-mode", "3"][..]
        } else {
            &[][..]
        })
        .arg(mp4)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("ffmpeg: {e} (fd film needs ffmpeg on PATH)"))?;
    let mut pipe = enc.stdin.take().expect("piped");

    let wall = Instant::now();
    let mut r = Report {
        keyframes,
        ..Report::default()
    };
    let mut builds = BuildTimings::default();
    for (f, view) in views.iter().enumerate().take(to).skip(from) {
        if let Some(k) = keys.as_mut() {
            let t = Instant::now();
            let before = k.rendered.len();
            let width: f64 = view.width.parse().map_err(|_| "bad width".to_string())?;
            let off = veer(offset, f as f64 / (views.len() - 1) as f64, width / w0);
            let img = k.frame(
                width,
                off,
                view.rotation,
                look.as_ref(),
                &Appearance {
                    time: t0 + f as f64 / fps,
                    ..base
                },
            )?;
            let frame_seconds = t.elapsed().as_secs_f64();
            let t = Instant::now();
            if let Some(dir) = raw_dir {
                std::fs::write(format!("{dir}/frame-{f:06}.rgb"), &img.data)
                    .map_err(|e| format!("--raw-dir frame {f}: {e}"))?;
            }
            pipe.write_all(&img.data)
                .map_err(|e| format!("ffmpeg pipe: {e}"))?;
            r.encode_wait += t.elapsed().as_secs_f64();
            let kf: f64 = k.rendered[before..].iter().map(|x| x.1).sum();
            for &(_, secs, used) in &k.rendered[before..] {
                r.add(used, secs);
            }
            r.shade += frame_seconds - kf;
            r.output_frames[k.frame_route as usize] += 1;
            let done = f + 1 - from;
            if done == 1 || done % 60 == 0 || done == to - from {
                let el = wall.elapsed().as_secs_f64();
                let left = el / done as f64 * (to - from - done) as f64;
                eprintln!("  frame {done}/{} width {} (keyframes {}) {el:.0} s elapsed, ~{left:.0} s left", to - from, view.width, k.rendered.len());
            }
            continue;
        }
        let t = Instant::now();
        let (hd, mut s, used) = if jet_aa {
            let (hd, s, used, computed, st) =
                render_frame_jet(view, &p, zone.as_ref(), &mut frame_gpu, &mut builds)?;
            r.jet_samples += computed;
            (0..3).for_each(|k| r.jet_stage[k] += st[k]);
            r.jet_pixels += (p.nx / 2) as u64 * (p.ny / 2) as u64;
            (hd, s, used)
        } else {
            render_frame_profiled(view, &p, zone.as_ref(), &mut frame_gpu, &mut builds)?
        };
        // Light turns with the fractal, as in keyframe films: express normals in the
        // unrotated screen axes before shading (owner, FILM-14).
        if let Some(normal) = s.normal.as_mut().filter(|_| view.rotation != 0.0) {
            let turn = (view.rotation / std::f64::consts::TAU * 65536.0).round() as i64 as u16;
            normal.iter_mut().for_each(|a| *a = a.wrapping_sub(turn));
        }
        let secs = t.elapsed().as_secs_f64();
        r.add(used, secs);
        r.output_frames[used as usize] += 1;
        if used.legacy() == 0 && compare_every > 0 && (f - from).is_multiple_of(compare_every) {
            let t = Instant::now();
            render_frame(view, &p, None)?;
            r.compared.push((secs, t.elapsed().as_secs_f64()));
        }
        let t = Instant::now();
        let img = look.shade_with(
            &hd,
            &s,
            &Appearance {
                time: t0 + f as f64 / fps,
                ..base
            },
        );
        r.shade += t.elapsed().as_secs_f64();
        let t = Instant::now();
        if let Some(dir) = raw_dir {
            std::fs::write(format!("{dir}/frame-{f:06}.rgb"), &img.data)
                .map_err(|e| format!("--raw-dir frame {f}: {e}"))?;
        }
        pipe.write_all(&img.data)
            .map_err(|e| format!("ffmpeg pipe: {e}"))?;
        r.encode_wait += t.elapsed().as_secs_f64();
        let done = f + 1 - from;
        if done == 1 || done % 60 == 0 || done == to - from {
            let el = wall.elapsed().as_secs_f64();
            eprintln!(
                "  frame {done}/{} width {} ({}) {:.0} s elapsed, ~{:.0} s left",
                to - from,
                view.width,
                used.name(),
                el,
                el / done as f64 * (to - from - done) as f64
            );
        }
    }
    if let Some(k) = &keys {
        r.keyframe_prepare = k.prepare_seconds;
        r.keyframe_sample_shade = k.sample_shade_seconds;
        r.keyframe_resample = k.resample_seconds;
        r.gpu_device = k.gpu_device().map(str::to_owned);
        r.zone_readback_bytes = k.zone_readback_bytes;
    }
    r.builds = keys.map_or(builds, |k| k.builds);
    drop(pipe);
    let encode_finish = Instant::now();
    let status = enc.wait().map_err(|e| format!("ffmpeg: {e}"))?;
    if !status.success() {
        return Err(format!("ffmpeg failed: {status}"));
    }
    r.encode_wait += encode_finish.elapsed().as_secs_f64();
    println!(
        "{}",
        r.json(mp4, to - from, fps, wall.elapsed().as_secs_f64())
    );
    Ok(())
}

/// A span whose elapsed time integrates to exactly length * seconds_per_decade.
/// Consecutive spans share a time/depth derivative, so the zoom has no speed jump.
struct SpeedSpan {
    depth: f64,
    length: f64,
    elapsed: f64,
    duration: f64,
    pace: f64,
    rest: bool,
    d0: f64,
    d1: f64,
}

impl SpeedSpan {
    fn time_at(&self, u: f64) -> f64 {
        let u2 = u * u;
        let u3 = u2 * u;
        self.duration * (3.0 * u2 - 2.0 * u3)
            + self.length * (self.d0 * (u3 - 2.0 * u2 + u) + self.d1 * (u3 - u2))
    }

    fn depth_at(&self, seconds: f64) -> f64 {
        let t = seconds.clamp(0.0, self.duration);
        if self.rest {
            // v(t) = cruise * smoothstep(t / duration): starts at rest and reaches
            // cruise with zero acceleration. Its exact integral is used here.
            let u = t / self.duration;
            return self.depth + 2.0 * self.length * (u * u * u - u.powi(4) / 2.0);
        }
        // The time spline is monotone (harmonic-mean slopes); invert it without
        // approximating instantaneous speed or accumulating a per-frame step.
        let (mut lo, mut hi) = (0.0, 1.0);
        for _ in 0..48 {
            let u = (lo + hi) / 2.0;
            if self.time_at(u) < t {
                lo = u;
            } else {
                hi = u;
            }
        }
        self.depth + self.length * (lo + hi) / 2.0
    }
}

/// Each row is "log10_width seconds_per_decade", in descending log-width order.
/// A row sets the exact average pace until the next row (or --to). A first-row
/// "rest" uses a smoothstep-in-time ramp to the next row's pace; that span
/// takes twice its width in decades times that pace, and starts at zero speed.
struct SpeedCurve {
    log_from: f64,
    depth: f64,
    total_seconds: f64,
    spans: Vec<SpeedSpan>,
}

impl SpeedCurve {
    fn parse(text: &str, log_from: f64, log_to: f64) -> Result<Self, String> {
        if !(log_from.is_finite() && log_to.is_finite() && log_from > log_to) {
            return Err("--speed needs --from wider than --to".into());
        }
        let mut rows: Vec<(f64, Option<f64>)> = Vec::new();
        for (i, line) in text.lines().enumerate() {
            let fields: Vec<_> = line
                .split('#')
                .next()
                .unwrap_or("")
                .split_whitespace()
                .collect();
            if fields.is_empty() {
                continue;
            }
            if fields.len() != 2 {
                return Err(format!(
                    "--speed line {}: expected log10_width seconds_per_decade",
                    i + 1
                ));
            }
            let x: f64 = fields[0]
                .parse()
                .map_err(|_| format!("--speed line {}: invalid log10_width", i + 1))?;
            let pace = if fields[1] == "rest" && rows.is_empty() {
                None
            } else {
                let v: f64 = fields[1]
                    .parse()
                    .map_err(|_| format!("--speed line {}: invalid seconds_per_decade", i + 1))?;
                if !v.is_finite() || v <= 0.0 {
                    return Err(format!(
                        "--speed line {}: seconds_per_decade must be finite and > 0",
                        i + 1
                    ));
                }
                Some(v)
            };
            if !x.is_finite() || rows.last().is_some_and(|(last, _)| x >= *last) {
                return Err(format!(
                    "--speed line {}: log widths must be finite and strictly descending",
                    i + 1
                ));
            }
            rows.push((x, pace));
        }
        if rows.is_empty() || (rows[0].0 - log_from).abs() > 1e-7 {
            return Err("--speed: first log10_width must equal log10(--from)".into());
        }
        let depth = log_from - log_to;
        let mut spans: Vec<SpeedSpan> = Vec::new();
        let mut elapsed = 0.0;
        for (i, &(x, specified)) in rows.iter().enumerate() {
            let start = if i == 0 { 0.0 } else { log_from - x };
            if start > depth + 1e-7 {
                return Err("--speed: a log10_width is beyond --to".into());
            }
            if start >= depth - 1e-8 {
                if i + 1 != rows.len() {
                    return Err("--speed: no knots allowed after --to".into());
                }
                break;
            }
            let end = if i + 1 < rows.len() {
                (log_from - rows[i + 1].0).min(depth)
            } else {
                depth
            };
            if end <= start {
                return Err("--speed: curve spans must have positive width".into());
            }
            let rest = specified.is_none();
            let pace = if let Some(v) = specified {
                v
            } else {
                rows.get(i + 1).and_then(|row| row.1).ok_or_else(|| {
                    "--speed: first-row rest needs a following finite pace".to_string()
                })?
            };
            let length = end - start;
            let duration = length * pace * if rest { 2.0 } else { 1.0 };
            if !duration.is_finite() || duration <= 0.0 || !(elapsed + duration).is_finite() {
                return Err("--speed: duration is out of range".into());
            }
            spans.push(SpeedSpan {
                depth: start,
                length,
                elapsed,
                duration,
                pace,
                rest,
                d0: pace,
                d1: pace,
            });
            elapsed += duration;
        }
        if spans.is_empty() {
            return Err("--speed: curve has no span before --to".into());
        }
        // Harmonic means preserve monotone cumulative times and C1 zoom speed.
        let harmonic = |a: f64, b: f64| 2.0 * a.min(b) / (1.0 + a.min(b) / a.max(b));
        for i in 0..spans.len() {
            if spans[i].rest {
                spans[i].d1 = spans[i].pace;
            } else {
                if i > 0 && !spans[i - 1].rest {
                    spans[i].d0 = harmonic(spans[i - 1].pace, spans[i].pace);
                }
                if i + 1 < spans.len() {
                    spans[i].d1 = harmonic(spans[i].pace, spans[i + 1].pace);
                }
            }
        }
        Ok(Self {
            log_from,
            depth,
            total_seconds: elapsed,
            spans,
        })
    }

    fn log_at(&self, time: f64) -> f64 {
        let t = time.clamp(0.0, self.total_seconds);
        let s = self
            .spans
            .iter()
            .find(|s| t <= s.elapsed + s.duration)
            .unwrap_or_else(|| self.spans.last().expect("nonempty curve"));
        self.log_from - s.depth_at(t - s.elapsed)
    }

    fn path(
        &self,
        re: &str,
        im: &str,
        fps: f64,
        twist: f64,
        spin: Option<(f64, f64, f64)>,
    ) -> Result<Vec<View>, String> {
        fd_fixed::Decimal::parse(re)?;
        fd_fixed::Decimal::parse(im)?;
        let n = (self.total_seconds * fps).round();
        if !(2.0..=1e7).contains(&n) || !twist.is_finite() {
            return Err(format!(
                "--speed duration x --fps must give 2 to 1e7 frames, got {n}"
            ));
        }
        let n = n as usize;
        Ok((0..n)
            .map(|f| {
                let t = self.total_seconds * f as f64 / (n - 1) as f64;
                let log = self.log_at(t);
                let progress = (self.log_from - log) / self.depth;
                View {
                    center_re: re.to_string(),
                    center_im: im.to_string(),
                    width: sig6_log(log),
                    rotation: std::f64::consts::TAU
                        * spin.map_or(twist * progress, |sp| spin_turns(t, sp)),
                }
            })
            .collect())
    }
}

/// Six significant digits, retaining a deep width's decimal exponent (DEC-21).
fn sig6_log(log: f64) -> String {
    let exponent = log.floor() as i64;
    let (mantissa, carry) = {
        let rounded = sig6(10f64.powf(log - exponent as f64));
        let (m, e) = rounded.split_once('e').expect("scientific notation");
        (
            m.to_string(),
            e.parse::<i64>().expect("scientific exponent"),
        )
    };
    format!("{mantissa}e{}", exponent + carry)
}

/// Decades zoomed by time `t` at unit full speed when the speed follows smoothstep(t/s)
/// for `t < s` and holds at 1 after (zero speed and acceleration at the start).
pub(crate) fn eased_depth(t: f64, s: f64) -> f64 {
    if t >= s {
        t - s / 2.0
    } else {
        let u = t / s;
        s * (u * u * u - u * u * u * u / 2.0)
    }
}

/// Placed spins, each with a start time, duration and total signed turns.
fn parse_spins(text: &str, seconds: f64) -> Result<Vec<(f64, f64, f64)>, String> {
    let mut spins = Vec::new();
    let mut previous_end = 0.0;
    for (i, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let fields: Vec<_> = line.split_whitespace().collect();
        let bad = || {
            format!(
                "--spins line {}: expected start_seconds duration_seconds turns",
                i + 1
            )
        };
        if fields.len() != 3 {
            return Err(bad());
        }
        let nums = fields
            .iter()
            .map(|v| v.parse::<f64>().map_err(|_| bad()))
            .collect::<Result<Vec<_>, _>>()?;
        let (start, duration, turns) = (nums[0], nums[1], nums[2]);
        let end = start + duration;
        if !(start.is_finite()
            && duration.is_finite()
            && turns.is_finite()
            && start >= previous_end
            && duration > 0.0
            && end.is_finite()
            && end <= seconds)
        {
            return Err(format!(
                "--spins line {}: spins must be non-overlapping, start >= 0, duration > 0 and end <= film duration ({seconds} s)",
                i + 1
            ));
        }
        spins.push((start, duration, turns));
        previous_end = end;
    }
    if spins.is_empty() {
        return Err("--spins: no spins in file".into());
    }
    Ok(spins)
}

/// Integrated sin² angular speed; before and after a spin, the full turns persist.
fn placed_spin_turns(t: f64, spins: &[(f64, f64, f64)]) -> f64 {
    spins
        .iter()
        .map(|&(start, duration, turns)| {
            let u = ((t - start) / duration).clamp(0.0, 1.0);
            turns * (u - (std::f64::consts::TAU * u).sin() / std::f64::consts::TAU)
        })
        .sum()
}

/// Turns rotated by time `t` under `--spin (peak, on, off)`: still for `off / 2`, then
/// alternately spinning for `on` seconds at speed `peak * sin²(pi * r / on)` (easing in
/// from and back out to no spin; each spin turns `peak * on / 2`) and still for `off`.
pub(crate) fn spin_turns(t: f64, (peak, on, off): (f64, f64, f64)) -> f64 {
    let t = t - off / 2.0;
    if t <= 0.0 {
        return 0.0;
    }
    let k = (t / (on + off)).floor();
    let r = t - k * (on + off);
    let partial = if r < on {
        r / 2.0 - on / (4.0 * std::f64::consts::PI) * (std::f64::consts::TAU * r / on).sin()
    } else {
        on / 2.0
    };
    peak * (k * on / 2.0 + partial)
}

/// The frame centre's offset from the target at film fraction `u` when the frame is
/// `zoom` times the starting width (FILM-06): the target's position on screen, as a
/// fraction of the frame, eases from `offset / w0` to the middle. An offset fixed in the
/// plane instead would swing the target off-screen as the frame shrinks.
pub(crate) fn veer(offset: (f64, f64), u: f64, zoom: f64) -> (f64, f64) {
    let k = (1.0 - u * u * (3.0 - 2.0 * u)) * zoom;
    (offset.0 * k, offset.1 * k)
}

/// Decades zoomed by time `t` at unit full speed in a `total`-second film whose speed
/// holds 1, then eases to rest over the last `s` seconds (1 - smoothstep).
pub(crate) fn out_depth(t: f64, total: f64, s: f64) -> f64 {
    let start = total - s;
    if t <= start {
        t
    } else {
        let tau = (t - start).min(s);
        start + tau - eased_depth(tau, s)
    }
}

/// The camera path: `n = round(seconds * fps)` frames from width `w0` to `w1`, evenly
/// spaced in log width (or smoothstep-eased, or eased in over `ease_in` seconds and then
/// constant), rotating `twist` full turns over the film or by `spin` (see spin_turns).
#[allow(clippy::too_many_arguments)]
pub(crate) fn path(
    re: &str,
    im: &str,
    w0: f64,
    w1: f64,
    seconds: f64,
    fps: f64,
    twist: f64,
    ease: bool,
    ease_in: f64,
    spin: Option<(f64, f64, f64)>,
    ease_out: f64,
) -> Result<Vec<View>, String> {
    fd_fixed::Decimal::parse(re)?;
    fd_fixed::Decimal::parse(im)?;
    if !(w0.is_normal() && w1.is_normal() && w0 > 0.0 && w1 > 0.0 && twist.is_finite()) {
        return Err("widths must be positive normal numbers and --twist finite".into());
    }
    let n = (seconds * fps).round();
    if !(2.0..=1e7).contains(&n) {
        return Err(format!(
            "--seconds x --fps must give 2 to 1e7 frames, got {n}"
        ));
    }
    let n = n as usize;
    let (l0, l1) = (w0.log10(), w1.log10());
    Ok((0..n)
        .map(|f| {
            let s = f as f64 / (n - 1) as f64;
            let e = if ease_out > 0.0 {
                out_depth(s * seconds, seconds, ease_out) / out_depth(seconds, seconds, ease_out)
            } else if ease_in > 0.0 {
                eased_depth(s * seconds, ease_in) / eased_depth(seconds, ease_in)
            } else if ease {
                s * s * (3.0 - 2.0 * s)
            } else {
                s
            };
            View {
                center_re: re.to_string(),
                center_im: im.to_string(),
                width: sig6(10f64.powf(l0 + (l1 - l0) * e)),
                rotation: std::f64::consts::TAU
                    * spin.map_or(twist * e, |sp| spin_turns(s * seconds, sp)),
            }
        })
        .collect())
}

/// Render one frame with the cheapest valid kernel: the zone when it covers the view,
/// otherwise fd's reference plus its own BLA table (none on the scaled tier, or when
/// the table has no valid block). A zone frame opens its own wgpu zone renderer.
pub(crate) fn render_frame(
    view: &View,
    p: &Params,
    zone: Option<&Zone>,
) -> Result<(Header, Samples, Used), String> {
    let mut gpu = zone
        .map(|z| crate::zone_gpu::choose(None, z))
        .transpose()?
        .map(crate::zone_gpu::Renderer::new);
    render_frame_kernel(view, p, zone, &mut gpu)
}

/// [`render_frame`] with the frame's zone samples on `gpu`'s wgpu zone renderer
/// (CLEA-04); with no renderer, a declined frame or a device failure, the zone frame
/// renders as an uncovered one.
pub(crate) fn render_frame_kernel(
    view: &View,
    p: &Params,
    zone: Option<&Zone>,
    gpu: &mut Option<crate::zone_gpu::Renderer>,
) -> Result<(Header, Samples, Used), String> {
    render_frame_profiled(view, p, zone, gpu, &mut BuildTimings::default())
}

pub(crate) fn render_frame_profiled(
    view: &View,
    p: &Params,
    zone: Option<&Zone>,
    gpu: &mut Option<crate::zone_gpu::Renderer>,
    builds: &mut BuildTimings,
) -> Result<(Header, Samples, Used), String> {
    if let Some(z) = zone {
        if zone_covers(view, p, z)? {
            if let Some((h, s)) = crate::zone_gpu::render_zone_frame(gpu, view, p, z) {
                let used = Used::zone(&h);
                return Ok((h, s, used));
            }
        }
    }
    let t = Instant::now();
    let orbit = fd_kernel::reference(view, p)?;
    let key = BuildTimings::reference_key(view, p, orbit.1);
    builds.reference(&key, t.elapsed().as_secs_f64());
    let (r, bits) = (&orbit.0, orbit.1);
    let table = match bla_dc_max(view, p) {
        Ok(dc) => {
            let t = Instant::now();
            let table = Bla::build(r, EPS, dc)?;
            builds.bla(&key, dc, t.elapsed().as_secs_f64());
            Some(table).filter(|t| !t.levels.is_empty())
        }
        Err(_) => None,
    };
    if let Some((series, use_bla)) = prepare_cpu_series(view, p, &orbit, table.as_ref(), None)? {
        let (h, s, _, _) = series.render_masked(
            view,
            p,
            orbit,
            if use_bla { table.as_ref() } else { None },
            None,
        )?;
        return Ok((
            h,
            s,
            if use_bla {
                Used::SeriesBla
            } else {
                Used::SeriesPlain
            },
        ));
    }
    let (orbit, _) = orbit;
    if let Some(table) = table {
        let (h, s, _, _) = render_bla(view, p, (orbit, bits), &table)?;
        return Ok((h, s, Used::Bla));
    }
    let (h, s, _) = render_with(view, p, Some((orbit, bits)))?;
    let used = if h.kernel.contains("scaled") {
        Used::Scaled
    } else {
        Used::Plain
    };
    Ok((h, s, used))
}

/// A film frame with jet-AA (FILM-16): pass 1 computes each pixel's top-left ss 2
/// sample, [`crate::jetaa::rebuild`] predicts the rest, pass 2 computes the rejected
/// ones. BLA or plain perturbation as [`render_frame_kernel`] (CPU), with the reference
/// orbit and BLA table built once for both passes; zone frames are rendered exactly, on
/// `gpu`'s wgpu zone renderer. Also returns the samples computed and the seconds of
/// pass 1, the rebuild and pass 2.
pub(crate) fn render_frame_jet(
    view: &View,
    p: &Params,
    zone: Option<&Zone>,
    gpu: &mut Option<crate::zone_gpu::Renderer>,
    builds: &mut BuildTimings,
) -> Result<(Header, Samples, Used, u64, [f64; 3]), String> {
    if p.ss != 2 {
        return Err("jet-AA rebuilds ss 2 frames".into());
    }
    let (w, h) = ((p.nx / 2) as usize, (p.ny / 2) as usize);
    let first = crate::jetaa::first(w, h);
    let in_zone = match zone {
        Some(z) => zone_covers(view, p, z)?,
        None => false,
    };
    // Zone samples are cheap (a 960x540 v0 dive: the rebuild costs about 0.6 samples per
    // pixel and the rejected samples are the long ones, so jet-AA ran 0-16% slower than
    // exact ss 2). Zone frames stay exact; jet-AA pays where samples are expensive (BLA
    // and perturbation frames: 1.84x faster on a tendril film at 1e-57).
    if let (true, Some(z)) = (in_zone, zone) {
        if let Some((hd, s)) = crate::zone_gpu::render_zone_frame(gpu, view, p, z) {
            let used = Used::zone(&hd);
            return Ok((hd, s, used, 4 * (w * h) as u64, [0.0; 3]));
        }
    }
    let t = Instant::now();
    let (o, bits) = fd_kernel::reference(view, p)?;
    let key = BuildTimings::reference_key(view, p, bits);
    builds.reference(&key, t.elapsed().as_secs_f64());
    let table = match bla_dc_max(view, p) {
        Ok(dc) => {
            let t = Instant::now();
            let table = Bla::build(&o, EPS, dc)?;
            builds.bla(&key, dc, t.elapsed().as_secs_f64());
            Some(table).filter(|t| !t.levels.is_empty())
        }
        Err(_) => None,
    };
    let orbit = (o, bits);
    // DEC-26: prepare once, then reuse the same initializer for both sample masks.
    let series = prepare_cpu_series(view, p, &orbit, table.as_ref(), Some(&first))?;
    let pass = |mask: &[bool]| -> Result<(Header, Samples, Used), String> {
        if let Some((series, use_bla)) = &series {
            let (hd, s, _, _) = series.render_masked(
                view,
                p,
                orbit.clone(),
                if *use_bla { table.as_ref() } else { None },
                Some(mask),
            )?;
            return Ok((
                hd,
                s,
                if *use_bla {
                    Used::SeriesBla
                } else {
                    Used::SeriesPlain
                },
            ));
        }
        match &table {
            Some(t) => {
                let (hd, s, _, _) = render_bla_masked(view, p, orbit.clone(), t, Some(mask))?;
                Ok((hd, s, Used::Bla))
            }
            None => {
                let (hd, s, _) = render_with_masked(view, p, Some(orbit.clone()), mask)?;
                let used = if hd.kernel.contains("scaled") {
                    Used::Scaled
                } else {
                    Used::Plain
                };
                Ok((hd, s, used))
            }
        }
    };
    let t = Instant::now();
    let (hd, mut s, used) = pass(&first)?;
    let t1 = t.elapsed().as_secs_f64();
    let t = Instant::now();
    let todo = crate::jetaa::rebuild(&mut s, w, h);
    let rest = todo.iter().filter(|&&t| t).count();
    let t2 = t.elapsed().as_secs_f64();
    let t = Instant::now();
    if rest > 0 {
        let (_, s2, _) = pass(&todo)?;
        crate::jetaa::merge(&mut s, &s2, &todo);
    }
    let t3 = t.elapsed().as_secs_f64();
    Ok((hd, s, used, (w * h + rest) as u64, [t1, t2, t3]))
}

#[derive(Default)]
struct Report {
    /// Oversampling multiplier, or None for direct per-frame rendering.
    keyframes: Option<u32>,
    frames: [usize; 3],
    route_frames: [usize; 8],
    output_frames: [usize; 8],
    route_seconds: [f64; 8],
    builds: BuildTimings,
    seconds: [f64; 3],
    shade: f64,
    gpu_device: Option<String>,
    /// Bytes of wgpu zone kernel output read back to the host (GPU-21).
    zone_readback_bytes: u64,
    keyframe_prepare: f64,
    keyframe_sample_shade: f64,
    keyframe_resample: f64,
    encode_wait: f64,
    /// Jet-AA (FILM-16): samples computed and output pixels rendered.
    jet_samples: u64,
    jet_pixels: u64,
    /// Seconds in pass 1, the rebuild and pass 2.
    jet_stage: [f64; 3],
    /// (zone seconds, per-frame BLA seconds) for the compared frames.
    compared: Vec<(f64, f64)>,
}

impl Report {
    fn add(&mut self, used: Used, secs: f64) {
        self.route_frames[used as usize] += 1;
        self.route_seconds[used as usize] += secs;
        let k = used.legacy();
        self.frames[k] += 1;
        self.seconds[k] += secs;
    }

    fn json(&self, mp4: &str, n: usize, fps: f64, wall: f64) -> String {
        let (zs, bs) = self
            .compared
            .iter()
            .fold((0.0, 0.0), |(a, b), (x, y)| (a + x, b + y));
        let ratio = if zs > 0.0 { bs / zs } else { 0.0 };
        // Estimated render time had every zone frame used per-frame BLA instead.
        let est = if zs > 0.0 {
            self.seconds[1] + self.seconds[2] + self.seconds[0] * ratio
        } else {
            0.0
        };
        let render = self.seconds.iter().sum::<f64>();
        let routes = Used::ALL
            .iter()
            .map(|&route| {
                let k = route as usize;
                format!(
                    "{}:{{\"frames\":{},\"kernel_frames\":{},\"seconds\":{}}}",
                    crate::bench::q(route.name()),
                    self.output_frames[k],
                    self.route_frames[k],
                    self.route_seconds[k]
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let b = &self.builds;
        let stages = format!(
            "\"routes\":{{{routes}}},\"route_frame_basis\":\"outer keyframe for keyframe films; actual kernel for direct films\",\"timing_basis\":\"route seconds include setup; reference and BLA seconds are subsets of render; keyframe stages are subsets of shade; encode wait is host blocking time\",\"reference_seconds\":{},\"bla_build_seconds\":{},\"reference_builds\":{{\"total\":{},\"repeated\":{},\"repeated_seconds\":{}}},\"bla_builds\":{{\"total\":{},\"repeated\":{},\"repeated_seconds\":{}}},",
            b.reference_seconds,
            b.bla_seconds,
            b.reference_builds,
            b.repeated_references,
            b.repeated_reference_seconds,
            b.bla_builds,
            b.repeated_blas,
            b.repeated_bla_seconds
        );

        format!(
            "{{\"schema\":\"fd-film/1\",{stages}\"mp4\":{},\"frames\":{n},\"fps\":{fps},\"film_seconds\":{},\"wall_seconds\":{wall},\
             \"render\":{{\"zone\":{{\"frames\":{},\"seconds\":{}}},\"bla\":{{\"frames\":{},\"seconds\":{}}},\"perturbation\":{{\"frames\":{},\"seconds\":{}}},\"total_seconds\":{render}}},\
             \"shade_seconds\":{},\"shade_seconds_per_frame\":{},\"gpu_device\":{},\"zone_readback_bytes\":{},\"keyframes\":{},\"keyframe_stages\":{{\"prepare_seconds\":{},\"sample_shade_seconds\":{},\"resample_seconds\":{}}},\"encode_wait_seconds\":{},\"jet_aa_samples_per_pixel\":{},\"jet_aa_stage_seconds\":[{},{},{}],\
             \"zone_vs_bla\":{{\"compared_frames\":{},\"zone_seconds\":{zs},\"bla_seconds\":{bs},\"speedup\":{ratio},\"estimated_render_seconds_without_zone\":{est}}}}}",
            crate::bench::q(mp4),
            n as f64 / fps,
            self.frames[0],
            self.seconds[0],
            self.frames[1],
            self.seconds[1],
            self.frames[2],
            self.seconds[2],
            self.shade,
            self.shade / n as f64,
            self.gpu_device.as_deref().map(crate::bench::q).unwrap_or_else(|| "null".into()),
            self.zone_readback_bytes,
            self.keyframes.map_or_else(|| "null".into(), |m| m.to_string()),
            self.keyframe_prepare,
            self.keyframe_sample_shade,
            self.keyframe_resample,
            self.encode_wait,
            if self.jet_pixels > 0 {
                format!("{:.4}", self.jet_samples as f64 / self.jet_pixels as f64)
            } else {
                "null".into()
            },
            self.jet_stage[0],
            self.jet_stage[1],
            self.jet_stage[2],
            self.compared.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RE: &str = "-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502";
    const IM: &str = "0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922";

    #[test]
    fn path_runs_from_w0_to_w1_with_twist_and_ease() {
        let v = path(RE, IM, 4.0, 1e-40, 2.0, 30.0, 0.5, false, 0.0, None, 0.0).unwrap();
        assert_eq!(v.len(), 60);
        assert_eq!(
            (v[0].width.as_str(), v[59].width.as_str()),
            ("4e0", "1e-40")
        );
        assert!((v[59].rotation - std::f64::consts::PI).abs() < 1e-12);
        // Constant rate: equal log steps.
        let lw = |i: usize| v[i].width.parse::<f64>().unwrap().log10();
        assert!(((lw(1) - lw(0)) - (lw(31) - lw(30))).abs() < 1e-4);
        // Eased: slow start, so the first step is smaller than a middle one.
        let e = path(RE, IM, 4.0, 1e-40, 2.0, 30.0, 0.0, true, 0.0, None, 0.0).unwrap();
        let le = |i: usize| e[i].width.parse::<f64>().unwrap().log10();
        assert!((le(1) - le(0)).abs() < 0.2 * (le(31) - le(30)).abs());
        assert!(path(RE, IM, 4.0, 1e-40, 0.01, 30.0, 0.0, false, 0.0, None, 0.0).is_err());
    }

    #[test]
    fn ease_in_ramps_from_zero_then_holds_full_speed() {
        // 30 s at 60 fps, 15 s ramp: depth = R * 22.5 decades.
        let r = 1.25f64.log10();
        let w1 = 4.0 / 10f64.powf(r * eased_depth(30.0, 15.0));
        let v = path(RE, IM, 4.0, w1, 30.0, 60.0, 0.0, false, 15.0, None, 0.0).unwrap();
        assert_eq!(v.len(), 1800);
        let lw = |i: usize| v[i].width.parse::<f64>().unwrap().log10();
        let step = |i: usize| lw(i) - lw(i + 1);
        assert!(step(0) < 1e-6, "starts from rest");
        assert!((1..1799).all(|i| step(i) >= 0.0), "monotone");
        // Full speed after the ramp: 1.25x per second = r/60 decades per frame (path uses
        // n-1 intervals over the film, so allow that 1/1800 stretch).
        assert!((step(1200) / (r / 60.0) - 1.0).abs() < 2e-3);
        assert!((lw(1799) - w1.log10()).abs() < 1e-5);
        assert!((eased_depth(15.0, 15.0) - 7.5).abs() < 1e-12);
    }

    #[test]
    fn veer_moves_the_target_monotonically_to_the_middle_of_the_screen() {
        let (o, w0) = ((0.2, -0.1), 1.0);
        // The frame shrinks 28x while the screen fraction of the offset falls to 0.
        let at = |i: usize| {
            let u = i as f64 / 100.0;
            let w = w0 / 28f64.powf(u);
            let (x, y) = veer(o, u, w / w0);
            (x / w, y / w)
        };
        assert_eq!(at(0), (0.2, -0.1));
        assert_eq!(at(100), (0.0, 0.0));
        assert!((1..=100)
            .all(|i| at(i).0.abs() <= at(i - 1).0.abs() && at(i).1.abs() <= at(i - 1).1.abs()));
    }

    #[test]
    fn ease_out_holds_speed_then_comes_to_rest() {
        assert_eq!(out_depth(5.0, 30.0, 30.0), 5.0 - eased_depth(5.0, 30.0));
        assert!((out_depth(30.0, 30.0, 30.0) - 15.0).abs() < 1e-12);
        assert_eq!(out_depth(10.0, 60.0, 30.0), 10.0);
        let w1 = 1e-20 / 10f64.powf(0.1 * 15.0);
        let v = path(RE, IM, 1e-20, w1, 30.0, 60.0, 0.0, false, 0.0, None, 30.0).unwrap();
        let lw = |i: usize| v[i].width.parse::<f64>().unwrap().log10();
        let step = |i: usize| lw(i) - lw(i + 1);
        assert!(
            (step(0) / (0.1 / 60.0) - 1.0).abs() < 2e-3,
            "starts at full speed"
        );
        assert!(step(1798) < 1e-6, "ends at rest");
        assert!((1..1799).all(|i| step(i) >= 0.0));
    }

    #[test]
    fn placed_spins_ease_and_reject_invalid_timelines() {
        let s = parse_spins(
            "# start duration turns\n1 2 0.5\n4 2 -0.25 # reverse\n",
            8.0,
        )
        .unwrap();
        let turns = |t| placed_spin_turns(t, &s);
        for (t, want) in [
            (0.0, 0.0),
            (1.0, 0.0),
            (2.0, 0.25),
            (3.0, 0.5),
            (4.0, 0.5),
            (5.0, 0.375),
            (6.0, 0.25),
            (8.0, 0.25),
        ] {
            assert!((turns(t) - want).abs() < 1e-12, "{t}: {}", turns(t));
        }
        let d = |t: f64| (turns(t + 1e-5) - turns(t - 1e-5)) / 2e-5;
        assert!(d(1.0).abs() < 1e-9 && d(3.0).abs() < 1e-9);
        assert!((d(2.0) - 0.5).abs() < 1e-9);
        for invalid in [
            "",
            "NaN 2 1",
            "0 0 1",
            "0 3 inf",
            "1 2 1\n2 1 1",
            "7 2 1",
            "0 1",
            "-1 1 1",
        ] {
            assert!(parse_spins(invalid, 8.0).is_err(), "{invalid:?}");
        }
    }

    #[test]
    fn spins_ease_in_and_out_between_still_stretches() {
        let sp = (0.25, 5.0, 10.0);
        let still = |a: f64, b: f64| (spin_turns(a, sp) - spin_turns(b, sp)).abs() < 1e-12;
        assert!(still(0.0, 5.0) && still(10.0, 20.0) && still(25.0, 30.0));
        // Each spin turns peak * on / 2, and its speed is ~0 at both ends, peak mid-spin.
        assert!(
            (spin_turns(10.0, sp) - 0.625).abs() < 1e-12
                && (spin_turns(30.0, sp) - 1.25).abs() < 1e-12
        );
        let speed = |t: f64| (spin_turns(t + 1e-4, sp) - spin_turns(t, sp)) / 1e-4;
        assert!(speed(5.0) < 1e-6 && speed(9.9999) < 1e-6 && (speed(7.5) - 0.25).abs() < 1e-6);
        let v = path(
            RE,
            IM,
            4.0,
            1e-3,
            30.0,
            60.0,
            0.0,
            false,
            15.0,
            Some(sp),
            0.0,
        )
        .unwrap();
        assert!((v[1799].rotation - std::f64::consts::TAU * 1.25).abs() < 1e-9);
    }

    #[test]
    fn speed_curve_gives_exact_decade_times_and_smooth_boundaries() {
        // One film, not three separately eased segments: 10, 30, 10 seconds.
        let c = SpeedCurve::parse("0 10\n-1 30\n-2 10\n", 0.0, -3.0).unwrap();
        assert!((c.total_seconds - 50.0).abs() < 1e-12);
        for (t, x) in [(0.0, 0.0), (10.0, -1.0), (40.0, -2.0), (50.0, -3.0)] {
            assert!((c.log_at(t) - x).abs() < 1e-10, "{t} s at {}", c.log_at(t));
        }
        let frames = c.path(RE, IM, 60.0, 0.0, None).unwrap();
        assert_eq!(frames.len(), 3000);
        let logs: Vec<f64> = frames
            .iter()
            .map(|v| v.width.parse::<f64>().unwrap().log10())
            .collect();
        let counts = [
            logs.iter().filter(|x| **x > -1.0).count(),
            logs.iter().filter(|x| **x <= -1.0 && **x > -2.0).count(),
            logs.iter().filter(|x| **x <= -2.0).count(),
        ];
        for (actual, expected) in counts.into_iter().zip([600, 1800, 600]) {
            assert!((actual as i64 - expected).abs() <= 1, "{counts:?}");
        }
        let speed = |t: f64| (c.log_at(t - 0.001) - c.log_at(t + 0.001)) / 0.002;
        for t in [10.0, 40.0] {
            let before = (c.log_at(t - 0.002) - c.log_at(t - 0.001)) / 0.001;
            let after = (c.log_at(t + 0.001) - c.log_at(t + 0.002)) / 0.001;
            assert!(
                (before - after).abs() < speed(t) * 0.01,
                "{t}: {before} vs {after}"
            );
        }
        assert!(logs.windows(2).all(|w| w[0] >= w[1]));
    }

    #[test]
    fn speed_curve_rest_reaches_cruise_without_a_jump() {
        let pace = 1.0 / 1.25f64.log10();
        let ramp_decades = 15.0 / (2.0 * pace);
        let file = format!("0 rest\n-{ramp_decades:.12} {pace:.12}\n");
        let c = SpeedCurve::parse(&file, 0.0, -3.0).unwrap();
        assert!((c.spans[0].duration - 15.0).abs() < 1e-8);
        assert!((c.log_at(15.0) + ramp_decades).abs() < 1e-9);
        let rate = |t: f64| (c.log_at(t - 0.001) - c.log_at(t + 0.001)) / 0.002;
        assert!(rate(0.001) < 1e-7, "starts effectively at rest");
        assert!((rate(15.0) - 1.0 / pace).abs() < 1e-5);
        assert!((rate(16.0) - 1.0 / pace).abs() < 1e-6);
        assert_eq!(sig6_log(-1000.0), "1e-1000");
        assert_eq!(sig6_log(4f64.log10()), "4e0");
        let deep = SpeedCurve::parse("0 10\n", 0.0, -1000.0).unwrap();
        assert_eq!(sig6_log(deep.log_at(deep.total_seconds)), "1e-1000");
    }

    #[test]
    fn speed_curve_rejects_uncovered_or_invalid_paces() {
        for file in [
            "",
            "-1 10\n",
            "0 0\n",
            "0 -10\n",
            "0 NaN\n",
            "0 inf\n",
            "0 rest\n",
            "0 rest\n-1 rest\n",
            "0 10\n0 20\n",
            "0 10\n1 20\n",
            "0 10\n-4 20\n",
        ] {
            assert!(SpeedCurve::parse(file, 0.0, -3.0).is_err(), "{file:?}");
        }
    }

    #[test]
    fn variable_film_keyframes_are_not_rerendered_at_decades() {
        use std::collections::HashSet;
        let curve = SpeedCurve::parse("0 10\n-1 30\n-2 10\n", 0.0, -3.0).unwrap();
        let p = Params {
            nx: 8,
            ny: 8,
            ss: 1,
            max_iter: 40,
            escape_radius: 1e10,
            columns: columns("nu,de,normal").unwrap(),
            threads: 1,
            tier: None,
        };
        let look = fd_shade::by_name("studio").unwrap();
        let render = |begin: f64, end: f64| {
            let mut keys = crate::keyframe::Keyframes::new(
                "0",
                "0",
                1.0 / 10f64.powf(begin),
                (4, 4),
                2,
                &p,
                None,
                None,
            );
            for j in 0..=30 {
                let d = begin + (end - begin) * j as f64 / 30.0;
                keys.frame(
                    1.0 / 10f64.powf(d),
                    (0.0, 0.0),
                    0.0,
                    look.as_ref(),
                    &Appearance::STILL,
                )
                .unwrap();
            }
            keys.rendered.iter().map(|x| x.0).collect::<Vec<_>>()
        };
        let one = render(0.0, 3.0);
        assert_eq!(one.len(), one.iter().copied().collect::<HashSet<_>>().len());
        let segments: usize = (0..3).map(|d| render(d as f64, d as f64 + 1.0).len()).sum();
        assert!(
            segments > one.len(),
            "{segments} segmented vs {} continuous",
            one.len()
        );
        assert!((curve.log_at(10.0) + 1.0).abs() < 1e-10);
    }

    #[test]
    fn frames_use_the_zone_only_where_it_covers_them() {
        // Smoke test of the film pipeline on a tiny grid: render (zone or BLA) and shade
        // with the film appearance, as `fd film` does per frame.
        let zone = Zone::parse(include_str!("../../../bench/zones/v0-core.zone")).unwrap();
        let p = Params {
            nx: 64,
            ny: 36,
            ss: 2,
            max_iter: 20_000,
            escape_radius: 1e10,
            columns: columns("nu,de,normal").unwrap(),
            threads: 4,
            tier: None,
        };
        let deep = View {
            center_re: RE.into(),
            center_im: IM.into(),
            width: "1e-40".into(),
            rotation: 0.4,
        };
        let shallow = View {
            width: "1e-20".into(),
            ..deep.clone()
        };
        let look = fd_shade::by_name("studio").unwrap();
        let film = Appearance {
            time: 1.5,
            flow: 0.1,
            aa: true,
            unresolved_interior: true,
            ..Appearance::STILL
        };
        for (v, want) in [(&deep, Used::ZoneGpu), (&shallow, Used::Bla)] {
            let (h, s, used) = render_frame(v, &p, Some(&zone)).unwrap();
            assert_eq!(used, want, "{}", v.width);
            let img = look.shade_with(&h, &s, &film);
            assert_eq!((img.w, img.h, img.data.len()), (32, 18, 32 * 18 * 3));
        }
        // Without a zone the deep frame falls back to fd's own kernel.
        assert_ne!(render_frame(&deep, &p, None).unwrap().2, Used::ZoneGpu);
    }
}
