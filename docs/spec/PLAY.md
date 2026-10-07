# Atlas v0 Player (VIDE-02)

`fd play` renders a compiled path (COMPILE.md) from the atlas: SPEC.md "Atlas
renderer/player". It reads each frame's frame manifest, its tile manifests, the orbit
(orbit manifest and slabs) and the BLA table they name, and renders the frame with
that stored orbit and that stored table. It computes no reference orbit and builds no
operator (DEC-02); every acceleration it applies keeps the contract it was compiled
with (DEC-10); colour is a late pass over the samples (DEC-07). Code:
`crates/fd-cli/src/play.rs`.

```text
fd play COMPILE_LOG --store DIR [--frames A..B] [--threads N] [-o DIR] [--look L[,L...]]
        [--mp4 FILE] [--oracle tools/oracle.py [--every N] [--k K] [--python P]]
```

`COMPILE_LOG` is the stdout of the `fd compile` that built the store. The player reads
two things from it: the frame manifest ids in path order (`frame F ... manifest ID`
lines) and `fps`. The store has no path index, so the log is the compile record that
says which frame manifest is frame `F`; nothing else in it is used.
`--frames A..B` plays frames `A` to `B - 1` (`A..` to the end). `--threads` is the
render thread count. `-o DIR` writes `DIR/frame-NNNNN.fds` (the `fd control -o`
layout); `--look` also writes `DIR/frame-NNNNN.LOOK.png` per look, shaded from the
in-memory samples by `fd-shade` (the same passes as `fd shade`); `--mp4 FILE` muxes the
first look's PNGs at the logged `fps` with `ffmpeg` (H.264, yuv420p, CRF 16) if `ffmpeg`
is on PATH, and otherwise reports `"skipped":"ffmpeg not on PATH"` and carries on: the
PNG sequence is the video, the mp4 is a convenience. `--oracle` runs the oracle on
every `--every`-th played frame (default every frame; needs `-o`).

## What a frame reads

1. The frame manifest (always read; one per frame).
2. Its tile manifests. Together their refs must name exactly one orbit manifest, the
   slabs that manifest lists, and at most one BLA table built over that orbit; anything
   else is an error.
3. **Camera**, from the manifests alone: the exact decimal centre is the orbit
   manifest's; width is `width x 2^(2 - L)` (`L` the anchor level), written as the
   shortest f64 decimal (the compiler checked this reproduces the path's sample grid,
   `frames.camera_exact`); rotation, size, ss, iteration limit and columns are the frame
   manifest's. The anchor and offset must place that centre (re-located exactly,
   within 1e-9 of a tile side) or the frame fails.
4. **Not in the manifests**, stated in the totals record under `assumed` rather than
   derived: the kernel choice (`auto`, which is what `fd compile` defaults to; a forced
   tier at compile time would show up as an orbit-precision refusal, not a silent
   change) and the escape radius (`1e10`, the only one `fd render` uses).
5. **BLA**: an empty table (no valid block; every f64-tier frame and the shallowest fx
   frames of the v0 path) is skipped and the frame renders with the stored orbit alone
   (COMPILE.md: rendering through an empty table costs 1.45x). Recorded per frame as
   `bla_use` `used`, `skipped_empty` or `none`.

Without an applied table a played frame is `fd render --store DIR --orbit ID` of that
view, byte for byte. With one it is `fd render --bla ID`.

**Reuse.** Decoded tile manifests, orbits and tables stay loaded while the next frame
names them; after each frame everything that frame did not use is dropped, so the
working set is one frame's chunks. A frame is `cold` when it read its orbit or its BLA
table from the store, `warm` when both were already loaded. Reads go through the OS
page cache: a store compiled just before playback is in RAM, so `load_seconds` is
decode and SHA-256 verification, not disk.

## Output

JSON lines, schema `fd-play/1`: one `"record":"frame"` line per frame, then one
`"record":"totals"` line. Exit status as `fd control`: 2 for a usage error before the
first frame; 1 when a frame fails the oracle or a runtime error stops the path (the
totals line then covers the frames done and names the error in `error`); 0 otherwise.

