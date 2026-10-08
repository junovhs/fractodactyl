//! `fd explore`: a local browser explorer (EXPL-01, a dev-tool sidequest). A std-only
//! HTTP server on 127.0.0.1 serves one page (explore.html) and `/render`, which renders
//! a view with the ordinary kernel and shades it with the relief look, or with `raw=1`
//! returns per-pixel shading inputs so the page can colour (and animate) them itself
//! (EXPL-03). The page does the navigation and the coarse-to-fine refinement; the server
//! only renders what it is asked.
//!
//! Look mode (EXPL-05): `raw=2` returns every sample (not per-pixel averages) as four
//! little-endian f32 (nu minus the frame's `X-NuBase`, de, normal angle 0..65536, class),
//! rendered with the cheapest valid kernel (the zone fast path where `--zone` covers the
//! view, else per-frame BLA, as `fd film`), so the page can shade it with the studio
//! look on the GPU exactly as fd-shade does. `GET /looks` lists the presets (built-ins
//! and `looks/*.look`), `GET /look?name=` returns one, `POST /look?name=` saves one
//! (validated by `fd_shade::Look::parse`), so `fd film --preset NAME` renders it.
//! `raw=3` (EXPL-07) is the same per-sample data from the fast navigation kernel, for a
//! studio preset as the live look while navigating.
//! Places (EXPL-08): `GET /places` lists `places/*.place` as tab-separated
//! `name re im width iter` lines, `POST /place?name=` saves one, for `fd film --place NAME`.
//! `--capture FILE` (tests): the page in `?test` mode posts its look-mode pixels to
//! `POST /capture`, written to FILE, and the served samples go to FILE.fds.
use crate::args::Args;
use fd_kernel::{render_with, Params, Zone};
use fd_samples::{Class, Column, ColumnSet, Evidence, Header, Kind, Samples, View};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const PAGE: &str = include_str!("explore.html");

/// Shared between connection threads: one render runs at a time, and a request older
/// than the newest generation the page has sent is skipped instead of rendered.
struct Server {
    threads: usize,
    zone: Option<Zone>,
    capture: Option<String>,
    latest: AtomicU64,
    render: Mutex<()>,
}

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let a = Args::parse(argv, &["port", "threads", "zone", "capture"])?;
    let port: u16 = a.num("port", 8737)?;
    let threads = a.num("threads", std::thread::available_parallelism().map_or(1, |n| n.get()))?;
    // The v0 zone, when it has been built (tools/research/misiurewicz/make_zone.sh), speeds
    // up look-mode renders of deep v0 views; `--zone FILE` picks another.
    let zone = match a.str("zone") {
        Some(f) => Some(Zone::load(f)?),
        None => Some("data/zones/v0.zone").filter(|f| std::path::Path::new(f).is_file()).map(Zone::load).transpose()?,
    };
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|e| format!("127.0.0.1:{port}: {e}"))?;
    println!(
        "fd explore: http://127.0.0.1:{port}/ ({threads} render threads, zone {}, ctrl-c to stop)",
        zone.as_ref().map_or("none".to_string(), |z| format!("P={}", z.period))
    );
    let capture = a.str("capture").map(String::from);
    let server = Arc::new(Server { threads, zone, capture, latest: AtomicU64::new(0), render: Mutex::new(()) });
    for stream in listener.incoming().flatten() {
        let server = Arc::clone(&server);
        std::thread::spawn(move || {
            let _ = serve(stream, &server);
        });
    }
    Ok(())
}

