//! `fd play`: the Atlas v0 renderer/player (VIDE-02, docs/spec/PLAY.md). Renders the
//! frames of a compiled path from the atlas alone: per frame it reads the frame manifest,
//! its tile manifests, the orbit (orbit manifest and slabs) and the BLA table they name,
//! and renders with the stored orbit and the stored table. No reference orbit is
//! computed and no operator is built at play time (DEC-02). The camera and render
//! parameters come from the frame manifest; the frame order and the frame rate come from
//! the compile log (`fd compile` stdout), the one compile record the store does not hold.
//! Decoded chunks stay loaded while consecutive frames use them (the warm reuse being
//! measured); a frame that touches a math chunk for the first time is `cold`.
//! Output: one fd-play/1 JSON line per frame (the fd-control/1 field names, so BENC-01
//! can diff them key by key), then a totals line.
use crate::args::Args;
use crate::bench::{peak_rss, q, reset_peak_rss, run_oracle, sample_bytes, AtlasWork, Frame};
use crate::plan::anchor;
use fd_atlas::{BlaTable, ChunkId, FrameManifest, Kind, OrbitManifest, OrbitSlab, Store, TileManifest};
use fd_fixed::exp2i;
use fd_kernel::{reference_bits, render_bla, render_with, Bla, BlaStats, Block, Params, Reference, Stats};
use fd_samples::{write, ColumnSet, Header, Samples, View};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::process::Command;
use std::time::Instant;

const USAGE: &str = "usage: fd play COMPILE_LOG --store DIR [--frames A..B] [--threads N] [-o DIR] [--look L[,L...]] [appearance flags of fd shade]
               [--mp4 FILE] [--oracle tools/oracle.py [--every N] [--k K] [--python P]]";

/// Escape radius of every `fd render`/`fd compile` (`render::params`); not stored in the
/// frame manifest, so the player states it rather than reading it.
const ESCAPE_RADIUS: f64 = 1e10;

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let wall = Instant::now();
    let known: Vec<&str> =
        ["store", "frames", "threads", "o", "look", "mp4", "oracle", "every", "k", "python"].into_iter().chain(crate::shade::APPEARANCE_FLAGS).chain(crate::shade::LOOK_FLAGS).collect();
    let a = Args::parse(argv, &known)?;
    let mut base = crate::shade::appearance(&a)?;
    let [log] = a.positional.as_slice() else { return Err(USAGE.into()) };
    let store = crate::chunk::store(&a)?;
    let record = CompileLog::read(log)?;
    let n = record.frames.len();
    let (from, to) = match a.str("frames") {
        None => (0, n),
        Some(r) => range(r, n)?,
    };
    let threads = a.num("threads", std::thread::available_parallelism().map_or(1, |n| n.get()))?;
    let dir = a.str("o").map(|d| d.trim_end_matches('/').to_string());
    let looks: Vec<(String, Box<dyn fd_shade::Pass>)> = match a.str("look") {
        None => Vec::new(),
        Some(names) => crate::shade::passes(names, &a, &mut base)?,
    };
    let oracle = a.str("oracle");
    let every: usize = a.num("every", 1)?;
    if every == 0 {
        return Err("--every must be at least 1".into());
    }
    if (oracle.is_some() || !looks.is_empty() || a.str("mp4").is_some()) && dir.is_none() {
        return Err("--oracle, --look and --mp4 need -o DIR".into());
    }
    if a.str("mp4").is_some() && looks.is_empty() {
        return Err("--mp4 needs --look (it muxes the first look's PNG frames)".into());
    }
    if let Some(d) = &dir {
        std::fs::create_dir_all(d).map_err(|e| format!("{d}: {e}"))?;
    }

    // From here on a failure is a runtime one (exit 1), reported with the totals so far.
    let mut cache = Cache::default();
    let mut t = Totals { ok: true, log10_width: (f64::INFINITY, f64::NEG_INFINITY), ..Totals::default() };
    let mut error = None;
    for f in from..to {
        let check = oracle.filter(|_| (f - from).is_multiple_of(every));
        // Film time of frame f: animation is tied to the frame, not to render speed.
        let look_at = fd_shade::Appearance { time: f as f64 / record.fps, ..base };
        if let Err(e) = frame(&a, &store, &mut cache, &record.frames[f], f, threads, dir.as_deref(), &looks, &look_at, check, &mut t) {
            error = Some(format!("frame {f}: {e}"));
            break;
        }
    }
    let mp4 = match (a.str("mp4"), &dir, looks.first()) {
        (Some(out), Some(d), Some((look, _))) if error.is_none() => mux(out, d, look, from, record.fps)?,
        _ => Mux::NotAsked,
    };
    println!("{}", totals(log, &record, (from, to), threads, &t, &mp4, error.as_deref(), wall.elapsed().as_secs_f64()));
    if let Some(e) = &error {
        eprintln!("fd: {e}");
    }
    if error.is_some() || !t.ok {
        std::process::exit(1);
    }
    Ok(())
}

