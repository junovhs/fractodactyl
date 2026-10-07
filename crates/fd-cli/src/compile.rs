//! `fd compile`: the Atlas v0 compiler (VIDE-01, docs/spec/COMPILE.md). From a known
//! camera path it plans each frame's logical tiles (`fd plan`), derives and schedules the
//! orbit and BLA chunks the path needs (`fd schedule`'s core) and builds them into the
//! store, emits one tile manifest per (tile, chunk set) and one frame manifest per frame
//! linking the frame to exactly the chunks a warm render of it reads, enforces the byte
//! budget (refusing a path whose estimated atlas exceeds the hard cap before building
//! anything; the store refuses any put past the cap), then walks and verifies the DAG
//! and prints the compiler metrics of SPEC.md.
use crate::args::Args;
use crate::chunk::{bytes as byte_count, class};
use crate::orbit::same;
use crate::plan::{footprint, Footprint};
use crate::render::{params, FLAGS};
use crate::reuse::path_frames;
use crate::schedule::{
    derive, effective, estimate, execute, missed_exit, outcome, print_compare, print_jobs, print_outcome, store_probe, Done, Job,
    Policy, Share, Uses, Work,
};
use fd_atlas::{walk, BlaTable, Budget, Chunk, ChunkId, Evidence, FrameManifest, Kind, OrbitManifest, Put, Store, TileManifest};
use fd_fixed::{exp2i, limbs_for, Decimal};
use fd_kernel::{bla_dc_max, Params, Plane, Tier};
use fd_samples::View;
use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::Instant;

