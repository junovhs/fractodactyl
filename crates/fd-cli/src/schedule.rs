//! `fd schedule`: the atlas compiler's scheduling core (SCHE-02, docs/spec/SCHEDULE.md).
//! It derives the build jobs a known camera path needs (reference orbits, BLA tables
//! over them), estimates each job's cost before building, and builds the chunks into the
//! store with `--workers` threads, least slack first over the dependency DAG. The compile
//! log says, per job, whether its chunk was ready before the playback time of the first
//! frame that uses it, and compares the order against earliest-deadline and first-use orders.
use crate::args::Args;
use crate::orbit::{put_sized, EPS};
use crate::render::{params, FLAGS};
use crate::reuse::{groups, lead, path_frames};
use fd_atlas::{BlaTable, Chunk, ChunkId, Contract, Formula, Kind, Rounding, Store};
use fd_kernel::{bla_dc_max, reference, Bla, Block, Params, Reference};
use fd_samples::View;
use std::panic::AssertUnwindSafe;
use std::sync::{mpsc, Arc};
use std::time::Instant;

const USAGE: &str = "usage: fd schedule PATH --store DIR [render flags without --re/--im/--width] [--fps F] [--lead S]
                   [--workers N] [--policy slack|edf|first-use] [--bla frame|group|none] [--slab N] [--on-miss fail|report]";

/// Orbit points the cost probe computes (and BLA-builds) per orbit job.
const PROBE: u64 = 8192;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Policy {
    /// Least slack `D - now - cost` over effective deadlines (the scheduler).
    Slack,
    /// Earliest effective deadline, ignoring cost (optimal for maximum lateness on one
    /// worker, Lawler's rule; a baseline).
    Edf,
    /// Own first-use deadline, ignoring cost and dependents (the naive baseline).
    FirstUse,
}

impl Policy {
    pub(crate) fn parse(s: &str) -> Result<Policy, String> {
        match s {
            "slack" => Ok(Policy::Slack),
            "edf" => Ok(Policy::Edf),
            "first-use" => Ok(Policy::FirstUse),
            x => Err(format!("--policy: expected slack, edf or first-use, got {x:?}")),
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Policy::Slack => "slack",
            Policy::Edf => "edf",
            Policy::FirstUse => "first-use",
        }
    }
}

/// What a job builds. Certificates would be a third kind; no producer exists yet.
pub(crate) enum Work {
    /// The reference orbit of this frame (a group's deepest), as slabs plus a manifest.
    Orbit { frame: usize },
    /// A BLA table over the orbit built by job `orbit`, for `|dc| <= dc_max`.
    Bla { orbit: usize, dc_max: f64 },
}

pub(crate) struct Job {
    pub(crate) name: String,
    pub(crate) work: Work,
    deps: Vec<usize>,
    /// First frame whose render reads the chunk.
    pub(crate) first_use: usize,
    /// `lead + first_use / fps`: when that frame is presented.
    deadline: f64,
    /// `min(deadline, min over dependents k of (effective_k - est_k))`.
    effective: f64,
    /// Estimated build seconds, from the probes.
    est: f64,
    /// Estimated encoded bytes.
    pub(crate) est_bytes: u64,
}

/// A finished job: times are seconds since the compile clock started.
pub(crate) struct Done {
    start: f64,
    finish: f64,
    worker: usize,
    pub(crate) chunk: ChunkId,
    pub(crate) bytes: usize,
    pub(crate) points: usize,
    /// Seconds the builder spent computing (orbit or table), before encoding and storing.
    pub(crate) compute: f64,
}

/// A built job: chunk id, encoded bytes, orbit points, compute seconds, and (orbit
/// jobs) the orbit with its manifest id for dependents.
type Built = (ChunkId, usize, usize, f64, Option<Arc<(Reference, ChunkId)>>);

/// Per frame: the orbit job it reads and its BLA table job, if any.
pub(crate) type Uses = (usize, Option<usize>);

