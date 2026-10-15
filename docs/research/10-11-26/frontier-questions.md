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
level. Numbering is kept so that answer files stay matched to their questions.

The remaining questions were then re-read against **north star v4** (`docs/spec/NORTH-STAR.md`,
set 2026-10-15) and **DEC-29**. Video is the main product: the fastest real deep-zoom films,
much longer and deeper, that viewers cannot tell from an exact render. Precision a viewer
cannot see is a cost. Films are judged by shaded frames: within 1 level of the exact render
on ≥ 99.9% of pixels, no extra flicker, nothing visible skipped. Exact renders (DEC-17,
1e-3 px) stay available for stills. Consequences:
- **Q6** is the purest test of "precision a viewer cannot see": it is now first, and is
  scored by DEC-29 directly.
- **Q2** is what makes films "much deeper and longer" beyond the one v0 ladder. It now also
  owns build cost per decade of film.
- **Q5** (live steering) is retired. The north star is films on a path known in advance,
  not interactive diving, and the build-cost part of Q5 moved into Q2.

| # | Question | Status | Answer |
|---|---|---|---|
| 1 | Parallel-in-time reference orbits | **killed** | [q1-parallel-in-time-reference-orbits.md](q1-parallel-in-time-reference-orbits.md): killed as a global method; certified segment Newton is the remaining lead |
| 2 | Nested minibrots: one operator hierarchy, interiors included | **open, rewritten** (absorbs Q3) | pending |
| 3 | Deep minibrot interiors in O(1) | **merged into Q2** | partial answer below |
| 4 | Two-level references for float32 pixels | **killed** (GPU follow-up measured) | [notes](../2026-10-14-q4-two-level-float32-references-notes.md): fallback 1.3–6% > 1% (kept error ≤ 3.1e-4 px); on real zone inputs the two-level hybrid is visibly worse than float-float and is parked; float-float alone is kept on the RTX 3050 Ti |
| 5 | Compiling ahead of a live dive | **retired** (outside north star v4; build cost moved to Q2) | — |
| 6 | Supersampling from jets | **passed, built** (FILM-15: `fd film --jet-aa on`) | [q6-supersampling-from-jets.md](q6-supersampling-from-jets.md): DEC-29 met everywhere, no added flicker; 2.1 samples/px over a v0 dive (2.9–3.6 on the densest frames); 1.84x faster on a BLA film; zone frames stay exact |

**Suggested order: Q6 (done, 2026-10-15), then Q2.**
- Q6 is CPU-only and cheap to probe, and it would cut the fractal samples of every film
  frame by up to 4x.
- Q2 is the biggest gap: today's fast path works for one spiral-centre ladder, not for the
  nested-minibrot dives that real zooms and our variety planner make.

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

**Decided from north star v4 (2026-10-15, Claude; record as Ishoo decisions when Ishoo is available):**

1. **The "exact render" in DEC-29 is the exact-mode render (DEC-17), not a higher-precision
   truth.**
   - Near edges, binary64 differs from a double-double finish by up to 236 levels: roughly
     0.1–0.4% of pixels per depth (23k–88k of 20.7M pixel-looks over 10 looks).
   - Every one of those pixels lies within 1e-3 px of the set. There, an orbit is chaotic at
     sub-pixel scale: moving the sample by 1e-3 px changes its colour as much as the
     precision does. The "true" colour of the exact sample point is therefore not something
     a viewer can see; it is noise either way.
   - The north star says precision a viewer cannot see is a cost. So exact mode stays
     DEC-17 as written, and binary64 films need no re-check against truth.
2. **The film default for the f32-only finish is plain float-float. The targeted re-run tier
   is built only if a measured flicker test fails.**
   - DE/normal failures are invisible (≤ 1 level beyond 0.01 px from an edge).
   - The ν jumps of plain float-float touch 0.01–0.02% of pixel-looks, inside DEC-29's 0.1%
     allowance.
   - What DEC-29 can still catch is flicker: single near-filament pixels jumping about 50
     levels between frames. Building the re-run tier before that is measured would buy
     precision nobody may see.
   - GPU-05 is already gated by film equivalence. Its gate must include a frame-to-frame
     flicker measurement on a deep film (v0 down to 2e-49), comparing the film from the
     float-float finish with the exact-mode film. If float-float adds flicker, build the
     re-run (λ = 16, Δν ≤ 0.01; triple-float on f32-only devices) and measure again.
