# Atlas v0 vs independent frames (BENC-01, Gate C)

SPEC.md's first decisive question:

> Can a bounded path-specific mathematical atlas make later genuine deep-zoom frames
> reuse enough prior computation to become materially cheaper than rendering each frame
> independently?

**Answer, for Atlas v0 on the v0 path: no, not against a competent independent
renderer.** The atlas plays the path 1.35x faster than plain independent frames, but
that whole gain comes from BLA. An independent renderer that builds its own BLA table
in every frame gets the same gain. Against that renderer the atlas is 0.996x, a tie
within noise, so it never breaks even. The atlas reuses chunks across frames as
designed (an orbit serves 375 frames, a table 4.5), but rebuilding those chunks costs
about 18 ms per frame, 2.9% of a 0.64 s frame. Of the atlas player's time, 99.3% goes
to per-pixel iteration, and the v0 atlas caches nothing of that. Full results are in
`bench/benc-01-results.json` (schema `fd-benc01/1`).

## Arms

All three arms run on the identical path and settings: `bench/path-atlas-v0.txt` (750
frames, 25 s at 30 fps, width 4 down to 2e-49, one exact centre), `--size 960x540
--iter 100000`, `ss 1`, columns `nu,de,normal`, `--kernel auto`, 12 render threads. The
arms ran one after another, never at the same time.

- **A, independent plain:** `fd control`. Each frame computes its own reference and
  renders with plain perturbation, with no BLA.
