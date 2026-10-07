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
| 2026-10-08 | Is the minibrot approach band (1e-25 to 1e-49 on v0) structured like the Misiurewicz zone? | pointwise mpmath, 120-160 random pixels per depth, 1e-30 to 2e-48 | **Keep: decomposition proven exact.** Naive z² map: fails. Per pixel: k returns around the minibrot (ν += 764 each; k = 2-7), then a tail depending only on the exit point ζ through **one fixed dynamical-plane function E_{c0}(ζ)**. 0 class mismatches, max 2.3e-9 px down to 2e-48. Open: a cheap return map (NanoMB/AT) and a sampled E table with an error contract. Potential: about 7 cheap evals plus 1 lookup per deep pixel, shared across frames. `docs/research/10-8-26/misiurewicz-frame-transfer.md` |

| 2026-10-07 | Can one fixed cheap biseries replace every original period-764 return? (PROB-03) | `.venv/Scripts/python.exe tools/research/misiurewicz/returns_exit_tail.py --probe --jobs 18`; 48 committed 110-dps truth points, six depths; 7.7 s | **Kill this all-return replacement, not NanoMB in general.** Degrees 1, 2, 4, 6, 8, 10, 12 all score 0 across depths. Degree 4 passes the complete 1e-38 cohort (max 2.17e-4 px, 0/8 class mismatches); later return inputs exceed the local domain elsewhere. Keep raw fallback. See the contract and table below. |

| 2026-10-07 | Tight biseries exit plus a shared fixed-C tail (PROB-07) | `returns_exit_tail.py --tight-probe --jobs 18`; unchanged 48-point truth, 2.49 s incl. BLA control | **Keep on sample:** degree 4/6/8 pass every depth at input guard 1e-26, zero class mismatches; degree 4 max error 2.24e-4 px. Degree 2 fails three depths at that guard but passes all at 1e-30. Exit histogram and projected costs below; table not built. |

### PROB-03: fixed period-764 biseries (negative result)

