# How We Work (research method)

NORTH-STAR.md says **what** Fractadactyl is for. This document says **how** we go after
it. It binds every contributor, human or agent. DEC-14 records its core rule.

The reason it exists: the first decisive experiment (BENC-01) took a day to build and
about half an hour per arm to run, and it answered no. That is a fine result, but at
that pace we get one answer a day. Rendering fractals is slow, so the research loop must
not be rendering fractals.

## 1. Three gates

No hypothesis gets an expensive run until it has won a cheap one.

| Gate | Budget | Input | Purpose |
|---|---|---|---|
| **Probe** | under 30 s | hundreds to a few thousand points or samples, one tile, a few tiny frames | kill or keep. Run constantly |
| **Promotion** | under 5 min | 20-50 frames at 160x90 to 320x180 | check cross-frame behaviour and an honest wall-clock time |
| **Full benchmark** | as long as it takes | the real path at real size | confirm a win the first two gates already showed |

A full benchmark that is not confirming a probe win is a mistake. If a probe cannot be
built for an idea, building the probe is the first task.

## 2. Every claim has a fair opponent

A speedup counts only against the best competitor that could adopt the same trick
without our machinery. For atlas claims, that is the independent renderer that does the
same thing per frame (BENC-01 arm B). For kernel claims, it is the current best kernel
path.

Every experiment states which question it answers:

- **Cheaper frames:** does this make a genuine frame cheaper for anyone? This serves the
  north star directly.
- **Atlas wins:** does this make compiled, reused work beat recomputing per frame? It
  wins only if what it caches is expensive to rebuild or inherently cross-frame.

Both questions are worth asking, but they must not be confused. BENC-01 confused them
until arm B separated them.

## 3. Measure cost, not just counts

Iteration counts are the fast first signal, but not the verdict. Operations differ in
cost: BENC-01 measured BLA doing fewer iterations than plain perturbation yet running
0.78-0.97x as fast between 1e-14 and 1e-29. A probe reports cost-weighted operations, or
real time on a fixed batch, next to the counts. A step reduction under about 2x is
treated as noise until real time agrees.

## 4. Frozen truth

Correctness is checked against a frozen truth pack, not recomputed each time. The pack is
a few nasty locations times about a thousand chosen points (boundary, filament, deep
minibrot, near-parabolic), computed once by the oracle at high precision and committed.
A candidate is compared in seconds. The independent oracle (tools/oracle.py) runs only
when a candidate is promoted. Errors use the oracle's measure, equivalent pixel
displacement (DEC-10).

## 5. Generate and test

Ideas are cheap and scoring decides. Each probe ends in **one number per candidate**,
with correctness as a gate, so that many variants can be generated and ranked
automatically, including overnight by agents. The work is made searchable before it is
made clever.

## 6. Where ideas come from

- **Measured constraints.** Every idea starts from a measured bottleneck and a numeric
  target, for example: "remove the 511 fallback steps/pixel on the 32% of pixels no BLA
  block covers, using under 1 KB per tile". Vague goals produce textbook answers.
- **Forced analogies.** Map the measured problem through other fields on purpose: maps,
  video codecs, JIT and trace compilers, ray-tracing acceleration structures, multigrid,
  virtual texturing, compression, interval arithmetic. Most die. That is the point.
- **Human intuition as a constraint.** The owner's hunches ("Google Maps for fractals",
  "1-3 px features can be smooshed") are hypotheses to put through a probe, not
  decorations and not dogma.
- **Prior art first.** Before building, spend a few minutes on who did this already
  (Kalles Fraktaler, Fraktaler 3, nanomb, zoomasm, the deep-zoom forums and papers).
  Build on it. Prior art is the floor, not the destination.
- **Deep research before a new investigation.** Before opening a new line of
  investigation, the agent drafts 2-3 focused deep-research questions. The owner runs
  them as external deep-research jobs across many sources. A good question names the
  measured bottleneck, the constraints (genuine depth, error contract, bytes, CPU and
  GPU), what we already tried and measured, and the exact form the answer should take:
  methods, their cost scaling, sources, known failures. The answers are saved under
  `docs/research/<date>/` and read before the probe is designed.

## 7. Kill fast, write it down

A dead idea is a result. Record negative and positive results in the log below, with
the command and the numbers, so nobody pays for the same answer twice.

## Results log