fn serve(mut stream: TcpStream, server: &Server) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    // Headers: only Content-Length is used (POST bodies).
    let mut h = String::new();
    let mut length = 0usize;
    while reader.read_line(&mut h)? > 2 {
        if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
            length = v.trim().parse().unwrap_or(0);
        }
        h.clear();
    }
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let target = parts.next().unwrap_or("/");
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    if method == "POST" {
        if length > 256 << 20 {
            return respond(&mut stream, "413 Payload Too Large", "text/plain", &[], b"too large");
        }
        let mut body = vec![0u8; length];
        std::io::Read::read_exact(&mut reader, &mut body)?;
        let r = match path {
            "/look" => save_look(query, &body),
            "/place" => save_place(query, &body),
            "/capture" => capture(server, &body),
            _ => Err("not found".into()),
        };
        return match r {
            Ok(msg) => respond(&mut stream, "200 OK", "text/plain", &[], msg.as_bytes()),
            Err(e) => respond(&mut stream, "400 Bad Request", "text/plain", &[], e.as_bytes()),
        };
    }
    match path {
        "/looks" => respond(&mut stream, "200 OK", "text/plain; charset=utf-8", &[], list_looks().join("\n").as_bytes()),
        "/look" => match load_look(query) {
            Ok(text) => respond(&mut stream, "200 OK", "text/plain; charset=utf-8", &[], text.as_bytes()),
            Err(e) => respond(&mut stream, "404 Not Found", "text/plain", &[], e.as_bytes()),
        },
        "/places" => {
            let rows: Vec<String> = crate::place::list().into_iter().map(|(n, p)| format!("{n}\t{}\t{}\t{}\t{}", p.re, p.im, p.width, p.iter)).collect();
            respond(&mut stream, "200 OK", "text/plain; charset=utf-8", &[], rows.join("\n").as_bytes())
        }
        "/" => respond(&mut stream, "200 OK", "text/html; charset=utf-8", &[], PAGE.as_bytes()),
        "/render" => match render(query, server) {
            Ok(Some((w, h, info, rgb))) => {
                let headers = [("X-Width", w.to_string()), ("X-Height", h.to_string()), ("X-Info", info)];
                respond(&mut stream, "200 OK", "application/octet-stream", &headers, &rgb)
            }
            Ok(None) => respond(&mut stream, "204 No Content", "text/plain", &[], b""),
            Err(e) => respond(&mut stream, "400 Bad Request", "text/plain", &[], e.as_bytes()),
        },
        _ => respond(&mut stream, "404 Not Found", "text/plain", &[], b"not found"),
    }
}