**Frame record.** `frame`, `manifest` (frame manifest id), then the fd-control/1 /
fd-bench/1 fields with the same names (`view`, `grid`, `kernel`, `threads`, `depth`,
`cache`, `runs`, `timing`, `memory`, `iterations`, `bytes`, `atlas_work`, `fallback`,
`classes`), so BENC-01 can diff a played frame against its control frame key by key.
Differences:

| Field | Player meaning |
|---|---|
| `cache` | `{"atlas":"store","cross_frame_reuse":true}` |
| `runs[0]` | one run; `state` `cold` or `warm` (above); `seconds` = load + render; `reference_seconds` 0 (the orbit is handed to the kernel, never computed); `load_seconds` |
| `timing` | `cold_seconds` or `warm_seconds` set (the other `null`); `warm_statistic` `"frame"` when warm |
| `bytes.atlas_bytes_read` | chunk bytes read from the store for this frame (cache misses) |
| `bytes.atlas_bytes_referenced` | chunk bytes the frame's manifests reach (frame + tile manifests, orbit, table), read now or reused |
| `atlas_work.tiles_touched` | tile manifests the frame names |
| `atlas_work.microblocks_touched` | 0: the v0 atlas has no microblocks |
| `atlas_work.macro_operators_per_pixel` | BLA blocks applied per output pixel |
| `fallback.pixel_fraction` | samples that applied no block (closed-form interior included); 1 without a table |
| `fallback.iterations_per_pixel` | plain perturbation steps per pixel (`iterations.total - blocks`) |
| `iterations.total` | plain steps plus blocks (what the kernel executed) |

Then `play`: `render_seconds`, `load_seconds`, `reference_seconds` 0, `operator_seconds`
0, `shade_seconds`, `orbit` (id), `orbit_precision_bits`, `need_bits`, `orbit_points`,
`bla_table` (id or null), `bla_use`, `bla_blocks`, `bla_skipped_steps`,
`iterations_equivalent` (steps a plain render of the same samples takes),
`fallback_samples`, `closed_form_samples`, `shift_px_max` (ATLAS.md first-order BLA
shift; null without a table); then `fds`, `oracle`, `oracle_seconds`, `ok`.

**Totals record.** `compile_log`, `frames`, `range`, `threads`, `fps`, `fps_source`,
`assumed`, `cache`; `timing` (`cold_seconds`, `cold_frames`, `cold_seconds_per_frame`,
the same for warm, `seconds`, `seconds_per_frame`, `render_seconds`, `wall_seconds`
including writing, shading, muxing and oracle runs); `load_seconds` (`total`,
`per_frame`, `cold_per_frame`, `warm_per_frame`); `reference_seconds` and
`operator_seconds` 0; `iterations` (`total`, `per_pixel`, `per_sample`, `equivalent`,
`reference_length_max`); `bytes` (`fds_bytes`, `atlas_bytes_read` and `_per_frame`,
`atlas_bytes_referenced` and `_per_frame`); `atlas_work` (`tiles_touched`,
`tiles_touched_per_frame`, `microblocks_touched` 0, `macro_operators_per_pixel`);
`fallback`; `bla` (`frames_used`, `frames_empty_table_skipped`,
`frames_without_table`, `blocks`, `skipped_steps`, `shift_px_max`); `memory`
(`peak_rss_bytes`, the largest per-frame peak with the high-water mark reset before each
frame as in fd-bench/1, so it includes the loaded chunks; `sample_bytes`, `device`
`cpu`, `peak_vram_bytes` 0); `classes`; `depth`; `outputs` (`pngs`, `mp4`);
`oracle_frames`, `oracle_failures`, `error`, `ok`.

## Measured