- **B, independent with per-frame BLA:** `fd control --bla per-frame` (new, see
  SAMPLES.md "Independent-frame control"). Inside each frame's clock it computes the
  frame's own reference, then builds its own BLA table with `Bla::build`. The table
  uses `eps = 2^-50` (the atlas's eps) and the frame's own `dc_max`, which is the
  `fd compile --bla frame` contract. It then renders with `render_bla`. A table with no
  valid block is skipped, as `fd play` skips one (230 frames, all of them byte-identical
  to A). Nothing carries over from one frame to the next. This arm is the fair control:
  it separates "BLA beats plain iteration" from "reuse beats recomputation".
- **C, atlas:** `fd compile` (offline, timed), then one `fd play` of the whole path. The
  store was compiled just before the play, so its chunks sit in the OS page cache.

Arm B's table for each frame is built for that frame's own `|dc|`, so it is at least as
tight as the atlas's per-level table, which is built for the largest `|dc|` at that
level. Both arms executed the same work: 2.0838e11 iterations in B and 2.0839e11 in C,
with the same 520 frames using a table.

## Results

Machine: Ryzen 9 3900X (12 cores, 24 threads), shared, but nothing else was running
during the runs. The 1-minute load averaged 11.1 (min 5.0, max 14.4, 121 samples), which
is the benchmark's own 12 threads plus the oracle's single Python thread between frames.

| | A independent plain | B independent + own BLA | C atlas play |
|---|---|---|---|
| frame seconds, total | 647.4 | 478.1 | 480.0 |
| seconds/frame, cold | 0.863 (every frame) | 0.638 (every frame) | 0.651 (165 frames) |
| seconds/frame, warm | n/a (no cross-frame state) | n/a | 0.637 (585 frames) |
| of which reference / operator build / load | 12.6 ms / 0 / 0 | 11.7 ms / 6.6 ms / 0 | 0 / 0 / 4.7 ms (20.7 cold, 0.2 warm) |
| process wall (`time -v`, incl. writing 5.8 GB `.fds` and oracle runs) | 695.0 s | 526.3 s | 524.2 s |
| iterations executed (block = 1) | 4.697e11 | 2.084e11 | 2.084e11 |
| plain-equivalent iterations | 4.697e11 | 4.697e11 | 4.697e11 |
| macro-operators (blocks)/pixel | 0 | 24.72 | 24.72 |
| fallback pixel fraction | 1 | 0.322 | 0.323 |
| fallback (plain) iterations/pixel | 1208 | 511 | 511 |
| bytes read | 0 | 0 | 651.9 MB (0.87 MB/frame; 5.18 MB/frame referenced) |
| peak RSS per frame (max) / process | 27 MB / 25 MB | 33 MB / 31 MB | 42 MB / 39 MB |
| unresolved fraction | 5.97e-4 | 5.97e-4 | 5.97e-4 |
| BLA `shift_px` max | n/a | 9.0e-8 | 9.0e-8 |

**Compile (C):** `--workers 12`, which matches the 12 render threads; COMPILE.md used 8.
Wall time was 59.8 s: plan 6.4 s, BLA build 1.33 s, reference 0.03 s, manifests 46.8 s
(fsync-bound, 22,389 puts), verify 5.6 s. The atlas is 651,851,528 bytes (0.607 GiB):
20% of the 3 GiB cap and 24% of the 2.5 GiB target, with 2.57 GB of headroom. BLA tables
are 96.6% of the bytes. Compiler peak RAM was 290 MB. Reuse: 2 orbits (375 frames per
orbit on average, 594 max), 165 BLA tables (4.5 frames per table, 5 max), 21,639 tile
manifests (2.98 frames per tile, with 16,354 tiles shared by two or more frames). Dedup
is 5.96x by bytes: the atlas would be 3.88 GB if every frame owned its chunks. There are
no certificates, so certification time is 0.

**Speedups (total frame seconds):** A/C 1.349, B/C 0.996, A/B 1.354.

**Break-even:**

- **C vs A:** each play saves 167.4 s, so compile plus play beats A after 0.36 plays.
  Within the first play, compile plus the first F frames of C catches up with A's first
  F frames at F = 693, which is 92.4% of the path. Frames get more expensive with depth,
  so the gain comes at the end.
- **C vs B:** each play loses 1.9 s, so C never breaks even. A second play does not help,
  because C warm (0.637 s/frame) is no cheaper than B (0.638 s/frame).

**By depth** (seconds/frame, 75-frame bins):

| frames | log10 width | A | B | C | A/C | B/C |
|---|---|---|---|---|---|---|
| 0-74 | 0.6 to -4.3 | 0.070 | 0.069 | 0.070 | 1.00 | 0.99 |
| 75-149 | -4.3 to -9.2 | 0.184 | 0.176 | 0.181 | 1.02 | 0.97 |
| 150-224 | -9.3 to -14.1 | 0.389 | 0.401 | 0.381 | 1.02 | 1.05 |
| 225-299 | -14.2 to -19.1 | 0.419 | 0.543 | 0.538 | 0.78 | 1.01 |
| 300-374 | -19.1 to -24.0 | 0.515 | 0.565 | 0.582 | 0.88 | 0.97 |
| 375-449 | -24.1 to -29.0 | 0.828 | 0.834 | 0.857 | 0.97 | 0.97 |
| 450-524 | -29.0 to -33.9 | 0.885 | 0.850 | 0.830 | 1.07 | 1.02 |
| 525-599 | -34.0 to -38.8 | 1.123 | 0.857 | 0.875 | 1.28 | 0.98 |
| 600-674 | -38.9 to -43.8 | 1.657 | 1.129 | 1.154 | 1.44 | 0.98 |
| 675-749 | -43.8 to -48.7 | 2.563 | 0.950 | 0.933 | 2.75 | 1.02 |

The BLA kernel, whether its table is per-frame (B) or from the atlas (C), is slower than
plain perturbation between widths 1e-14 and 1e-29: 0.78-0.97x of A in those bins. It pays
off only below about 1e-29, and reaches 2.7x in the deepest bin. B/C stays between 0.97
and 1.05 in every bin.

## Correctness

`fd compare` (new) checks two things for every frame. Class kinds must match exactly. For
samples that escaped in both renders, `nu` displacement, measured in output pixels as
the oracle measures it, must be at most 1e-3 px.

| | frames class-identical | class mismatches | `nu` px max | over 1e-3 px | byte-identical frames |
|---|---|---|---|---|---|
| B vs A | 749 / 750 | 2 (frame 748) | 2.8e-9 | 0 | 230 (every frame that skips its table) |
| C vs A | 749 / 750 | 2 (frame 748) | 2.8e-9 | 0 | 154 (the f64 frames; deep frames use the 300-bit orbit) |
| C vs B | 750 / 750 | 0 | 1.5e-10 | 0 | 158 |

Out of 388.8M samples in each arm, the only class differences are 2 samples in frame
748 (2e-49). In A those samples reach the iteration budget and are `Unresolved`. In B
and C they are `Escaped`, at nu 99862 and 99892 respectively. A direct mpmath iteration
at 500 bits (no perturbation) settles them. The first sample escapes at 99960, so A is
wrong on it. The second does not escape within 1e5, so A is right and the BLA arms are
wrong on it. Both samples sit at the edge of the iteration budget, where a sub-pixel
shift of the sample moves the escape count by hundreds. On escaped samples, BLA changes
the integer escape count of 325,920 samples (0.086%), by up to 1359 iterations, but at a
displacement of at most 2.8e-9 px. This is the same sensitivity and well within the
contract. No Escaped/Interior disagreement occurs anywhere.

**Oracle** (`tools/oracle.py --k 8`, frames 0, 75, ..., 675): 10 of 10 pass in every arm,
with 0 class mismatches.

`fd compare` accepts widths that differ by at most 2 f64 ulps. On 8 frames (51, 57, 295,
343, 347, 351, 640, 641) the player's width, `width x 2^(2 - L)` from the manifests,
sits 1 ulp off the path's 6-digit width. That moves samples by about 1e-16 of the frame.
`frames.camera_exact 750` is the compiler's own check of the same thing. The first
comparison rejected these 8 frames as "views differ", and the 2-ulp rule was added after
that run. The rule is a tool correction, not a benchmark setting.

**Recolour (criterion 6):** `fd shade` on two played frames (375 and 749) with
`--look umber,palette,relief` wrote 6 PNGs in 0.22 s, with `iterations 0` and
`kernel_calls 0`. It needs no atlas, kernel or rebuild.

## SPEC success criteria

| # | Criterion | Verdict | Evidence |
|---|---|---|---|
| 1 | Genuine 20-30 s zoom | met | 750 frames, 25 s, 49.3 decades, oracle 10/10 in every arm |
| 2 | No fake-location substitution | met | Exact centre from the orbit manifest; classes identical to independent renders on 749/750 frames (the 2 mismatches are budget-edge `Unresolved`) |
| 3 | Under the 3 GiB ceiling | met | 0.607 GiB (20% of cap) |
| 4 | Reuse expensive chunks across frames | partly | Chunks are reused (375 frames/orbit, 4.5 frames/table, 2.98 frames/tile, dedup 5.96x), but they are not expensive. Building them costs 18.3 ms/frame in B (reference 11.7, BLA 6.6), 2.9% of the frame. |
| 5 | Beat independent rendering after precomputation | met vs A, not met vs B | 1.35x vs plain, break-even after 0.36 plays. 0.996x vs per-frame BLA, never breaks even. |
| 6 | Recolor/re-light without rebuilding | met | `fd shade`, 3 looks, 0 iterations |
| 7 | Record fallback and error metrics | met | fallback 0.323 of pixels and 511 steps/pixel, `shift_px` 9.0e-8, oracle, `fd compare` |
| 8 | Identify the next bottleneck | met | See below |

## Falsification criteria

| Criterion | Triggered? | Evidence |
|---|---|---|
| Atlas growth exceeds the path budget | no | 0.607 GiB on a 3 GiB cap |
| Most pixels repeatedly leave cached validity domains | partly | 32.3% of pixels apply no block, and 511 of the 536 executed steps per pixel are plain fallback steps (blocks skip 57.7% of plain-equivalent steps) |
| Operator/reference traffic costs more than direct arithmetic | no, but inverted | Loading costs 4.7 ms/frame and rebuilding 18.3 ms/frame. Both are negligible next to the 0.64 s render, so caching them cannot pay. |
| GPU divergence erases saved math | not tested | CPU only |
| Certification costs more than it saves | not tested | No certificate producer (CERT-01) |
| Screen-space LOD cannot safely defer work | not tested | LOD is not in the play path |
| Warm cost still climbs strongly with depth | **yes** | C warm cost grows from 0.070 s/frame (frames 0-74) to 0.933 s/frame (675-749), 13x over 45 decades, and peaks at 1.15 s/frame around 1e-41 |
| Return-map payloads scale with raw orbit history | not applicable | No return maps in v0 |
| 2-3 GiB only supports trivial paths | not triggered here | 0.6 GiB, but this is the favourable fixed-centre path (COMPILE.md); panning paths are untested |

## Next bottleneck (criterion 8)

The next bottleneck is the per-pixel iteration in the render kernel. It takes 476.5 of
C's 480.0 frame seconds (99.3%). Loading takes 3.6 s, and the reference and operator
take 0 s. On every pixel of every frame the kernel still runs 511 plain steps and 24.7
BLA blocks, and in both arms that work is recomputed in every frame. Two measured facts
follow:

1. The BLA kernel is slower than plain perturbation from 1e-14 to 1e-29 (bins at
   0.78-0.97x of A). In that range it pays a lookup cost for short blocks that save
   little. A cost-aware block threshold, or a per-frame choice between plain and BLA,
   would recover up to 0.12 s/frame there in both B and C. That makes it a kernel
   improvement, not an atlas advantage.
2. To beat B, the atlas has to cache work that is expensive to recompute. That means
   per-sample or per-tile iteration state (for example, skip-ahead to a shared iterate
   for whole tiles, series or return-map operators that remove the 511 fallback
   steps/pixel, or certified tiles whose samples do not need iterating at all). It
   cannot be the reference and BLA table, which a frame rebuilds in 18 ms.

On this path the v0 atlas is a correct, bounded and well-reused cache of cheap data.
The SPEC thesis holds only once the atlas holds something that saves per-pixel
iterations across frames.

## Limits

- This is one path, the favourable fixed-centre zoom at 1.97 decades/s, with one play of
  each arm and one machine. Run-to-run noise was not measured. B/C sits within ±5% in
  every bin, so the 0.4% total difference is noise.
- The C play read a page-cache-warm store. A cold-disk play would add reads of 0.87
  MB/frame.
- Depth-scaling families are BENCH-02.

## Reproduce

```text
fd control bench/path-atlas-v0.txt --size 960x540 --iter 100000 --threads 12 -o A --oracle tools/oracle.py --every 75 > A.jsonl
fd control bench/path-atlas-v0.txt --size 960x540 --iter 100000 --threads 12 --bla per-frame -o B --oracle tools/oracle.py --every 75 > B.jsonl
fd compile bench/path-atlas-v0.txt --store store --size 960x540 --iter 100000 --workers 12 > C.compile.log
fd play C.compile.log --store store --threads 12 -o C --oracle tools/oracle.py --every 75 > C.jsonl
fd compare A B > cmpAB.jsonl; fd compare A C > cmpAC.jsonl; fd compare B C > cmpBC.jsonl
python3 tools/benc01.py RUN_DIR > bench/benc-01-results.json
```

Each command ran under `/usr/bin/time -v` (to `A.time`, and so on), with 1-minute load
sampled every 15 s to `load.log`.