/// What `fd play` takes from the compile log: the frame manifests in path order and the
/// frame rate (`fps` line; 30, `fd compile`'s default, when the log has none).
struct CompileLog {
    frames: Vec<ChunkId>,
    fps: f64,
    fps_logged: bool,
}

impl CompileLog {
    /// Parse `frame F ... manifest ID` and `fps X` lines; frames must run 0, 1, 2, ...
    fn read(file: &str) -> Result<CompileLog, String> {
        let text = std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
        let (mut frames, mut fps) = (Vec::new(), None);
        for (k, line) in text.lines().enumerate() {
            let w: Vec<&str> = line.split(' ').collect();
            match w.as_slice() {
                ["fps", x] => fps = Some(x.parse::<f64>().map_err(|_| format!("{file}:{}: bad fps", k + 1))?),
                ["frame", f, ..] => {
                    let bad = || format!("{file}:{}: expected `frame F ... manifest ID`", k + 1);
                    let at = w.iter().position(|x| *x == "manifest").ok_or_else(bad)?;
                    let id: ChunkId = w.get(at + 1).ok_or_else(bad)?.parse()?;
                    if f.parse::<usize>().ok() != Some(frames.len()) {
                        return Err(format!("{file}:{}: frame {f} out of order (expected {})", k + 1, frames.len()));
                    }
                    frames.push(id);
                }
                _ => {}
            }
        }
        if frames.is_empty() {
            return Err(format!("{file}: no `frame F ... manifest ID` lines (expected `fd compile` output)"));
        }
        Ok(CompileLog { frames, fps: fps.unwrap_or(30.0), fps_logged: fps.is_some() })
    }
}

/// `A..B` (half-open) or `A..` within `0..n`.
fn range(r: &str, n: usize) -> Result<(usize, usize), String> {
    let bad = || format!("--frames: expected A..B within 0..{n}, got {r:?}");
    let (a, b) = r.split_once("..").ok_or_else(bad)?;
    let a: usize = a.parse().map_err(|_| bad())?;
    let b: usize = if b.is_empty() { n } else { b.parse().map_err(|_| bad())? };
    if a < b && b <= n {
        Ok((a, b))
    } else {
        Err(bad())
    }
}

/// A loaded orbit: decoded points, precision, exact centre, and its chunks' bytes.
struct Orbit {
    r: Reference,
    bits: u32,
    centre: (String, String),
    slabs: Vec<ChunkId>,
    bytes: u64,
}

/// A loaded BLA table (`None` when it holds no valid block) and its chunk bytes.
struct Table {
    orbit: ChunkId,
    bla: Option<Bla>,
    bytes: u64,
}

/// Decoded chunks kept across frames. After each frame only what that frame used stays,
/// so the working set is one frame's chunks (consecutive frames share their orbit and
/// mostly their table and tiles).
#[derive(Default)]
struct Cache {
    tiles: HashMap<ChunkId, (Vec<(Kind, ChunkId)>, u64)>,
    orbits: HashMap<ChunkId, Orbit>,
    tables: HashMap<ChunkId, Table>,
}

/// Per-frame reads: bytes from the store, and whether a math chunk was first touched.
#[derive(Default)]
struct Reads {
    bytes: u64,
    math_miss: bool,
}

impl Reads {
    fn get(&mut self, s: &Store, id: &ChunkId) -> Result<fd_atlas::Chunk, String> {
        let c = s.get(id).map_err(|e| e.to_string())?;
        self.bytes += c.bytes().len() as u64;
        Ok(c)
    }
}

