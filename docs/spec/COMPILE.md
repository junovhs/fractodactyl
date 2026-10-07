# Atlas v0 Compiler (VIDE-01)

`fd compile` compiles one known camera path into a bounded atlas (SPEC.md "Atlas
compiler", "Atlas v0 success criteria" 1-4). It plans each frame's logical tiles, builds
the reference orbits and BLA tables the path needs in deadline order, emits tile and
frame manifests that link every frame to exactly the chunks a warm render of it reads,
enforces the byte budget, and walks and verifies the result. Playback is VIDE-02 (`fd play`, PLAY.md).
Governed by DEC-01 (genuine location, no substitution), DEC-02 (compile known paths
offline, optimise warm), DEC-03 (2.5 GiB target, 3 GiB hard cap, enforced), DEC-04
(mathematical payloads only, never raster) and DEC-10 (every approximation carries
validity, error and fallback). Code: `crates/fd-cli/src/compile.rs`, on top of
`plan.rs` (PLAN.md) and `schedule.rs` (SCHEDULE.md).

```text
fd compile PATH --store DIR [--size WxH] [--ss N] [--iter N] [--columns C] [--kernel K]
           [--tile-px N] [--bla level|frame|group|none] [--bla-levels K]
           [--fps F] [--lead S] [--workers N] [--policy slack|edf|first-use] [--slab N]
           [--target BYTES] [--cap BYTES] [--on-miss fail|report]
fd path zoom --re X --im Y --from W0 --to W1 --seconds S --fps F [--rotation R]
```

`PATH` is a camera path (PLAN.md "Path file"). Render flags mean what they mean for
`fd render` (`--threads` is accepted and unused: the builders are single-threaded jobs
on `--workers` threads). Defaults: `--tile-px 128`, `--bla level`, `--bla-levels 1`,
`--fps 30`, `--lead 0`, `--workers 1`, `--policy slack`, `--slab 4096`,
`--on-miss report` (offline compile: playback starts after the compile, so a missed
just-in-time deadline is reported, not fatal; `fail` makes it exit 1 as `fd schedule`
does). `--target`/`--cap` (`N`, `NKiB`, `NMiB`, `NGiB`) persist the store's budget
first, as `fd chunk budget` does.

`fd path zoom` writes a constant-rate zoom: one exact centre, `round(S x F)` frames,
widths evenly spaced in log width from `W0` to `W1`, each written with 6 significant
digits (so a 1-ulp `powf` difference between platforms cannot change the file, and
every width round-trips through f64, which the frame manifest relies on). The first
comment line is the generating command.

## The v0 path

`bench/path-atlas-v0.txt`: 750 frames, 25 s at 30 fps, width 4 to 2e-49 (49.3 decades,
1.97 decades/s, constant rate), rotation 0, one exact centre:

```text
re -0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502
im  0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922
```

**What it is.** The nucleus of a period-764 minibrot (size ~4e-50 by the standard
`1 / (beta lambda^2)` estimate) about 1.7e-25 from the Misiurewicz point M(24,2) =
-0.743291890852430202931624325972510757... + 0.131240552308797604770845906581478143...i
in seahorse valley (multiplier |lambda| = 1.153, arg 0.473 rad). Found with mpmath:
Newton on `f^26(c) = f^24(c)` from the classic valley coordinate for M(24,2), then the
ball-period method at radius 2.5e-26 from M(24,2) + 1e-25 (period 764) and Newton on
`f^764(c) = 0` at 160 digits, confirmed at 320 digits (moved 4e-162). Written to 100
decimal places (9e-101 from the nucleus, far below the last frame's 2e-52 sample
spacing at 960 px). The zoom passes the whole set, the valley, then the M(24,2) spiral
(self-similar every factor 1.153, rotating) down to ~1e-24, leaves it along a spiral arm
and ends with the period-764 minibrot filling about half the frame. Genuine coordinates
throughout (DEC-01).

**Why this one.** A minibrot destination is the canonical deep-zoom target, and its
reference orbit never escapes, so the orbit has `--iter` + 1 points: the expensive case
for references and the BLA tables built over them (100001 points, 1.6 MB of orbit, up
to 5.6 MB per table). A Misiurewicz point itself was rejected: its repelling orbit
escapes from any finite-digit centre after ~3000 iterations, which would make every
chunk small and flatter the byte budget. 594 of 750 frames (79%) need the fx tier
(widths below ~2e-10, up to 300 bits); none needs the scaled tier. Per-frame cost grows
with depth (control at 960x540 on 12 threads: 0.02 s at frame 0, 0.41 s at width
~3e-13, 0.75 s at ~2e-26, 3.3 s at ~8e-49),
because escape counts near the minibrot grow in multiples of its period.

**What is favourable.** A fixed-centre zoom is the best case for orbit reuse: frames
are grouped by exact centre (ATLAS.md "Orbit reuse"), so one 300-bit orbit serves all
594 deep frames and one f64 orbit the 156 shallow ones. Panning paths need off-centre
references (REF-05) and will reuse far less. Zoom rate is fast (1.97 decades/s, not the
~0.15 decades/s a viewing path would use) to fit 49 decades into 25 s; at a slower rate
each BLA table and tile would be shared by proportionally more frames.

**Benchmark settings (BENC-01 runs both control and atlas on them):** `--size 960x540
--iter 100000`, `--ss 1`, default columns (`nu,de,normal`), `--kernel auto`. Estimated
`fd control` time for the whole path on 12 threads (Ryzen 3900X, shared machine):
every 10th frame (75 frames) took 61.7 s wall (0.81 s/frame mean), so the full path is
about 10-11 minutes (the last 10 frames, the most expensive, are 3-10 s each). At
640x360 the same sample took 28 s (about 5 minutes for the path). At 960x540, 2.8% of
those samples are interior and 0.01% unresolved at 1e5 iterations.

## Pipeline

1. **Plan.** `fd plan`'s footprint for every frame (PLAN.md): its tile level `L`, its
   level-`L` tiles, and the tile holding its centre (the anchor) with the centre's
   position in it. Every sample of every frame is located; one unpredicted sample fails
   the compile.
2. **Jobs.** As `fd schedule` (SCHEDULE.md "Jobs"): one orbit job per exact-centre group
   (f64-tier frames apart), and BLA table jobs over it shared as `--bla` says:
   - `level` (default): one table per run of `--bla-levels` tile levels of a group,
     built for the largest `dc_max` among its frames at those levels. Every tile of one
     level then names one table, so tile manifests are shared across frames.
   - `frame`: a table per frame (its own `dc_max`); `group`: one per group (largest
     `dc_max`); `none`.
   A table is valid for every frame whose `dc_max` is at most the table's (ATLAS.md "BLA
   tables"), so each sharing mode is correct; they trade bytes against block length.
   No certificate jobs: no certificate producer exists yet (CERT-01), so the atlas has
   none and certification time is 0 by construction.
3. **Estimate and budget check.** The scheduler's cost probes estimate every job's
   bytes; manifest bytes are computed from the plan (their layout is fixed). If the
   estimate exceeds the store's hard cap, the compile is refused before anything is
   built (exit 2, `atlas over budget: ... nothing built`, with the bytes by kind).
4. **Build.** The jobs run on `--workers` threads, least slack first (SCHEDULE.md), into
   the content-addressed store. The store refuses any put of a new chunk that would
   take it over its cap (ATLAS.md "Byte budget"); a refused put fails the job and the
   compile (exit 2). Nothing a frame needs is ever dropped to fit.
5. **Manifests.** Per frame, its math chunks are the orbit manifest, every slab it names
   and the frame's BLA table (if any). One tile manifest per (tile, orbit, table):
   evidence `heuristic` (no certified evidence exists yet), no children, refs = those
   chunks. One frame manifest per frame naming its tiles. Manifests are built after
   all math jobs.
6. **Verify.** `fd manifest walk` over all frame manifests (every chunk re-hashed,
   kinds and canonical form checked), then per frame: the math chunks reachable from its
   tiles are exactly its own; the orbit manifest's centre is the frame's exact centre;
   its precision is exactly 53 for an f64-tier frame and at least the frame's need
   otherwise; the BLA table names that orbit manifest, covers its points and its
   `dc_max` covers the frame's. Any failure is exit 2.

## What a player reads (VIDE-02)

A frame renders warm from its frame manifest alone, with no reference or operator
computed:

- **centre**: the orbit manifest's exact decimal centre (frames are grouped by exact
  centre, so this is the frame's own; the manifest's `anchor`/`offset` place it in the
  tile hierarchy to 2^-33 of a tile).
