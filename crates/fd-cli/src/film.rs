//! `fd film` (FILM-01): one command from a location to an mp4. It generates the camera
//! path (constant zoom rate, optional ease and twist), renders every frame with the
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
use fd_kernel::{bla_dc_max, render_bla, render_with, render_zone, zone_covers, Bla, Params, Zone};
use fd_samples::{Header, Samples, View};
use fd_shade::Appearance;
use std::io::Write as _;
use std::process::{Command, Stdio};
use std::time::Instant;

const USAGE: &str = "usage: fd film (RE IM --to WIDTH | --place NAME [--to WIDTH (the place's)]) --mp4 FILE [--from W0 (4)] [--fps F (60)] \
[--seconds S | --rate DECADES_PER_S (0.15)] [--twist TURNS (0)] [--ease on|off (off)] \
[--size WxH (1920x1080)] [--ss N (2)] [--iter N (100000)] [--threads N] [--zone FILE] \
[--look L (studio)] [--preset LOOK (ice)] [look knobs as fd shade] [--crf N (16)] [--x264 P (slow)] [--chroma 420|444 (420)] [--compare-every K (0)] \
[--frames A..B] [fd shade's appearance flags; film defaults --aa on --unresolved interior]";

/// Which kernel rendered a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Used {
    Zone,
    Bla,
    Plain,
}

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let mut known = vec![
        "to", "mp4", "from", "fps", "seconds", "rate", "twist", "ease", "size", "ss", "iter", "threads", "zone", "look", "crf",
        "x264", "chroma", "compare-every", "frames", "place",
    ];
    known.extend(APPEARANCE_FLAGS);
    known.extend(LOOK_FLAGS);
    let a = Args::parse(argv, &known)?;
    // `--place NAME` (EXPL-08): a place saved by fd explore gives RE IM and the default --to.
    let place = a.str("place").map(crate::place::load).transpose()?;
    let (re, im, to) = match (&place, a.positional.as_slice()) {
        (Some(p), []) => (&p.re, &p.im, a.str("to").unwrap_or(&p.width)),
        (None, [re, im]) => (re, im, a.need("to")?),
        _ => return Err(USAGE.into()),
    };
    let mp4 = a.need("mp4")?;
    let fps: f64 = a.num("fps", 60.0)?;
    let (w0, w1): (f64, f64) = (a.num("from", 4.0)?, to.parse().ok().filter(|w: &f64| *w > 0.0).ok_or_else(|| format!("--to: bad or out-of-range width {to:?}"))?);
    let seconds = match a.str("seconds") {
        Some(_) => a.num("seconds", 0.0)?,
        None => (w0 / w1).log10().abs() / a.num("rate", 0.15)?,
    };
    let ease = match a.str("ease").unwrap_or("off") {
        "on" => true,
        "off" => false,
        v => return Err(format!("--ease: expected on or off, got {v:?}")),
    };
    let views = path(re, im, w0, w1, seconds, fps, a.num("twist", 0.0)?, ease)?;
    let (w, h) = size(a.str("size").unwrap_or("1920x1080"))?;
    let ss: u32 = a.num("ss", 2)?;
    let p = Params {
        nx: w * ss,
        ny: h * ss,
        ss,
        max_iter: a.num("iter", 100_000)?,
        escape_radius: 1e10,
        columns: columns("nu,de,normal")?,
        threads: a.num("threads", std::thread::available_parallelism().map_or(1, |n| n.get()))?,
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
    let zone = a.str("zone").map(Zone::load).transpose()?;
    let compare_every: usize = a.num("compare-every", 0)?;
    let (from, to) = match a.str("frames") {
        None => (0, views.len()),
        Some(r) => {
            let bad = || format!("--frames: expected A..B within 0..{}, got {r:?}", views.len());
            let (x, y) = r.split_once("..").ok_or_else(bad)?;
            let (x, y): (usize, usize) = (x.parse().map_err(|_| bad())?, y.parse().map_err(|_| bad())?);
            if x >= y || y > views.len() {
                return Err(bad());
            }
            (x, y)
        }
    };
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
        if look_name == "studio" { format!(" preset {}", a.str("preset").unwrap_or("ice")) } else { String::new() },
        zone.as_ref().map_or("none".to_string(), |z| format!("P={} within {:e}", z.period, z.max_dc))
    );
    let mut enc = Command::new("ffmpeg")
        .args(["-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", &format!("{w}x{h}"), "-r", &fps.to_string()])
        .args(["-i", "-", "-c:v", "libx264", "-crf", &crf.to_string(), "-preset", preset, "-pix_fmt", pix_fmt, mp4])
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("ffmpeg: {e} (fd film needs ffmpeg on PATH)"))?;
    let mut pipe = enc.stdin.take().expect("piped");

    let wall = Instant::now();
    let mut r = Report::default();
    for (f, view) in views.iter().enumerate().take(to).skip(from) {
        let t = Instant::now();
        let (hd, s, used) = render_frame(view, &p, zone.as_ref())?;
        let secs = t.elapsed().as_secs_f64();
        r.add(used, secs);
        if used == Used::Zone && compare_every > 0 && (f - from).is_multiple_of(compare_every) {
            let t = Instant::now();
            render_frame(view, &p, None)?;
            r.compared.push((secs, t.elapsed().as_secs_f64()));
        }
        let t = Instant::now();
        let img = look.shade_with(&hd, &s, &Appearance { time: f as f64 / fps, ..base });
        r.shade += t.elapsed().as_secs_f64();
        let t = Instant::now();
        pipe.write_all(&img.data).map_err(|e| format!("ffmpeg pipe: {e}"))?;
        r.encode_wait += t.elapsed().as_secs_f64();
        let done = f + 1 - from;
        if done == 1 || done % 60 == 0 || done == to - from {
            let el = wall.elapsed().as_secs_f64();
            eprintln!(
                "  frame {done}/{} width {} ({}) {:.0} s elapsed, ~{:.0} s left",
                to - from,
                view.width,
                match used {
                    Used::Zone => "zone",
                    Used::Bla => "bla",
                    Used::Plain => "perturbation",
                },
                el,
                el / done as f64 * (to - from - done) as f64
            );
        }
    }
    drop(pipe);
    let status = enc.wait().map_err(|e| format!("ffmpeg: {e}"))?;
    if !status.success() {
        return Err(format!("ffmpeg failed: {status}"));
    }
    println!("{}", r.json(mp4, to - from, fps, wall.elapsed().as_secs_f64()));
    Ok(())
}

