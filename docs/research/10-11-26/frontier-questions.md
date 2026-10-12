# Six frontier research questions (2026-10-11)

Written by Claude after reading every report in `docs/research/` (10-6, 10-8, 10-9, the
10-10 agent investigations, 10-10, shapes). They avoid what those reports already cover:
BLA/LA/AT/NanoMB prior art, certification, near-parabolic transits, transfer maps, structural
banding and the cycle compiler, factored ladders, cross-frame reuse, exterior fields, GPU
float-float and queues, Misiurewicz frame transfer, and the universal-landing torus. The owner
gives them to a research agent with a compute sandbox. Answers are filed in this folder, one
file per question.

| # | Question | Answer |
|---|---|---|
| 1 | Parallel-in-time reference orbits | [q1-parallel-in-time-reference-orbits.md](q1-parallel-in-time-reference-orbits.md): **killed** as a global method; certified segment Newton is the remaining lead |
| 2 | Multiplicative tuning chains | pending |
| 3 | Deep minibrot interiors in O(1) | pending |
| 4 | Two-level references for float32 pixels | pending |
| 5 | The math of a live dive | pending |
| 6 | Supersampling from jets | pending |

---

**Shared context (paste first)**

> You are a research mathematician and numerical engineer with a compute sandbox (Python with mpmath/numpy/gmpy2, C/Rust if you like). The project is **Fractodactyl** (github.com/junovhs/fractodactyl): a Mandelbrot deep-zoom renderer. The goal is real-time diving into the set at 1080p, 0.05–0.5 decades of zoom per second, continuing ever deeper, where every pixel is the true set at that location (shortcuts must be exact or carry an error bound ≤1e-3 px).
>
> **Before anything else, read `docs/research/` and `docs/spec/METHOD.md` in that repo.** About 30 reports already cover standard deep-zoom prior art, certification, near-parabolic transits, transfer maps, the cycle compiler, factored ladders, exterior fields and GPU precision. Do not re-derive or re-summarise them. Start where they stop.
>
> **What already works:**
> - Perturbation plus skip tables.
> - Exact Koenigs jumps at repelling cycles (8–39x on one path).
> - A factored return operator for additive Misiurewicz ladders (rendered at 1e-1000).
> - One keyframe per 2x of zoom.
>
> Hardware: Ryzen 9 3900X, RX 5700 XT (FP64 is 1/16 of FP32), RTX 3050 Ti laptop.
>
> **Method:** skim prior art fast and from any field, then invent, implement small prototypes, run them, and report numbers with code. State a kill criterion before each experiment and honour it. Bring back ranked leads, each with:
> - the mechanism;
> - your measured evidence;
> - the projected speed-up and what it is relative to;
> - the cheapest next experiment that would confirm or kill it.
>
> Label speculation as speculation.

---

**1. Parallel-in-time reference orbits: can a million-step, million-bit orbit be computed in log time?**
The high-precision reference orbit is strictly sequential, and at extreme depth it becomes the bottleneck. FractalShark reports it dominating from about 1e10000 down.

**Lead to attack:** treat the whole orbit z_0..z_N as one unknown and solve F(z)=0 (z_{n+1} − z_n² − c = 0) by Newton. The Jacobian is bidiagonal, so each correction δ_{n+1} = 2z_n δ_n − r_n is a linear recurrence. Affine maps compose associatively, so each Newton step is a parallel prefix scan. Quadratic convergence could double the precision each step: start from a double-precision orbit and reach p bits in log2(p/53) parallel sweeps. For a periodic nucleus reference, the multiple-shooting version is natural.

**Things to settle:**
- The chaotic amplification ∏2z_n makes the scan ill-conditioned. How many guard bits does each sweep need?
- Does the total work stay near O(N·M(p)), with parallel depth O(log N · log p)?
- Does mixed precision help: high-precision residuals with lower-precision scan corrections?

**Raid:** parareal and parallel-in-time integration, multiple shooting, shadowing theory, parallel scans of linear recurrences, and FractalShark's NTT reference code.

**Prototype:** compare against sequential MPFR at N = 1e4 to 1e6 and p = 1k to 100k bits. Report wall-clock scaling for 8, 64 and 4096 simulated lanes.

**Kill if** the scan needs full precision at every step and total work exceeds the sequential cost by more than 4x.

**2. Multiplicative tuning chains: one operator hierarchy for minibrots inside minibrots.**
Our factored operator handles additive ladders, where periods grow as q + rn. The deep targets our variety planner actually dives into are mostly tuning chains: minibrots nested inside minibrots, where periods multiply (P, P·m₁, P·m₁·m₂, …). Under renormalization, the child's return map is the parent-copy's own period-m₁ return in renormalized coordinates.

**Questions:**
- Can a child's bivariate return operator (state × parameter) be built by composing the parent's operator m₁ times, in normalized coordinates, instead of iterating P·m₁ raw steps?
- How does conditioning and polynomial degree evolve across 3, 5 and 10 levels?
- Is per-pixel cost O(levels), meaning about O(log depth)?

**Raid:** Douady–Hubbard tuning, internal addresses, NanoMB2's unfinished multi-level chain (the reports cover it), renormalization-group numerics (Feigenbaum fixed-point computations), and composition of truncated power series.

**Build:** a real chain, for example period 3 → 3·2 → 3·2·3 → …, or one found by atom-domain search near a variety-film target. Construct the operator hierarchy and render sample pixels. Report per level:
- build cost against direct construction;
- coefficient growth;
- the validated domain;
- returns per pixel;
- state and derivative error against high-precision iteration.