3. **The DE/normal gate is not relaxed.**
   - Films do not use it any more (DEC-29).
   - Exact stills keep DEC-17's 1e-3 px gate unchanged, because "full numerical accuracy"
     is what exact mode promises.
   - The 0.1 px proposal from the shading proof is withdrawn.
4. **Q4's reopen option (notes §8.0) is closed: not reopened.** The two-level hybrid's
   errors are visible (up to 61 levels ≥ 1 px from an edge), so even the film bar rejects
   it.
5. **A race against the best existing renderers is needed, but as a benchmark, not a
   frontier question.**
   - "Faster than anything else can" is a claim the north star makes, and nothing measures
     it yet. BENC-10 races our own GPU against our own CPU.
   - It should be filed as a benchmark issue: the same films in Kalles Fraktaler,
     FractalShark and Imagina against `fd film`, timed on the owner's machines and checked
     against DEC-29.

---

**Shared context (paste first)**

> You are a research mathematician and numerical engineer with a compute sandbox (Python with mpmath/numpy/gmpy2, C/Rust if you like). The project is **Fractodactyl** (github.com/junovhs/fractodactyl): the fastest deep-zoom generator that shows the real Mandelbrot set and looks visually perfect to humans. Video is the main product: much longer and much deeper zoom films, made much faster than anything else can. Depth is never faked. Precision a viewer cannot see is a cost, not a feature.
>
> **The bars:**
> - **Films (DEC-29):** shaded frames within one 8-bit level of the exact render on ≥ 99.9% of pixels, with no extra flicker and nothing visible skipped, measured.
> - **Exact stills (DEC-17):** every pixel within 1e-3 px.
>
> **Before anything else, read `docs/spec/NORTH-STAR.md`, `docs/research/` and `docs/spec/METHOD.md` in that repo.** About 30 reports already cover standard deep-zoom prior art, certification, near-parabolic transits, transfer maps, the cycle compiler, factored ladders, exterior fields and GPU precision. Do not re-derive or re-summarise them. Start where they stop.
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
factor is now paid almost entirely in fractal samples, on every frame of every film.

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

**Scoring is DEC-29's film bar**: within one 8-bit level of the ground truth on ≥ 99.9% of
pixels, with no added flicker. Also break the result down with the visible-colour check
(8-bit levels by DE band over the 10 looks), so that failures can be located. Report:
- the fraction of pixels resolved from jets;
- the effective sample count per pixel;
- the error and flicker metrics;
- the extra cost of the jet in the zone path.

**Kill if** jet-AA fails DEC-29 on boundary-heavy deep views at any flag threshold that
keeps the effective sample count below 2.5 per pixel.

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

This is also what limits how deep and long a film can go. Zone builds take 10–14 s per
ladder rung at 1e-50 and 1e-100, but 158 s at 1e-1000 (PROB-17/19). If build cost grows
with depth like that, it becomes the film's cost. Interiors matter for the same reason:
interior pixels that are not classified run to the iteration cap, and they are the most
expensive pixels in a deep minibrot frame.

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

Also report **build cost per decade of film**, against the 10–158 s per-rung zone builds.

Compare whole 1080p frames against fd per-frame BLA at the same centre and width. Score
operators against DEC-17 (exact mode), and score shaded frames against DEC-29 (film mode).

**Kill if** any of these holds:
- building a child costs O(P_child) raw steps anyway;
- the composed operators lose more than 10 digits by level 4;
- a 3-level frame, build cost included and amortised over a film's frames, is not faster
  than per-frame BLA.

The interior part alone is killed if certification covers less than 80% of interior pixels.

---

## Answered, merged or retired (kept for the record)

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


**5. Compiling ahead of a live dive, inside the frame budget.** *Retired 2026-10-15.*
It asked whether every structure a steering user might dive toward could be discovered and
compiled within one octave of a live dive (about 0.6 s at 0.5 decades per second). North
star v4 makes films on a path known in advance the product, not interactive diving, so the
per-octave live budget no longer applies. The part that still matters, build cost per
decade of film and inheritance between nesting levels, is now in Q2. The evidence it would
have started from: zone builds of 10–14 s per rung (158 s at 1e-1000), AUTO-02 failing on
speed with no wrong pixel, and the variety planner finding 34 deep targets in 50 s.