The question is **cheaper frames**. A per-frame renderer can build the same map;
this is not an atlas win. Read RESE-02's scope first: its primary-source distillation
is still backlog, so this experiment does not claim to implement Imagina AT or the
full KF NanoMB algorithm. Primary formulation: [Heiland-Allen's deep-zoom write-up](https://mathr.co.uk/blog/2021-05-14_deep_zoom_theory_and_practice.html)
and [KF's NanoMB manual](https://mathr.co.uk/kf/manual.html). Both describe a biseries
in orbit and parameter offsets, repeated for one period at a time, then regular
iteration outside its escape radius. This implementation derives its own coefficients
from the quadratic recurrence; no upstream code was copied.

For reference orbit Z beginning at C, write z=Z+s*u, c=C+s*v, s=1e-25.
Compose `u_next=2*Z*u+s*u*u+v` for 764 steps, truncating total degree after
each step and retaining all mixed terms. Reference and nucleus residual use 110-dps
mpmath; coefficients use NumPy complex128. Start the pixel at z1=C+d and apply the
map once for every original return. The frozen truth fixes the original return
count for this diagnostic: this is not a runnable production exit scheduler.
After those returns, directly iterate the candidate at the actual pixel parameter
(rather than c0) to isolate map error from the separate exit-table hypothesis.

**Validity/error/fallback contract:** original exit radius is 1.7e-15 around C;
the candidate's conservative empirical input guard is 1e-26 for degree >=4.
This guard is sampled, not a certified disk. Error limit is 1e-3 px, both for
smooth-iteration displacement (truth DE divided by w/480) and the local equivalent
parameter displacement `|z_map-z_truth|/|dz_truth/dc|/(w/480)`. Outside-guard
polynomial evaluations are diagnostic only, never accepted. Nonfinite/huge output,
escape inside a raw period, an input outside the guard, class mismatch, or error
above the limit prevents an all-return pass. The unchanged legacy raw-iteration
path remains the fallback. No points are silently removed from the eight-per-depth
denominator. Class checks are Escaped versus Unresolved at 20,000 iterations;
Unresolved is not a proof of interior membership. The frozen fixture is generated
by `--freeze --jobs 18`; normal probes never recompute truth. This is a small
issue-local pack, not completion of TRUT-01 or an independent-oracle promotion.

Highest tested degree (12), original radius; errors below are **unguarded
diagnostics** on finite candidates, including candidates needing fallback:

| Width | Max tail px error | Max local state px error | Class mismatches / evaluated | Fallback / 8 | Map return ops/px | Raw return ops/px |
|---|---:|---:|---:|---:|---:|---:|
| 1e-35 | no finite candidate | no finite candidate | 0 / 0 | 8 | 2800 | 9168 |
| 1e-38 | 1.35e-12 | 1.12e-12 | 0 / 8 | 0 | 2800 | 9168 |
| 1e-40 | 2.79e-12 | 1.12e-12 | 0 / 7 | 1 | 2975 | 9741 |
| 1e-43 | 13.53 | 4.29e6 | 0 / 6 | 7 | 4200 | 13752 |
| 1e-46 | 3.50e-12 | 1.48e-12 | 0 / 4 | 5 | 5600 | 18336 |
| 2e-48 | 8.42e-11 | 7.63e-11 | 0 / 5 | 7 | 7525 | 24639 |

Build times for degrees 1/2/4/6/8/10/12 were
0.009/0.011/0.032/0.092/0.213/0.477/0.897 s. Map costs per return are
36/90/240/446/708/1026/1400 real arithmetic operations against 4584 for
764 raw squares/adds (complex multiply=6, square=4, add=2).
Per-pixel columns multiply this by the truth's mean number of requested returns;
they are theoretical attempted-map costs, not successful end-to-end costs.
Tail, derivatives, fallback, memory traffic, and build amortization are excluded.
Degree 1 is a local linear-return diagnostic, **not** our hierarchical BLA renderer.
Because every candidate fails correctness/domain coverage, no speedup over BLA
or promotion is claimed; a fair BLA timing is required before any later keep result.

**Handoff:** this finite sweep kills one fixed, low-degree map over the original
return schedule. It does not rule out higher degrees, a smaller exit radius with
raw transition steps, adaptive charts, or multi-stage LA/AT. RESE-02 remains the
prior-art prerequisite for that next experiment. PROB-04 must not assume that all
returns are now cheap. The exit table, Rust kernel and nested chains were not changed.

### PROB-07: tight exits and one fixed-parameter E_C tail

2026-10-07. **Keep the decomposition on the frozen sample.** With the common
1e-26 input guard, degree 4 is the lowest tested degree passing all six depths;
degree 2 fails at 1e-38, 1e-43 and 1e-46, while degrees 6 and 8 also pass everywhere.
All 192 candidate evaluations have zero class mismatches and zero numerical/return-limit
failures. This corrects the overly wide 1.7e-15 exit schedule tested in PROB-03.

Command (Python environment needs mpmath and NumPy):

```text
.venv/Scripts/python.exe tools/research/misiurewicz/returns_exit_tail.py --tight-probe --jobs 18 --report C:/Users/SpencerNunamakerTrav/fractadactyl/target/PROB-07.json --bla-exe C:/Users/SpencerNunamakerTrav/fractadactyl/target/release/fd.exe
```

Exit 0, 2.494 s including the BLA control. The original 48-point,
110-dps fixture is unchanged. Candidate scheduling starts at z1=C+d, applies a
biseries only while its **input** satisfies |z-C| <= guard, and stops immediately
after an output leaves that domain. It never uses truth's old return count.
At exit zeta, the candidate smooth value is **1+k*764+E_C(zeta)**, where E_C
is directly evaluated at fixed C with a fixed 20,000-step tail budget. No c0 or
pixel parameter is used in that tail. Total escape steps are then checked against
the fixture's 20,000-step horizon. Direct tail evaluation stands in for the future
shared table; there is no table, interpolation, Rust change or promotion here.

**DEC-10 contract:** keep requires both smooth displacement and local equivalent
parameter displacement <=1e-3 px, zero class mismatches and zero failures. The local
exit-state diagnostic runs only an auxiliary exact prefix at the pixel parameter,
at the candidate's exit count, and divides state difference by its d/dc derivative
and w/480. Frozen smooth/class truth is not recomputed. A second control runs E_C
from that exact prefix state: its worst smooth error is 1.45e-11 px with the common
guard, separating map truncation from fixed-C substitution. These are sampled
guards, not certified disks; unsupported inputs and observed errors require raw
fallback in a later implementation. All eight samples per depth stay in the score.

Common input guard 1e-26; every row has class mismatches 0/8:

| Width | Degree | Max smooth px | Max local px | Returns range | Mean direct-tail steps | Projected map ops/px | Verdict |
|---|---:|---:|---:|---:|---:|---:|---|
| 1e-35 | 2 | 1.169e-8 | 1.127e-8 | 1–1 | 722.3 | 90.0 + lookup | KEEP |
| 1e-38 | 2 | 1.550e+0 | 7.798e-1 | 2–2 | 155.5 | 180.0 + lookup | KILL |
| 1e-40 | 2 | 6.284e-5 | 7.652e-5 | 2–2 | 426.1 | 180.0 + lookup | KEEP |
| 1e-43 | 2 | 1.024e-2 | 2.235e-2 | 2–3 | 753.6 | 191.3 + lookup | KILL |
| 1e-46 | 2 | 6.015e-2 | 3.132e-2 | 3–4 | 550.8 | 303.8 + lookup | KILL |
| 2e-48 | 2 | 3.079e-5 | 1.832e-5 | 4–5 | 701.0 | 405.0 + lookup | KEEP |
| 1e-35 | 4 | 1.450e-11 | 6.472e-13 | 1–1 | 722.3 | 240.0 + lookup | KEEP |
| 1e-38 | 4 | 2.172e-4 | 2.237e-4 | 2–2 | 161.1 | 480.0 + lookup | KEEP |
| 1e-40 | 4 | 2.787e-12 | 1.117e-12 | 2–2 | 426.1 | 480.0 + lookup | KEEP |
| 1e-43 | 4 | 6.506e-7 | 5.636e-7 | 2–3 | 753.6 | 510.0 + lookup | KEEP |
| 1e-46 | 4 | 1.553e-6 | 1.495e-6 | 3–4 | 550.8 | 810.0 + lookup | KEEP |
| 2e-48 | 4 | 2.776e-12 | 1.471e-12 | 4–5 | 701.0 | 1080.0 + lookup | KEEP |
| 1e-35 | 6 | 1.450e-11 | 6.472e-13 | 1–1 | 722.3 | 446.0 + lookup | KEEP |
| 1e-38 | 6 | 7.974e-8 | 3.990e-8 | 2–2 | 161.1 | 892.0 + lookup | KEEP |
| 1e-40 | 6 | 2.787e-12 | 1.116e-12 | 2–2 | 426.1 | 892.0 + lookup | KEEP |
| 1e-43 | 6 | 1.430e-11 | 8.498e-12 | 2–3 | 753.6 | 947.8 + lookup | KEEP |
| 1e-46 | 6 | 8.389e-11 | 4.382e-11 | 3–4 | 550.8 | 1505.3 + lookup | KEEP |
| 2e-48 | 6 | 2.776e-12 | 1.471e-12 | 4–5 | 701.0 | 2007.0 + lookup | KEEP |
| 1e-35 | 8 | 1.450e-11 | 6.472e-13 | 1–1 | 722.3 | 708.0 + lookup | KEEP |
| 1e-38 | 8 | 2.217e-11 | 9.802e-12 | 2–2 | 161.1 | 1416.0 + lookup | KEEP |
| 1e-40 | 8 | 2.787e-12 | 1.116e-12 | 2–2 | 426.1 | 1416.0 + lookup | KEEP |
| 1e-43 | 8 | 1.329e-12 | 1.116e-12 | 2–3 | 753.6 | 1504.5 + lookup | KEEP |
| 1e-46 | 8 | 1.166e-12 | 1.351e-12 | 3–4 | 550.8 | 2389.5 + lookup | KEEP |
| 2e-48 | 8 | 2.776e-12 | 1.471e-12 | 4–5 | 701.0 | 3186.0 + lookup | KEEP |

The remaining direct-tail work is real: 161–849 steps/pixel for the passing
degree-4 cohort. With an actual E_C table it would become one lookup; lookup cost
and interpolation error remain unmeasured. Arithmetic uses PROB-03's model
(complex multiply=6, square=4, add=2), excluding derivatives, guards, memory and
build amortization. Maps cost 90/240/446/708 ops per return at degrees 2/4/6/8;
measured build seconds were 0.012/0.025/0.077/0.192. Worst-depth scores against frozen raw arithmetic,
with lookup unpriced, are 0/21.127/11.369/7.162. They are projections, not measured
speedups.

**Fair opponent (DEC-14):** the existing fd binary runs per-frame BLA on separate
8x4 grids at the same six widths, via `fd control <six-line-path> --size 8x4
--iter 20000 --columns nu,de --threads 18 --bla per-frame --runs 1`.
All 192 grid samples escaped. BLA work is measured, but its grid differs from the
frozen eight points, so this is contextual cost comparison, not a matched-point
speedup. BLA arithmetic is modeled at 14 ops per perturbation fallback or linear
block (2Z*dz+dz^2+dc or A*dz+B*dc), excluding derivative and guard work.
A per-frame rival can build the same biseries. The shared E_C function is the
cross-frame candidate; atlas benefit is still unproven until PROB-04 builds and
prices the table.

| Width | Frozen direct ops/px | BLA raw fallback/px | BLA blocks/px | Modeled BLA ops/px | Degree-4 map ops/px |
|---|---:|---:|---:|---:|---:|
| 1e-35 | 8923.5 | 670.19 | 37.00 | 9900.6 | 240.0 + lookup |
| 1e-38 | 10140.8 | 658.28 | 44.13 | 9833.7 | 480.0 + lookup |
| 1e-40 | 11730.8 | 761.19 | 47.63 | 11323.4 | 480.0 + lookup |
| 1e-43 | 14268.8 | 616.25 | 51.50 | 9348.5 | 510.0 + lookup |
| 1e-46 | 18781.5 | 675.66 | 67.97 | 10410.8 | 810.0 + lookup |
| 2e-48 | 24840.0 | 714.00 | 81.78 | 11140.9 | 1080.0 + lookup |

**Exit-radius histogram:** bins are floor(log10(|zeta-C|)); each cell sums to eight.
All four degrees at the common guard have these same bin counts. Their outputs
can overshoot far beyond the input guard: degree-4 exit radii range from
1.249e-26 to 7.648e-5. PROB-04 must cover the actual exit distribution, not assume
it ends at 1e-22. The annuli alone do not determine table size or interpolation error.

| Width | Common 1e-26 guard, degrees 2/4/6/8 (exponent: count) | Smaller degree-2 guard 1e-30 (exponent: count) |
|---|---|---|
| 1e-35 | -22: 3, -21: 5 | -22: 3, -21: 5 |
| 1e-38 | -7: 1, -6: 3, -5: 4 | -28: 3, -27: 5 |
| 1e-40 | -15: 1, -14: 3, -13: 4 | -15: 1, -14: 3, -13: 4 |
| 1e-43 | -26: 3, -25: 4, -6: 1 | -27: 1, -26: 3, -25: 4 |
| 1e-46 | -26: 1, -24: 4, -10: 1, -6: 1, -5: 1 | -30: 1, -28: 1, -27: 1, -26: 1, -24: 4 |
| 2e-48 | -26: 4, -25: 1, -18: 1, -16: 1, -12: 1 | -30: 1, -26: 4, -25: 1, -18: 1, -16: 1 |

**Separate smaller-domain quadratic variant:** `--quadratic-guard 1e-30` exits
earlier and passes all six depths (exit 0, 2.519 s). It uses a narrower sampled
guard rather than claiming degree 2 is valid throughout 1e-26. Its worst error is
7.65e-5 px; max fixed-C-only control error is 1.78e-9 px. It trades more tail work
for fewer map operations, and has exit radii 1.441e-30 to 7.648e-13:

| Width | Max smooth px | Max local px | Returns range | Mean direct-tail steps | Projected map ops/px | Verdict |
|---|---:|---:|---:|---:|---:|---|
| 1e-35 | 1.169e-8 | 1.127e-8 | 1–1 | 722.3 | 90.0 + lookup | KEEP |
| 1e-38 | 1.783e-9 | 1.189e-11 | 1–1 | 925.1 | 90.0 + lookup | KEEP |
| 1e-40 | 6.284e-5 | 7.652e-5 | 2–2 | 426.1 | 180.0 + lookup | KEEP |
| 1e-43 | 1.388e-10 | 7.744e-11 | 2–2 | 849.1 | 180.0 + lookup | KEEP |
| 1e-46 | 9.226e-11 | 7.188e-11 | 3–3 | 837.3 | 270.0 + lookup | KEEP |
| 2e-48 | 1.064e-7 | 5.349e-8 | 4–5 | 796.5 | 393.8 + lookup | KEEP |

**Handoff:** degree 4 at 1e-26 is the common-domain baseline for PROB-04; the
narrower quadratic variant is a separate viable option. E_C must use **C**, not c0,
through the transition region. The exit samples suggest a wide transition domain
and motivate testing table coverage/interpolation before claiming atlas speed.