const USAGE: &str = "usage: fd compile PATH --store DIR [render flags without --re/--im/--width/--rotation] [--tile-px N]
                  [--bla level|frame|group|none] [--bla-levels K] [--fps F] [--lead S] [--workers N]
                  [--policy slack|edf|first-use] [--slab N] [--target BYTES] [--cap BYTES] [--on-miss fail|report]";

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let wall = Instant::now();
    let view_flags = ["re", "im", "width", "rotation", "o"];
    let extra =
        ["store", "tile-px", "bla", "bla-levels", "fps", "lead", "workers", "policy", "slab", "target", "cap", "on-miss"];
    let known: Vec<&str> = FLAGS.iter().copied().filter(|f| !view_flags.contains(f)).chain(extra).collect();
    let a = Args::parse(argv, &known)?;
    let [file] = a.positional.as_slice() else { return Err(USAGE.into()) };
    let p = params(&a)?;
    let tile_px: u32 = a.num("tile-px", 128)?;
    let span: u32 = a.num("bla-levels", 1)?;
    let slab: u32 = a.num("slab", 4096)?;
    let fps: f64 = a.num("fps", 30.0)?;
    let lead_s: f64 = a.num("lead", 0.0)?;
    let workers: usize = a.num("workers", 1)?;
    if !(fps > 0.0 && fps.is_finite() && lead_s >= 0.0 && lead_s.is_finite()) || workers == 0 || tile_px == 0 || span == 0 || slab == 0 {
        return Err("--fps, --workers, --tile-px, --bla-levels and --slab must be positive, --lead non-negative".into());
    }
    let policy = Policy::parse(a.str("policy").unwrap_or("slack"))?;
    let bla_mode = a.str("bla").unwrap_or("level");
    if !["level", "frame", "group", "none"].contains(&bla_mode) {
        return Err(format!("--bla: expected level, frame, group or none, got {bla_mode:?}"));
    }
    let fail = match a.str("on-miss").unwrap_or("report") {
        "fail" => true,
        "report" => false,
        x => return Err(format!("--on-miss: expected fail or report, got {x:?}")),
    };
    let dir = a.need("store")?;
    let mut store = crate::chunk::store(&a)?;
    if a.str("target").is_some() || a.str("cap").is_some() {
        let old = store.budget();
        let flag = |k: &str, default: u64| a.str(k).map_or(Ok(default), |v| byte_count(k, v));
        let new = Budget { target: flag("target", old.target)?, cap: flag("cap", old.cap)? };
        store.set_budget(new).map_err(|e| e.to_string())?;
    }
    let budget = store.budget();
    let before = store.stats().map_err(|e| e.to_string())?.bytes;
    let frames = path_frames(file, &p)?;

    // Plan: every frame's tiles, checked against every sample (PLAN.md).
    let t = Instant::now();
    let mut plan: Vec<Footprint> = Vec::with_capacity(frames.len());
    for (f, (v, _)) in frames.iter().enumerate() {
        let fp = footprint(v, p.nx, p.ny, tile_px * p.ss).map_err(|e| format!("frame {f}: {e}"))?;
        if fp.unpredicted > 0 {
            return Err(format!("frame {f}: {} samples fall in tiles the plan did not predict", fp.unpredicted));
        }
        plan.push(fp);
    }
    let plan_s = t.elapsed().as_secs_f64();
    let levels: Vec<u32> = plan.iter().map(|fp| fp.level).collect();
    let share = match bla_mode {
        "level" => Share::Level { levels: &levels, span },
        "frame" => Share::Frame,
        "group" => Share::Group,
        _ => Share::None,
    };

    // Jobs, cost estimates and the budget check, all before anything is built.
    let t0 = Instant::now();
    let (mut jobs, uses, skipped) = derive(&frames, &p, &share, fps, lead_s);
    let cost = store_probe(dir)?;
    estimate(&mut jobs, &frames, &p, cost, slab)?;
    let estimate_s = t0.elapsed().as_secs_f64();
    let est = Estimate::of(&jobs, &plan, &uses, slab);
    if est.total() > budget.cap {
        return Err(format!(
            "atlas over budget: this path's estimated atlas is {} bytes (orbits {}, BLA tables {}, manifests {}), over the hard cap {} \
             (target {}); nothing built. Share BLA tables more (--bla level with a larger --bla-levels, or --bla group), \
             lower --iter, or shorten the path",
            est.total(),
            est.orbit,
            est.bla,
            est.manifests,
            budget.cap,
            budget.target
        ));
    }
    effective(&mut jobs);
    let done = execute(&jobs, &frames, &p, &store, slab, workers, policy, t0)?;
    let build_s = t0.elapsed().as_secs_f64() - estimate_s;

    // Manifests: per frame, the chunks its warm render reads.
    let t = Instant::now();
    let mut atlas: BTreeMap<ChunkId, (Kind, u64)> = BTreeMap::new();
    let mut orbits: HashMap<usize, OrbitInfo> = HashMap::new();
    for (j, job) in jobs.iter().enumerate() {
        match job.work {
            Work::Orbit { .. } => {
                let info = orbit_info(&store, &done[j])?;
                for (id, b) in &info.slabs {
                    atlas.insert(*id, (Kind::ORBIT_SLAB, *b));
                }
                atlas.insert(done[j].chunk, (Kind::ORBIT_MANIFEST, info.manifest_bytes));
                orbits.insert(j, info);
            }
            Work::Bla { .. } => {
                atlas.insert(done[j].chunk, (Kind::BLA, done[j].bytes as u64));
            }
        }
    }
    let mut puts = (0u64, 0u64); // stored (or repaired), deduplicated
    let mut put = |c: &Chunk, atlas: &mut BTreeMap<ChunkId, (Kind, u64)>| -> Result<ChunkId, String> {
        let (id, outcome) = store.put(c).map_err(|e| format!("manifest: {e}"))?;
        if outcome == Put::Deduplicated {
            puts.1 += 1;
        } else {
            puts.0 += 1;
        }
        atlas.insert(id, (c.contract().kind, c.bytes().len() as u64));
        Ok(id)
    };
    let mut tile_ids: HashMap<(String, usize, Option<usize>), ChunkId> = HashMap::new();
    let mut frame_ids = Vec::with_capacity(frames.len());
    let mut camera_exact = 0;
    for (f, ((v, _), fp)) in frames.iter().zip(&plan).enumerate() {
        let refs = refs(&orbits[&uses[f].0], &done, uses[f]);
        let mut tiles = Vec::with_capacity(fp.tiles.len());
        for tile in &fp.tiles {
            let key = (tile.to_string(), uses[f].0, uses[f].1);
            let id = match tile_ids.get(&key) {
                Some(id) => *id,
                None => {
                    let m = TileManifest { tile: tile.clone(), evidence: Evidence::HEURISTIC, children: [None; 4], refs: refs.clone() };
                    let id = put(&m.to_chunk(), &mut atlas)?;
                    tile_ids.insert(key, id);
                    id
                }
            };
            tiles.push(id);
        }
        let width = anchor_width(v, fp.level)?;
        let m = FrameManifest {
            anchor: fp.anchor.clone(),
            offset: (fp.at.0 - 0.5, 0.5 - fp.at.1),
            width,
            rotation: v.rotation,
            size: (p.nx / p.ss, p.ny / p.ss),
            ss: p.ss,
            max_iter: p.max_iter,
            columns: p.columns.0,
            tiles,
        };
        camera_exact += usize::from(camera_round_trips(v, &m, &p)?);
        frame_ids.push(put(&m.to_chunk().map_err(|e| format!("frame {f}: {e}"))?, &mut atlas)?);
    }
    let manifest_s = t.elapsed().as_secs_f64();

    // Verify: walk the DAG (re-hashing every chunk), then check each frame reaches exactly
    // its chunks and that they hold for it (centre, precision, dc_max).
    let t = Instant::now();
    let w = walk(&store, &frame_ids).map_err(|e| format!("verify: {e}"))?;
    let tables = verify(&store, &frames, &p, &jobs, &done, &orbits, &uses, &frame_ids)?;
    let verify_s = t.elapsed().as_secs_f64();

    // Report.
    let n = frames.len();
    let tier = |v: &View| Plane::new(v, p.nx, p.ny).map(|pl| pl.tier);
    let mut tiers = [0usize; 3];
    for (v, _) in &frames {
        tiers[match tier(v)? {
            Tier::F64 => 0,
            Tier::Fixed => 1,
            Tier::Scaled => 2,
        }] += 1;
    }
    let log10_width = |v: &View| Decimal::parse(&v.width).map(|d| d.log2_abs() * std::f64::consts::LOG10_2);
    let (lw0, lw1) = (log10_width(&frames[0].0)?, log10_width(&frames[n - 1].0)?);
    println!("frames {n}\nfps {fps}\npath_seconds {:.6}", n as f64 / fps);
    println!("log10_width.first {lw0:.6}\nlog10_width.last {lw1:.6}\ndecades {:.6}", (lw0 - lw1).abs());
    println!("decades_per_second {:.6}", (lw0 - lw1).abs() * fps / (n.max(2) - 1) as f64);
    println!("frames.f64 {}\nframes.fx {}\nframes.scaled {}", tiers[0], tiers[1], tiers[2]);
    println!("precision_bits_max {}", frames.iter().map(|f| f.1).max().unwrap_or(0));
    println!("size {}x{}\nss {}\niter {}\ntile_px {tile_px}", p.nx / p.ss, p.ny / p.ss, p.ss, p.max_iter);
    println!("workers {workers}\npolicy {}\nlead_seconds {lead_s}\nbla_mode {bla_mode}\nbla_levels {span}", policy.name());
    println!("bla_skipped_frames {skipped}");
    for (f, fp) in plan.iter().enumerate() {
        let (o, b) = uses[f];
        println!(
            "frame {f} level {} tiles {} need_bits {} orbit {} bla {} manifest {}",
            fp.level,
            fp.tiles.len(),
            frames[f].1,
            jobs[o].name,
            b.map_or("-", |b| jobs[b].name.as_str()),
            frame_ids[f]
        );
    }
    print_jobs(&jobs, &done);
    print_compare(&jobs, &done, workers, estimate_s, fps);

    // Reuse.
    let users = |j: usize| uses.iter().filter(|u| u.0 == j || u.1 == Some(j)).count();
    for (j, job) in jobs.iter().enumerate() {
        match job.work {
            Work::Orbit { .. } => {
                let o = &orbits[&j];
                println!(
                    "orbit {} frames {} points {} precision {} slabs {} bytes {} chunk {}",
                    job.name,
                    users(j),
                    done[j].points,
                    o.bits,
                    o.slabs.len(),
                    o.manifest_bytes + o.slabs.iter().map(|s| s.1).sum::<u64>(),
                    done[j].chunk
                );
            }
            Work::Bla { dc_max, .. } => {
                let (levels, valid) = tables[&j];
                println!(
                    "bla {} frames {} first_use {} dc_max {dc_max:e} levels {levels} valid_blocks {valid} bytes {} chunk {}",
                    job.name,
                    users(j),
                    job.first_use,
                    done[j].bytes,
                    done[j].chunk
                );
            }
        }
    }
    let orbit_jobs: Vec<usize> = (0..jobs.len()).filter(|&j| matches!(jobs[j].work, Work::Orbit { .. })).collect();
    let bla_jobs: Vec<usize> = (0..jobs.len()).filter(|&j| matches!(jobs[j].work, Work::Bla { .. })).collect();
    let mean_max = |v: &[usize]| {
        let m = if v.is_empty() { 0.0 } else { v.iter().sum::<usize>() as f64 / v.len() as f64 };
        (m, v.iter().copied().max().unwrap_or(0))
    };
    let per_orbit: Vec<usize> = orbit_jobs.iter().map(|&j| users(j)).collect();
    let per_bla: Vec<usize> = bla_jobs.iter().map(|&j| users(j)).collect();
    let mut per_tile: BTreeMap<String, usize> = BTreeMap::new();
    let mut keys_manifests: BTreeMap<&str, usize> = BTreeMap::new();
    for fp in &plan {
        for t in &fp.tiles {
            *per_tile.entry(t.to_string()).or_default() += 1;
        }
    }
    for k in tile_ids.keys() {
        *keys_manifests.entry(k.0.as_str()).or_default() += 1;
    }
    let demand: usize = per_tile.values().sum();
    let tile_uses: Vec<usize> = per_tile.values().copied().collect();
    println!("orbits {}\nbla_tables {}\ncertificates 0", orbit_jobs.len(), bla_jobs.len());
    let (m, x) = mean_max(&per_orbit);
    println!("reuse.frames_per_orbit.mean {m:.3}\nreuse.frames_per_orbit.max {x}");
    let (m, x) = mean_max(&per_bla);
    println!("reuse.frames_per_bla.mean {m:.3}\nreuse.frames_per_bla.max {x}");
    println!("reuse.frames_without_bla {}", uses.iter().filter(|u| u.1.is_none()).count());
    println!("tiles {}\ntile_demand {demand}", per_tile.len());
    let (m, x) = mean_max(&tile_uses);
    println!("reuse.frames_per_tile.mean {m:.3}\nreuse.frames_per_tile.max {x}");
    println!("reuse.tiles_shared {}", tile_uses.iter().filter(|&&u| u > 1).count());
    println!("tile_manifests {}\ntile_manifests.split {}", tile_ids.len(), keys_manifests.values().filter(|&&c| c > 1).count());
    println!("frame_manifests {}\nframes.camera_exact {camera_exact}", BTreeSet::from_iter(&frame_ids).len());

    // Bytes and dedup over this atlas: every chunk some frame of the path reaches.
    let atlas_bytes: u64 = atlas.values().map(|v| v.1).sum();
    let mut kinds: BTreeMap<u16, (u64, u64)> = BTreeMap::new();
    let mut classes = [("operators", 0u64), ("evidence", 0), ("manifests", 0), ("other", 0)];
    for (kind, b) in atlas.values() {
        let e = kinds.entry(kind.0).or_default();
        e.0 += 1;
        e.1 += b;
        classes[class(*kind)].1 += b;
    }
    println!("atlas.chunks {}\natlas.bytes {atlas_bytes}", atlas.len());
    for (k, (c, b)) in &kinds {
        println!("atlas.kind.{} {c} {b}", Kind(*k));
    }
    for (name, b) in classes {
        println!("atlas.class.{name} {b}");
    }
    let share_of = |b: u64| if atlas_bytes == 0 { 0.0 } else { b as f64 / atlas_bytes as f64 };
    println!("atlas.bla_share {:.6}", share_of(kinds.get(&Kind::BLA.0).map_or(0, |k| k.1)));
    println!("estimate.bytes {}\nestimate.bytes.orbit {}\nestimate.bytes.bla {}\nestimate.bytes.manifests {}", est.total(), est.orbit, est.bla, est.manifests);
    // Logical references: per frame, its frame manifest, its tile manifests and the math
    // chunks they name, as if no frame shared anything with another.
    let tiles_bytes: HashMap<ChunkId, u64> = tile_ids.values().map(|id| (*id, atlas[id].1)).collect();
    let (mut refs_n, mut refs_b) = (0u64, 0u64);
    for (f, fid) in frame_ids.iter().enumerate() {
        let math = refs(&orbits[&uses[f].0], &done, uses[f]);
        let tile_set: BTreeSet<&ChunkId> = plan[f].tiles.iter().map(|t| &tile_ids[&(t.to_string(), uses[f].0, uses[f].1)]).collect();
        refs_n += 1 + tile_set.len() as u64 + math.len() as u64;
        refs_b += atlas[fid].1 + tile_set.iter().map(|t| tiles_bytes[*t]).sum::<u64>() + math.iter().map(|(_, id)| atlas[id].1).sum::<u64>();
    }
    println!("dedup.logical_chunks {refs_n}\ndedup.logical_bytes {refs_b}");
    println!("dedup.ratio.chunks {:.3}\ndedup.ratio.bytes {:.3}", refs_n as f64 / atlas.len() as f64, refs_b as f64 / atlas_bytes as f64);
    println!("dedup.manifest_puts.stored {}\ndedup.manifest_puts.deduplicated {}", puts.0, puts.1);
    println!("walk.frames {}\nwalk.tiles {}\nwalk.math_chunks {}\nwalk.math_bytes {}", w.frames, w.tiles, w.math_chunks, w.math_bytes);
    println!("walk.math_bytes_per_frame {}\nwalk.manifest_bytes {}", w.math_bytes_per_frame, w.manifest_bytes);
    println!("budget.target {}\nbudget.cap {}\nbudget.headroom {}", budget.target, budget.cap, budget.cap.saturating_sub(atlas_bytes));
    println!("budget.over_target {}\nbudget.within_cap {}", u8::from(atlas_bytes > budget.target), u8::from(atlas_bytes <= budget.cap));

    // Time and memory.
    let compute = |k: &dyn Fn(&Work) -> bool| jobs.iter().zip(&done).filter(|(j, _)| k(&j.work)).map(|(_, d)| d.compute).sum::<f64>();
    let reference_s = compute(&|w| matches!(w, Work::Orbit { .. }));
    let operator_s = compute(&|w| matches!(w, Work::Bla { .. }));
    let m = outcome(&jobs, &done, fps);
    println!("jobs {}", jobs.len());
    print_outcome(&jobs, &m);
    println!("seconds.plan {plan_s:.6}\nseconds.estimate {estimate_s:.6}\nseconds.build {build_s:.6}");
    println!("seconds.reference {reference_s:.6}\nseconds.operator {operator_s:.6}\nseconds.certification 0");
    println!("seconds.manifests {manifest_s:.6}\nseconds.verify {verify_s:.6}");
    let after = store.stats().map_err(|e| e.to_string())?.bytes;
    println!("stored_bytes {}", after.saturating_sub(before));
    println!("peak_rss_bytes {}", crate::bench::peak_rss().map_or("null".into(), |b| b.to_string()));
    println!("seconds.wall {:.6}", wall.elapsed().as_secs_f64());
    crate::chunk::report_target(&store)?;
    if fail && m.missed > 0 {
        missed_exit("chunks", m.missed, jobs.len(), lead_s, m.min_lead);
    }
    Ok(())
}