/// The camera path: `n = round(seconds * fps)` frames from width `w0` to `w1`, evenly
/// spaced in log width (or smoothstep-eased), rotating `twist` full turns over the film.
#[allow(clippy::too_many_arguments)]
pub(crate) fn path(re: &str, im: &str, w0: f64, w1: f64, seconds: f64, fps: f64, twist: f64, ease: bool) -> Result<Vec<View>, String> {
    fd_fixed::Decimal::parse(re)?;
    fd_fixed::Decimal::parse(im)?;
    if !(w0.is_normal() && w1.is_normal() && w0 > 0.0 && w1 > 0.0 && twist.is_finite()) {
        return Err("widths must be positive normal numbers and --twist finite".into());
    }
    let n = (seconds * fps).round();
    if !(2.0..=1e7).contains(&n) {
        return Err(format!("--seconds x --fps must give 2 to 1e7 frames, got {n}"));
    }
    let n = n as usize;
    let (l0, l1) = (w0.log10(), w1.log10());
    Ok((0..n)
        .map(|f| {
            let s = f as f64 / (n - 1) as f64;
            let e = if ease { s * s * (3.0 - 2.0 * s) } else { s };
            View {
                center_re: re.to_string(),
                center_im: im.to_string(),
                width: sig6(10f64.powf(l0 + (l1 - l0) * e)),
                rotation: std::f64::consts::TAU * twist * e,
            }
        })
        .collect())
}

