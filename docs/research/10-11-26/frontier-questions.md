# Frontier research questions (2026-10-11, revised 2026-10-15)

Written by Claude after reading every report in `docs/research/` (10-6, 10-8, 10-9, the
10-10 agent investigations, 10-10, shapes). They avoid what those reports already cover:
BLA/LA/AT/NanoMB prior art, certification, near-parabolic transits, transfer maps, structural
banding and the cycle compiler, factored ladders, cross-frame reuse, exterior fields, GPU
float-float and queues, Misiurewicz frame transfer, and the universal-landing torus. The owner
gives them to a research agent with a compute sandbox. Answers are filed in this folder, one
file per question (Q4's answer is the notes file linked below).

**Revision of 2026-10-15.** Q1 and Q4 are answered. Q3 is folded into Q2, because the
interior test it asked for comes from the same return operator and already works for one
level. Q5 and Q6 are rewritten from what the project learned since 2026-10-11. Numbering is
kept so that answer files stay matched to their questions.

| # | Question | Status | Answer |
|---|---|---|---|
| 1 | Parallel-in-time reference orbits | **killed** | [q1-parallel-in-time-reference-orbits.md](q1-parallel-in-time-reference-orbits.md): killed as a global method; certified segment Newton is the remaining lead |
| 2 | Nested minibrots: one operator hierarchy, interiors included | **open, rewritten** (absorbs Q3) | pending |
| 3 | Deep minibrot interiors in O(1) | **merged into Q2** | partial answer below |
| 4 | Two-level references for float32 pixels | **killed** (GPU follow-up measured) | [notes](../2026-10-14-q4-two-level-float32-references-notes.md): fallback 1.3–6% > 1% (kept error ≤ 3.1e-4 px); on real zone inputs the two-level hybrid is visibly worse than float-float and is parked; float-float alone is kept on the RTX 3050 Ti |
| 5 | Compiling ahead of a live dive | **open, rewritten**, after Q2 | pending |
| 6 | Supersampling from jets | **open, rewritten**, do first | pending |

**Suggested order: Q6, then Q2, then Q5.**
- Q6 is CPU-only and cheap to probe, and it would cut the fractal samples of every film
  frame by up to 4x.
- Q2 is the biggest gap: today's fast path works for one spiral-centre ladder, not for the
  nested-minibrot dives that real zooms and our variety planner make.
- Q5 needs Q2's build costs before its budget can be judged.

---

## What the answers so far established

**Q1, killed (2026-10-11).**
- Whole-orbit Newton with an affine prefix scan diverges from a double seed: at
  c = −2 + 2^-256 the residual goes from 2^-256 to 2^+1015 after one sweep.
- The guard bits needed grow with orbit length, and one full-precision sweep already costs
  5.4–5.5x sequential MPFR.
- Still open, speculative: certified Newton on short segments whose entry states are
  already known.

**Q4, killed under its criterion (2026-10-14), then measured on GPU (2026-10-15).**
- **Part A.** Tile references remove the view-size error exactly (error ∝ tile size), but
  1.3–6% of pixels need the fallback. The cause is a floor that no reference layout
  changes: in minibrot views 1% of pixels lie within about 8e-10 px of the set.
  Precision is the lever, not structure.
- **Engineering facts kept:**
  - orbits re-read by a float32 kernel must carry their own exponent and be stored hi+lo;
  - WGSL compilers on NVIDIA, Intel and RADV reassociate away TwoSum error terms unless
    every intermediate is made opaque;
  - RADV splits `fma`, so float-float TwoProd must self-test and fall back to Dekker.
- **Part B and the GPU run** (real v0-zone finish inputs, six depths, 1080p):
  - **Speed.** On the RTX 3050 Ti the float-float finish is 2.20–2.84x faster than the
    binary64 finish over the whole stage (GPU-05 KEEP). On the RX 5700 XT it is slower
    (0.70–0.84x), so native f64 stays the faster option there. It also runs on the
    f32-only Iris Xe.
  - **Parity.**
    - Under the current contract it fails DE/normal on 27–410 px per frame at four
      depths. All of these pixels are under 1 px from an edge.
    - One near-cap chaotic pixel flips class at 2e-49.
  - **Shading proof** (10 looks against a double-double truth):
    - Beyond 0.01 px from an edge, DE/normal errors change colours by at most 1 level.
    - The visible risk is ν instead: about 50-level jumps on 0.01–0.02% of near-edge
      pixels that pass the written ν rule.
    - Float-float plus a targeted re-run (0–0.84% of pixels) matches truth to within 1
      level beyond 0.01 px.
  - **The two-level hybrid changes up to 61 levels even ≥ 1 px from an edge, so it is
    parked.**

**Q3, partial answer from existing work (not a research run).**
- Inside a compiled zone, interior is already decided from the return operator. The zone's
  return-map test plus a q ≤ 4 attracting-cycle Newton check cut unresolved pixels at v0
  frame 749 from 34,002 to 6,129 of 518,400.
- The remainder is the minibrot's near-parabolic rim. FIX-35 makes uncertified near
  returns decline as Unresolved instead of claiming Interior.
- What is still missing:
  - interiors of minibrots that have no compiled zone. Per-frame BLA leaves them all
    Unresolved (FIX-04), and the variety planner's deep targets are such minibrots.
  - the rim.

Both are now part of Q2.

**Owner decisions these results are waiting on (not research):**
- Relax the DE/normal gate from 1e-3 px to 0.1 px (10x headroom per the shading proof).
- Ship float-float plus a targeted re-run, rather than plain float-float, as the f32-only
  finish.
- Add a visible-colour check to the test gate. The ν rule lets about 50-level changes
  through near edges.

---

**Shared context (paste first)**

> You are a research mathematician and numerical engineer with a compute sandbox (Python with mpmath/numpy/gmpy2, C/Rust if you like). The project is **Fractodactyl** (github.com/junovhs/fractodactyl): a Mandelbrot deep-zoom renderer. The goal is real-time diving into the set at 1080p, 0.05–0.5 decades of zoom per second, continuing ever deeper, where every pixel is the true set at that location (shortcuts must be exact or carry an error bound ≤1e-3 px).
>
> **Before anything else, read `docs/research/` and `docs/spec/METHOD.md` in that repo.** About 30 reports already cover standard deep-zoom prior art, certification, near-parabolic transits, transfer maps, the cycle compiler, factored ladders, exterior fields and GPU precision. Do not re-derive or re-summarise them. Start where they stop.
>
> **What already works:**
> - Perturbation plus skip tables.
> - The zone fast path, built for one spiral-centre ladder (v0, near Misiurewicz point M(24,2)):
>   - a minibrot return map, an exact Koenigs jump over the spiral-out, and a short finish;
>   - rendered at about 1e-50 and 1e-100 at 17–24x over per-frame BLA, and at 1e-1000 at about 1,000x over plain scaled perturbation, with 0 wrong pixels.
> - A factored return operator for additive Misiurewicz ladders.
> - Interior classification from the return map, inside a compiled zone.
> - One keyframe per 2x of zoom.
> - GPU: a hand-written WGSL zone kernel on wgpu (binary64, CPU fallback), plus a float-float finish for f32-only devices.
> - A variety planner that finds deep minibrot targets by atom domain plus Newton.
>
> **Already killed, do not retry:**
> - whole-orbit parallel-in-time Newton (Q1);
> - two-level float32 tile references (Q4);
> - an interpolated exit table (PROB-04);
> - one shared fixed-C tail for shallow bands (PROB-12).
>
> **Hardware:**
> - CPU: Ryzen 9 3900X.
> - RX 5700 XT (FP64 is 1/16 of FP32; native f64 beats float-float here).
> - RTX 3050 Ti laptop (FP64 is 1/64; float-float is 2.2–2.8x faster than f64 here).
> - Intel Iris Xe (no f64).
>
> **Method:** skim prior art fast and from any field, then invent, implement small prototypes, run them, and report numbers with code. State a kill criterion before each experiment and honour it. Bring back ranked leads, each with:
> - the mechanism;
> - your measured evidence;
> - the projected speed-up and what it is relative to;
> - the cheapest next experiment that would confirm or kill it.
>
> Label speculation as speculation.

---

## Open questions

**6. Supersampling from jets: one exact sample plus derivatives instead of 4 samples per pixel.**
Films render 2×2 supersamples per pixel (ss 2) plus 2x-oversized keyframes. Shading is no
longer the bottleneck: GPU-06 shades a 1080p frame in 7.4 ms on the RX 5700 XT. So that 4x
factor is now paid almost entirely in fractal samples, on every frame of every film and on
every live keyframe.

**Lead:** each sample carries its Taylor jet in c:
- ν, ∇ν, and second derivatives from the d²z/dc² recurrence, which is cheap alongside z and
  dz/dc;
- the shading function integrated analytically over the pixel footprint;
- true supersampling only for pixels flagged because the jet's validity radius is smaller
  than the footprint (filaments, boundary).

**Questions:**
- What fraction of pixels can be resolved from one sample plus a jet?
- Which jet order is needed?
- What is the cheapest reliable flag for pixels that need true supersamples?
- Does the jet survive the zone fast path? The Koenigs jump and the return map must carry
  the second derivative too. The cost of that is part of the answer.

**Raid:** analytic and prefiltered antialiasing, ray differentials, texture-space derivative
filtering, interval shading, and distance-estimate antialiasing.

**Build:** a small renderer using the project's own shading (`fd-shade`, the studio looks)
on captured zone outputs. Compare jet-AA against 4×4 supersampled ground truth on 5 deep
views and a short zoom sequence that tests flicker.

Ground truth near edges needs more than binary64. Binary64 differs from a double-double
finish by up to 236 colour levels within 1e-3 px of the set. Use a double-double finish
there, as the 2026-10-15 shading proof did
(`tools/research/twolevel/zone_finish/capture/src/bin/shadecmp.rs`).

**Scoring:** use that same visible-colour check: 8-bit levels by DE band over the 10 looks.
Also use the film check: at most one 8-bit level of difference on 99.9% of pixels outside
noise, with no added flicker. Report:
- the fraction of pixels resolved from jets;
- the effective sample count per pixel;
- the error and flicker metrics;
- the extra cost of the jet in the zone path.

**Kill if** fewer than 70% of pixels pass on boundary-heavy deep views, or the effective
sample count stays above 2.5 per pixel.

**2. Nested minibrots: one operator hierarchy for tuning chains, interiors included.**
The zone fast path wins on v0 because v0 is a spiral-centre dive: 89% of its orbit is 341
laps of one repelling 2-cycle. That is not the general case.
- **Random minibrots** show 0–4.7 laps (PROB-12 seed tests).
- **On Maths Town's "Eye of the Universe"**, 94% of the orbit is loops of the centre's own
  period-159,413 minibrot. Its record returns form a nesting chain: periods 5, 64, 197,
  655, 2217, 19205, 57190, 102716, 159413.
- **The variety planner's best film** dives into a period-2535 minibrot at 7.6e-29, and fd
  reports deep minibrot interiors as Unresolved.

Nested minibrots are what deep zooms are made of, and the compiler builds one loop level.

**Lead:** under renormalization, a child minibrot's return map is the parent copy's own
period-m return in renormalized coordinates. Build the child's bivariate return operator
(state × parameter) by composing the parent's operator m times in normalized coordinates,
instead of iterating P·m raw steps at high precision.

The same operator gives the multiplier λ_P(c), and the tuned copy's interior is exactly
|λ_P(c)| < 1. So per-pixel interior classification at every level comes from the hierarchy
itself. Today this works for one level inside the v0 zone, where 6,129 of 518,400 pixels at
frame 749 stay Unresolved, all on the near-parabolic rim.

**Questions:**
- Can the child operator be built without O(P_child) raw steps?
- How do conditioning and polynomial degree evolve across 3, 5 and 10 levels?
- Is per-pixel cost O(levels), meaning about O(log depth)?
- Can interior membership be certified per pixel or per tile from the hierarchy, with
  boundary pixels falling back?
- How much of the near-parabolic rim can be closed with a transit model (see the 10-10
  near-parabolic report)?

**Raid:** Douady–Hubbard tuning, internal addresses, NanoMB2's unfinished multi-level chain
(the reports cover it), renormalization-group numerics (Feigenbaum fixed-point
computations), composition of truncated power series, the multiplier map as a conformal
isomorphism onto the disk, and certified Newton (Krawczyk).

**Build:** two real chains:
- the Eye of the Universe record periods, as far down as the sandbox allows;
- one variety-planner target (`tools/research/variety/`) found by atom-domain search.

Report per level:
- build cost against direct construction;
- coefficient growth;
- the validated domain;
- returns per pixel;
- state and derivative error against high-precision iteration;
- the share of interior pixels certified, and the rim left Unresolved, with misclassifications
  (must be zero).

Compare whole 1080p frames against fd per-frame BLA at the same centre and width.

**Kill if** any of these holds:
- building a child costs O(P_child) raw steps anyway;
- the composed operators lose more than 10 digits by level 4;
- a 3-level frame is not faster than per-frame BLA.

The interior part alone is killed if certification covers less than 80% of interior pixels.

**5. Compiling ahead of a live dive, inside the frame budget.**
The planner and the zone compiler work offline, along a known path. Live steering means the
user may head anywhere in the view, so the next octave's operators must already be ready or
quick to finish. That covers nucleus references, Koenigs charts and return maps for every
minibrot and spiral that could become the target.

What is measured so far says compile cost is the constraint:
- **A zone build takes 10–14 s per ladder rung at 1e-50 and 1e-100, and 158 s at 1e-1000**
  (PROB-17/19). At 0.5 decades per second an octave lasts about 0.6 s, so even the cheap
  builds are 17–23x over budget.
- **The blind automatic-compiler corpus (AUTO-02) failed on speed** with no wrong pixel:
  discovery and build cost more than they saved.
- **The variety planner found 34 deep targets in 50 s.**

**Quantify on real locations:**
- How many distinct structures per octave are large enough to matter, for example an atom
  domain over 8 px or a Misiurewicz domain over 8 px, at depths 1e-5 to 1e-100?
- What does it cost to discover and compile each one, using Q2's hierarchy where it applies?
- How much of octave k+1's catalogue is derivable from octave k's? Children of known nuclei,
  rungs of known ladders, and incrementally extended references all count.
- Does a per-octave budget hold: about 0.6 s at 0.5 decades per second, on 12 CPU cores plus
  the GPU?
- If it does not, what is the fallback during the build (per-frame BLA, a coarser keyframe),
  and how often does the user see it?

**Raid:** atom-domain and box-period scans, Misiurewicz domains, speculative execution and
branch prediction, game-engine streaming, and anytime algorithms.

**Invent** the incremental catalogue update rule. Then measure the catalogue size and compile
cost on 5 dives of 30 decades each, including the Eye of the Universe path and two variety
planner films.

**Kill if** the catalogue grows faster than linearly in depth, or a full octave's compile
exceeds 3x the budget even with inheritance.

---

## Answered or merged (original text, kept for the record)

**1. Parallel-in-time reference orbits: can a million-step, million-bit orbit be computed in log time?**
The high-precision reference orbit is strictly sequential, and at extreme depth it becomes the bottleneck. FractalShark reports it dominating from about 1e10000 down.

**Lead to attack:** treat the whole orbit z_0..z_N as one unknown and solve F(z)=0 (z_{n+1} − z_n² − c = 0) by Newton. The Jacobian is bidiagonal, so each correction δ_{n+1} = 2z_n δ_n − r_n is a linear recurrence. Affine maps compose associatively, so each Newton step is a parallel prefix scan. Quadratic convergence could double the precision each step: start from a double-precision orbit and reach p bits in log2(p/53) parallel sweeps. For a periodic nucleus reference, the multiple-shooting version is natural.

**Kill if** the scan needs full precision at every step and total work exceeds the sequential cost by more than 4x. *Killed; see the answer file.*

**3. Deep minibrot interiors in O(1) per pixel, certified.** *Merged into Q2.*
Inside a deep minibrot every pixel ran to the iteration cap: at 1e-90, period 764, every
interior pixel came back "unresolved". The lead was to decide interior membership from the
multiplier map λ_P(c), evaluated from the parameter-space return jet, with certified error
and boundary fallback, and to shade interiors from the same jets. Kill was below 80%
certified at 1080p. The zone's return-map test now does this for one level (partial answer
above); Q2 carries it to every level.

**4. Two-level references so per-pixel arithmetic can be float32 on a consumer GPU.**
Already measured in the project: naive float32 fails the 1e-3 px bar, and a guarded float32 finish only ties FP64 on the RX 5700 XT. The question is structural, not about precision tricks.

**Lead:** compute one FP64 (or exponent-extended) secondary reference per tile, say 16×16 pixels, as a perturbation off the global high-precision reference. Then pixels run in float32-with-exponent relative to their tile reference. Their deltas are about 1/100 of the view, so absolute rounding error shrinks proportionally, while the FP64 work is amortized over 256 pixels.

**Kill if** more than 1% of pixels need the FP64 fallback or the worst error exceeds 1e-3 px. *Killed; see the notes file.*