fn respond(s: &mut TcpStream, status: &str, kind: &str, headers: &[(&str, String)], body: &[u8]) -> std::io::Result<()> {
    let mut head = format!("HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n", body.len());
    for (k, v) in headers {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    head.push_str("\r\n");
    s.write_all(head.as_bytes())?;
    s.write_all(body)
}

/// A shaded render: width, height, the `X-Info` line and RGB8 rows.
type Image = (usize, usize, String, Vec<u8>);

/// `/render?re=&im=&width=&w=&h=&ss=&iter=&gen=[&raw=1]`: RGB8 rows, `w x h` output pixels,
/// or with `raw=1` four little-endian f32 per pixel (see `raw_pixels`).
/// `None` when a newer generation has arrived, so the stale view is not rendered.
fn render(query: &str, server: &Server) -> Result<Option<Image>, String> {
    let q = |k: &str| query.split('&').find_map(|kv| kv.strip_prefix(k).and_then(|v| v.strip_prefix('='))).ok_or_else(|| format!("missing {k}"));
    let n = |k: &str| q(k).and_then(|v| v.parse::<u64>().map_err(|_| format!("bad {k} {v:?}")));
    let generation = n("gen")?;
    server.latest.fetch_max(generation, Ordering::SeqCst);
    let (w, h, ss) = (n("w")? as u32, n("h")? as u32, n("ss")? as u32);
    let raw = q("raw").is_ok_and(|v| v == "1");
    let samples_mode = q("raw").is_ok_and(|v| v == "2");
    // raw=3: studio inputs for every sample (as raw=2) from the fast navigation kernel
    // (as raw=1), so a studio preset can be the live look while exploring.
    let nav_samples = q("raw").is_ok_and(|v| v == "3");
    if w == 0 || h == 0 || !(1..=4).contains(&ss) || w as u64 * h as u64 > 16_000_000 {
        return Err("bad size".into());
    }
    let view = View { center_re: q("re")?.into(), center_im: q("im")?.into(), width: q("width")?.into(), rotation: 0.0 };
    let p = Params {
        nx: w * ss,
        ny: h * ss,
        ss,
        max_iter: n("iter")?.clamp(100, 2_000_000),
        escape_radius: 1e10,
        columns: if raw || samples_mode || nav_samples {
            ColumnSet::of(&[Column::Class, Column::Nu, Column::De, Column::Normal])
        } else {
            ColumnSet::of(&[Column::Class, Column::De, Column::Normal])
        },
        threads: server.threads,
        tier: None,
    };
    let _one_at_a_time = server.render.lock().unwrap_or_else(|e| e.into_inner());
    if generation < server.latest.load(Ordering::SeqCst) {
        return Ok(None);
    }
    let t = Instant::now();
    if samples_mode {
        let (header, samples, used) = crate::film::render_frame(&view, &p, server.zone.as_ref())?;
        let seconds = t.elapsed().as_secs_f64();
        if let Some(c) = &server.capture {
            let mut f = std::io::BufWriter::new(std::fs::File::create(format!("{c}.fds")).map_err(|e| format!("{c}.fds: {e}"))?);
            fd_samples::write(&mut f, &header, &samples).map_err(|e| e.to_string())?;
        }
        let (base, data) = sample_data(&samples);
        let (w, h) = header.pixels();
        let info = format!("kernel={} seconds={seconds:.4} used={used:?} ss={ss} nubase={base:?}", header.kernel);
        return Ok(Some((w, h, info, data)));
    }
    let (header, mut samples, stats) = render_with(&view, &p, None)?;
    let seconds = t.elapsed().as_secs_f64();
    // Display only: draw samples that ran out of iterations as interior, the usual
    // max-iteration convention. The page reports how many there were.
    let mut unresolved = 0u64;
    for c in samples.class.iter_mut() {
        if c.kind() == Some(Kind::Unresolved) {
            unresolved += 1;
            *c = Class::new(Kind::Interior, Evidence::Heuristic);
        }
    }
    if nav_samples {
        let (base, data) = sample_data(&samples);
        let (w, h) = header.pixels();
        let info = format!(
            "kernel={} seconds={seconds:.4} iterations={} unresolved={:.6} ss={ss} nubase={base:?}",
            header.kernel,
            stats.iterations,
            unresolved as f64 / samples.class.len().max(1) as f64
        );
        return Ok(Some((w, h, info, data)));
    }
    let (iw, ih, data) = if raw {
        let (w, h) = header.pixels();
        (w, h, raw_pixels(&header, &samples))
    } else {
        let img = fd_shade::by_name("relief").ok_or("relief look missing")?.shade(&header, &samples);
        (img.w, img.h, img.data)
    };
    let info = format!(
        "kernel={} seconds={seconds:.4} iterations={} unresolved={:.6}",
        header.kernel,
        stats.iterations,
        unresolved as f64 / samples.class.len().max(1) as f64
    );
    Ok(Some((iw, ih, info, data)))
}

/// Every sample for look mode: nu minus the smallest escaped nu (returned, as f64, so the
/// page keeps full phase precision), de, the normal angle (0..65536) and the class kind
/// (0 escaped, 1 interior, 2 unresolved), four little-endian f32 each, row-major.
fn sample_data(s: &Samples) -> (f64, Vec<u8>) {
    let (nu, de, nm) = (s.nu.as_deref().unwrap_or(&[]), s.de.as_deref().unwrap_or(&[]), s.normal.as_deref().unwrap_or(&[]));
    let escaped = |i: usize| s.class[i].kind() == Some(Kind::Escaped);
    let base = (0..s.class.len()).filter(|&i| escaped(i)).map(|i| nu[i]).fold(f64::INFINITY, f64::min);
    let base = if base.is_finite() { base } else { 0.0 };
    let mut out = Vec::with_capacity(s.class.len() * 16);
    for i in 0..s.class.len() {
        let kind = match s.class[i].kind() {
            Some(Kind::Escaped) => 0.0f32,
            Some(Kind::Interior) => 1.0,
            _ => 2.0,
        };
        let (n, d, a) = if escaped(i) { ((nu[i] - base) as f32, de[i], f32::from(nm[i])) } else { (0.0, 0.0, 0.0) };
        for v in [n, d, a, kind] {
            out.extend_from_slice(&v.to_le_bytes());
        }
    }
    (base, out)
}

/// Preset names: the built-ins, then `looks/*.look` (sorted).
fn list_looks() -> Vec<String> {
    let mut names: Vec<String> = fd_shade::BUILTIN.iter().map(|(n, _)| n.to_string()).collect();
    let mut own: Vec<String> = std::fs::read_dir(crate::shade::looks_dir())
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(".look")).map(String::from))
                .filter(|n| valid_name(n))
                .collect()
        })
        .unwrap_or_default();
    own.sort();
    for n in own {
        if !names.contains(&n) {
            names.push(n);
        }
    }
    names
}