/// Render one frame with the cheapest valid kernel: the zone when it covers the view,
/// otherwise fd's reference plus its own BLA table (none on the scaled tier, or when
/// the table has no valid block).
pub(crate) fn render_frame(view: &View, p: &Params, zone: Option<&Zone>) -> Result<(Header, Samples, Used), String> {
    if let Some(z) = zone {
        if zone_covers(view, p, z)? {
            let (h, s, _, _) = render_zone(view, p, z)?;
            return Ok((h, s, Used::Zone));
        }
    }
    let (orbit, bits) = fd_kernel::reference(view, p)?;
    if let Ok(dc) = bla_dc_max(view, p) {
        let table = Bla::build(&orbit, EPS, dc)?;
        if !table.levels.is_empty() {
            let (h, s, _, _) = render_bla(view, p, (orbit, bits), &table)?;
            return Ok((h, s, Used::Bla));
        }
    }
    let (h, s, _) = render_with(view, p, Some((orbit, bits)))?;
    Ok((h, s, Used::Plain))
}

#[derive(Default)]
struct Report {
    frames: [usize; 3],
    seconds: [f64; 3],
    shade: f64,
    encode_wait: f64,
    /// (zone seconds, per-frame BLA seconds) for the compared frames.
    compared: Vec<(f64, f64)>,
}

impl Report {
    fn add(&mut self, used: Used, secs: f64) {
        let k = used as usize;
        self.frames[k] += 1;
        self.seconds[k] += secs;
    }

    fn json(&self, mp4: &str, n: usize, fps: f64, wall: f64) -> String {
        let (zs, bs) = self.compared.iter().fold((0.0, 0.0), |(a, b), (x, y)| (a + x, b + y));
        let ratio = if zs > 0.0 { bs / zs } else { 0.0 };
        // Estimated render time had every zone frame used per-frame BLA instead.
        let est = if zs > 0.0 { self.seconds[1] + self.seconds[2] + self.seconds[0] * ratio } else { 0.0 };
        let render = self.seconds.iter().sum::<f64>();
        format!(
            "{{\"schema\":\"fd-film/1\",\"mp4\":{},\"frames\":{n},\"fps\":{fps},\"film_seconds\":{},\"wall_seconds\":{wall},\
             \"render\":{{\"zone\":{{\"frames\":{},\"seconds\":{}}},\"bla\":{{\"frames\":{},\"seconds\":{}}},\"perturbation\":{{\"frames\":{},\"seconds\":{}}},\"total_seconds\":{render}}},\
             \"shade_seconds\":{},\"encode_wait_seconds\":{},\
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
            self.encode_wait,
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
        let v = path(RE, IM, 4.0, 1e-40, 2.0, 30.0, 0.5, false).unwrap();
        assert_eq!(v.len(), 60);
        assert_eq!((v[0].width.as_str(), v[59].width.as_str()), ("4e0", "1e-40"));
        assert!((v[59].rotation - std::f64::consts::PI).abs() < 1e-12);
        // Constant rate: equal log steps.
        let lw = |i: usize| v[i].width.parse::<f64>().unwrap().log10();
        assert!(((lw(1) - lw(0)) - (lw(31) - lw(30))).abs() < 1e-4);
        // Eased: slow start, so the first step is smaller than a middle one.
        let e = path(RE, IM, 4.0, 1e-40, 2.0, 30.0, 0.0, true).unwrap();
        let le = |i: usize| e[i].width.parse::<f64>().unwrap().log10();
        assert!((le(1) - le(0)).abs() < 0.2 * (le(31) - le(30)).abs());
        assert!(path(RE, IM, 4.0, 1e-40, 0.01, 30.0, 0.0, false).is_err());
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
        let deep = View { center_re: RE.into(), center_im: IM.into(), width: "1e-40".into(), rotation: 0.4 };
        let shallow = View { width: "1e-20".into(), ..deep.clone() };
        let look = fd_shade::by_name("studio").unwrap();
        let film = Appearance { time: 1.5, flow: 0.1, aa: true, unresolved_interior: true, ..Appearance::STILL };
        for (v, want) in [(&deep, Used::Zone), (&shallow, Used::Bla)] {
            let (h, s, used) = render_frame(v, &p, Some(&zone)).unwrap();
            assert_eq!(used, want, "{}", v.width);
            let img = look.shade_with(&h, &s, &film);
            assert_eq!((img.w, img.h, img.data.len()), (32, 18, 32 * 18 * 3));
        }
        // Without a zone the deep frame falls back to fd's own kernel.
        assert_ne!(render_frame(&deep, &p, None).unwrap().2, Used::Zone);
    }
}