**Kill if** building a child costs O(P_child) raw steps anyway, or the composed operators lose more than 10 digits by level 4.

**3. Deep minibrot interiors in O(1) per pixel, certified.**
Inside a deep minibrot every pixel currently runs to the iteration cap: at 1e-90, period 764, every interior pixel comes back "unresolved". That makes interiors the worst pixels for a real-time deadline.

**Lead:** the hyperbolic components of the tuned copy (its cardioid and bulbs) are exactly the regions where |λ_P(c)| < 1, with λ_P the multiplier map. Near a nucleus we already hold the parameter-space return map as a bivariate jet, from which λ_P(c), and the sub-bulb multipliers at periods k·P, might be evaluable as polynomials in c.

**Questions:**
- Can interior membership be decided per pixel or per tile from a few polynomial evaluations, with a certified error so boundary pixels fall back?
- Can interior shading (attracting-cycle Koenigs coordinate, interior distance estimate) come from the same jets?

**Raid:** the multiplier map as a conformal isomorphism onto the disk, interior distance estimation (Heiland-Allen, Albert Lobo), certified Newton (Krawczyk), and the tera-polynomial disk arithmetic.

**Measure:** on 3 or more deep minibrots (periods 100 to 10,000, widths 1e-50 to 1e-300), report:
- the fraction of interior pixels certified without iteration;
- cost per pixel against running to max_iter and against the best known interior-detection method;
- misclassifications, which must be zero.

**Kill if** certification covers less than 80% of interior pixels at 1080p.

**4. Two-level references so per-pixel arithmetic can be float32 on a consumer GPU.**
Already measured in the project: naive float32 fails the 1e-3 px bar, and a guarded float32 finish only ties FP64 on the RX 5700 XT. The question is structural, not about precision tricks.

**Lead:** compute one FP64 (or exponent-extended) secondary reference per tile, say 16×16 pixels, as a perturbation off the global high-precision reference. Then pixels run in float32-with-exponent relative to their tile reference. Their deltas are about 1/100 of the view, so absolute rounding error shrinks proportionally, while the FP64 work is amortized over 256 pixels.

**Derive and simulate:**
- the error growth in output pixels, as a function of tile size, iteration count, skip-table use and rebasing;
- the optimum tile size;
- where it breaks (near critical returns, weak expansion, interiors);
- a cheap per-pixel guard that sends failures back to FP64.

**Raid:** multi-reference schemes (Kalles Fraktaler glitch correction), hierarchical and multigrid methods, mixed-precision iterative refinement, and HDR float in FractalShark.

**Run:** at widths 1e-10, 1e-30, 1e-100 and 1e-300 on 3 locations. Report the error distribution and the projected GPU throughput from operation counts on RDNA1.

**Kill if** more than 1% of pixels need the FP64 fallback or the worst error exceeds 1e-3 px.

**5. The math of a live dive: compile every reachable target ahead of the user, inside the frame budget.**
Our automatic compiler discovers structure only along the centre orbit. Live steering means the user may head anywhere in the view, so the next octave's operators must already be ready or quick to finish. That covers nucleus references, Koenigs charts and return maps for every minibrot and spiral that could become the target.

**Quantify on real locations:**
- How many distinct structures per octave are large enough to matter, for example an atom domain over 8 px or a Misiurewicz domain over 8 px, at depths 1e-5 to 1e-100?
- What does it cost to discover and compile each one?
- How much of octave k+1's catalogue is derivable from octave k's? Children of known nuclei, rungs of known ladders, and incrementally extended references all count.
- Does a per-octave budget hold: about 0.6 s at 0.5 decades per second, on 12 CPU cores plus the GPU?

**Raid:** atom-domain and box-period scans, Misiurewicz domains, speculative execution and branch prediction, game-engine streaming, and anytime algorithms.

**Invent** the incremental catalogue update rule. Then measure the catalogue size and compile cost on 5 dives of 30 decades each.

**Kill if** the catalogue grows faster than linearly in depth, or a full octave's compile exceeds 3x the budget even with inheritance.

**6. Supersampling from jets: one exact sample plus derivatives instead of 4 samples per pixel.**
Films render 2×2 supersamples per pixel, plus 2x-oversized keyframes. That 4x factor is paid on every frame of every film, and on every live keyframe.

**Lead:** each sample can carry its Taylor jet in c: ν, ∇ν, and second derivatives from the d²z/dc² recurrence, which is cheap alongside z and dz/dc. Integrate the shading function analytically over the pixel footprint, and flag only pixels where the jet's validity radius is smaller than the footprint (filaments, boundary) for true supersampling.

**Questions:**
- What fraction of pixels can be resolved from one sample plus a jet, within a perceptual tolerance? The project's film check uses at most one 8-bit level of difference on 99.9% of pixels outside noise, with no added flicker.
- Which jet order is needed?
- What is the cheapest reliable flag for pixels that need true supersamples?

**Raid:** analytic and prefiltered antialiasing, ray differentials, texture-space derivative filtering, interval shading, and distance-estimate antialiasing.

**Build:** a small renderer with a smooth-colouring shader similar to ours (palette on ν with terraces, plus slope light from the normal). Compare jet-AA against 4×4 supersampled ground truth on 5 deep views and a short zoom sequence that tests flicker. Report:
- the fraction of pixels resolved from jets;
- the effective sample count per pixel;
- the error and flicker metrics.

**Kill if** fewer than 70% of pixels pass on boundary-heavy deep views.
