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

Frozen points rank and kill; they do not promote. A per-pixel shortcut is **kept** only
after it matches fd (or the oracle) on every pixel of at least one whole frame per tested
depth (≤1e-3 px, 0 class mismatches), with the worst pixels re-checked at high precision
to see which side is wrong (DEC-17, proposed). On 2026-10-07 two configurations passed 48
frozen points and failed whole frames. Whole-frame checks run on GitHub Actions
(`gh workflow run koenigs-bench.yml`).

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

| 2026-10-07 | Can a shared fixed-C log-polar exit table replace the tail? (PROB-04) | `returns_exit_tail.py --table-probe --jobs 18 --report <external>/table.json --bla-exe <existing>/fd.exe`; 48 frozen points, 4.840 s, exit 0 | **Kill the two uniform bilinear nu tables.** 135,936 / 2,165,760 bytes; max 154.713 / 27.004 px versus 0.001 px required, 0/48 class mismatches. Exact 110-dps corner values leave the interpolation failure intact. Correctness-gated scores 0; no wider promotion or atlas-win claim. |

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

### PROB-04: shared exit table, uniform interpolation rejected

Question: **atlas wins**, conditional on a correct shared E_C payload. The probe
builds two actual tables at fixed parameter C, then reuses each across the six
depth cohorts. It stores float64 smooth escape nu and int32 escape step count
per node (12 bytes/node), rather than resume states, de or normal. This measures
only the smooth/class requirement; it cannot establish complete shading accuracy.
The 764-step degree-4 prefix retains the PROB-07 input guard 1e-26.

Representation: explicit log-polar rings centred at C, log10 radius -26 through
-4, periodic angular seam, bilinear interpolation in log radius and angle.
This covers the measured 1.249e-26 to 7.648e-5 exits. No M(24,2) self-similar
wrap is used: transferring E_C across C-c0 without a measured error contract
would add another unsupported approximation. Table nodes use fixed-C
perturbation around a 110-dps reference starting at C; C-relative deltas preserve
the small offsets instead of rounding C+delta to a double.

Validity/fallback (DEC-10): lookups outside log10 radius [-26,-4), non-finite
inputs, unresolved corners, an ambiguous 20,000-step horizon, or a 60-return
prefix limit require direct iteration. No interpolation bound is certified.
The frozen-pack correctness gate rejects an entire configuration if any point
exceeds 1e-3 px, changes escape class or needs fallback. Both tested
configurations are rejected, so their interpolated values must not be used in
production. Every query remains in the denominator; none of the 48 queries
needed domain/horizon fallback and all escaped in both truth and tables.

Reproduce from the PROB-04 worktree using its .venv (mpmath 1.4.1, numpy 2.5.3):

```powershell
.venv/Scripts/python.exe tools/research/misiurewicz/returns_exit_tail.py --table-probe --jobs 18 --report C:/Users/SpencerNunamakerTrav/fractadactyl/target/prob04/table.json --bla-exe C:/Users/SpencerNunamakerTrav/fractadactyl/target/release/fd.exe
```

The unchanged committed `return_map_truth.json` has SHA-256
`aa93abe304f55d3c28cde6d58c9e4814bd5dfcac0b639cf5b27c560693e5fa89`.
The report records payload sizes, timings, controls, per-depth errors and sharing.
Reports and BLA output stay outside the worktree. The final run took 4.840 s,
exit 0; the biseries build took about 0.03 s. Timing is host-dependent.
Regression: `--tight-probe --jobs 18 --report <external>/tight-regression.json`
took 2.254 s, exit 0, preserving the degree-4/6/8 passes and degree-2 failures.
`git diff --check` also passed (exit 0).

| Radial intervals/decade | Angles | Nodes | Payload bytes | Build s | Warm map+lookup us/px | Max px | Class mismatches | Score |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 8 | 64 | 11,328 | 135,936 | 0.0381 | 18.776 | 154.713 | 0/48 | 0 |
| 32 | 256 | 180,480 | 2,165,760 | 1.8657 | 28.123 | 27.004 | 0/48 | 0 |

These are array payload bytes, excluding metadata, the temporary reference and
construction scratch. Both payloads are small against DEC-03's 2.5 GiB target /
3 GiB cap, but neither meets accuracy. No estimate of sufficient table size is
established by these failures; errors do not decrease uniformly with resolution.

| Width | Max px, 8x64 | Max px, 32x256 | 32x256 with exact corner values | Direct double-tail control px |
|---|---:|---:|---:|---:|
| 1e-35 | 39.6736 | 0.962934 | 0.962934 | 1.450e-11 |
| 1e-38 | 2.03553 | 1.32289 | 1.32289 | 2.172e-4 |
| 1e-40 | 33.5045 | 1.09474 | 1.09474 | 2.787e-12 |
| 1e-43 | 154.713 | 3.78794 | 3.78794 | 6.506e-7 |
| 1e-46 | 53.0709 | 8.04783 | 8.04783 | 1.553e-6 |
| 2e-48 | 16.0497 | 27.0038 | 27.0038 | 2.883e-12 |