Ryzen 3900X, shared machine. `fd compile bench/path-atlas-v0.txt --store S --size
960x540 --iter 100000 --workers 8 > S.log` (37.6 s, 651.9 MB), then `fd play S.log
--store S --threads 12 -o OUT --look umber --mp4 OUT/atlas-v0.umber.mp4 --oracle
tools/oracle.py --every 75`: all 750 frames, 9 min 0 s wall (including writing 5.8 GB of
`.fds`, 750 PNGs, a 24.6 MB mp4 muxed in 4.0 s, and 10 oracle runs).

| | |
|---|---|
| seconds/frame | 0.618 mean (load + render); cold 0.628 (165 frames), warm 0.615 (585) |
| load | 4.7 ms/frame mean; cold 20.8 ms, warm 0.19 ms; reference 0, operator 0 |
| bytes read | 651.9 MB total (every atlas chunk exactly once), 0.87 MB/frame; 5.18 MB/frame referenced |
| tiles | 86.1 touched per frame; microblocks 0 (none exist) |
| BLA | used on 520 frames, empty table skipped on 230; 8.35 blocks/pixel, 2.74e11 steps skipped, iterations 1.99e11 executed against 4.70e11 equivalent (2.36x) |
| fallback | pixel fraction 0.323, 503.7 plain steps/pixel; `shift_px` max 9.1e-8 |
| peak RAM | 41 MB per frame (high-water mark reset per frame), 329 MB process (`time -v`) |

The deepest frame (2e-49) plays in 1.03 s against 9.6 s for `fd render`; at the depth
where tables are empty, a frame costs what `fd render` does (frames 0-229 play
byte-identical to `fd render` with the same orbit). Load time is negligible next to
render time, so cold and warm frames cost the same: the reuse that pays is the BLA
table itself, not avoiding the reads. The comparison with `fd control` is BENC-01's.

**Correctness.** Oracle on every 75th frame: 8 of 10 pass. Frame 150 (f64 tier, no
table) fails on `de`/`normal` exactly as `fd render` of the view does (byte-identical
files): an existing f64-tier issue, not the player's. Frame 375 has one class mismatch
(a sample the oracle sees escape is marked interior). Classes against `fd render` on
frames 0, 75, ..., 675, 749: identical on 9 of 11; frame 375 has 392 and frame 749 has
30 samples that escape in the plain render (and, for the 4 checked, in the oracle) but
are interior under BLA. `fd render --bla` with the same table gives the same bytes, so
this is the BLA kernel's periodicity check (ACC-01), not the player.

*Fixed by FIX-02* (the numbers above predate it). Two causes, both in the kernel's
periodicity judgement. (1) BLA blocks landed past Brent save points and saved there, so
BLA compared against other iterates than plain (frame 749 sample (324, 10): saved at
4097, met a near-return at 6475; plain saves at 4096 and escapes at 7412, as mpmath
does). Blocks now end at the save points. (2) The near-exact-return test (within 1e-13)
itself was unsound next to the path's minibrot: every interior sample the plain render
found on frames 375 (1159) and 749 (57) escapes in mpmath (frame 375 sample
(287, 212): back within 9e-14 of iterate 256 at 378, escapes at 788). A near-return now
counts only if the orbit contracts over it. After both, frames 0, 75, ..., 675, 749
play with classes identical to `fd render` (0 mismatches); frames 375 and 749 have no
interior samples left, and frame 0's (86730, 43 spot-checked) stay bounded in mpmath; BLA executes 3-8% more iterations on the BLA frames (frame 749: 4.29e8 to
4.65e8, still 20x fewer than the 9.31e9 equivalent) at wall-time parity within noise.

## Limits and follow-ups

- The frame order comes from the compile log; a path index chunk in the store would
  make the store self-describing.
- Kernel tier and escape radius are not in the frame manifest (stated as `assumed`).
- CPU only; `device` and `peak_vram_bytes` are constants until a GPU kernel exists.
- Loads go through the page cache; a cold-disk measurement needs the cache dropped.
- Frames are played one after another; loading the next frame's chunks while the
  current one renders is not done (loads are milliseconds against renders of 0.2-1 s).