/// Frames sharing one BLA table, each with its `dc_max`.
type Users = Vec<(usize, f64)>;

/// How BLA tables are shared among the frames of an orbit group (SCHEDULE.md "Jobs").
pub(crate) enum Share<'a> {
    /// No tables.
    None,
    /// One table per frame, for its own `dc_max`.
    Frame,
    /// One table per group, at the largest `dc_max` of its frames.
    Group,
    /// One table per run of `span` tile levels (`levels[f]` is frame `f`'s level, PLAN.md),
    /// at the largest `dc_max` of the group's frames at those levels (VIDE-01).
    Level { levels: &'a [u32], span: u32 },
}

#[derive(Clone, Copy, PartialEq)]
enum Status {
    Waiting,
    Running,
    Done,
}

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let view_flags = ["re", "im", "width", "rotation", "o"];
    let extra = ["store", "slab", "fps", "lead", "workers", "policy", "bla", "on-miss"];
    let known: Vec<&str> = FLAGS.iter().copied().filter(|f| !view_flags.contains(f)).chain(extra).collect();
    let a = Args::parse(argv, &known)?;
    let [file] = a.positional.as_slice() else { return Err(USAGE.into()) };
    let p = params(&a)?;
    let slab: u32 = a.num("slab", 4096)?;
    let fps: f64 = a.num("fps", 30.0)?;
    let lead_s: f64 = a.num("lead", 0.0)?;
    let workers: usize = a.num("workers", 1)?;
    if !(fps > 0.0 && fps.is_finite() && lead_s >= 0.0 && lead_s.is_finite()) || workers == 0 {
        return Err("--fps and --workers must be positive, --lead non-negative".into());
    }
    let policy = Policy::parse(a.str("policy").unwrap_or("slack"))?;
    let bla_mode = a.str("bla").unwrap_or("frame");
    let share = match bla_mode {
        "frame" => Share::Frame,
        "group" => Share::Group,
        "none" => Share::None,
        _ => return Err(format!("--bla: expected frame, group or none, got {bla_mode:?}")),
    };
    let fail = match a.str("on-miss").unwrap_or("fail") {
        "fail" => true,
        "report" => false,
        x => return Err(format!("--on-miss: expected fail or report, got {x:?}")),
    };
    let store = crate::chunk::store(&a)?;
    let frames = path_frames(file, &p)?;
    let before = store.stats().map_err(|e| e.to_string())?.bytes;

    // The compile clock: probes, scheduling and builds all count against deadlines.
    let t0 = Instant::now();
    let (mut jobs, _, skipped) = derive(&frames, &p, &share, fps, lead_s);
    let cost = store_probe(a.need("store")?)?;
    estimate(&mut jobs, &frames, &p, cost, slab)?;
    let estimate_s = t0.elapsed().as_secs_f64();
    effective(&mut jobs);
    let done = execute(&jobs, &frames, &p, &store, slab, workers, policy, t0)?;

    let count = |f: &dyn Fn(&Work) -> bool| jobs.iter().filter(|j| f(&j.work)).count();
    println!("frames {}\nfps {fps}\nlead_seconds {lead_s}\nworkers {workers}\npolicy {}", frames.len(), policy.name());
    println!("bla_mode {bla_mode}\nbla_skipped_frames {skipped}");
    print_jobs(&jobs, &done);
    print_compare(&jobs, &done, workers, estimate_s, fps);
    let measured: Vec<f64> = done.iter().map(|d| d.finish - d.start).collect();
    let estimated: Vec<f64> = jobs.iter().map(|j| j.est).collect();
    let m = outcome(&jobs, &done, fps);
    let (est_total, act_total): (f64, f64) = (estimated.iter().sum(), measured.iter().sum());
    let mean_err = jobs.iter().zip(&measured).map(|(j, a)| ((j.est - a) / a).abs()).sum::<f64>() / jobs.len() as f64;
    println!("jobs {}", jobs.len());
    println!("jobs.orbit {}", count(&|w| matches!(w, Work::Orbit { .. })));
    println!("jobs.bla {}", count(&|w| matches!(w, Work::Bla { .. })));
    println!("jobs.certificate 0");
    print_outcome(&jobs, &m);
    println!("estimate_seconds {estimate_s:.6}");
    println!("store_cost.per_chunk_seconds {:.6}\nstore_cost.per_mib_seconds {:.6}", cost.per_chunk, cost.per_byte * f64::from(1u32 << 20));
    println!("cost.estimated_seconds {est_total:.6}\ncost.actual_seconds {act_total:.6}");
    println!("cost.total_error {:+.3}\ncost.mean_abs_error {mean_err:.3}", (est_total - act_total) / act_total);
    let after = store.stats().map_err(|e| e.to_string())?.bytes;
    println!("stored_bytes {}", after.saturating_sub(before));
    crate::chunk::report_target(&store)?;
    if fail && m.missed > 0 {
        // A missed deadline is a result, not a usage or build error: its own exit code.
        missed_exit("chunks", m.missed, jobs.len(), lead_s, m.min_lead);
    }
    Ok(())
}

