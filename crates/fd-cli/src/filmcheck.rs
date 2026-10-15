//! DEC-29 film equivalence on shaded RGB24 frames, before x264 when available.
//! Exclude pixels with a reference RGB step >= 16 levels to a direct neighbour.

use crate::args::Args;
use crate::render::size;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

const USAGE: &str =
    "usage: fd filmcheck REF SHORTCUT [--size WxH (legacy RGB folders)] [--heatmap FILE (out/filmcheck-worst.ppm)]";
const NOISE_STEP: u8 = 16;

enum Input {
    Frames {
        files: Vec<(usize, PathBuf)>,
        next: usize,
        rgb: bool,
    },
    Video {
        child: Child,
        ended: bool,
    },
}

struct Source {
    input: Input,
    dimensions: (u32, u32),
    frame_bytes: usize,
}

fn checked_bytes((w, h): (u32, u32)) -> Result<usize, String> {
    (w as usize)
        .checked_mul(h as usize)
        .and_then(|n| n.checked_mul(3))
        .filter(|&n| n > 0)
        .ok_or_else(|| "frame dimensions overflow".into())
}

fn png_size(path: &Path) -> Result<(u32, u32), String> {
    let mut header = [0u8; 24];
    fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut header))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if &header[..8] != b"\x89PNG\r\n\x1a\n" || &header[12..16] != b"IHDR" {
        return Err(format!("{}: not a PNG image", path.display()));
    }
    let w = u32::from_be_bytes(header[16..20].try_into().unwrap());
    let h = u32::from_be_bytes(header[20..24].try_into().unwrap());
    checked_bytes((w, h))?;
    Ok((w, h))
}