/// Preset names are `[A-Za-z0-9_-]{1,64}`: a file name under `looks/`, never a path.
fn valid_name(n: &str) -> bool {
    !n.is_empty() && n.len() <= 64 && n.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn name_of(query: &str) -> Result<&str, String> {
    let n = query.split('&').find_map(|kv| kv.strip_prefix("name=")).ok_or("missing name")?;
    if valid_name(n) {
        Ok(n)
    } else {
        Err(format!("bad preset name {n:?}: use letters, digits, - and _"))
    }
}

/// `looks/NAME.look` if it exists, else the built-in.
fn load_look(query: &str) -> Result<String, String> {
    let n = name_of(query)?;
    match std::fs::read_to_string(crate::shade::looks_dir().join(format!("{n}.look"))) {
        Ok(t) => Ok(t),
        Err(_) => fd_shade::BUILTIN.iter().find(|(b, _)| *b == n).map(|(_, t)| t.to_string()).ok_or_else(|| format!("no preset {n:?}")),
    }
}

/// Validate and write `looks/NAME.look` (normalised through `Look::parse`/`to_text`).
fn save_look(query: &str, body: &[u8]) -> Result<String, String> {
    let n = name_of(query)?;
    let text = std::str::from_utf8(body).map_err(|_| "preset is not UTF-8")?;
    let look = fd_shade::Look::parse(text)?;
    let dir = crate::shade::looks_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(format!("{n}.look"));
    std::fs::write(&path, look.to_text()).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(format!("saved {}", path.canonicalize().unwrap_or(path).display()))
}

/// Validate and write `places/NAME.place` (EXPL-08).
fn save_place(query: &str, body: &[u8]) -> Result<String, String> {
    let n = name_of(query)?;
    let place = crate::place::Place::parse(std::str::from_utf8(body).map_err(|_| "place is not UTF-8")?)?;
    Ok(format!("saved {}", crate::place::save(n, &place)?))
}

/// Test hook: write the page's look-mode pixels (RGBA8, bottom-up rows) to `--capture`.
fn capture(server: &Server, body: &[u8]) -> Result<String, String> {
    let c = server.capture.as_ref().ok_or("start fd explore with --capture FILE")?;
    std::fs::write(c, body).map_err(|e| format!("{c}: {e}"))?;
    Ok(format!("captured {} bytes", body.len()))
}

/// Shading inputs per output pixel, averaged over its escaped samples: ln(nu), relief
/// light from the normal (upper-left light, as the relief look), edge factor tanh(de/2),
/// then the escaped share of its samples (0 = all interior). Little-endian f32 x4.
fn raw_pixels(h: &Header, s: &Samples) -> Vec<u8> {
    const LIGHT: [f32; 3] = [-0.5, -0.5, std::f32::consts::FRAC_1_SQRT_2];
    const HEIGHT: f32 = 1.5;
    let k = 1.0 / (1.0 + HEIGHT * HEIGHT).sqrt();
    let (nu, de, nm) = (s.nu.as_deref().unwrap_or(&[]), s.de.as_deref().unwrap_or(&[]), s.normal.as_deref().unwrap_or(&[]));
    let (w, ht) = h.pixels();
    let (ss, nx) = (h.ss as usize, h.nx as usize);
    let mut out = Vec::with_capacity(w * ht * 16);
    for py in 0..ht {
        for px in 0..w {
            let (mut acc, mut esc) = ([0.0f32; 3], 0u32);
            for sy in 0..ss {
                let base = (py * ss + sy) * nx + px * ss;
                for i in base..base + ss {
                    if s.class[i].kind() == Some(Kind::Escaped) {
                        let (x, y) = Samples::unit(nm[i]);
                        let lit = ((x * LIGHT[0] + y * LIGHT[1]) * k + HEIGHT * k * LIGHT[2]).clamp(0.0, 1.0);
                        acc = [acc[0] + nu[i].max(1.0).ln() as f32, acc[1] + lit, acc[2] + (de[i] * 0.5).tanh()];
                        esc += 1;
                    }
                }
            }
            let m = 1.0 / esc.max(1) as f32;
            for v in [acc[0] * m, acc[1] * m, acc[2] * m, esc as f32 / (ss * ss) as f32] {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_names_are_file_names_not_paths() {
        for ok in ["ice", "my-sunset", "v2_blue"] {
            assert!(valid_name(ok), "{ok}");
        }
        for bad in ["", "../evil", "a/b", "a.look", "x y", &"n".repeat(65)] {
            assert!(!valid_name(bad), "{bad:?}");
        }
        assert!(name_of("name=../x").is_err() && name_of("other=1").is_err());
        assert_eq!(name_of("gen=1&name=ice").unwrap(), "ice");
    }

    #[test]
    fn sample_data_packs_every_sample_relative_to_the_smallest_escaped_nu() {
        let cols = ColumnSet::of(&[Column::Class, Column::Nu, Column::De, Column::Normal]);
        let mut s = Samples::alloc(3, cols);
        s.class[0] = Class::new(Kind::Escaped, Evidence::Heuristic);
        s.class[1] = Class::new(Kind::Interior, Evidence::Heuristic);
        s.class[2] = Class::new(Kind::Escaped, Evidence::Heuristic);
        s.nu.as_mut().unwrap().copy_from_slice(&[1000.25, 0.0, 1003.5]);
        s.de.as_mut().unwrap().copy_from_slice(&[2.0, 0.0, 0.5]);
        s.normal.as_mut().unwrap().copy_from_slice(&[100, 0, 65535]);
        let (base, data) = sample_data(&s);
        assert_eq!(base, 1000.25);
        let f: Vec<f32> = data.chunks(4).map(|b| f32::from_le_bytes(b.try_into().unwrap())).collect();
        assert_eq!(f, [0.0, 2.0, 100.0, 0.0, 0.0, 0.0, 0.0, 1.0, 3.25, 0.5, 65535.0, 0.0]);
    }

    #[test]
    fn the_page_shades_with_the_same_constants_as_fd_shade() {
        // The GPU studio maths must stay in step with crates/fd-shade/src/studio.rs:
        // S0 0.02, the 0.6 log factor, CREVICE 0.6 with 1.5/sample decay, LINE_SPACE 3,
        // the 0.7-sample aa width, cosine easing and the terrace exponent 1.5.
        for needle in [
            "cycles / 0.02",
            "0.6 * log(1.0 +",
            "1.0 - 0.6 * exp(-de * float(ss) * 1.5)",
            "(1.0 / perPx - 3.0) / 3.0",
            "float(ss) / 0.7",
            "0.5 - 0.5 * cos(PI * fr)",
            "pow(1.0 - fr, 1.5)",
        ] {
            assert!(PAGE.contains(needle), "explore.html lost {needle:?}");
        }
    }
}