/// A stored orbit: its slabs with their bytes, its manifest's bytes and precision.
struct OrbitInfo {
    slabs: Vec<(ChunkId, u64)>,
    manifest_bytes: u64,
    bits: u32,
}

fn orbit_info(store: &Store, d: &Done) -> Result<OrbitInfo, String> {
    let chunk = store.get(&d.chunk).map_err(|e| e.to_string())?;
    let m = OrbitManifest::from_chunk(&chunk).map_err(|e| e.to_string())?;
    let slabs = m.slabs.iter().map(|id| Ok((*id, store.get(id).map_err(|e| e.to_string())?.bytes().len() as u64))).collect::<Result<_, String>>()?;
    Ok(OrbitInfo { slabs, manifest_bytes: chunk.bytes().len() as u64, bits: m.precision_bits })
}

/// The math chunks a frame's warm render reads: its orbit manifest and slabs, and its
/// BLA table if it has one. Sorted, as a tile manifest stores them.
fn refs(orbit: &OrbitInfo, done: &[Done], (o, b): Uses) -> Vec<(Kind, ChunkId)> {
    let mut r: Vec<(Kind, ChunkId)> = orbit.slabs.iter().map(|s| (Kind::ORBIT_SLAB, s.0)).collect();
    r.push((Kind::ORBIT_MANIFEST, done[o].chunk));
    if let Some(b) = b {
        r.push((Kind::BLA, done[b].chunk));
    }
    r.sort();
    r
}