fn video_size(path: &Path) -> Result<(u32, u32), String> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=p=0:s=x",
        ])
        .arg(path)
        .output()
        .map_err(|e| format!("ffprobe: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "ffprobe {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    size(String::from_utf8_lossy(&output.stdout).trim())
}

impl Source {
    fn open(path: &Path, requested: Option<(u32, u32)>) -> Result<Self, String> {
        if path.is_dir() {
            let mut files = Vec::new();
            for entry in fs::read_dir(path).map_err(|e| format!("{}: {e}", path.display()))? {
                let entry = entry.map_err(|e| e.to_string())?;
                let name = entry.file_name().to_string_lossy().into_owned();
                if let Some((number, ext)) =
                    name.strip_prefix("frame-").and_then(|s| s.rsplit_once('.'))
                {
                    if ["rgb", "png"].contains(&ext)
                        && !number.is_empty()
                        && number.bytes().all(|c| c.is_ascii_digit())
                    {
                        let n = number
                            .parse::<usize>()
                            .map_err(|_| format!("{name}: bad frame index"))?;
                        files.push((n, entry.path()));
                    }
                }
            }
            files.sort_by_key(|(n, _)| *n);
            if files.is_empty() {
                return Err(format!(
                    "{}: no frame-NNNNNN.rgb or .png files",
                    path.display()
                ));
            }
            if files
                .windows(2)
                .any(|f| f[0].0.checked_add(1) != Some(f[1].0))
            {
                return Err(format!(
                    "{}: duplicate or missing frame numbers",
                    path.display()
                ));
            }
            let rgb = files[0].1.extension().is_some_and(|s| s == "rgb");
            if files
                .iter()
                .any(|(_, p)| p.extension().is_some_and(|s| (s == "rgb") != rgb))
            {
                return Err(format!("{}: mixed PNG and RGB frames", path.display()));
            }
            let meta = path.join("size.txt");
            let from_meta = if meta.exists() {
                Some(size(
                    fs::read_to_string(&meta)
                        .map_err(|e| format!("{}: {e}", meta.display()))?
                        .trim(),
                )?)
            } else {
                None
            };
            let dimensions = match (from_meta, requested, rgb) {
                (Some(dimensions), _, _) => dimensions,
                (None, Some(dimensions), _) => dimensions,
                (None, None, false) => png_size(&files[0].1)?,
                (None, None, true) => {
                    return Err(format!(
                        "{}: RGB frames need size.txt or --size WxH",
                        path.display()
                    ));
                }
            };
            if requested.is_some_and(|expected| expected != dimensions) {
                return Err(format!(
                    "{}: --size does not match frame dimensions",
                    path.display()
                ));
            }
            if !rgb && png_size(&files[0].1)? != dimensions {
                return Err(format!(
                    "{}: PNG dimensions differ from size.txt",
                    path.display()
                ));
            }
            Ok(Self {
                frame_bytes: checked_bytes(dimensions)?,
                dimensions,
                input: Input::Frames {
                    files,
                    next: 0,
                    rgb,
                },
            })
        } else {
            let dimensions = video_size(path)?;
            if requested.is_some_and(|expected| expected != dimensions) {
                return Err(format!(
                    "{}: --size does not match video dimensions",
                    path.display()
                ));
            }
            let child = Command::new("ffmpeg")
                .args(["-nostdin", "-v", "error", "-i"])
                .arg(path)
                .args(["-vsync", "0", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"])
                .stdout(Stdio::piped())
                .spawn()
                .map_err(|e| format!("ffmpeg: {e}"))?;
            Ok(Self {
                frame_bytes: checked_bytes(dimensions)?,
                dimensions,
                input: Input::Video {
                    child,
                    ended: false,
                },
            })
        }
    }

    fn indices(&self) -> Option<Vec<usize>> {
        match &self.input {
            Input::Frames { files, .. } => Some(files.iter().map(|(n, _)| *n).collect()),
            Input::Video { .. } => None,
        }
    }

    fn next(&mut self) -> Result<Option<Vec<u8>>, String> {
        match &mut self.input {
            Input::Frames { files, next, rgb } => {
                let Some((_, path)) = files.get(*next) else {
                    return Ok(None);
                };
                *next += 1;
                if !*rgb && png_size(path)? != self.dimensions {
                    return Err(format!("{}: PNG frame dimensions differ", path.display()));
                }
                let data = if *rgb {
                    fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?
                } else {
                    let output = Command::new("ffmpeg")
                        .args(["-nostdin", "-v", "error", "-i"])
                        .arg(path)
                        .args(["-frames:v", "1", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"])
                        .output()
                        .map_err(|e| format!("ffmpeg: {e}"))?;
                    if !output.status.success() {
                        return Err(format!(
                            "ffmpeg {}: {}",
                            path.display(),
                            String::from_utf8_lossy(&output.stderr)
                        ));
                    }
                    output.stdout
                };
                if data.len() != self.frame_bytes {
                    return Err(format!(
                        "{}: expected {} RGB bytes, got {}",
                        path.display(),
                        self.frame_bytes,
                        data.len()
                    ));
                }
                Ok(Some(data))
            }
            Input::Video { child, ended } => {
                if *ended {
                    return Ok(None);
                }
                let mut data = vec![0u8; self.frame_bytes];
                let first = child
                    .stdout
                    .as_mut()
                    .ok_or("ffmpeg: missing stdout")?
                    .read(&mut data[..1])
                    .map_err(|e| format!("ffmpeg read: {e}"))?;
                if first == 0 {
                    *ended = true;
                    let status = child.wait().map_err(|e| format!("ffmpeg wait: {e}"))?;
                    if !status.success() {
                        return Err(format!("ffmpeg decode failed: {status}"));
                    }
                    return Ok(None);
                }
                child
                    .stdout
                    .as_mut()
                    .ok_or("ffmpeg: missing stdout")?
                    .read_exact(&mut data[1..])
                    .map_err(|e| format!("ffmpeg: truncated RGB frame: {e}"))?;
                Ok(Some(data))
            }
        }
    }
}

/// Max RGB step to the four neighbours, from the reference alone.
fn noise_mask(rgb: &[u8], w: usize, h: usize) -> Vec<bool> {
    let mut mask = vec![false; w * h];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let mut step = 0u8;
            for (nx, ny) in [
                (x.saturating_sub(1), y),
                ((x + 1).min(w - 1), y),
                (x, y.saturating_sub(1)),
                (x, (y + 1).min(h - 1)),
            ] {
                let j = ny * w + nx;
                for c in 0..3 {
                    step = step.max(rgb[3 * i + c].abs_diff(rgb[3 * j + c]));
                }
            }
            mask[i] = step >= NOISE_STEP;
        }
    }
    mask
}

#[derive(Default)]
struct Stats {
    all_good: usize,
    eligible: usize,
    within: usize,
    noise: usize,
    flicker_good: usize,
    flicker_eligible: usize,
    flicker_diff_sum: u64,
    flicker_extra_sum: u64,
}

impl Stats {
    fn passes(&self) -> bool {
        self.eligible > 0
            && self.all_good * 1000 >= self.eligible * 999
            && (self.flicker_eligible == 0
                || self.flicker_good * 1000 >= self.flicker_eligible * 999)
    }

    fn severity(&self) -> f64 {
        let spatial = 1.0 - self.all_good as f64 / self.eligible.max(1) as f64;
        let temporal = if self.flicker_eligible == 0 {
            0.0
        } else {
            1.0 - self.flicker_good as f64 / self.flicker_eligible as f64
        };
        spatial.max(temporal)
    }
}

/// Scores every output pixel and every adjacent frame pair (DEC-17).
fn score(
    reference: &[u8],
    shortcut: &[u8],
    w: usize,
    h: usize,
    previous: Option<(&[u8], &[u8], &[bool])>,
) -> (Stats, Vec<u8>, Vec<bool>) {
    let noise = noise_mask(reference, w, h);
    let mut errors = vec![0; w * h];
    let mut stats = Stats::default();
    for k in 0..w * h {
        let mut diff = 0u8;
        for c in 0..3 {
            let i = 3 * k + c;
            diff = diff.max(reference[i].abs_diff(shortcut[i]));
        }
        stats.within += usize::from(diff <= 1);
        if noise[k] {
            stats.noise += 1;
        } else {
            stats.eligible += 1;
            stats.all_good += usize::from(diff <= 1);
        }
        errors[k] = diff;
        if let Some((old_ref, old_short, old_noise)) = previous {
            if !noise[k] && !old_noise[k] {
                let (mut ref_step, mut short_step, mut change_diff) = (0u8, 0u8, 0u16);
                for c in 0..3 {
                    let i = 3 * k + c;
                    let dr = reference[i] as i16 - old_ref[i] as i16;
                    let ds = shortcut[i] as i16 - old_short[i] as i16;
                    ref_step = ref_step.max(dr.unsigned_abs() as u8);
                    short_step = short_step.max(ds.unsigned_abs() as u8);
                    change_diff = change_diff.max((dr - ds).unsigned_abs());
                }
                let extra = short_step.saturating_sub(ref_step);
                stats.flicker_eligible += 1;
                stats.flicker_good += usize::from(extra <= 1);
                stats.flicker_diff_sum += u64::from(change_diff);
                stats.flicker_extra_sum += u64::from(extra);
                errors[k] = errors[k].max(extra);
            }
        }
    }
    (stats, errors, noise)
}

fn percent(a: usize, n: usize) -> f64 {
    if n == 0 {
        0.0
    } else {
        a as f64 * 100.0 / n as f64
    }
}

fn heatmap(
    path: &Path,
    dimensions: (u32, u32),
    errors: &[u8],
    noise: &[bool],
) -> Result<(), String> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    let (w, h) = dimensions;
    let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
    for (&error, &masked) in errors.iter().zip(noise) {
        ppm.extend_from_slice(&[error.saturating_mul(8), 0, if masked { 48 } else { 0 }]);
    }
    fs::write(path, ppm).map_err(|e| format!("{}: {e}", path.display()))
}

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let args = Args::parse(argv, &["size", "heatmap"])?;
    let [reference, shortcut] = args.positional.as_slice() else {
        return Err(USAGE.into());
    };
    let requested = args.str("size").map(size).transpose()?;
    let (mut reference, mut shortcut) = (
        Source::open(Path::new(reference), requested)?,
        Source::open(Path::new(shortcut), requested)?,
    );
    if reference.dimensions != shortcut.dimensions {
        return Err("reference and shortcut dimensions differ".into());
    }
    if let (Some(a), Some(b)) = (reference.indices(), shortcut.indices()) {
        if a != b {
            return Err("reference and shortcut frame numbers differ".into());
        }
    }
    let out = Path::new(args.str("heatmap").unwrap_or("out/filmcheck-worst.ppm"));
    let (w, h) = reference.dimensions;
    let mut previous: Option<(Vec<u8>, Vec<u8>, Vec<bool>)> = None;
    let (mut count, mut passing) = (0usize, true);
    let (mut worst, mut worst_error) = (0usize, -1.0f64);
    let mut worst_map = Vec::new();
    let mut worst_noise = Vec::new();
    loop {
        let a = reference.next()?;
        let b = shortcut.next()?;
        let (a, b) = match (a, b) {
            (None, None) => break,
            (Some(a), Some(b)) => (a, b),
            _ => return Err("reference and shortcut frame counts differ".into()),
        };
        let prior = previous
            .as_ref()
            .map(|(a, b, n)| (a.as_slice(), b.as_slice(), n.as_slice()));
        let (stats, errors, noise) = score(&a, &b, w as usize, h as usize, prior);
        let ok = stats.passes() && (previous.is_none() || stats.flicker_eligible > 0);
        passing &= ok;
        let severity = if stats.eligible == 0 {
            1.0
        } else {
            stats.severity()
        };
        if severity > worst_error {
            worst_error = severity;
            worst = count;
            worst_map = errors;
            worst_noise = noise.clone();
        }
        let flicker = if stats.flicker_eligible == 0 {
            "n/a".to_string()
        } else {
            format!(
                "{:.4}%",
                percent(stats.flicker_good, stats.flicker_eligible)
            )
        };
        println!(
            "frame {count:06}: within_1={:.4}% ({}/{}) outside_noise_within_1={:.4}% ({}/{}) noise={} flicker_diff={:.3} flicker_extra={:.3} flicker_within_1={flicker} {}",
            percent(stats.within, (w as usize) * (h as usize)),
            stats.within, (w as usize) * (h as usize),
            percent(stats.all_good, stats.eligible),
            stats.all_good, stats.eligible, stats.noise,
            stats.flicker_diff_sum as f64 / stats.flicker_eligible.max(1) as f64,
            stats.flicker_extra_sum as f64 / stats.flicker_eligible.max(1) as f64,
            if ok { "PASS" } else { "FAIL" },
        );
        previous = Some((a, b, noise));
        count += 1;
    }
    if count < 2 {
        return Err("filmcheck needs at least two frames to measure flicker".into());
    }
    heatmap(out, (w, h), &worst_map, &worst_noise)?;
    println!(
        "{} fd-filmcheck/1 frames={count} worst_frame={worst:06} heatmap={} (>=99.9% within 1 outside reference local-step noise; no excess flicker >1 on >=99.9% of eligible temporal pixels)",
        if passing { "PASS" } else { "FAIL" }, out.display()
    );
    if !passing {
        std::process::exit(1);
    }
    Ok(())
}