fn load_orbit(s: &Store, id: &ChunkId, reads: &mut Reads) -> Result<Orbit, String> {
    let c = reads.get(s, id)?;
    let mut bytes = c.bytes().len() as u64;
    let m = OrbitManifest::from_chunk(&c).map_err(|e| format!("{id}: {e}"))?;
    let mut slabs = Vec::with_capacity(m.slabs.len());
    for sid in &m.slabs {
        let chunk = reads.get(s, sid)?;
        bytes += chunk.bytes().len() as u64;
        if chunk.contract().precision_bits != m.precision_bits {
            return Err(format!("orbit slab {sid} disagrees with its manifest on precision"));
        }
        slabs.push(OrbitSlab::from_chunk(&chunk).map_err(|e| format!("{sid}: {e}"))?);
    }
    let (re, im) = OrbitSlab::join(&slabs).map_err(|e| e.to_string())?;
    Ok(Orbit { r: Reference { re, im }, bits: m.precision_bits, centre: (m.center_re, m.center_im), slabs: m.slabs, bytes })
}

fn load_table(s: &Store, id: &ChunkId, reads: &mut Reads) -> Result<Table, String> {
    let c = reads.get(s, id)?;
    let t = BlaTable::from_chunk(&c).map_err(|e| format!("{id}: {e}"))?;
    let orbit = t.orbit;
    // COMPILE.md: an empty table (no valid block) costs more than no table; skip it.
    let bla = if t.levels.is_empty() {
        None
    } else {
        let levels = t.levels.into_iter().map(|l| l.into_iter().map(Block::from_array).collect()).collect();
        Some(Bla::new(t.eps, t.dc_max, t.points, levels).map_err(|e| format!("{id}: {e}"))?)
    };
    Ok(Table { orbit, bla, bytes: c.bytes().len() as u64 })
}

/// How the frame used its BLA table.
#[derive(Clone, Copy, PartialEq)]
enum TableUse {
    /// The frame's tiles name no table.
    None,
    /// The table holds no valid block: skipped, the frame renders without it.
    Empty,
    Used,
}

impl TableUse {
    fn name(self) -> &'static str {
        match self {
            TableUse::None => "none",
            TableUse::Empty => "skipped_empty",
            TableUse::Used => "used",
        }
    }
}