/// Report missed deadlines on stderr and exit with [`crate::EXIT_MISSED`].
pub(crate) fn missed_exit(what: &str, missed: usize, of: usize, lead_s: f64, min_lead: f64) -> ! {
    eprintln!("fd: {missed} of {of} {what} were not ready by their first-use frame (lead {lead_s} s; this run needed {min_lead:.6} s)");
    let _ = std::io::Write::flush(&mut std::io::stdout());
    std::process::exit(crate::EXIT_MISSED);
}

/// Missed deadlines and lateness of the jobs as they really ran.
pub(crate) fn outcome(jobs: &[Job], done: &[Done], fps: f64) -> Metrics {
    let finish: Vec<f64> = done.iter().map(|d| d.finish).collect();
    metrics(jobs, &finish, fps)
}

/// The `met`, `missed`, lateness, makespan and minimum-lead lines.
pub(crate) fn print_outcome(jobs: &[Job], m: &Metrics) {
    println!("met {}\nmissed {}", jobs.len() - m.missed, m.missed);
    println!("max_lateness_seconds {:.6}\nmakespan_seconds {:.6}\nmin_lead_seconds {:.6}", m.max_lateness, m.makespan, m.min_lead);
}

/// One `job` line per job, in start order (SCHEDULE.md "Compile log").
pub(crate) fn print_jobs(jobs: &[Job], done: &[Done]) {
    let mut order: Vec<usize> = (0..jobs.len()).collect();
    order.sort_by(|&x, &y| done[x].start.total_cmp(&done[y].start).then(x.cmp(&y)));
    for &j in &order {
        let (job, d) = (&jobs[j], &done[j]);
        let deps: Vec<&str> = job.deps.iter().map(|&k| jobs[k].name.as_str()).collect();
        let actual = d.finish - d.start;
        println!(
            "job {} kind {} deps {} first_use {} deadline {:.6} effective {:.6} est_seconds {:.6} start {:.6} slack_at_start {:.6} finish {:.6} actual_seconds {actual:.6} est_error {:+.3} slack {:.6} ready {} worker {} points {} est_bytes {} bytes {} chunk {}",
            job.name,
            kind(&job.work),
            if deps.is_empty() { "-".into() } else { deps.join(",") },
            job.first_use,
            job.deadline,
            job.effective,
            job.est,
            d.start,
            job.effective - d.start - job.est,
            d.finish,
            (job.est - actual) / actual,
            job.deadline - d.finish,
            if d.finish <= job.deadline { "yes" } else { "no" },
            d.worker,
            d.points,
            job.est_bytes,
            d.bytes,
            d.chunk
        );
    }
}