- **width**: `width x 2^(2 - L)` with `L` the anchor's level is the 53-bit width the
  kernel uses; written as the shortest f64 decimal it gives the same sample spacing
  as the path's width (`frames.camera_exact` counts frames for which the compiler
  checked this, all 750 of the v0 path). `rotation`, `size`, `ss`, `iter`, `columns`
  are stored as is.
- **orbit**: the slabs, joined (`fd render --orbit ID` takes the orbit manifest id);
  its precision is the group's deepest frame's, which every deep frame accepts
  (bit-identical to the cold render for the deepest frame, at least as accurate for
  the others).
- **operator**: the BLA table (`fd render --bla ID`), whose contract the compiler checked.

## Output

`name value` lines; seconds with 6 decimals. In order:

- Path: `frames`, `fps`, `path_seconds`, `log10_width.first`/`.last`, `decades`,
  `decades_per_second`, `frames.f64`/`.fx`/`.scaled` (tier per frame), `precision_bits_max`,
  `size`, `ss`, `iter`, `tile_px`, `workers`, `policy`, `lead_seconds`, `bla_mode`,
  `bla_levels`, `bla_skipped_frames` (scaled-tier frames, which get no table).
- One line per frame: `frame F level L tiles n need_bits B orbit JOB bla JOB|- manifest ID`.
- The scheduler's `job` and `compare` lines (SCHEDULE.md "Compile log").
- One line per orbit: `orbit JOB frames n points P precision B slabs s bytes X chunk ID`,
  and per table: `bla JOB frames n first_use F dc_max D levels l valid_blocks v bytes X chunk ID`.