| Date | Question | Probe | Answer |
|---|---|---|---|
| 2026-10-07 | Does the v0 atlas beat independent frames? (BENC-01) | full benchmark, 750 frames | **No.** It ties per-frame BLA (0.996x). It cached reference and tables worth 18 ms of a 640 ms frame. BENCH.md |
| 2026-10-07 | At extreme depth, does reference cost grow enough that caching it wins? | `fd control` with one frame, 960x540, iter 1e5, BLA none and per-frame: 1e-48, 1e-90, c=i at 1e-300 and 1e-1000 | **No.** The reference took 14-111 ms against renders of 0.8-2.8 s (at most 4%). Side findings: (a) BLA builds no usable table on the scaled tier (1e-300 and beyond run 800-2700 plain steps/pixel); (b) inside the period-764 minibrot at 1e-90, every pixel is Unresolved (interior not detected): 48 s plain, 1.75 s with BLA |
| 2026-10-08 | Prior art: cross-frame reuse in zoom videos (deep research, `docs/research/10-8-26/zoom-computation-reuse.md`) | report, plus an analytic check | **Keep: exponential-map strips** (Kalles Fraktaler / Fraktaler 3 export, zoomasm assembly). Each zoom octave is computed once as a log-polar strip of palette-independent samples, and frames are reprojected from it. Rotation and zoom timing are free at assembly time; panning off the fixed centre breaks it. The report's "95-99.9%" compares undersampled strips. At matched sample density the saving is frames-per-octave dependent: **1.8x** on the v0 path (1.97 dec/s, 30 fps), **5.7x** at 0.5 dec/s and 24 fps, **11.3x** at 0.25 dec/s. Its claim that the reference dominates at 1e500 contradicts our measurement (111 ms against a 2.8 s render at 1e-1000). Next: PROB-02 |
| 2026-10-08 | Prior art: return maps near minibrots (deep research, `docs/research/10-8-26/minibrot-renormalization-local-maps.md`) | report | **Keep, needs primary sources.** The idea exists as NanoMB1/NanoMB2 (Heiland-Allen, in Kalles Fraktaler): a per-period bivariate series iterated until escape. BLA's linear validity collapses as the reference passes near 0, about once per period near a minibrot, which is a likely cause of the 32% of pixels with no BLA skip. A map with a quadratic term survives that pass (Douady-Hubbard: the local return map is quadratic-like). The report has no measured speedups and several garbled formulas; read the NanoMB source and blog before designing the probe |
| 2026-10-08 | Prior art: what removes the per-pixel tail beyond perturbation + BLA (deep research, `docs/research/10-8-26/state-of-the-art-beyond-perturbation-bla-for-cpu-deep-zoom-mandelbrot.md`) | report | **Keep, three leads.** (1) Our "no BLA at 1e-300+" is most likely a range bug: BLA coefficients and radii must carry an explicit exponent (Imagina switches to FloatExp below about 2^-896 half-height, roughly where our scaled tier starts); see FIX-03. (2) Deep-minibrot interiors: atom-domain period detection on the reference, Newton nucleus refinement, then per-pixel derivative contraction (about 1e-3, valid only with a nucleus reference; our v0 centre already is one); see FIX-04. (3) The 511-step tail: Imagina's multi-stage LA plus "approximation transformation" (AT; one transformed step equals StepLength raw steps, N' = N/L) is the most direct CPU prior art, then second-order BLA (a 2026 WebGPU implementation reports 21x and 96.9% skipped at 2.8e40, GPU only). All three are per-frame techniques an independent renderer can adopt (the "cheaper frames" question, not "atlas wins"). The report's citations are placeholders, so verify against the Imagina and FractalShark source before porting |
| 2026-10-08 | Can analytic (Böttcher-coordinate) patches replace iteration in M-free regions? | de histogram on 10 views, 480x270 | **Kill as a general accelerator.** On the hard views (valley 1e-28, v0 at 1e-40 and 2e-49), 0% of pixels are ≥32 px from the set; only dendrite views (c=i) have large empty areas. `docs/research/10-8-26/misiurewicz-frame-transfer.md` |
| 2026-10-08 | Are deep frames near a Misiurewicz point exact transforms of shallower frames (Tan Lei similarity)? | paired renders, 480x270, c=i at 1e-300 and the v0 target near M(24,2) at 1e-6 to 1e-23 | **Keep: strongest lead so far.** Using the multiplier ρ (zoom by \|ρ\|, rotate by −arg ρ, shift the centre to c0+ρ(C−c0), ν += period): 0 class mismatches at every depth; max displacement 9.8e-4 px at 1e-10, 1.2e-5 at 1e-12, ≤3.5e-7 px (kernel noise) from 1e-16 to 1e-23. One 0.062-decade ring covers the whole zone at any depth, speed or rotation, and it is inherently cross-frame. The near-minibrot analogue (scale by λ) is **not** a similarity: the body matches but the surrounding decoration hairs do not. `docs/research/10-8-26/misiurewicz-frame-transfer.md` |
