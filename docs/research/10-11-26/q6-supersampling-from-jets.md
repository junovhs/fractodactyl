<!-- Research answer to question 6 of frontier-questions.md, 2026-10-15. Claude, a cloud session with a compute sandbox (4 CPU cores, no GPU). Code: tools/research/jetaa/ (README there).  DRAFT: the minibrot x10 and seahorse-valley rows of §3.1 are still pending. -->

# Q6: supersampling from jets

**Verdict:**
- **Kept, as a lead.**
- **Killed under the kill criterion as literally written, on the densest frames only.**

Jet-AA rebuilds three of a pixel's four ss 2 samples from one real sample. It passes DEC-29
against the exact ss 2 render on every probed view and on every full 1080p v0 zone frame:
the worst look has at most 0.02% of pixels over 1 level, with no added flicker.

It costs 1.0 to 2.5 fractal samples per pixel instead of 4 on all probed content except the
densest. There it needs 2.9 to 3.6: the v0 frames at 5e-49 and 2e-49, next to the p764 minibrot, and
the p764 spiral crop. So the literal kill line ("fails DEC-29 at every threshold that keeps
the effective sample count below 2.5") is crossed on those frames.

The cause is not the jets. On those frames 40–50% of pixels are within 0.3 px of the set.
There the exact ss 2 colour is sampling noise: two equally exact 4-sample patterns with the
same pixel centre (grid and rotated grid) disagree by more than 1 level on 24–53% of
pixel-looks. Only real samples reproduce one particular noise draw. So the saving is bounded
by boundary density, not by jet accuracy.

**Nothing is ever lost:** a pixel that needs all its samples costs exactly 4, as today.

**Result in one line.** Jets of order 3 come for free from dz/dc, which films already
compute. Holomorphy lets a 4-neighbour stencil of F' = (log z_n)'/2^n recover F'' and F'''
to high order. With a two-sided consistency check this rebuilds ss 2 within DEC-29 at
1.0–2.5 samples per pixel on typical deep content, and as a late pass over stored `.fds`
columns. That is a 1.6–4x cut in film fractal samples, measured by frame, not projected.

## 0. Filing checklist (for the filing agent)

1. Set row 6 of the table in `frontier-questions.md` to: "**kept as a lead; literal kill on
   the densest frames** (eff 2.9–3.6 on v0 5e-49 and 2e-49 / p764 spiral, 1.0–2.5 elsewhere; DEC-29
   met everywhere, no added flicker), see q6 answer".
2. Owner decision needed (§6): read the kill criterion per frame or per film, and/or make
   DEC-29 noise-aware in dense regions.
3. File the ranked leads in §7 as issues. Lead 1 (FILM: jet-AA corner scheme as a refine
   pass) is the build candidate.
4. Add the METHOD.md results-log entry (text in §8).

## 1. What was built and how it was scored

- **Own perturbation renderer** (`render`):
  - exact-centre reference orbit (gmpy2, stored hi+lo);
  - binary64 deltas with rebasing, escape radius 1e10 as films use;
  - derivatives dz/du, d²z/du², d³z/du³ with u in output pixels.
  - For each pixel of a 480x270 crop at the centre of a 1920x1080 frame (1080p pixel
    scale), it stores:
    - the centre sample with its third-order jet;
    - the four exact ss 2 samples (binary64, production arithmetic);
    - the 4x4 truth, re-run in double-double where DE < 0.01 px, as the 2026-10-15 shading
      proof did.
- **Views** (`tools/research/jetaa/views.txt`):
  - five Misiurewicz shape views at 2e-58..2.5e-56: dendrite, tendril, whirlpool, galaxy,
    hook;
  - v0 at 1e-30;
  - the p764 minibrot near v0 (spiral crop at 4x size);
  - p8007 (all interior);
  - three Newton-refined minibrots framed at 10x their size (p764, p2217 near the Eye,
    p1111 at 4e-59);
  - seahorse valley at 1e-28.
- **Zone path end to end** (`zonejet`):
  - fd's own `render_zone` renders the v0 frame at full 1920x1080 ss 2 (`--iter 100000`)
    at 13 depths from 1e-29 to 2e-49;
  - jet-AA then rebuilds three samples per pixel from the stored columns of the fourth.
- **Shading:** fd-shade, film appearance (aa on, unresolved drawn as interior; dither off
  so differences are not masked). Nine looks: umber, palette, relief, ice, coral, steel,
  smoke, zebra, master-ice.
- **Score (DEC-29):** the share of output pixels whose max channel differs by more than 1
  level from the exact ss 2 render, per look. "Worst" is the worst look; the bar is ≤ 0.1%.
  Effective samples per pixel ("eff") counts every real sample. "work" weights them by
  iterations: the samples that get rejected are near the set and longer than average.

## 2. The mechanism

**Jet of log z.** For an escaped sample with z = z_n, let ℓ = log z_n and w = z'/z, all
derivatives in u (output pixels).
- ν = n + 1 − log₂(Re ℓ / ln 2).
- DE = 2 Re ℓ / |w| px.
- The screen normal is the direction of (Re w, Im w).

At a sub-pixel offset δ (u = dx − i·dy), the expansions are:
- ℓ(δ) = ℓ + wδ + w'δ²/2 + w''δ³/6;
- w(δ) = w + w'δ + w''δ²/2.

Each virtual subsample gets ν, DE and normal from these and is shaded by the unchanged
look. "Virtual supersampling" was chosen over analytic integration of the look over the
footprint because the looks depend on `ss` (band limiting, relief fade, crevice, haze). Their
filtered colour is defined by averaging their own per-sample output, so the exact ss 2
render can only be matched by evaluating the look at the four sample points.

**Jet accuracy at the ss 2 offsets (|δ| = 0.35 px)**, tendril view (`check`), in ν units,
by the centre sample's DE:

| centre DE | order 0 (copy) p99 | order 1 p99 | order 2 p99 | order 3 p99 | order 3 max |
|---|---|---|---|---|---|
| ≥ 3 px | 0.26 | 1.8e-2 | 1.3e-3 | 1.2e-4 | 1.0e-3 |
| 1–3 px | 1.2 | 0.39 | 0.10 | 3.5e-2 | 1.3 |
| < 1 px | diverges | diverges | diverges | diverges | — |

The validity radius scales with DE: the Taylor series of log z_n converges out to the
nearest zero of z_n, which is about DE away.

**Which order (shaded, dendrite view, centre scheme, flag DE < τ):**

| jet | τ = 2 px | τ = 3 px | τ = 5 px |
|---|---|---|---|
| order 1 (dz/dc only) | 1.59% worst | 1.47% | 0.98% (fails) |
| order 2 | 0.012% | 0.0077% | 0.0054% |
| order 3 (exact) | 0.0046% | 0 | 0 |
| order 3 (neighbour stencil) | 0.073% | 0.0046% | 0 |

**Order 3 is needed**, with order 2 marginal. Order 1, the jet films already have, is not
enough.

**Cost of the exact jet:** `costbench`, single thread. Carrying d² and d³ costs **1.9–2.3x
per iteration** over z + dz/dc, so the jet sample alone costs about 2 samples. In the zone
path it would also need the second and third partials of the degree-4 return biseries and of
φ, ψ and the tail patch. That is all once-per-pixel chain-rule work except the per-return
partials and the tail steps.

**The free jet (neighbour stencil).** F = 2⁻ⁿ log z_n is the log-Böttcher coordinate. It is
holomorphic in u and continuous across escape counts, since z_{n+1} = z_n² + c changes
2⁻ⁿ log z by O(|c|/|z|²) ≤ 1e-20. Each neighbour's F' = w/2^{n_k} is therefore comparable.
With F' known at u = ±1 and ±i (the four neighbouring pixel centres):

- F'' = [(F'(1) − F'(−1)) − i(F'(i) − F'(−i))] / 4. The error is O(F⁽⁶⁾), since the
  h³ terms cancel.
- F''' = [F'(1) + F'(−1) − F'(i) − F'(−i)] / 2. The error is O(F⁽⁷⁾): the h⁴ terms cancel
  because i⁴ = 1.

This needs only z and dz/dc, which film frames already compute for DE and the normal.
- **No extra recurrence and no zone-path change.**
- It also works **from the stored columns alone**. ln|z| and the scale come from ν, |F'|
  from DE, and the direction from the u16 normal. The `QUANT=1` arm gives identical scores.
  So jet-AA can be a late pass over ss 1 `.fds` samples.

**Centre scheme vs corner scheme.**
- **Centre:** one real sample at the pixel centre, four virtual ones at (±¼, ±¼). A flagged
  pixel then needs its 4 real ss 2 samples: eff = 1 + 4f.
- **Corner:** the real sample *is* the top-left ss 2 sample. The other three, at offsets
  (½, 0), (0, ½) and (½, ½), are virtual or computed: eff = 1 + (computed)/N. This is never
  more than 4.

**Consistency flag (corner scheme).**
- Each virtual subsample is predicted twice: from the pixel's own real sample, and from the
  right, lower or diagonal neighbour's real sample (mirrored offset).
- It is accepted when the two agree: |Δν| < m·1e-3, |Δ ln DE| < m·1e-2 and
  |Δangle| < m·3e-3 rad. It then takes their mean (odd error terms cancel).
- Otherwise that one subsample is computed.
- An unresolved real sample is predicted unresolved only when its 4-neighbourhood and its
  three corner neighbours are unresolved too.

This a-posteriori flag beats any DE threshold because many pixels with DE of 0.3–2 px are
predictable. At the same pass level it costs 0.1–0.4 fewer samples per pixel than the
centre scheme's DE flag.

## 3. Results

### 3.1 Crops at 1080p pixel scale (DEC-29 against exact ss 2; worst look; corner scheme, neighbour stencil)

| view | px with min-ss2 DE < 0.3 | exact RGSS-4 vs exact ss2 (noise floor) | m = 1: eff / worst | m = 3: eff / worst | m = 10: eff / worst |
|---|---|---|---|---|---|
| hook (2e-56) | 0% | 0.31% | 1.00 / 0 | 1.00 / 0 | 1.00 / 0 |
| dendrite (2.5e-56) | 2.4% | 6.0% | 1.18 / 0.0008% | 1.16 / 0.0008% | 1.14 / 0.0069% |
| v0 1e-30 | 23% | 31% | 1.99 / 0.0008% | 1.93 / 0.0031% | 1.88 / 0.011% |
| tendril (2.2e-57) | 25% | 35% | 2.17 / 0.0023% | 2.10 / 0.0054% | 2.03 / 0.017% |
| whirlpool (4.5e-58) | 25% | 35% | 2.20 / 0.0015% | 2.12 / 0.0039% | 2.05 / 0.022% |
| galaxy (3.6e-57) | 34% | 43% | 2.51 / 0 | 2.43 / 0.0031% | 2.35 / 0.011% |
| p764 spiral (1.6e-49) | 55% | 77% | 3.81 / 0.0008% | 3.64 / 0.010% | 3.49 / 0.12% (fails) |
| p8007 interior (3.6e-32) | all interior | 0 | 1.00 / 0 | 1.00 / 0 | 1.00 / 0 |
| minibrots at 10x size | (pending) | | | | |
| seahorse valley 1e-28 | (pending) | | | | |

- The iteration-weighted work is within +0.05 of eff everywhere.
- ss 1 (the real sample copied) fails by 0.5% (hook) up to 94% (p764 spiral) of
  pixel-looks.
- **Centre scheme**, for comparison:
  - exact order-3 jet with flag DE < 3: eff 1.19 (dendrite), 2.20 (v0 1e-30), 2.41
    (tendril), 2.84 (galaxy), 4.37 (p764);
  - the neighbour stencil needs about τ = 3–5 for the same quality.
  - The exact jet's own cost (§2) makes the centre scheme strictly worse.

### 3.2 Zone fast path, full 1920x1080 v0 frames (render_zone ss 2, iter 100000)

Real samples are the top-left ss 2 samples of fd's own zone render. The other three are
rebuilt from **stored columns only** (ν, DE, u16 normal) and scored against the full zone
ss 2 frame (2,073,600 px, nine looks).

| width | real px DE < 0.3 / < 3 px | interior | ss 1 copy: mean > 1 lvl | m = 3: eff / worst | m = 10: eff / worst |
|---|---|---|---|---|---|
| 1e-29 | 12.8% / 20.6% | 0 | 25% | 1.65 / 0.0007% | 1.62 / 0.0071% |
| 1e-31 | 14.7% / 23.7% | 0 | 28% | 1.74 / 0.0015% | 1.71 / 0.0072% |
| 1e-33 | 18.1% / 29.4% | 0 | 35% | 1.92 / 0.0016% | 1.87 / 0.011% |
| 1e-35 | 20.5% / 33.3% | 0 | 39% | 2.04 / 0.0022% | 1.99 / 0.011% |
| 1e-37 | 27.4% / 45.2% | 0 | 52% | 2.41 / 0.0024% | 2.34 / 0.019% |
| 1e-39 | 18.2% / 29.2% | 0 | 36% | 1.92 / 0.0015% | 1.87 / 0.011% |
| 1e-41 | 19.0% / 30.6% | 0 | 37% | 1.96 / 0.0016% | 1.91 / 0.0094% |
| 1e-43 | 25.9% / 42.3% | 0 | 51% | 2.32 / 0.0024% | 2.25 / 0.014% |
| 1e-45 | 21.9% / 35.4% | 0 | 44% | 2.11 / 0.0019% | 2.05 / 0.014% |
| 1e-47 | 26.1% / 43.4% | 0 | 54% | 2.36 / 0.0031% | 2.28 / 0.017% |
| 2e-48 | 30.0% / 50.3% | 0.1% | 65% | 2.58 / 0.0043% | 2.50 / 0.019% |
| 5e-49 | 36.6% / 63.4% | 1.7% | 71% | 3.00 / 0.0076% | 2.88 / 0.030% |
| 2e-49 | 42.6% / 71.3% | 10.4% | 68% | 3.24 / 0.017% | 3.11 / 0.048% |
| 1e-50 | — | 100% | 0 | 1.01 / 0 | 1.01 / 0 |

- DEC-29 is met at every depth; the worst look is ≤ 0.048%.
- **Film average** over the v0 dive from 1e-29 to 2e-49 (19.7 decades, weighted per
  decade): **eff 2.10 at m = 3 (1.90x fewer fractal samples than ss 2), 2.05 at m = 10
  (1.95x).**
- The dive inside the p764 minibrot costs 1.0.
- Frame cost tracks boundary density: eff ≈ 1 + 3 × (share of real samples with
  DE < ~0.5 px).
- The zone render itself takes 5–10 s per 1080p ss 2 frame on 4 contended cores.

### 3.3 Flicker (30 frames, 0.5 decades/s at 30 fps, 320x180)

- **Columns:**
  - "frame" is the per-frame DEC-29 share.
  - |dX| and |dJ| are the mean frame-to-frame change per pixel (max channel) of the exact
    and jet films.
  - "dJ−dX > 1" is the share of pixel-pairs where the jet film changes more than 1 level
    beyond the exact film's change.
  - "|d(J−X)| > 1" is the share where the error itself jumps by more than 1 level between
    frames.
| view, method | frame > 1 lvl mean / worst | mean abs dX = mean abs dJ | dJ − dX > 1 | dJ − dX > 4 | abs d(J−X) > 1 |
|---|---|---|---|---|---|
| galaxy, corner m = 3 | 0.0014% / 0.0087% | 28.10 = 28.10 | 0.0014% | 0.0007% | 0.0039% |
| galaxy, corner m = 10 | 0.0071% / 0.031% | 28.10 = 28.10 | 0.0069% | 0.0022% | 0.023% |
| galaxy, centre n3 τ = 5 | 0.0001% / 0.0035% | 28.10 = 28.10 | 0.0001% | 0.0001% | 0.0003% |
| galaxy, ss 1 (control) | 38% / 59% | 28.10 vs 31.26 | 25.7% | 20.7% | 47% |
| dendrite, corner m = 3 | 0.0003% / 0.0069% | 6.23 = 6.23 | 0.0002% | 0.0001% | 0.0008% |
| dendrite, centre n3 τ = 5 | 0 / 0.0017% | 6.23 = 6.23 | 0 | 0 | 0 |
| dendrite, ss 1 (control) | 4.5% / 10.7% | 6.23 vs 6.69 | 3.7% | 2.9% | 6.4% |

- **No added flicker:** the jet film's frame-to-frame change equals the exact film's to
  three decimals. Added changes occur on ≤ 0.007% of pixel-pairs, against 4–26% for the
  ss 1 control, so the test is sensitive.
- The exact galaxy film itself changes by 28 levels per pixel per frame. That is the
  sampling noise of §5.

### 3.4 Virtual ss 4: better than today's AA

From one centre jet (exact order 3), 16 virtual samples against the 4x4 truth (dendrite):

| | eff | worst look > 1 level vs 4x4 truth |
|---|---|---|
| exact ss 2 (today) | 4.00 | 6.2% |
| virtual ss 4, flag DE < 3 | 1.75 | 0.0023% |
| virtual ss 4, flag DE < 5 | 1.98 | 0.0008% |

On smooth content, jets buy ss 4 quality for under half of today's ss 2 cost. Flagged pixels
here pay 16 samples, so this does not carry to dense views without the corner/consistency
machinery at ss 4 (lead 3).

## 4. Answers to the question's four parts

1. **Fraction resolved from jets.** The share of virtual subsamples accepted is (4 − eff)/3
   (corner scheme, m = 3):
   - hook 100%;
   - dendrite 95%;
   - v0 1e-30 69%;
   - tendril and whirlpool 63%;
   - galaxy 52%;
   - p764 spiral 12%.

   The rejected share tracks the share of pixels within about 0.3–0.5 px of the set.
2. **Jet order.**
   - Order 3, with order 2 marginal.
   - Order 1, dz/dc alone, fails: 1–1.6% of pixel-looks over 1 level at every useful flag.
   - Order 3 comes free from the neighbour stencil.
3. **Cheapest reliable flag.**
   - The two-sided consistency check (corner scheme). A DE threshold (DE < 3 px for the
     centre scheme) is the simple fallback.
   - Both cost nothing beyond the neighbours' first-order data.
   - Neither is a bound. They are measured: 0 failures over 1 level on most views at m = 1,
     ≤ 0.02% at m = 10.
4. **Does the jet survive the zone fast path?**
   - The free jet needs only dz/dc, which the zone path already carries through returns, the
     Koenigs jump, the tail patch and the fixed-C finish. **Extra cost in the zone path: 0.**
   - Measured end to end on fd's own `render_zone` output (§3.2).
   - The exact jet would need second and third partials through every stage. The per-pixel
     chain rule is cheap, but the per-return partials and the per-step tail recurrences cost
     about 2x (§2). It is not worth building.

## 5. Kill criterion

"Kill if jet-AA fails DEC-29 on boundary-heavy deep views at any flag threshold that keeps
the effective sample count below 2.5 per pixel."
- **Passes on** hook, dendrite, v0 1e-30, tendril, whirlpool, galaxy, and the v0 zone frames
  from 1e-29 to 2e-48: DEC-29 met at eff ≤ 2.5.
- **Fails on** the p764 spiral crop and the v0 frames at 5e-49 and 2e-49. They meet DEC-29 only at eff
  2.9–3.6.

There:
- 43–55% of pixels lie within 0.3 px of the set;
- even the exact renderer fails DEC-29 against itself: the rotated grid against the grid
  differs on 53–77% (worst look);
- **no method that computes fewer samples can match one particular noise draw.**

Under the literal reading, Q6 is killed on those frames. It is kept as a lead because:
- it meets the bar everywhere;
- it saves 1.6–4x on most of a deep film;
- it never costs more than ss 2.

## 6. Owner decisions

1. **Per frame or per film?** A film's cost is the sum over its frames. On the v0 dive the
   frame eff rises from 1.6 (1e-29) to 3.2 (2e-49) (§3.2). Read the kill line per frame or
   per film average?
2. **A noise-aware DEC-29 in dense regions** (decision-1 style). Where two exact 4-sample
   patterns disagree, the exact render is not a reproducible reference. The proposal: score
   such pixels against the spread of equally exact renders, not one draw.
   - **Speculation:** no arm here exploits this. A cheaper estimator of a dense pixel's
     colour statistics would still have to be built and measured for flicker.
   - Jet-AA itself needs no change: it is already exact there.

## 7. Ranked leads

1. **FILM: jet-AA as a refine pass (corner scheme, m = 3).**
   - **Mechanism:**
     - Pass 1 renders the ss 2 top-left sample of every pixel (ss 1 cost), with a 1 px
       apron.
     - A late pass rebuilds ν/DE/normal at the other three ss 2 positions from the
       neighbour stencil and the consistency check, from stored columns.
     - Pass 2 computes only the rejected subsamples, compacted like LOD-02's dense phase.
   - **Evidence:** §3.1–3.3. DEC-29 met everywhere, worst ≤ 0.01% at m = 3 (0.0031% at
     m = 3 on the densest passing view), no added flicker.
   - **Projected speed-up**, in fractal samples against the ss 2 film:
     - about 4x on smooth content;
     - 1.6–2.5x through a v0 dive (zone path);
     - 1.1–1.3x on the densest frames.
   - Shading cost is unchanged (4 look evaluations per pixel). Jet reconstruction is a few
     flops plus one log2 per subsample.
   - **Cheapest next experiment:** fd `film --jet-aa` on CPU for a 10 s v0 film. Measure the
     wall clock against ss 2, and repeat the DEC-29 and flicker numbers on the real path,
     including keyframe films (§7.2).
2. **Keyframe films (FILM-04, `--keyframes 2`).**
   - Corner-scheme geometry is exactly an M = 2 keyframe grid built from an M = 1 real
     lattice. The same rebuild gives M = 2 keyframes at the §3.1 costs instead of 4 samples
     per output pixel.
   - **Next:** run the keyframe film path with jet-rebuilt keyframes against plain M = 2.
3. **Virtual ss 4 (§3.4).**
   - Ss 4 quality (0.002% from the 4x4 truth) at 1.75 samples per pixel on smooth content.
   - **Next:** the corner/consistency scheme at ss 4 (real lattice at 1 px, 15 virtual
     subsamples). Kill if eff > 4 on the tendril view.
4. **GPU float-float finish.**
   - The stencil needs only F' to about 1e-4 relative, and f32 dz/dc gives 6e-8, so the
     f32-only float-float finish (DEC-25) feeds it unchanged.
   - Untested on GPU output.

## 8. METHOD.md results-log text (suggested)

> ### Q6: supersampling from jets (2026-10-15, Claude, CPU probe)
> Three of each pixel's four ss 2 samples are rebuilt from the fourth with a third-order jet
> of log z. Its higher terms come free from a 4-neighbour stencil of dz/dc
> (log-Böttcher holomorphy). A two-sided consistency check flags the subsamples that need
> computing. Scored with fd-shade's nine looks against exact ss 2:
> - DEC-29 met on every view and on 1080p v0 zone frames 1e-29..2e-49, worst ≤ 0.02%;
> - no added flicker.
>
> Cost is 1.0–2.5 samples per pixel on typical deep content, and 2.9–3.6 on the densest
> frames (v0 5e-49 and 2e-49, p764 spiral). There two exact 4-sample patterns disagree on up to 77% of
> pixels, so the exact render is noise. The literal kill line (eff < 2.5) fails on those
> frames; kept as a lead (FILM jet-AA refine pass). The exact order-3 recurrences cost 2x
> per iteration and are not needed.

## 9. Reproduction

`tools/research/jetaa/README.md`. In outline:
- `cargo build --release`;
- `./run_views.sh SCRATCH`, then `score` / `CORNER=... score`;
- `zonejet WIDTH 3 10`;
- `run_seq.sh` for flicker.

Rendering the 13 crops takes about 15 minutes on 4 cores, and the zone sweep about 2
minutes per depth.