- **Reuse**: `orbits`, `bla_tables`, `certificates` (0), `reuse.frames_per_orbit.mean`/`.max`,
  `reuse.frames_per_bla.mean`/`.max`, `reuse.frames_without_bla`, `tiles` (distinct
  tile keys), `tile_demand` (sum over frames of tiles per frame),
  `reuse.frames_per_tile.mean`/`.max`, `reuse.tiles_shared` (tiles read by two or more
  frames), `tile_manifests`, `tile_manifests.split` (tile keys with more than one
  manifest because frames at one level use different orbits or tables), `frame_manifests`,
  `frames.camera_exact`.
- **Bytes** of this atlas (every chunk some frame of the path reaches, plus the frame
  manifests): `atlas.chunks`, `atlas.bytes`, `atlas.kind.NAME CHUNKS BYTES` per kind,
  `atlas.class.operators|evidence|manifests|other` (ATLAS.md allocation classes),
  `atlas.bla_share` (BLA bytes over atlas bytes), `estimate.bytes` and
  `estimate.bytes.orbit|bla|manifests` (the pre-build estimate the budget check used).
- **Dedup**: `dedup.logical_chunks` and `dedup.logical_bytes` (per frame: its frame
  manifest, its tile manifests and its math chunks, summed over frames as if nothing
  were shared), `dedup.ratio.chunks` and `dedup.ratio.bytes` (logical over unique),
  `dedup.manifest_puts.stored`/`.deduplicated` (manifest puts this run that wrote bytes
  or found them stored), then the walk: `walk.frames`, `walk.tiles`, `walk.math_chunks`,
  `walk.math_bytes`, `walk.math_bytes_per_frame`, `walk.manifest_bytes` (ATLAS.md "Walk";
  `walk.math_bytes + walk.manifest_bytes = atlas.bytes`).
- **Budget**: `budget.target`, `budget.cap`, `budget.headroom` (cap minus atlas bytes),
  `budget.over_target`, `budget.within_cap`.