/// The same jobs under each policy, simulated with this run's measured and estimated
/// costs (list scheduling on `workers` workers, starting when the real first dispatch did).
pub(crate) fn print_compare(jobs: &[Job], done: &[Done], workers: usize, estimate_s: f64, fps: f64) {
    let measured: Vec<f64> = done.iter().map(|d| d.finish - d.start).collect();
    let estimated: Vec<f64> = jobs.iter().map(|j| j.est).collect();
    for pol in [Policy::Slack, Policy::Edf, Policy::FirstUse] {
        for (name, costs) in [("measured", &measured), ("estimated", &estimated)] {
            let finish = simulate(jobs, pol, workers, costs, estimate_s);
            let m = metrics(jobs, &finish, fps);
            println!(
                "compare {} costs {name} missed {} max_lateness_seconds {:.6} makespan_seconds {:.6} min_lead_seconds {:.6}",
                pol.name(),
                m.missed,
                m.max_lateness,
                m.makespan,
                m.min_lead
            );
        }
    }
}

fn kind(w: &Work) -> &'static str {
    match w {
        Work::Orbit { .. } => "orbit",
        Work::Bla { .. } => "bla",
    }
}

/// Jobs for the path: one orbit per exact-centre group (REF-02 grouping), then BLA
/// tables over it as `share` says. Parents get lower indices than their dependents.
/// Also returns, per frame, the orbit job and table job it reads, and the number of
/// frames with no BLA table because they are on the scaled tier (BLA is unavailable there).
pub(crate) fn derive(frames: &[(View, u32)], p: &Params, share: &Share, fps: f64, lead_s: f64) -> (Vec<Job>, Vec<Uses>, usize) {
    let at = |f: usize| lead_s + f as f64 / fps;
    let mut jobs = Vec::new();
    let mut uses = vec![(0, None); frames.len()];
    let mut skipped = 0;
    for (g, members) in groups(frames).iter().enumerate() {
        let orbit = jobs.len();
        let first = members[0];
        let work = Work::Orbit { frame: lead(frames, members) };
        jobs.push(Job { name: format!("orbit.{g}"), work, deps: vec![], first_use: first, deadline: at(first), effective: 0.0, est: 0.0, est_bytes: 0 });
        // Table key -> (name, member frames with their dc_max), in order of first member.
        let mut tables: Vec<(i64, String, Users)> = Vec::new();
        for &f in members {
            uses[f].0 = orbit;
            let Ok(dc) = bla_dc_max(&frames[f].0, p) else {
                skipped += 1;
                continue;
            };
            let (key, name) = match share {
                Share::None => continue,
                Share::Frame => (f as i64, format!("bla.{f}")),
                Share::Group => (0, format!("bla.g{g}")),
                Share::Level { levels, span } => {
                    let k = levels[f] / span.max(&1);
                    (i64::from(k), format!("bla.g{g}.l{}", k * span.max(&1)))
                }
            };
            match tables.iter_mut().find(|t| t.0 == key) {
                Some(t) => t.2.push((f, dc)),
                None => tables.push((key, name, vec![(f, dc)])),
            }
        }
        for (_, name, users) in tables {
            let dc_max = users.iter().map(|u| u.1).fold(0.0, f64::max);
            let f = users[0].0;
            for &(u, _) in &users {
                uses[u].1 = Some(jobs.len());
            }
            let work = Work::Bla { orbit, dc_max };
            jobs.push(Job { name, work, deps: vec![orbit], first_use: f, deadline: at(f), effective: 0.0, est: 0.0, est_bytes: 0 });
        }
    }
    (jobs, uses, skipped)
}

