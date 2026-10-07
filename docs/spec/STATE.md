# Where We Are (updated 2026-10-07)

Read this first in a new session, then run `ishoo_status`. Rules for how we work are in
METHOD.md (DEC-14). Every result with its numbers is in the METHOD.md results log.

## In one paragraph

The Atlas v0 machinery is built and correct: kernels, addressing, the chunk store,
manifests, the budget, the planner, scheduler, compiler and player, late shading, the
oracle and a fair benchmark. Gate C's answer was **no**: v0 ties a renderer that builds
its own BLA per frame, because it cached only cheap things (BENCH.md, DEC-15). The
search for something worth caching produced one strong lead.

**Deep zooms are built from exact repeats that complex-dynamics theorems describe.**
- Near a Misiurewicz point, deeper frames are exact rotated and scaled copies of
  shallower ones.
- On the approach to a minibrot, every pixel is "k loops around the minibrot" plus "one
  shared exit function".

Frame transfer passed its sampled comparisons; the tight degree-4 decomposition
passed 48 frozen points with max error 2.24e-4 px. PROB-04 rejected two uniform
log-polar exit tables: the finer 2.07 MiB table still has 27.0 px interpolation
error. The shared exit-table accelerator is therefore not established. If new representations
work, a deep pixel costs a few cheap steps plus a table lookup instead of thousands of
iterations, and the tables are shared by every frame: the atlas idea, with something
worth caching in it. Full write-up: `docs/research/10-8-26/misiurewicz-frame-transfer.md`.

## Do next, in order

1. **Done: the cheap loop shortcut works.** PROB-03 killed a fixed map over the
   too-wide exit schedule. PROB-07 (aead5b7) kept a degree-4 biseries used only within
   1e-26 of C, followed by a tail at the fixed parameter C: 0 wrong pixels, max
   2.24e-4 px, 1e-35 to 2e-48. Projected cost: 240-1,080 ops plus one lookup per deep
   pixel, against about 10,000 with BLA. That is only a projection, on 8 points per
   depth.
2. **Done: PROB-04 rejects uniform log-polar nu interpolation.** Tables with
   11,328 / 180,480 nodes (135,936 / 2,165,760 payload bytes) fail the existing
   48-point kill gate: max errors 154.7 / 27.0 px, zero class mismatches.
   Replacing all used nodes with 110-dps truth leaves the failure intact.
   Warm map+lookup and matched-point BLA timings are recorded in METHOD.md;
   both correctness-gated scores are 0. No 1,000-point/depth promotion or
   full-v0 sharing claim. This rejects these configurations, not adaptive tables,
   resume-state representations or the return decomposition itself.
3. **Done: PROB-08 timed it for real.** All-double deep pixel (biseries loops, Koenigs jump
   on the exit tail, plain-double finish) vs per-frame BLA on whole 480x270 frames, 1e-35 to
   2e-48, on GitHub Actions (`gh workflow run koenigs-bench.yml`): **8.5-23x faster, 0 wrong
   pixels out of 777,600** (max 3.2e-4 px). Valid for frames inside the minibrot band centred
   at the zone nucleus. It is a per-frame technique (DEC-15). Next: shallower frames and
   off-centre pixels (a c ≠ C correction in the tail), other zones (PROB-06), Fatou
   coordinates for the remaining "seahorse gate" steps (deep-research questions offered
   2026-10-07), and whether it becomes an fd-kernel fast path.
4. **PROB-05.** Misiurewicz-zone frame transfer prototype: one ring, similarity
   transforms, spot-check error contract.
5. **PROB-06.** Nested minibrot chains (real deep zooms) and automatic zone/c0
   detection.
6. In parallel, cheaper-frame fixes from prior art: FIX-03 (BLA beyond 1e-270), FIX-04
   (deep interior detection), FIX-09 (BLA slower than plain at 1e-14 to 1e-29).
7. The research engine: TRUT-01 (frozen truth pack), then PROB-01 (`fd probe`). PROB-02
   (exponential-map strips, prior art: 2-11x) is still worth measuring. RESE-02 is the
   Imagina/NanoMB source read (lower-degree or LA/AT returns).

## Decisions to know

- DEC-01 to DEC-10: the atlas principles.
- DEC-14: fast probes against fair opponents.
- DEC-15: Gate C's lesson. The atlas must hold work that is expensive to rebuild or
  inherently cross-frame.
- DEC-16 (**PROPOSED, needs the owner**): theorem-backed frame transfer counts as
  genuine depth under an error contract. It is not twin substitution.

## Things that exist for the owner

- `viewer/explore.cmd`: double-click to open the browser explorer (EXPLORE.md; follow-up ideas are in EXPL-02).
- `fd play --mp4` and `tools/research/twist_zoom_path.py`: zoom videos with speed ramps
  and twists. Three demo videos are in the owner's Downloads/mandelbrot.
- Deep-research reports: `docs/research/<date>/`. Process: before a new investigation,
  the agent drafts 2-3 questions, the owner runs them, and the answers are filed there.

## Known loose ends

- On Windows, the `bench.rs`/`control.rs` tests expect Linux peak-RSS fields (FIX-10).
- The ACC-01 worktree leaked (`.ishoo/worktrees/ACC-01`); clean it with Ishoo.
- The Ishoo binary is behind its source (`ishoo reinstall`).
- The plan "Atlas Gate C" still lists LOD-04/05/06, REF-04/05; they are not the priority
  now (see DEC-15).
- Pushes occasionally fail an LFS lock check; it cleared on retry. If it persists:
  `git config lfs.https://github.com/junovhs/fractadactyl.git/info/lfs.locksverify false`
  (needs the owner's OK).