/// Play frame `f`: load, render, write, shade, check and print it, adding it to `t`.
#[allow(clippy::too_many_arguments)]
fn frame(
    a: &Args,
    store: &Store,
    cache: &mut Cache,
    fid: &ChunkId,
    f: usize,
    threads: usize,
    dir: Option<&str>,
    looks: &[(String, Box<dyn fd_shade::Pass>)],
    look_at: &fd_shade::Appearance,
    oracle: Option<&str>,
    t: &mut Totals,
) -> Result<(), String> {
    let reset = reset_peak_rss();
    let load = Instant::now();
    let mut reads = Reads::default();
    let fm = FrameManifest::from_chunk(&reads.get(store, fid)?).map_err(|e| format!("frame manifest {fid}: {e}"))?;
    let mut referenced = reads.bytes;
    let mut refs: HashSet<(Kind, ChunkId)> = HashSet::new();
    for tid in &fm.tiles {
        if !cache.tiles.contains_key(tid) {
            let c = reads.get(store, tid)?;
            let tm = TileManifest::from_chunk(&c).map_err(|e| format!("tile manifest {tid}: {e}"))?;
            cache.tiles.insert(*tid, (tm.refs, c.bytes().len() as u64));
        }
        let (r, b) = &cache.tiles[tid];
        referenced += b;
        refs.extend(r.iter().copied());
    }
    let of = |k: Kind| refs.iter().filter(|r| r.0 == k).map(|r| r.1).collect::<Vec<_>>();
    let (oids, bids) = (of(Kind::ORBIT_MANIFEST), of(Kind::BLA));
    let [oid] = oids.as_slice() else { return Err(format!("tiles name {} orbit manifests, need exactly 1", oids.len())) };
    if bids.len() > 1 {
        return Err(format!("tiles name {} BLA tables, need at most 1", bids.len()));
    }
    if !cache.orbits.contains_key(oid) {
        reads.math_miss = true;
        let o = load_orbit(store, oid, &mut reads)?;
        cache.orbits.insert(*oid, o);
    }
    let orbit = &cache.orbits[oid];
    referenced += orbit.bytes;
    let want: HashSet<ChunkId> = orbit.slabs.iter().copied().collect();
    if of(Kind::ORBIT_SLAB).into_iter().collect::<HashSet<_>>() != want {
        return Err("tiles' orbit slabs differ from the orbit manifest's".into());
    }
    let mut table_use = TableUse::None;
    if let Some(bid) = bids.first() {
        if !cache.tables.contains_key(bid) {
            reads.math_miss = true;
            let tb = load_table(store, bid, &mut reads)?;
            cache.tables.insert(*bid, tb);
        }
        let tb = &cache.tables[bid];
        if tb.orbit != *oid {
            return Err(format!("BLA table {bid} was built over orbit {}, not the frame's {oid}", tb.orbit));
        }
        referenced += tb.bytes;
        table_use = if tb.bla.is_some() { TableUse::Used } else { TableUse::Empty };
    }

    // Camera: exact centre from the orbit manifest, width from the frame manifest
    // (COMPILE.md "What a player reads"); the anchor and offset must place that centre.
    let width = fm.width * exp2i(2 - i64::from(fm.anchor.level));
    if !width.is_normal() {
        return Err(format!("frame width {width:e} is not a normal f64"));
    }
    let view = View { center_re: orbit.centre.0.clone(), center_im: orbit.centre.1.clone(), width: format!("{width:e}"), rotation: fm.rotation };
    let (tile, at) = anchor(&view, fm.anchor.level)?;
    let off = (at.0 - 0.5, 0.5 - at.1);
    if tile != fm.anchor || (off.0 - fm.offset.0).abs() > 1e-9 || (off.1 - fm.offset.1).abs() > 1e-9 {
        return Err(format!("anchor {} offset {:?} does not place the orbit's centre (at {tile} {off:?})", fm.anchor, fm.offset));
    }
    let p = Params {
        nx: fm.size.0 * fm.ss,
        ny: fm.size.1 * fm.ss,
        ss: fm.ss,
        max_iter: fm.max_iter,
        escape_radius: ESCAPE_RADIUS,
        columns: ColumnSet(fm.columns),
        threads,
        tier: None,
    };
    for (name, pass) in looks {
        let need = pass.columns().with(fd_samples::Column::Class);
        if need.0 & !p.columns.with(fd_samples::Column::Class).0 != 0 {
            return Err(format!("look {name} needs columns the atlas frame does not have (frame columns mask {})", p.columns.0));
        }
    }
    let supplied = (Reference { re: orbit.r.re.clone(), im: orbit.r.im.clone() }, orbit.bits);
    let (orbit_bits, orbit_points) = (orbit.bits, orbit.r.re.len());
    let load_seconds = load.elapsed().as_secs_f64();

    // Render from the stored orbit (and table): the kernel computes no reference.
    let r = Instant::now();
    let (h, s, st, bla): (Header, Samples, Stats, Option<BlaStats>) = match bids.first().and_then(|b| cache.tables[b].bla.as_ref()) {
        Some(bla) => {
            let (h, s, st, b) = render_bla(&view, &p, supplied, bla)?;
            (h, s, st, Some(b))
        }
        None => {
            let (h, s, st) = render_with(&view, &p, Some(supplied))?;
            (h, s, st, None)
        }
    };
    let render_seconds = r.elapsed().as_secs_f64();
    let mut bytes = Vec::new();
    write(&mut bytes, &h, &s).map_err(|e| e.to_string())?;
    let peak = peak_rss();

    // Keep only what this frame used.
    let used_tiles: HashSet<&ChunkId> = fm.tiles.iter().collect();
    cache.tiles.retain(|k, _| used_tiles.contains(k));
    cache.orbits.retain(|k, _| k == oid);
    cache.tables.retain(|k, _| bids.contains(k));

    let count = |k: fd_samples::Kind| s.class.iter().filter(|c| c.kind() == Some(k)).count();
    let classes = [count(fd_samples::Kind::Escaped), count(fd_samples::Kind::Interior), count(fd_samples::Kind::Unresolved)];
    let state = if reads.math_miss { "cold" } else { "warm" };
    let seconds = load_seconds + render_seconds;
    let fr = Frame {
        view: &view,
        p: &p,
        kernel: h.kernel.clone(),
        stats: st,
        bytes,
        // Reference seconds are 0: the orbit is handed to the kernel, never computed.
        times: vec![(seconds, 0.0)],
        deterministic: true,
        samples: s.class.len(),
        sample_bytes: sample_bytes(&s),
        classes,
        peak_rss: peak,
        peak_rss_scope: if reset { "run" } else { "process" },
        atlas: Some(AtlasWork { state, load_seconds, bytes_read: reads.bytes, bytes_referenced: referenced, tiles_touched: fm.tiles.len(), bla }),
        own_bla: None,
    };

    // Outputs: the .fds (same layout as `fd control -o`) and one PNG per look.
    let out = dir.map(|d| format!("{d}/frame-{f:05}.fds"));
    if let Some(out) = &out {
        std::fs::write(out, &fr.bytes).map_err(|e| format!("{out}: {e}"))?;
    }
    let shade = Instant::now();
    for (name, pass) in looks {
        let png = format!("{}/frame-{f:05}.{name}.png", dir.expect("checked"));
        std::fs::write(&png, fd_shade::png(&pass.shade_with(&h, &s, look_at))).map_err(|e| format!("{png}: {e}"))?;
        t.pngs += 1;
    }
    let shade_seconds = shade.elapsed().as_secs_f64();

    let mut j = format!("{{\"schema\":\"fd-play/1\",\"record\":\"frame\",\"frame\":{f},\"manifest\":\"{fid}\"");
    fr.body(&mut j, "warm", None)?;
    let need_bits = reference_bits(&view, &p)?;
    let _ = write!(
        j,
        ",\"play\":{{\"render_seconds\":{render_seconds},\"load_seconds\":{load_seconds},\"reference_seconds\":0,\"operator_seconds\":0,\
         \"shade_seconds\":{shade_seconds},\"orbit\":\"{oid}\",\"orbit_precision_bits\":{orbit_bits},\"need_bits\":{need_bits},\
         \"orbit_points\":{},\"bla_table\":{},\"bla_use\":\"{}\"",
        orbit_points,
        bids.first().map_or("null".into(), |b| format!("\"{b}\"")),
        table_use.name()
    );
    match bla {
        Some(b) => {
            let equivalent = st.iterations - b.blocks + b.skipped;
            let _ = write!(
                j,
                ",\"bla_blocks\":{},\"bla_skipped_steps\":{},\"iterations_equivalent\":{equivalent},\"fallback_samples\":{},\"closed_form_samples\":{},\"shift_px_max\":{}}}",
                b.blocks,
                b.skipped,
                b.fallback_samples,
                b.closed_form_samples,
                b.shift_px.map_or("null".into(), |x| x.to_string())
            );
        }
        None => {
            let _ = write!(j, ",\"bla_blocks\":0,\"bla_skipped_steps\":0,\"iterations_equivalent\":{},\"fallback_samples\":null,\"closed_form_samples\":null,\"shift_px_max\":null}}", st.iterations);
        }
    }
    let _ = write!(j, ",\"fds\":{}", out.as_deref().map_or("null".into(), q));
    let mut ok = true;
    match (oracle, &out) {
        (Some(script), Some(out)) => {
            let (report, passed, secs) = run_oracle(a, script, out)?;
            ok &= passed;
            t.oracle_frames += 1;
            t.oracle_failures += usize::from(!passed);
            let _ = write!(j, ",\"oracle\":{report},\"oracle_seconds\":{secs}");
        }
        _ => j.push_str(",\"oracle\":null"),
    }
    let _ = write!(j, ",\"ok\":{ok}}}");
    println!("{j}");

    let pixels = fr.pixels();
    let log10_width = (fr.log2_spacing()? + f64::from(p.nx).log2()) * std::f64::consts::LOG10_2;
    t.ok &= ok;
    t.frames += 1;
    let slot = usize::from(state == "warm");
    t.seconds[slot] += seconds;
    t.load[slot] += load_seconds;
    t.count[slot] += 1;
    t.render += render_seconds;
    t.iterations += st.iterations;
    t.pixels += pixels;
    t.samples += fr.samples as u64;
    t.sample_bytes += fr.sample_bytes as u64;
    t.fds_bytes += fr.bytes.len() as u64;
    t.bytes_read += reads.bytes;
    t.bytes_referenced += referenced;
    t.tiles += fm.tiles.len() as u64;
    t.reference_len_max = t.reference_len_max.max(st.reference_len);
    t.peak_rss = t.peak_rss.max(peak);
    for (sum, c) in t.classes.iter_mut().zip(classes) {
        *sum += c as u64;
    }
    t.log10_width = (t.log10_width.0.min(log10_width), t.log10_width.1.max(log10_width));
    t.bits_max = t.bits_max.max(need_bits);
    t.table_use[table_use as usize] += 1;
    if let Some(b) = bla {
        t.blocks += b.blocks;
        t.skipped += b.skipped;
        t.fallback_samples += b.fallback_samples + b.closed_form_samples;
        if let Some(px) = b.shift_px {
            t.shift_px_max = t.shift_px_max.max(px);
        }
    } else {
        t.fallback_samples += fr.samples as u64;
    }
    Ok(())
}