/// Cost model, before anything is built. Store cost: `store_probe` times puts into a
/// scratch store on the same filesystem, giving seconds per chunk and per byte. Orbit
/// job: compute the first `PROBE` points of the same orbit (same centre, same precision)
/// and time it; the orbit's predicted length is the probe's if the probe escaped, else
/// `min(max_iter, MAX_LEN - 1) + 1`; compute = length x seconds per probe point; bytes
/// = 16 per point plus `CHUNK_OVERHEAD` per slab and manifest. BLA job: build its table
/// over the probe orbit, scale the time by the same length; bytes = 56 per block of the
/// probe table's levels at the predicted length. Estimate = compute + chunks x per-chunk
/// + bytes x per-byte. Not modelled: encoding, contention between workers.
pub(crate) fn estimate(jobs: &mut [Job], frames: &[(View, u32)], p: &Params, cost: StoreCost, slab: u32) -> Result<(), String> {
    let mut probes: Vec<Option<(Reference, u64)>> = (0..jobs.len()).map(|_| None).collect();
    let store_s = |chunks: u64, bytes: u64| chunks as f64 * cost.per_chunk + bytes as f64 * cost.per_byte;
    for j in 0..jobs.len() {
        match jobs[j].work {
            Work::Orbit { frame } => {
                let q = Params { max_iter: p.max_iter.min(PROBE), ..*p };
                let t = Instant::now();
                let (r, _) = reference(&frames[frame].0, &q)?;
                let secs = t.elapsed().as_secs_f64();
                let full = p.max_iter.min(Reference::MAX_LEN as u64 - 1) + 1;
                let points = if (r.len() as u64) < q.max_iter + 1 { r.len() as u64 } else { full };
                let chunks = points.div_ceil(u64::from(slab)) + 1;
                let bytes = 16 * points + CHUNK_OVERHEAD * chunks;
                jobs[j].est = secs / r.len() as f64 * points as f64 + store_s(chunks, bytes);
                jobs[j].est_bytes = bytes;
                probes[j] = Some((r, points));
            }
            Work::Bla { orbit, dc_max } => {
                let (r, points) = probes[orbit].as_ref().expect("an orbit job precedes its BLA jobs");
                let t = Instant::now();
                let levels = Bla::build(r, EPS, dc_max)?.levels.len();
                let secs = t.elapsed().as_secs_f64();
                let steps = points.saturating_sub(2);
                let bytes = 56 * (1..=levels).map(|l| steps >> l).sum::<u64>() + CHUNK_OVERHEAD;
                jobs[j].est = secs / r.len() as f64 * *points as f64 + store_s(1, bytes);
                jobs[j].est_bytes = bytes;
            }
        }
    }
    Ok(())
}