- **Schedule and time**: `jobs`, `met`, `missed`, `max_lateness_seconds`,
  `makespan_seconds`, `min_lead_seconds` (SCHEDULE.md; math chunks only, manifests
  are built afterwards), `seconds.plan`, `seconds.estimate` (probes), `seconds.build`
  (job execution), `seconds.reference` (sum of orbit compute time inside the jobs),
  `seconds.operator` (sum of BLA build time), `seconds.certification` (0: no producer),
  `seconds.manifests`, `seconds.verify`, `stored_bytes` (new bytes this run),
  `peak_rss_bytes` (process `VmHWM`, the compiler's peak RAM), `seconds.wall`, then the
  store's `atlas_bytes` and `over_target` (whole store, as every store command prints).

Exit codes: 0 on success; 1 with `--on-miss fail` when a chunk missed its first-use
deadline; 2 for usage errors, a path the plan cannot cover, a budget refusal (estimate
or put), a builder error, or a verification failure.

## Measured

Ryzen 3900X, shared machine, `fd compile bench/path-atlas-v0.txt --store DIR --size
960x540 --iter 100000 --workers 8` into an empty store, exit 0, 41.0 s wall:

| | |
|---|---|
| atlas | 651,851,528 bytes (0.607 GiB; 24% of the 2.5 GiB target), 22,582 chunks |
| by kind | BLA 165 tables 629.7 MB (96.6%); tile manifests 21,639, 18.4 MB; frame manifests 750, 2.2 MB; orbit slabs 26, 1.6 MB; orbit manifests 2 |
| estimate | 651,840,688 bytes (-0.002%) |
| orbits | 2: f64 (498 points, escapes) for 156 frames; 300-bit (100001 points) for 594 frames |
| reuse | 375 frames/orbit mean (594 max); 4.5 frames/table (5 max); 2.98 frames/tile (5 max), 16,354 of 21,639 tiles shared |
| dedup | 3.62 (chunks), 5.96 (bytes): 3.88 GB if every frame owned its chunks |
| time | reference 0.035 s, operators 1.30 s, certification 0, build makespan 0.91 s, plan 6.7 s, manifests 27.3 s, verify 5.8 s |
| schedule | 164 of 167 met with `--lead 0` (frame 0's chunks cannot be ready at t = 0; min lead 0.114 s) |
| peak RAM | 200 MB |

BLA tables dominate the bytes, and manifest writes (22,389 puts, each fsynced) dominate
the time. Other sharing modes on the same path and settings:

| `--bla` | tables | atlas bytes | BLA share | wall s |
|---|---|---|---|---|
| `level` (default, 1 level) | 165 | 651.9 MB | 96.6% | 41 |
| `level --bla-levels 8` | 22 | 99.6 MB | 77.7% | 35 |
| `group` | 2 | 22.2 MB | 0.0% | 40 |
| `frame` (at 640x360) | 750 | 2,905.1 MB (2.71 GiB) | 99.0% | 157 |

`frame` is over the 2.5 GiB target (reported, allowed) and 316 MB (10%) under the cap. `group` is
useless here: the deep group's largest `dc_max` (~1e-10) admits no block at the default
`eps`, so both tables are empty 96-byte chunks. One measurement suggests per-level
tables over-provision: at 640x360 the 2e-49 frame skipped 96.5% of its steps with its
own table and with a table built at 1e-40 (20.3x fewer iterations with one built at
1e-20). Whether that holds over the path is for VIDE-02/BENC-01 to measure before the
default changes. The reference orbit is cheap next to rendering on this path (a 300-bit,
1e5-point orbit is ~12 ms against 0.8 s per frame), so orbit reuse alone saves little;
the BLA tables are where the atlas has to pay off.

## Limits and follow-ups

- No certificates (CERT-01): evidence is `heuristic` only; certification time is 0.
- Tile manifests carry no children, fallback ancestors or envelope tiles: the atlas
  covers exactly the planned path (ARCHITECTURE.md envelope is not built).
- Manifests are written after all math jobs, so `min_lead_seconds` covers math chunks
  only; the atlas is complete at `seconds.wall`.
- An empty BLA table (no valid block, e.g. every f64-tier frame and the shallowest fx
  frames) is still referenced; rendering through it is slower than the plain kernel
  (measured 1.45x at 640x360), so `fd play` skips empty tables (PLAY.md).
- `--bla level` with one level per table is conservative; a coarser default needs
  per-frame skip measurements over the whole path.
- Fixed-centre paths only reuse orbits this well; panning paths need REF-05.