/// The mp4 mux, done only when asked and when `ffmpeg` is on PATH.
enum Mux {
    NotAsked,
    NoFfmpeg,
    Done(String, f64),
}

fn mux(out: &str, dir: &str, look: &str, start: usize, fps: f64) -> Result<Mux, String> {
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        return Ok(Mux::NoFfmpeg);
    }
    let t = Instant::now();
    let pattern = format!("{dir}/frame-%05d.{look}.png");
    let o = Command::new("ffmpeg")
        .args(["-y", "-loglevel", "error", "-framerate", &fps.to_string(), "-start_number", &start.to_string(), "-i", &pattern])
        .args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "16", out])
        .output()
        .map_err(|e| format!("ffmpeg: {e}"))?;
    if !o.status.success() {
        return Err(format!("ffmpeg failed: {}", String::from_utf8_lossy(&o.stderr)));
    }
    Ok(Mux::Done(out.to_string(), t.elapsed().as_secs_f64()))
}

/// Sums over the played frames. Index 0 of `seconds`/`load`/`count` is cold, 1 warm;
/// `table_use` is indexed by [`TableUse`].
#[derive(Default)]
struct Totals {
    frames: usize,
    ok: bool,
    seconds: [f64; 2],
    load: [f64; 2],
    count: [usize; 2],
    render: f64,
    iterations: u64,
    pixels: f64,
    samples: u64,
    sample_bytes: u64,
    fds_bytes: u64,
    bytes_read: u64,
    bytes_referenced: u64,
    tiles: u64,
    reference_len_max: usize,
    peak_rss: Option<u64>,
    classes: [u64; 3],
    log10_width: (f64, f64),
    bits_max: u32,
    table_use: [usize; 3],
    blocks: u64,
    skipped: u64,
    fallback_samples: u64,
    shift_px_max: f64,
    pngs: usize,
    oracle_frames: usize,
    oracle_failures: usize,
}