Displacement is |nu_predicted-nu_truth| ln(2) de_truth / (width/480).
The deep truth derivative encoded in de accounts for sensitivity through the
returns; interpolated tail derivatives are not substituted into the error metric.
The exact-prefix fixed-C control stays within 1.450e-11 px; the map's local state
error stays within 2.237e-4 px. Direct double-tail controls also pass, with no class
mismatches. Independent 110-dps evaluations replace **all** used corner values:
150 coarse / 152 fine nodes. Maximum node smooth discrepancies are 0.008478 /
4.021e-5, with zero node class or escape-step differences. Re-interpolation from
those exact corners still fails by the values above. This isolates interpolation
as the decisive failure, rather than mistaking a node-evaluation defect for it.

**Measured opponent and timing limits:** `fd control` receives 48 1x1 views,
each centred exactly at the corresponding frozen C+d, same width, 20,000-step
budget, nu/de columns, one thread and per-frame BLA. Pixel-centre sampling makes
these matched coordinates, unlike PROB-07's separate 8x4 grid. Final measured
BLA render cost averages 283.213 us/pixel. Candidate timing is 20 repetitions of
the same predecoded scalar Python degree-4 map + lookup batch, excluding table
build, truth checks and fallback. BLA includes its render/column work; the
candidate computes nu only. Tiny one-pixel views, distinct per-point references
and Python/Rust differences prevent a full-frame speed claim. Score is matched
BLA render time / warm candidate time **only after correctness passes**; both
scores are 0, regardless of the apparent lookup-time advantage. A fair rival can
adopt the same biseries; the table, rather than that map, is the reuse hypothesis.

**Sharing and handoff:** one table serves six depth cohorts, a measured 6:1
frames-per-table ratio on these diagnostic views (48 lookups). Full-v0 frame
coverage and effective accepted sharing are unmeasured; no frame is accepted by
these failed configurations. DEC-14 therefore stops before the >=1,000 points
per depth validation. This satisfies PROB-04's recorded-kill alternative, not its
success alternative. It rejects uniform bilinear nu interpolation at these two
resolutions, not adaptive/derivative-aware tables, resume-state tables, the
return decomposition, or all tables fitting the budget. Next planned work is
PROB-05; another exit-table representation needs its own winning cheap probe.