/// The view width in anchor sides: the 53-bit width the kernel uses (`Plane::new`)
/// times `2^(L-2)`, exact.
fn anchor_width(v: &View, level: u32) -> Result<f64, String> {
    let d = Decimal::parse(&v.width)?;
    let lw = d.log2_abs();
    let (wm, we) = d.to_fixed(limbs_for((64.0 - lw).max(64.0).ceil() as u64))?.frexp().ok_or("view width underflowed")?;
    Ok(wm * exp2i(we + i64::from(level) - 2))
}

/// Whether the frame manifest's camera gives back this frame's exact sample grid: its
/// width, scaled back to plane units and written as the shortest f64 decimal, must give
/// the same sample spacing, and rotation and size must match.
fn camera_round_trips(v: &View, m: &FrameManifest, p: &Params) -> Result<bool, String> {
    let w = m.width * exp2i(2 - i64::from(m.anchor.level));
    if !w.is_normal() {
        return Ok(false);
    }
    let back = View { width: format!("{w:e}"), ..v.clone() };
    let (a, b) = (Plane::new(v, p.nx, p.ny)?, Plane::new(&back, p.nx, p.ny)?);
    Ok(a.h_m == b.h_m && a.h_e == b.h_e && m.rotation == v.rotation && m.size.0 * m.ss == p.nx && m.size.1 * m.ss == p.ny)
}