#[allow(clippy::too_many_arguments)]
fn totals(log: &str, record: &CompileLog, (from, to): (usize, usize), threads: usize, t: &Totals, mp4: &Mux, error: Option<&str>, wall: f64) -> String {
    let n = t.frames;
    let num = |x: f64, d: usize| if d == 0 { "null".to_string() } else { (x / d as f64).to_string() };
    let ratio = |x: f64, d: f64| if d == 0.0 { "null".to_string() } else { (x / d).to_string() };
    let mut j = format!("{{\"schema\":\"fd-play/1\",\"record\":\"totals\",\"compile_log\":{},\"frames\":{n}", q(log));
    let _ = write!(j, ",\"range\":[{from},{to}],\"threads\":{threads},\"fps\":{},\"fps_source\":\"{}\"", record.fps, if record.fps_logged { "compile_log" } else { "default" });
    // Not in the frame manifest: stated, not silently derived.
    let _ = write!(j, ",\"assumed\":{{\"kernel\":\"auto\",\"escape_radius\":{ESCAPE_RADIUS}}}");
    j.push_str(",\"cache\":{\"atlas\":\"store\",\"cross_frame_reuse\":true,\"keep\":\"chunks of the previous frame\"}");
    let _ = write!(
        j,
        ",\"timing\":{{\"cold_seconds\":{},\"cold_frames\":{},\"cold_seconds_per_frame\":{},\"warm_seconds\":{},\"warm_frames\":{},\"warm_seconds_per_frame\":{},\"seconds\":{},\"seconds_per_frame\":{},\"render_seconds\":{},\"wall_seconds\":{wall}}}",
        t.seconds[0],
        t.count[0],
        num(t.seconds[0], t.count[0]),
        t.seconds[1],
        t.count[1],
        num(t.seconds[1], t.count[1]),
        t.seconds[0] + t.seconds[1],
        num(t.seconds[0] + t.seconds[1], n),
        t.render
    );
    let _ = write!(
        j,
        ",\"load_seconds\":{{\"total\":{},\"per_frame\":{},\"cold_per_frame\":{},\"warm_per_frame\":{}}}",
        t.load[0] + t.load[1],
        num(t.load[0] + t.load[1], n),
        num(t.load[0], t.count[0]),
        num(t.load[1], t.count[1])
    );
    j.push_str(",\"reference_seconds\":{\"total\":0,\"per_frame\":0},\"operator_seconds\":{\"total\":0,\"per_frame\":0}");
    let _ = write!(
        j,
        ",\"iterations\":{{\"total\":{},\"per_pixel\":{},\"per_sample\":{},\"equivalent\":{},\"reference_length_max\":{}}}",
        t.iterations,
        ratio(t.iterations as f64, t.pixels),
        ratio(t.iterations as f64, t.samples as f64),
        t.iterations - t.blocks + t.skipped,
        t.reference_len_max
    );
    let _ = write!(
        j,
        ",\"bytes\":{{\"fds_bytes\":{},\"atlas_bytes_read\":{},\"atlas_bytes_read_per_frame\":{},\"atlas_bytes_referenced\":{},\"atlas_bytes_referenced_per_frame\":{}}}",
        t.fds_bytes,
        t.bytes_read,
        num(t.bytes_read as f64, n),
        t.bytes_referenced,
        num(t.bytes_referenced as f64, n)
    );
    let _ = write!(
        j,
        ",\"atlas_work\":{{\"tiles_touched\":{},\"tiles_touched_per_frame\":{},\"microblocks_touched\":0,\"macro_operators_per_pixel\":{}}}",
        t.tiles,
        num(t.tiles as f64, n),
        ratio(t.blocks as f64, t.pixels)
    );
    let _ = write!(
        j,
        ",\"fallback\":{{\"pixel_fraction\":{},\"iterations_per_pixel\":{},\"unresolved_fraction\":{}}}",
        ratio(t.fallback_samples as f64, t.samples as f64),
        ratio((t.iterations - t.blocks) as f64, t.pixels),
        ratio(t.classes[2] as f64, t.samples as f64)
    );
    let _ = write!(
        j,
        ",\"bla\":{{\"frames_used\":{},\"frames_empty_table_skipped\":{},\"frames_without_table\":{},\"blocks\":{},\"skipped_steps\":{},\"shift_px_max\":{}}}",
        t.table_use[TableUse::Used as usize],
        t.table_use[TableUse::Empty as usize],
        t.table_use[TableUse::None as usize],
        t.blocks,
        t.skipped,
        t.shift_px_max
    );
    let _ = write!(
        j,
        ",\"memory\":{{\"peak_rss_bytes\":{},\"sample_bytes\":{},\"device\":\"cpu\",\"peak_vram_bytes\":0}}",
        t.peak_rss.map_or("null".into(), |b| b.to_string()),
        t.sample_bytes
    );
    let _ = write!(j, ",\"classes\":{{\"escaped\":{},\"interior\":{},\"unresolved\":{}}}", t.classes[0], t.classes[1], t.classes[2]);
    let _ = write!(
        j,
        ",\"depth\":{{\"log10_width_min\":{},\"log10_width_max\":{},\"precision_bits_max\":{}}}",
        if n == 0 { "null".into() } else { t.log10_width.0.to_string() },
        if n == 0 { "null".into() } else { t.log10_width.1.to_string() },
        if n == 0 { "null".into() } else { t.bits_max.to_string() }
    );
    let mp4 = match mp4 {
        Mux::NotAsked => "null".to_string(),
        Mux::NoFfmpeg => "{\"file\":null,\"skipped\":\"ffmpeg not on PATH\"}".to_string(),
        Mux::Done(f, s) => format!("{{\"file\":{},\"seconds\":{s}}}", q(f)),
    };
    let _ = write!(j, ",\"outputs\":{{\"pngs\":{},\"mp4\":{mp4}}}", t.pngs);
    let ok = t.ok && error.is_none();
    let _ = write!(
        j,
        ",\"oracle_frames\":{},\"oracle_failures\":{},\"error\":{},\"ok\":{ok}}}",
        t.oracle_frames,
        t.oracle_failures,
        error.map_or("null".into(), q)
    );
    j
}