| 2026-10-08 | Can the exit tail skip its spiral-out along the repelling 2-cycle analytically (Koenigs jump), with no table? | `python tools/research/misiurewicz/koenigs_tail.py tools/research/misiurewicz/return_map_truth.json TERMS R0`; frozen 48-point truth, exact prefix to the PROB-07 1e-26 exit, then a fixed-C tail; about 1 s per configuration | **Keep.** The tail goes 23 direct steps to the 2-cycle point α_C, then φ (Koenigs series of f_C² at α_C, ρ_C) jumps j = ⌊log(R0/\|φ\|)/log\|ρ\|⌋ cycle-steps, then ψ = φ⁻¹ (Newton) and finishes directly. 40 terms with R0 = 1e-3, 1e-2 or 3e-2 is exact against truth (max 6.5e-12 px, 0 class mismatches, all six depths). **12 terms with R0 = 0.03: max 2.6e-8 px; 8 terms: max 2.3e-5 px.** R0 ≥ 0.1 fails (outside the series' useful radius). Tail steps fall from 113-996 to 56-342 (most 56-150); no orbit re-approached α. Replaces the rejected PROB-04 interpolated table for the spiral-out part. It is a per-frame technique ("cheaper frames", DEC-15), not an atlas payload. Open: why some tails still need about 300 steps after the jump; ψ by series reversion instead of Newton; real timing against BLA |
| 2026-10-07 | After the Koenigs jump, why do some tails still need about 300 steps? Is there a second spiral to skip? | `python tools/research/misiurewicz/koenigs_leftover.py tools/research/misiurewicz/return_map_truth.json` (from that directory); same 48 points, 12 terms, R0 = 0.03; under 1 s | **No simple second spiral.** f_C's α fixed point has multiplier −0.9976+0.1314i (\|·\| = 1.0062, very slowly repelling), but the long tails (279 and 319 steps; most others 33-151 after the 23 approach steps) do not spiral out from it monotonically: they wander 0.1-0.5 away from it, through the region shared by the fixed point and the 2-cycle. That is the near-parabolic "seahorse gate" around c = −3/4. A Koenigs series cannot linearise it. Skipping it would need near-parabolic Fatou coordinates (Lavaurs/Douady), a harder construction. It is not attempted here. |
| 2026-10-07 | Can the whole exit tail (approach, Koenigs jump, finish) run in plain IEEE double? | `python tools/research/misiurewicz/koenigs_double.py tools/research/misiurewicz/return_map_truth.json TERMS R0` (from that directory); exact mpmath loop prefix, then the exit point is passed as the double offset δ = ζ − C; 23 approach steps are double perturbation against C's critical orbit, h0 = (Z_24 − α) + δ_24; φ is Horner in double, ψ is Newton in double, the finish is plain double; about 1 s | **Keep.** Matches the 80-digit tail: 40 terms, R0 = 0.03: max 1.2e-10 px. 12 terms, R0 = 0.03: max 2.65e-8 px. 12 terms, R0 = 0.01: 1.3e-10 px. 8 terms, R0 = 0.03: 2.3e-5 px. 6 terms, R0 = 0.01: 3.3e-6 px. 0 wrong pixels on all six depths; tail steps 56-342, as before. Rounding ζ itself to double fails (ζ − C is 1e-26 to 1e-5), so the offset form is required. Two script bugs were caught before logging: ζ was rounded directly (fixed by the offset form), and an off-by-one in ν (exactly 1.0 ν, 3-64 px). The tail is therefore cheap hardware arithmetic (one complex square-add per step plus about 100 flops for the jump). Still open: the loop prefix in double/FloatExp, and real timing against per-frame BLA. |
| 2026-10-07 | Can the whole deep pixel (loops + tail) run in IEEE double with no per-pixel multiprecision? Which loop degree/guard is cheapest? | `python tools/research/misiurewicz/koenigs_double_full.py tools/research/misiurewicz/return_map_truth.json DEGREE 12 0.03 [GUARD]` (from that directory); per pixel only v = (c−C)/1e-25 is rounded from high precision; biseries coefficients come from `returns_exit_tail.build_map` (double); tail from `koenigs_double`; under 1 s | **Keep.** Degree 4 with guard 1e-26: max 2.17e-4 px, 0 wrong pixels, identical to the 80-digit PROB-07 result. Guard sweep (max px / wrong pixels > 1e-3): 1e-26 → degree 2 1.55 / 11, degree 3 2.9e-2 / 4, degree 4 2.2e-4 / 0. 1e-28 → degree 2 9.2e-5 / 0, degree 3 1.75e-6 / 0, degree 4 1.75e-6 / 0. 1e-30 → 6.1e-3 / 1 for every degree. 1e-32 → 7.9 / 7 for every degree. Guards ≤ 1e-30 fail independently of degree: the fixed-C tail ignores c − C, which matters when the exit offset is too small (the expected PROB-07 lower bound, not a precision bug). Cheapest passing loop: degree 2 (5 terms) at guard 1e-28; degree 3 at 1e-28 has more margin. 8 points per depth only. Next: real timing in Rust against per-frame BLA. |
| 2026-10-07 | PROB-08: real timing. Is the all-double deep-pixel pipeline (biseries loops + Koenigs tail) faster than per-frame BLA on whole frames, with every pixel correct? | `bash tools/research/misiurewicz/koenigs_bench/run.sh target/release/fd OUT` (Rust bench, 480x270, 4 threads, best of 2-3 runs, six frames centred at C from 1e-35 to 2e-48, max_iter 20,000); manual CI: `gh workflow run koenigs-bench.yml` | **Keep (local, Windows, 20-core host).** Degree 4, guard 1e-28, 12 φ terms, R0 0.03, scored against `fd control --bla per-frame` on all 777,600 pixels: 0 wrong pixels (> 1e-3 px), 0 class mismatches, max 3.2e-4 px. The worst pixels were checked against 90-digit direct iteration: fd is exact (≤ 1e-11 px), so all residual error is the candidate's. Frame seconds, fd BLA / lean perturbation / Koenigs: 1e-35 0.327/0.505/0.033 (9.9x vs fd); 1e-38 0.551/0.577/0.048 (11.4x); 1e-40 0.923/0.673/0.036 (25.9x); 1e-43 0.482/0.802/0.044 (11.0x); 1e-46 0.497/1.080/0.044 (11.2x); 2e-48 0.569/1.457/0.040 (14.1x). Against the lean perturbation baseline (same Rust file, so the comparison isn't distorted by code quality): 12-36x. That baseline is slower than fd, so fd is not a bloated opponent. Candidate time is flat with depth; both opponents grow. **Full-frame kills:** degree 4 at guard 1e-26 (32-178 wrong pixels per frame, up to 0.14 px, which the 48-point probe missed); degree 2 at 1e-28 (up to 4,720 wrong, 0.5 px, though it passed 48 points); 1e-29 (up to 4 wrong). Degree 3 at 1e-28 also passes. Scope: frames inside the minibrot band centred at the zone nucleus; shallower frames (pixels farther than the guard from C) need another method. Constants are per zone (built once in Python in about 0.1 s) and shared by every frame. This is per-frame speed ("cheaper frames", DEC-15), adoptable by any renderer that knows the zone. |
| 2026-10-07 | PROB-08 on GitHub Actions: does the local result hold on a clean runner? | `gh workflow run koenigs-bench.yml -f threads=4 -f runs=5` (run 37672629317, ubuntu-latest, AMD EPYC 7763, 480x270, 4 threads, best of 5) | **Keep, confirmed.** CORRECTNESS PASS: 0 wrong pixels, 0 class mismatches over 777,600 pixels, max 3.2e-4 px. Frame seconds, fd per-frame BLA / lean perturbation / Koenigs: 1e-35 0.511/0.776/0.060 (8.5x vs fd); 1e-38 0.840/0.905/0.082 (10.3x); 1e-40 1.367/1.040/0.059 (23.4x); 1e-43 0.678/1.263/0.071 (9.6x); 1e-46 0.726/1.685/0.071 (10.2x); 2e-48 0.782/2.258/0.070 (11.2x). Against lean perturbation: 11.1-32.2x. The deep band is 8.5-23x faster than our BLA renderer, timed and exact, on frames centred at the zone nucleus. |
| 2026-10-07 | PROB-08 repeat runs on GitHub Actions: does the speed-up repeat, and does it hold on one thread? | `gh workflow run koenigs-bench.yml -f threads=4 -f runs=5` (run 37673459489) and `-f threads=1 -f runs=3` (run 37673455061); ubuntu-latest, 480x270 | **Keep, repeatable.** Both runs: CORRECTNESS PASS, 0 wrong pixels, 0 class mismatches over 777,600 pixels, max 1.1e-4 to 3.2e-4 px. Speed-up vs fd per-frame BLA at 1e-35/1e-38/1e-40/1e-43/1e-46/2e-48: 4 threads 8.7/10.9/25.1/10.1/10.6/11.7x (first run: 8.5/10.3/23.4/9.6/10.2/11.2x); 1 thread 8.1/10.5/24.9/9.6/10.2/11.2x. Single-thread frame seconds, fd → Koenigs: 1.05→0.13, 1.90→0.18, 3.18→0.13, 1.48→0.15, 1.60→0.16, 1.71→0.15. The gain is per-core, not a threading artefact. Against lean perturbation: 11-37x. |
| 2026-10-07 | Deep-research report: can the near-parabolic "seahorse gate" leftovers be skipped? (`docs/research/10-8-26/skipping-near-parabolic-transits-what-is-computable.md`) | Literature (Kapiamba, Lanford–Yampolsky, Petersen–Zakeri, Braverman) | **Lead, untested.** Exact skips exist: Kapiamba's q = 2 near-parabolic identity g^(2n+k) = χ∘T_(n−1/α_NP)∘ρ, and Koenigs at the weak α fixed point (\|λ\| = 1.00622). The report computes the canonical gate width as only about 22 raw iterations (1/α_NP ≈ 11.47 − 0.57i). It argues the hundreds of leftover steps are weak-repelling dwell near α, best skipped by a second Koenigs jump at α, with Buff coordinates (three logs plus an analytic correction) as a cheap gate model and Braverman long-iterates as a rigorous fallback. No published renderer uses Fatou/Lavaurs/horn maps per pixel. Caveat from our koenigs_leftover diagnostic: the long tails stay 0.1-0.5 from α without monotone spiralling, so they may be repeated gate passes rather than one dwell. Next cheap probe: count gate passes per long tail, and fit a Koenigs series at α (residual vs radius). |
| 2026-10-07 | PROB-08 lunch sweep on GitHub Actions: does the win hold at real resolutions, and which loop setting is cheapest? | `gh workflow run koenigs-bench.yml` with size/degree/guard inputs; runs 37675282913 (1280x720), 37675287111 (1920x1080), 37675291422 (deg 3, 1e-28), 37675295432 (deg 4, 1e-27), 37675299540 (deg 3, 1e-27), 37675303742 (960x540); 4 threads | **Speed holds at every resolution:** 7.4-24x vs fd per-frame BLA (1920x1080 per frame, fd → Koenigs: 8.1→0.94, 13.3→1.30, 21.7→0.92, 10.7→1.12, 11.5→1.13, 12.4→1.11 s). **Strict correctness slips at high resolution.** Degree 4 / 1e-28: 960x540 has 1-4 wrong pixels per frame (max 1.8e-3 px); 1280x720 has 15 at 1e-38 and 1 at 1e-46 (max 3.4e-3); 1920x1080 has 105 / 1 / 7 at 1e-38 / 1e-46 / 2e-48 out of 2.07M (max 4.4e-3 px). That is ≤0.005% of pixels, all under 0.005 px, invisible but over the 1e-3 bar; the worst depth is 1e-38. At 480x270: degree 3 / 1e-28 PASS (max 3.2e-4); degree 4 / 1e-27 PASS with smaller errors at most depths (max 7.4e-4); degree 3 / 1e-27 FAIL (up to 399 wrong, 0.098 px). More pixels expose rarer errors, as DEC-17 predicts. Next: find which stage causes the rare 1e-3 to 5e-3 px pixels (loop truncation, fixed-C tail, or Koenigs terms), re-run 1920x1080 with the fix, and give run.sh a TERMS input. |
| 2026-10-07 | FIX-20 on GitHub Actions: which setting causes the rare 1080p misses (105 of 2.07M pixels, max 4.4e-3 px)? | `gh workflow run koenigs-bench.yml -f size=1920x1080 -f runs=2` with `-f guard=1e-27` (run 37681537407), `-f terms=40` (37681541796), both (37681546005) | **Cause found: the 12-term Koenigs inverse.** 40 φ terms alone: CORRECTNESS PASS, 0 wrong over 12.4M pixels, max 2.8e-5 px. Guard 1e-27 alone: 20 wrong at 1e-38 (max 2.2e-3). Both: 6 wrong at 1e-38 (max 2.6e-3). Guard 1e-27 is rejected at 1080p; guard 1e-28 stays. 40 Newton terms are slow (1.7-2.1 s/frame), which the series ψ below fixes. |
| 2026-10-07 | PROB-09 step 1: are the post-jump tails one long dwell near α, or repeated gate passes? | `python tools/research/misiurewicz/gate_diag.py 12 0.03 300` (90,000 samples of the fundamental ring R0/\|ρ\| ≤ \|w\| < R0; the tail depends only on w) | **Neither, mostly.** Tail steps p50 42, p95 243, max 1409, mean 75. The top 10% of tails (mean 280 steps) spend only 4 steps within 0.05 of α and 16 within 0.1; they circulate 0.2-0.4 from α between α and the 2-cycle, and fall back within 0.03 of the 2-cycle point A about 3 times. A Koenigs jump at α could save at most about 5% of tail steps, so it was not built. **Profile (new `koenigs-loops`/`koenigs-jump` stage-cutoff modes):** the loops take 3-9 ms per 480x270 frame, the approach + jump about 80 ms (60% of the pixel), and the tail 30-80 ms. The jump's Newton ψ averages 8.6 iterations, and 12% of pixels hit the 30-iteration cap because the 1e-16 stop test is at double roundoff. |
| 2026-10-07 | PROB-09: ψ = φ⁻¹ as a series (series reversion of the 40-term φ) instead of Newton | `python tools/research/misiurewicz/psi_series.py 0.03`; bench `PSI=18`; CI `-f psi=18` (480x270 run 37682509050; 1920x1080 run 37682500239) | **Keep.** 18 terms reach a relative error of 2e-16 at \|w\| = 0.03; one Horner replaces about 17 φ evaluations. 480x270, 4 threads: CORRECTNESS PASS, max 1.7e-6 px (was 3.2e-4); fd → pipeline s: 0.509→0.032 (15.9x), 0.839→0.055 (15.3x), 1.367→0.031 (44.6x), 0.678→0.043 (15.9x), 0.725→0.044 (16.6x), 0.781→0.042 (18.6x); 1.5-1.9x faster than the Newton pipeline. **1920x1080: CORRECTNESS PASS, 0 wrong over 12.4M pixels, max 2.8e-5 px** (fixes FIX-20); 16.0-47.7x vs fd (0.49-0.87 s/frame, was 0.92-1.30). With guard 1e-27 it still fails (run 37682504736: 6 wrong at 1e-38). A larger jump radius is not worth it: ψ needs 24 terms at 0.05 and 40 at 0.1, costing about what it saves. |
| 2026-10-07 | PROB-09: tail patch atlas. F_n(w) = f_C^n(A + ψ(w)) is entire in w, unlike ν (PROB-04), so store it exactly: a quadtree over s = log w on the fundamental ring, with a degree-16 Taylor patch per leaf for the largest n that fits | `python tools/research/misiurewicz/tail_patches.py 16 1e-11 6 6 rel CONSTS` (patch error bounded as an s-shift: \|δz\| ≤ 1e-11·\|dF/ds\|; measured \|ds/dpixel\| ≥ 5e-6 at 480x270, so at most 1e-5 px at 1080p); bench reads the tree from the constants; CI `-f psi=18 -f patch=6` | **Keep, modest.** Absolute tolerance 1e-14 stalls at a mean tail of 37 steps whatever the depth; the s-shift tolerance keeps improving. Mean tail steps to \|z\| > 2 (from 69.4; p50 37, p95 225): 596 leaves → 29.5; 16,190 leaves (depth 6, 14 s build, about 4 MB) → 23.6 (p50 2, p95 137); 130,652 leaves (depth 8, 172 s) → 22.0. No invalid leaves. Local 1 thread, 480x270, s/frame, series ψ → + depth-6 patches: 0.067→0.050, 0.122→0.075, 0.064→0.049, 0.092→0.065, 0.094→0.066, 0.087→0.067; every pixel passes. CI 480x270 (run 37683668679, a faster runner): CORRECTNESS PASS, max 2.7e-6 px; 16.3/20.5/48.1/18.7/19.4/20.3x vs fd (series ψ alone: 15.9/15.3/44.6/15.9/16.6/18.6x). The patch atlas is per zone and shared by every frame, the first true atlas win in the DEC-15 sense, though small. Not tried: folding the 23 approach steps and φ into one series in δ (needs 24 terms; saves about 11 operations per pixel). |
| 2026-10-07 | PROB-09 at 1080p on GitHub Actions: does the tail patch atlas (with series ψ) hold on every pixel of full-HD frames, and what does it buy? | `gh workflow run koenigs-bench.yml --ref ishoo/PROB-09 -f size=1920x1080 -f runs=2 -f psi=18 -f patch=6` (run 37683672292) and the same with `-f terms=20` (run 37683675710); 4 threads | **Keep.** CORRECTNESS PASS: 0 wrong pixels, 0 class mismatches over 12.4M pixels, max 7.4e-5 px. Frame s, fd per-frame BLA → pipeline: 8.07→0.41 (19.8x), 13.32→0.56 (23.7x), 21.73→0.40 (54.2x), 10.73→0.48 (22.2x), 11.50→0.51 (22.7x), 12.38→0.52 (23.9x). Against series ψ alone at 1080p (run 37682500239: 0.51/0.87/0.49/0.68/0.70/0.67 s): 1.2-1.6x. Against this morning's Newton pipeline (0.94/1.30/0.92/1.12/1.13/1.11 s): 2.1-2.5x. 20 φ terms change nothing, so 12 stays. |
| 2026-10-07 | BENC-04: head-to-head against the community's BLA renderer. How does our deep pipeline compare with fraktaler-3 3.1 (mathr's successor to Kalles Fraktaler: perturbation + BLA, official Windows build, benchmarked wisdom) on the same machine? | `.github/workflows/f3-bench.yml` → `tools/research/misiurewicz/koenigs_bench/f3_bench.sh` (windows-latest, 4 cores, all three renderers at 4 threads/all cores; fraktaler-3 `--batch` with 20,000 iterations, escape radius 1e10, max perturb/reference/BLA steps 20,000); run 37688451277, 480x270, best of 3 | **Ours is 23-29x faster than fraktaler-3.** Seconds per frame, fraktaler-3 / fd per-frame BLA / ours: 1e-35 0.718/0.598/0.026; 1e-38 0.824/0.939/0.035; 1e-40 0.749/1.492/0.025; 1e-43 0.764/0.764/0.031; 1e-46 0.794/0.821/0.031; 2e-48 0.826/0.879/0.031. fraktaler-3's fixed per-process cost (16x9 frame) is 0.063 s, so the ratio excluding it is 21-27x. **fd's per-frame BLA is as fast as fraktaler-3** (within ±25%, faster at 1e-35, slower at 1e-40), which confirms fd as a fair opponent for all earlier results. **Agreement:** our bench sampled fraktaler-3's own jittered points (its hybrid.h jitter reproduced in Rust; matching variant: frame 0, rows bottom-up). 0 class mismatches on all frames; median disagreement 2-8e-6 px, p99 ≤ 3.3e-5 px. 0-58 points per frame differ by up to 5.4e-3 px as scored with fraktaler-3's DE, but 90-400-digit checks (`truth_f3.py`) show those points have a true distance to the boundary of 1e-9 to 1e-16 px: a last-bit change in the sample position moves ν by hundreds of iterations there, and fraktaler-3's DE is off by 10³-10⁸ at them. They are ill-conditioned measurement points, not errors in either renderer. Our pipeline vs fd on the same runner: CORRECTNESS PASS. |
| 2026-10-07 | BENC-04 at 1920x1080: does the lead over fraktaler-3 hold at full HD, where its 0.07 s process start-up is negligible? | `f3_bench.sh` via f3-bench.yml, run 37689189967 (windows-latest, 4 cores, best of 2) | **Yes: ours is 22-28x faster than fraktaler-3.** Seconds per frame, fraktaler-3 / fd per-frame BLA / ours: 1e-35 12.04/10.63/0.455 (26.5x); 1e-38 14.03/16.83/0.626 (22.4x); 1e-40 12.70/26.62/0.447 (28.4x); 1e-43 12.95/13.78/0.532 (24.3x); 1e-46 13.25/15.26/0.550 (24.1x); 2e-48 13.84/15.86/0.556 (24.9x). Our pipeline vs fd on the same runner: CORRECTNESS PASS, 0 wrong of 12.4M pixels, max 1.7e-4 px. Against fraktaler-3 at its jittered points: 0 class mismatches; 555-5,324 points per frame (0.03-0.26%) exceed 1e-3 px when scored with fraktaler-3's DE (max 3.4e-2). 120-digit checks of the 18 worst points, at the exact double-rounded sample position, show they all lie within 5e-9 px of the boundary (true DE 5e-18 to 5e-9 px). With the true DE, both renderers' displacement errors there are negligible (fraktaler-3 ≤ 4.2e-6 px, ours ≤ 1.6e-7 px); the large values come only from fraktaler-3's DE, which is wrong by orders of magnitude at such points. Follow-up: score with a reliable DE (export ours) rather than fraktaler-3's. |
| 2026-10-07 | PROB-12 seed test: is the v0 zone special? How much of a minibrot's nucleus orbit circles one repelling cycle (the part a Koenigs jump skips)? | `python tools/research/misiurewicz/zone_seed.py 20 1` (about 20 s: the v0 centre, the classic valley centre, 20 random near-boundary points; atom-domain period → nucleus Newton at 120 digits → longest run where the orbit repeats after r steps while repelling, for every r < p/2) | **v0 is the strong case; random minibrots are not.** v0: p 764, 683 steps (89%) within 1e-2 of the 2-cycle from step 24 = **341 laps** (\|λ\| 1.153); D-ranking also recovers q 24, r 2, distance 1.71e-25. Other minibrots (p 37-1601): the longest repeat runs are 0-503 steps, but only 0-4.7 laps of the cycle (best rnd13: 503 steps of a 108-cycle; valley 503 of 429; most < 2 laps). With r ≤ 8 only, every random case had 0 dwell. Reading: laps ≈ ln(minibrot size / distance to the Misiurewicz point)/ln\|λ\|, so the fast path pays off for minibrots parked near a spiral centre (as v0 was chosen, and as spiral-following art zooms do), not for arbitrary minibrots. The D-ranking picks spurious (q ≈ p/2, \|λ\| < 1) relations at random minibrots; the dwell/lap measure is the robust one. Next (PROB-12): measure laps band by band along real zoom paths. |
| 2026-10-07 | PROB-12 seed test: along whole zoom paths, which depth bands have the v0 structure? | `python tools/research/misiurewicz/path_laps.py NAME RE IM -48` (2.1 s for both paths: per 3-decade band, the lowest-period minibrot in view, i.e. smallest p with \|z_p/u_p\| < width; 150-digit Newton; laps = longest repelling-cycle repeat run / r) | **The v0 path has it at every depth from 1e-6 down.** Width → period, laps around the 2-cycle, skippable share: 1e-6 → 143, 40, 55%; 1e-9 → 241, 89, 73%; 1e-12 → 337, 137, 81%; 1e-15 → 435, 186, 85%; 1e-18 → 531, 234, 88%; 1e-21 → 627, 282, 90%; 1e-24 → 731, 332, 91%; 1e-27 to 1e-48 → 764, 342, 89%. Every band's minibrot circles the same r = 2 cycle (M(24,2)), with period growing about 96 per band. So the Koenigs series and the tail patch atlas (per 2-cycle) could serve 45 of the film's 49 decades from one build, with only the minibrot loop map changing per band (PROB-08/09 tested 1e-35 to 2e-48 only). **The classic valley path** (`bench/path-valley.txt` centre, 36 digits): p 35, 78 (0 laps), then p 998 with 1.2 laps from 1e-9 to 1e-15; no minibrot below p 5,000 deeper. It is not aimed at a spiral centre and would gain little. |
| 2026-10-07 | PROB-12 seed test: can shallower v0 bands reuse one fixed-parameter tail (Koenigs + patch atlas built once)? | `python tools/research/misiurewicz/shared_tail.py N` (80-digit truth vs the same orbit with the last T steps run at a shared parameter; error \|Δν\|·ln2·DE in 480-px frames). 1st run: 40 pixels/band, own band nucleus vs C764, T 100/300/600 (86 s). 2nd run: 16 pixels, C764 with and without a first-order c-correction d' = 2zd + (c − C), T 300/600/1000 (53 s) | **The reach of a shared tail shrinks with shallower bands; a first-order c-correction extends it about 1000x where it applies.** Max px error, swap the last T steps: 1e-24: T 100 0, 300 1.6e-11, 600 5.6e-3 (own c_H) / 2.3e-3 (C764). 1e-15: T 100 1.7e-9, 300 2.4e-3 (own) / 1.4e-3 (C764), 600 fails. 1e-9: T 100 3.0e-3 (own) / 1.0e-3 (C764), 300+ fails. The band's own nucleus is no better than C764 (the frame's pixels span the width either way). With the correction: 1e-15 T 300 → 1.2e-6 px (0 of 16 over 1e-3); T ≥ 600 still fails (non-escaping or wrong orbits); 1e-9 fails at every T. Reading: from about 1e-15 down, one tail plus a dF/dc derivative patch per atlas leaf is plausible if the tails there are ≲300 steps (to check); shallower bands need per-band tails or plain BLA, and are cheap anyway. These probes are slow (mpmath); write the next ones in double. |
| 2026-10-07 | PROB-12 seed test (double, < 1 s per band): how long are the tails in shallower v0 bands, and do they fit the ~300-step reach of a shared corrected tail? | `python tools/research/misiurewicz/tail_length.py WIDTH PERIOD 4000` (double perturbation against the band's nucleus, Zhuoran rebasing; tail = steps from the last close return \|z\| < 1e-3 to escape) | **No: a single shared tail covers only the deep end.** Tail steps median / p90 / max, share ≤ 300: 1e-15 (p 435): 465 / 572 / 1281, 0%; 1e-24 (p 731): 750 / 814 / 2178, 0%; 1e-40 (p 764): 420 / 542 / 1269, 0% (but there pixels are within 1e-35 of C, so a fixed-C tail is exact). In the 1e-15 and 1e-24 bands most pixels make no close return at all: they escape within about one period. This corrects FIX-23's "one build could serve 45 of 49 decades". Mid bands need parameter-dependent tails beyond first order (a real research item), or BLA. |
| 2026-10-07 | PROB-12 seed test on a famous published zoom: how much of Maths Town's "Eye of the Universe" (plain Mandelbrot, zoom 3.4e1091, ~17M iterations; location from maths.town/videos/eye-of-the-universe-video) would the spiral-centre Koenigs jump skip? | `python tools/research/misiurewicz/orbit_laps.py N RMAX 4 1e-3 - RCAP` (centre orbit at 1,191 digits, then numpy scan for runs where the orbit repeats after r steps for ≥ 4 laps around a repelling cycle). 20k steps: 1.2 s; 2M steps: 103 s; one minibrot period (160k steps, r ≤ 400 plus record periods below 150k): 8 s | **Modest there: about 24% of steps, against 89% on v0.** Record returns (nested minibrot periods): 5, 64, 197, 655, 2217, 19205, 57190, 102716, 159413. Over 2M steps, 94% of the orbit is 11.5 laps of the centre's own period-159,413 minibrot: the minibrot-loop structure NanoMB already exploits (and our loop map does), not the new spiral jump. Within one minibrot period, repelling short-cycle dwells of ≥ 4 laps cover 24.3% (largest: r 197 for 18.6 laps at \|λ\| 3.3; r 655 for 4.3 laps at \|λ\| 9.5, recurring). Reading: on zoom-doubling dives into nested minibrots the main skip is minibrot loops (prior art), and the Koenigs jump adds at most about 1.3x; the large new win is on spiral-centre dives like v0 (89% of steps, 341 laps). |
| 2026-10-07 | BENC-08 seed test: can the deep pipeline run in single precision (GPU speed)? | `python tools/research/misiurewicz/single_precision.py CONSTS FD_REF` (numpy reimplementation of the koenigs_bench pixel pipeline with a precision switch per stage, scored on all pixels of the six 480x270 PROB-08 frames against fd per-frame BLA; about 0.3-1 s per frame and variant) | **Not as-is, but most of it can be float32, and what must stay float64 is small.** Wrong px (> 1e-3) per frame, 1e-35 / 1e-38 / 1e-40 / 1e-43 / 1e-46 / 2e-48: A all f64 (sanity): 0/0/0 (= Rust bench). B all f32: 20,088 / - / 826 / - / - / 122,893 (median 150 px at 2e-48): fails. F f32 front, f64 finish: same as B, so the front end is the problem. G f32 with f64 glue (h0 = Z24-α + d, φ, log/arg, t): 104 / 17,856 / 53 / 929 / 123,704 / 122,896: the pixel offset (~1e-28) added to a ~1e-23 constant wiped it out in f32. L f64 loops + approach + glue, f32 patch polynomial and finish: 24 / 17,908 / 4 / 885 / 3,161 / 5,202 (median 1e-7 to 2e-4 px): the minibrot loop map needs f64 (its ~1e24-scale terms cancel near the minibrot). P all f64 except the f32 finish: 22 / 16,980 / 4 / 824 / 2,960 / 4,915, about the same as L, so the patch polynomial is fine in f32 and the remaining errors come from the plain escape steps. R as L with the finish as f32 offsets from a per-patch f64 reference orbit: worse (9,860-35,556 wrong; patch centres are too far from their pixels). Reading: float32 is safe for the patch polynomial; the loops (2-5 per pixel), the glue (a few ops) and the escape steps (median 2, long tail) need f64. That is roughly 600 f64 flops per pixel (estimate), so even a 1/64-rate consumer GPU is plausible. Measure it, don't assume it (BENC-08). |