/// Remove `BASE.schedule-probe-PID` directories left by killed runs: those whose PID is
/// no longer a live process (`/proc/PID` absent). Probes of live runs are left alone.
fn remove_stale_probes(base: &str) {
    let path = std::path::Path::new(base);
    let (Some(name), parent) = (path.file_name().and_then(|n| n.to_str()), path.parent()) else { return };
    let parent = match parent {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => std::path::Path::new("."),
    };
    let prefix = format!("{name}.schedule-probe-");
    let Ok(entries) = std::fs::read_dir(parent) else { return };
    for e in entries.flatten() {
        let file = e.file_name();
        let Some(pid) = file.to_str().and_then(|f| f.strip_prefix(&prefix)).and_then(|p| p.parse::<u32>().ok()) else { continue };
        if pid != std::process::id() && !std::path::Path::new(&format!("/proc/{pid}")).exists() {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
}

/// Header, contract fields and counts of one chunk, rounded up (ATLAS.md).
const CHUNK_OVERHEAD: u64 = 96;

/// Measured store write cost: seconds per put and per stored byte.
#[derive(Clone, Copy)]
pub(crate) struct StoreCost {
    pub(crate) per_chunk: f64,
    pub(crate) per_byte: f64,
}

/// Time `put` of small and 1 MiB chunks (median of 3 each) into a scratch store beside
/// `dir` (`DIR.schedule-probe-PID`, same filesystem, removed afterwards).
pub(crate) fn store_probe(dir: &str) -> Result<StoreCost, String> {
    let base = dir.trim_end_matches('/');
    remove_stale_probes(base);
    let root = format!("{base}.schedule-probe-{}", std::process::id());
    let _ = std::fs::remove_dir_all(&root);
    let s = Store::open(&root).map_err(|e| format!("{root}: {e}"))?;
    let contract = Contract { kind: Kind::SAMPLES, encoding: 1, formula: Formula::NONE, precision_bits: 0, rounding: Rounding::Exact };
    let time = |len: usize, k: u8| -> Result<f64, String> {
        let payload: Vec<u8> = (0..len).map(|i| (i as u8).wrapping_mul(31) ^ k).collect();
        let chunk = Chunk::new(contract, &payload);
        let t = Instant::now();
        s.put(&chunk).map_err(|e| e.to_string())?;
        Ok(t.elapsed().as_secs_f64())
    };
    let median = |len: usize| -> Result<f64, String> {
        let mut v = [time(len, 1)?, time(len, 2)?, time(len, 3)?];
        v.sort_by(f64::total_cmp);
        Ok(v[1])
    };
    let (small, big) = (median(64), median(1 << 20));
    let _ = std::fs::remove_dir_all(&root);
    let (small, big) = (small?, big?);
    Ok(StoreCost { per_chunk: small, per_byte: ((big - small) / f64::from(1u32 << 20)).max(0.0) })
}

/// Effective deadlines, dependents first (they have higher indices): a job must finish
/// early enough for every dependent to start by its own latest start, `effective - est`.
pub(crate) fn effective(jobs: &mut [Job]) {
    for j in &mut *jobs {
        j.effective = j.deadline;
    }
    for j in (0..jobs.len()).rev() {
        let latest_start = jobs[j].effective - jobs[j].est;
        for k in jobs[j].deps.clone() {
            jobs[k].effective = jobs[k].effective.min(latest_start);
        }
    }
}

/// The next job to start among those whose dependencies are all done. Slack: least
/// `effective - now - est` (now is common to all candidates, so least `effective - est`),
/// ties to the earlier effective deadline; edf: earliest effective deadline; first-use:
/// earliest own deadline. Then index.
fn pick(jobs: &[Job], status: &[Status], policy: Policy) -> Option<usize> {
    let ready = (0..jobs.len()).filter(|&j| status[j] == Status::Waiting && jobs[j].deps.iter().all(|&d| status[d] == Status::Done));
    let key = |j: usize| match policy {
        Policy::Slack => (jobs[j].effective - jobs[j].est, jobs[j].effective),
        Policy::Edf => (jobs[j].effective, 0.0),
        Policy::FirstUse => (jobs[j].deadline, 0.0),
    };
    ready.min_by(|&x, &y| {
        let (a, b) = (key(x), key(y));
        a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)).then(x.cmp(&y))
    })
}

/// Build every job into the store, `workers` at a time, in `policy` order. Each job runs
/// on its own scoped thread; the dispatcher starts the best ready job whenever a worker
/// is free and records start (dispatch) and finish (chunk stored) times.
#[allow(clippy::too_many_arguments)]
pub(crate) fn execute(
    jobs: &[Job],
    frames: &[(View, u32)],
    p: &Params,
    store: &Store,
    slab: u32,
    workers: usize,
    policy: Policy,
    t0: Instant,
) -> Result<Vec<Done>, String> {
    let n = jobs.len();
    let mut status = vec![Status::Waiting; n];
    let mut orbits: Vec<Option<Arc<(Reference, ChunkId)>>> = vec![None; n];
    let mut done: Vec<Option<Done>> = (0..n).map(|_| None).collect();
    let mut starts = vec![0.0; n];
    let (tx, rx) = mpsc::channel::<(usize, usize, f64, Result<Built, String>)>();
    let mut err = None;
    std::thread::scope(|sc| {
        let mut free: Vec<usize> = (0..workers).rev().collect();
        let (mut running, mut finished) = (0, 0);
        while finished < n {
            while err.is_none() && !free.is_empty() {
                let Some(j) = pick(jobs, &status, policy) else { break };
                let w = free.pop().expect("a free worker");
                status[j] = Status::Running;
                starts[j] = t0.elapsed().as_secs_f64();
                running += 1;
                let orbit = match jobs[j].work {
                    Work::Bla { orbit, .. } => orbits[orbit].clone(),
                    Work::Orbit { .. } => None,
                };
                let tx = tx.clone();
                let job = &jobs[j];
                sc.spawn(move || {
                    // A panicking builder reports an error instead of leaving the
                    // dispatcher waiting on `recv` forever.
                    let r = std::panic::catch_unwind(AssertUnwindSafe(|| build(job, orbit, frames, p, store, slab)))
                        .unwrap_or_else(|e| Err(format!("builder panicked: {}", panic_message(&*e))));
                    let _ = tx.send((j, w, t0.elapsed().as_secs_f64(), r));
                });
            }
            if running == 0 {
                break;
            }
            let Ok((j, w, finish, r)) = rx.recv() else {
                err = Some("a builder thread exited without reporting".into());
                break;
            };
            running -= 1;
            free.push(w);
            match r {
                Ok((chunk, bytes, points, compute, orbit)) => {
                    status[j] = Status::Done;
                    orbits[j] = orbit;
                    done[j] = Some(Done { start: starts[j], finish, worker: w, chunk, bytes, points, compute });
                    finished += 1;
                }
                Err(e) => err = Some(format!("{}: {e}", jobs[j].name)),
            }
        }
    });
    if let Some(e) = err {
        return Err(e);
    }
    done.into_iter().map(|d| d.ok_or_else(|| "a job was never scheduled".to_string())).collect()
}

