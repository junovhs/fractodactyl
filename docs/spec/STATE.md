# Where We Are (updated 2026-10-08)

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

Both were verified with 0 wrong pixels and errors ≤1e-6 px. If the remaining pieces
work, a deep pixel costs a few cheap steps plus a table lookup instead of thousands of
iterations, and the tables are shared by every frame: the atlas idea, with something
worth caching in it. Full write-up: `docs/research/10-8-26/misiurewicz-frame-transfer.md`.

## Do next, in order

1. **PROB-03 (make-or-break).** Is a cheap per-loop return map (NanoMB/Imagina-AT style)
   accurate enough to replace the 764-iteration loops?
2. **PROB-04.** Build the shared exit table E_{c0}: its size, resampling error and
   self-similar ring.
3. **PROB-05.** Misiurewicz-zone frame transfer prototype: one ring, similarity
   transforms, spot-check error contract.
4. **PROB-06.** Nested minibrot chains (real deep zooms) and automatic zone/c0
   detection.
5. In parallel, cheaper-frame fixes from prior art: FIX-03 (BLA beyond 1e-270), FIX-04
   (deep interior detection), FIX-09 (BLA slower than plain at 1e-14 to 1e-29).
6. The research engine: TRUT-01 (frozen truth pack), then PROB-01 (`fd probe`). PROB-02
   (exponential-map strips, prior art: 2-11x) is still worth measuring. RESE-02 is the
   Imagina/NanoMB source read; it feeds PROB-03.

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
