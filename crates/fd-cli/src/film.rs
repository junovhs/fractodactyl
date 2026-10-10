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
[--seconds S | --rate DECADES_PER_S (0.15)] [--twist TURNS (0) | --spin PEAK,ON,OFF] [--ease on|off (off) | --ease-in S (0)] \
[--size WxH (1920x1080)] [--ss N (2)] [--iter N (100000)] [--threads N] [--zone FILE] \
[--look L (studio)] [--preset LOOK (ice)] [look knobs as fd shade] [--crf N (16)] [--x264 P (slow)] [--chroma 420|444 (420)] [--compare-every K (0)] \
[--frames A..B] [--keyframes M (off; M x oversized keyframe per 2x zoom, see keyframe.rs)] \
[--ease-out S] [--offset DRE,DIM (keyframes only)] [--t0 SECONDS (0)] [fd shade's appearance flags; film defaults --aa on --unresolved interior --dither on]";

/// Which kernel rendered a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Used {
    Zone,
    Bla,
    Plain,
}

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let mut known = vec![
        "to",
        "mp4",
        "from",
        "fps",
        "seconds",
        "rate",
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
        "keyframes",
        "ease-out",
        "offset",
        "t0",
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
    let (w0, mut w1): (f64, f64) = (
        a.num("from", 4.0)?,
        to.parse()
            .ok()
            .filter(|w: &f64| *w > 0.0)
            .ok_or_else(|| format!("--to: bad or out-of-range width {to:?}"))?,
    );
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
    let rate: f64 = a.num("rate", 0.15)?;
    let seconds = match a.str("seconds") {
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
    if spin.is_some() && a.str("twist").is_some() {
        return Err("--spin and --twist are exclusive".into());
    }
    let views = path(
        re,
        im,
        w0,
        w1,
        seconds,
        fps,
        a.num("twist", 0.0)?,
        ease,
        ease_in,
        spin,
        ease_out,
    )?;
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
    // `--keyframes M` (FILM-04): frames are built from per-octave sample keyframes.
    let mut keys = match a.str("keyframes") {
        None => None,
        Some(_) => {
            let m: u32 = a.num("keyframes", 2)?;
            if !(1..=4).contains(&m)
                || a.str("ss").is_some()
                || a.str("twist").is_some()
                || spin.is_some()
            {
                return Err("--keyframes M takes M in 1..=4 and no --ss, --twist or --spin".into());
            }
            // Keyframe 0 must hold frame 0 including its offset from the target.
            let base = crate::keyframe::coverage(w0, offset, (w, h));
            Some(crate::keyframe::Keyframes::new(
                re,
                im,
                base,
                (w, h),
                m,
                &p,
                zone.as_ref(),
            ))
        }
    };
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
    let mut r = Report::default();
    for (f, view) in views.iter().enumerate().take(to).skip(from) {
        if let Some(k) = keys.as_mut() {
            let t = Instant::now();
            let before = k.rendered.len();
            let width: f64 = view.width.parse().map_err(|_| "bad width".to_string())?;
            let off = veer(offset, f as f64 / (views.len() - 1) as f64, width / w0);
            let img = k.frame(
                width,
                off,
                look.as_ref(),
                &Appearance {
                    time: t0 + f as f64 / fps,
                    ..base
                },
            )?;
            let kf: f64 = k.rendered[before..].iter().map(|x| x.1).sum();
            for &(_, secs, used) in &k.rendered[before..] {
                r.add(used, secs);
            }
            r.shade += t.elapsed().as_secs_f64() - kf;
            let t = Instant::now();
            pipe.write_all(&img.data)
                .map_err(|e| format!("ffmpeg pipe: {e}"))?;
            r.encode_wait += t.elapsed().as_secs_f64();
            let done = f + 1 - from;
            if done == 1 || done % 60 == 0 || done == to - from {
                let el = wall.elapsed().as_secs_f64();
                let left = el / done as f64 * (to - from - done) as f64;
                eprintln!("  frame {done}/{} width {} (keyframes {}) {el:.0} s elapsed, ~{left:.0} s left", to - from, view.width, k.rendered.len());
            }
            continue;
        }
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
    println!(
        "{}",
        r.json(mp4, to - from, fps, wall.elapsed().as_secs_f64())
    );
    Ok(())
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
/// the table has no valid block).
pub(crate) fn render_frame(
    view: &View,
    p: &Params,
    zone: Option<&Zone>,
) -> Result<(Header, Samples, Used), String> {
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