/// The text of a panic payload.
fn panic_message(e: &(dyn std::any::Any + Send)) -> String {
    e.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| e.downcast_ref::<String>().cloned()).unwrap_or_else(|| "(no message)".into())
}

/// Build one job's chunk with the existing builders and store it: chunk id, encoded
/// bytes, orbit points, and (orbit jobs) the orbit for dependents.
fn build(
    job: &Job,
    orbit: Option<Arc<(Reference, ChunkId)>>,
    frames: &[(View, u32)],
    p: &Params,
    store: &Store,
    slab: u32,
) -> Result<Built, String> {
    match job.work {
        Work::Orbit { frame } => {
            let view = &frames[frame].0;
            let t = Instant::now();
            let (r, bits) = reference(view, p)?;
            let compute = t.elapsed().as_secs_f64();
            let (id, bytes) = put_sized(store, view, &r, bits, slab, false)?;
            let points = r.len();
            Ok((id, bytes, points, compute, Some(Arc::new((r, id)))))
        }
        Work::Bla { dc_max, .. } => {
            let parent = orbit.ok_or("BLA job started before its orbit")?;
            let (r, orbit) = (&parent.0, parent.1);
            let t = Instant::now();
            let bla = Bla::build(r, EPS, dc_max)?;
            let compute = t.elapsed().as_secs_f64();
            let levels = bla.levels.iter().map(|l| l.iter().map(Block::to_array).collect()).collect();
            let table = BlaTable { orbit, eps: EPS, dc_max, points: bla.points, levels };
            let chunk = table.to_chunk().map_err(|e| e.to_string())?;
            let (id, _) = store.put(&chunk).map_err(|e| e.to_string())?;
            Ok((id, chunk.bytes().len(), r.len(), compute, None))
        }
    }
}

/// List-schedule the jobs on `workers` workers with the given durations, starting at
/// `t`; returns each job's finish time.
fn simulate(jobs: &[Job], policy: Policy, workers: usize, cost: &[f64], mut t: f64) -> Vec<f64> {
    let mut status = vec![Status::Waiting; jobs.len()];
    let mut finish = vec![0.0; jobs.len()];
    let mut running: Vec<(f64, usize)> = Vec::new();
    loop {
        while running.len() < workers {
            let Some(j) = pick(jobs, &status, policy) else { break };
            status[j] = Status::Running;
            running.push((t + cost[j], j));
        }
        let Some(k) = (0..running.len()).min_by(|&x, &y| running[x].0.total_cmp(&running[y].0).then(running[x].1.cmp(&running[y].1))) else {
            break;
        };
        let (f, j) = running.swap_remove(k);
        t = f;
        status[j] = Status::Done;
        finish[j] = f;
    }
    finish
}

