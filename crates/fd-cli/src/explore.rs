//! `fd explore`: a local browser explorer (EXPL-01, a dev-tool sidequest). A std-only
//! HTTP server on 127.0.0.1 serves one page (explore.html) and `/render`, which renders
//! a view with the ordinary kernel and shades it with the relief look. The page does the
//! navigation and the coarse-to-fine refinement; the server only renders what it is asked.
use crate::args::Args;
use fd_kernel::{render_with, Params};
use fd_samples::{Class, Column, ColumnSet, Evidence, Kind, View};
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
    latest: AtomicU64,
    render: Mutex<()>,
}

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let a = Args::parse(argv, &["port", "threads"])?;
    let port: u16 = a.num("port", 8737)?;
    let threads = a.num("threads", std::thread::available_parallelism().map_or(1, |n| n.get()))?;
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|e| format!("127.0.0.1:{port}: {e}"))?;
    println!("fd explore: http://127.0.0.1:{port}/ ({threads} render threads, ctrl-c to stop)");
    let server = Arc::new(Server { threads, latest: AtomicU64::new(0), render: Mutex::new(()) });
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
    // Drain the headers; nothing in them is used.
    let mut h = String::new();
    while reader.read_line(&mut h)? > 2 {
        h.clear();
    }
    let target = line.split_whitespace().nth(1).unwrap_or("/");
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    match path {
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

/// `/render?re=&im=&width=&w=&h=&ss=&iter=&gen=`: raw RGB8 rows, `w x h` output pixels.
/// `None` when a newer generation has arrived, so the stale view is not rendered.
fn render(query: &str, server: &Server) -> Result<Option<Image>, String> {
    let q = |k: &str| query.split('&').find_map(|kv| kv.strip_prefix(k).and_then(|v| v.strip_prefix('='))).ok_or_else(|| format!("missing {k}"));
    let n = |k: &str| q(k).and_then(|v| v.parse::<u64>().map_err(|_| format!("bad {k} {v:?}")));
    let generation = n("gen")?;
    server.latest.fetch_max(generation, Ordering::SeqCst);
    let (w, h, ss) = (n("w")? as u32, n("h")? as u32, n("ss")? as u32);
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
        columns: ColumnSet::of(&[Column::Class, Column::De, Column::Normal]),
        threads: server.threads,
        tier: None,
    };
    let _one_at_a_time = server.render.lock().unwrap_or_else(|e| e.into_inner());
    if generation < server.latest.load(Ordering::SeqCst) {
        return Ok(None);
    }
    let t = Instant::now();
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
    let relief = fd_shade::by_name("relief").ok_or("relief look missing")?;
    let img = relief.shade(&header, &samples);
    let info = format!(
        "kernel={} seconds={seconds:.4} iterations={} unresolved={:.6}",
        header.kernel,
        stats.iterations,
        unresolved as f64 / samples.class.len().max(1) as f64
    );
    Ok(Some((img.w, img.h, info, img.data)))
}