/// Check every frame manifest against the frame it was compiled for: its tiles name
/// exactly the frame's math chunks; the orbit is for the frame's exact centre at a
/// precision the frame's tier accepts; the BLA table is built over that orbit, covers
/// its points and holds for the frame's `|dc|`. Returns each table's (levels, valid blocks).
#[allow(clippy::too_many_arguments)]
fn verify(
    store: &Store,
    frames: &[(View, u32)],
    p: &Params,
    jobs: &[Job],
    done: &[Done],
    orbits: &HashMap<usize, OrbitInfo>,
    uses: &[Uses],
    frame_ids: &[ChunkId],
) -> Result<HashMap<usize, (usize, usize)>, String> {
    let err = |f: usize, m: String| format!("verify: frame {f}: {m}");
    let mut tile_refs: HashMap<ChunkId, Vec<(Kind, ChunkId)>> = HashMap::new();
    // Per table: (orbit, points, dc_max) and (levels, valid blocks); the blocks are dropped.
    let mut tables: HashMap<usize, TableInfo> = HashMap::new();
    let mut manifests: HashMap<usize, OrbitManifest> = HashMap::new();
    for (f, fid) in frame_ids.iter().enumerate() {
        let fm = FrameManifest::from_chunk(&store.get(fid).map_err(|e| err(f, e.to_string()))?).map_err(|e| err(f, e.to_string()))?;
        let mut reach = BTreeSet::new();
        for t in &fm.tiles {
            if let Entry::Vacant(e) = tile_refs.entry(*t) {
                let tm = TileManifest::from_chunk(&store.get(t).map_err(|e| err(f, e.to_string()))?).map_err(|e| err(f, e.to_string()))?;
                e.insert(tm.refs);
            }
            reach.extend(tile_refs[t].iter().copied());
        }
        let want: BTreeSet<(Kind, ChunkId)> = refs(&orbits[&uses[f].0], done, uses[f]).into_iter().collect();
        if reach != want {
            return Err(err(f, format!("reaches {} math chunks, needs exactly {}", reach.len(), want.len())));
        }
        let (v, need) = &frames[f];
        let o = uses[f].0;
        if let Entry::Vacant(e) = manifests.entry(o) {
            e.insert(OrbitManifest::from_chunk(&store.get(&done[o].chunk).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?);
        }
        let om = &manifests[&o];
        if !same(&om.center_re, &v.center_re)? || !same(&om.center_im, &v.center_im)? {
            return Err(err(f, "orbit is for another centre".into()));
        }
        let ok_bits = if *need == 53 { om.precision_bits == 53 } else { om.precision_bits >= *need && om.precision_bits != 53 };
        if !ok_bits {
            return Err(err(f, format!("needs {need}-bit reference, orbit has {}", om.precision_bits)));
        }
        if let Some(b) = uses[f].1 {
            if let Entry::Vacant(e) = tables.entry(b) {
                let t = BlaTable::from_chunk(&store.get(&done[b].chunk).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
                let valid = t.levels.iter().map(|l| l.iter().filter(|blk| blk[4] > 0.0).count()).sum();
                e.insert(((t.orbit, t.points, t.dc_max), (t.levels.len(), valid)));
            }
            let (orbit, points, dc_max) = tables[&b].0;
            let dc = bla_dc_max(v, p)?;
            if orbit != done[o].chunk || points != done[o].points as u64 || dc.is_nan() || dc > dc_max {
                return Err(err(f, format!("BLA table {} does not hold (orbit, points or dc_max {dc:e} > {dc_max:e})", jobs[b].name)));
            }
        }
    }
    Ok(tables.into_iter().map(|(k, v)| (k, v.1)).collect())
}

/// A BLA table's (orbit manifest, points, dc_max) and (levels, valid blocks).
type TableInfo = ((ChunkId, u64, f64), (usize, usize));

/// Atlas bytes predicted before building: the jobs' estimates plus the manifests, sized
/// from the plan (their layout is fixed, ATLAS.md "Manifests").
struct Estimate {
    orbit: u64,
    bla: u64,
    manifests: u64,
}

impl Estimate {
    fn of(jobs: &[Job], plan: &[Footprint], uses: &[Uses], slab: u32) -> Estimate {
        let sum = |k: &dyn Fn(&Work) -> bool| jobs.iter().filter(|j| k(&j.work)).map(|j| j.est_bytes).sum::<u64>();
        let pad = |n: u64| 32 + n.div_ceil(8) * 8;
        let slabs = |o: usize| (jobs[o].est_bytes / 16).div_ceil(u64::from(slab));
        let mut tiles: BTreeSet<(String, usize, Option<usize>)> = BTreeSet::new();
        let mut manifests = 0;
        for (f, fp) in plan.iter().enumerate() {
            let refs = slabs(uses[f].0) + 1 + u64::from(uses[f].1.is_some());
            for t in &fp.tiles {
                let key = t.to_string();
                let len = key.len() as u64;
                if tiles.insert((key, uses[f].0, uses[f].1)) {
                    manifests += pad(8 + len + 2 + 8 + 34 * refs);
                }
            }
            manifests += pad(8 + fp.anchor.to_string().len() as u64 + 32 + 12 + 8 + 4 + 8 + 32 * fp.tiles.len() as u64);
        }
        Estimate { orbit: sum(&|w| matches!(w, Work::Orbit { .. })), bla: sum(&|w| matches!(w, Work::Bla { .. })), manifests }
    }

    fn total(&self) -> u64 {
        self.orbit + self.bla + self.manifests
    }
}