pub(crate) struct Metrics {
    pub(crate) missed: usize,
    max_lateness: f64,
    makespan: f64,
    pub(crate) min_lead: f64,
}

/// Missed deadlines, worst `finish - deadline`, last finish, and the smallest lead that
/// would have met every deadline with these finish times (the order does not depend on
/// the lead: every deadline shifts by it).
fn metrics(jobs: &[Job], finish: &[f64], fps: f64) -> Metrics {
    let late = |j: usize| finish[j] - jobs[j].deadline;
    let missed = (0..jobs.len()).filter(|&j| late(j) > 0.0).count();
    let max_lateness = (0..jobs.len()).map(late).fold(f64::NEG_INFINITY, f64::max);
    let makespan = finish.iter().copied().fold(0.0, f64::max);
    let min_lead = (0..jobs.len()).map(|j| finish[j] - jobs[j].first_use as f64 / fps).fold(0.0, f64::max);
    Metrics { missed, max_lateness, makespan, min_lead }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(deadline: f64, est: f64, deps: Vec<usize>) -> Job {
        let work = Work::Orbit { frame: 0 };
        Job { name: String::new(), work, deps, first_use: 0, deadline, effective: 0.0, est, est_bytes: 0 }
    }

    fn lateness(jobs: &[Job], policy: Policy, workers: usize) -> f64 {
        let cost: Vec<f64> = jobs.iter().map(|j| j.est).collect();
        let finish = simulate(jobs, policy, workers, &cost, 0.0);
        (0..jobs.len()).map(|j| finish[j] - jobs[j].deadline).fold(f64::NEG_INFINITY, f64::max)
    }

    #[test]
    fn effective_deadline_leaves_room_for_dependents() {
        // An orbit due at 1.0 whose 0.5 s table is also due at 1.0 must be done by 0.5;
        // a grandchild tightens it further through its parent.
        let mut jobs = vec![job(1.0, 0.2, vec![]), job(1.0, 0.5, vec![0]), job(2.0, 0.1, vec![]), job(0.9, 0.3, vec![2])];
        effective(&mut jobs);
        assert_eq!(jobs[0].effective, 0.5);
        assert_eq!(jobs[1].effective, 1.0);
        assert!((jobs[2].effective - 0.6).abs() < 1e-12);
        // A dependent never starts before its parent finishes.
        let finish = simulate(&jobs, Policy::Slack, 2, &[0.2, 0.5, 0.1, 0.3], 0.0);
        assert!(finish[1] - 0.5 >= finish[0] - 1e-12 && finish[3] - 0.3 >= finish[2] - 1e-12);
    }

    #[test]
    fn least_slack_starts_the_long_job_first_on_two_workers() {
        // First-use runs the two short, earlier-due jobs first and the long one misses;
        // least slack starts the long one at once and every job is on time.
        let mut jobs = vec![job(1.0, 1.0, vec![]), job(0.9, 0.1, vec![]), job(0.95, 0.1, vec![])];
        effective(&mut jobs);
        assert!(lateness(&jobs, Policy::FirstUse, 2) > 0.09);
        assert!(lateness(&jobs, Policy::Slack, 2) <= 0.0);
    }

    #[test]
    fn on_one_worker_least_slack_can_lose_to_first_use() {
        // Known limit (SCHEDULE.md): on one worker earliest-deadline order minimises the
        // maximum lateness, and least slack may not. Kept as a test so a change is seen.
        let mut jobs = vec![job(5.0, 5.0, vec![]), job(0.6, 0.5, vec![])];
        effective(&mut jobs);
        assert!((lateness(&jobs, Policy::FirstUse, 1) - 0.5).abs() < 1e-12);
        assert!((lateness(&jobs, Policy::Slack, 1) - 4.9).abs() < 1e-12);
        assert!((lateness(&jobs, Policy::Edf, 1) - 0.5).abs() < 1e-12);
    }
}
