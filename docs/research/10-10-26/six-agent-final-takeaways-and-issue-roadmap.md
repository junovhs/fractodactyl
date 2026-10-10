# Six-agent acceleration investigation — final synthesis and ordered issue backlog

**Date:** 2026-10-10  
**Repository snapshot used:** `main` at `83942724755d4fbb566ab5bf67d0a246fe39e714`  
**Status:** research decision memo and **proposed Ishoo issue filing order**, not a declaration that new issues were created or that any unmeasured technique succeeded.  
**Goal:** make genuinely extreme-depth Mandelbrot rendering fast at useful resolutions and on long films, without mistaking a speedup over a weak baseline for a breakthrough.

## Executive decision

**Invest first in (1) factored high-period return maps, (2) automatic dynamical compilation, and (3) faster GPU execution of validated mathematical shortcuts.** Keep the near-parabolic transit method as a targeted high-upside research spike. Keep analytic exterior fields and a new shared mathematical atlas behind explicit whole-renderer and same-math amortization gates.

**Before promoting any accelerator, repair the correctness and extreme-depth address/scale path.** The project's October 9 state lists `GATE-01 → GATE-02 → FIX-35 → FIX-36` as first-order correctness blockers. The period-16,116 / `1e-1000` experiment also requires the numeric addressing work associated with `KERN-04` (and `FILM-09` before making a film at that depth). None of the six studies cancels those dependencies.

The most important research hypothesis is **not** that every location has a single magic formula. It is that a renderer can *discover its local dynamical structure*, construct a small parameter-dependent operator that compresses its expensive history, prove a useful validity domain, and execute the remaining work on an appropriate precision tier. Reuse construction across frames only where it demonstrably beats independently constructing that same operator.

### What is real versus still speculative

| Agent | Contribution | Best evidence available as of this memo | Decision |
|---|---|---|---|
| **#1: Automatic dynamical compiler** | Find repelling-cycle landings from the camera/critical orbit; synthesize parameter-dependent Koenigs jumps and abstain when unprofitable | Native **960×540**, 20k-iteration cold frames: median **1.435×** (`c=i`) and **1.168×** (previously unconfigured `q=3,p=2`) over per-frame BLA, including discovery and compilation; three passes per case, full class/`nu`/DE/normal research gates | **Advance**: working narrow prototype; generality, guarding and integration remain open |
| **#2: Near-parabolic transits** | Skip very long slow passages with Fatou/Abel-style translation or a bounded transit approximation | Restricted *real-cusp* experiment skipped about **3.14 million** steps and gave an exploratory ~**2,400× vs direct Python iteration**, not vs optimized BLA; no general complex-viewport/derivative result | **Targeted spike**, not production |
| **#3: Factored ladder return maps** | Express a long minibrot period as short entry + power of a low-period cycle + short exit; synthesize the *whole return operator* without iterating its raw period | Existing ladder nuclei at periods **764, 1,582, 16,116**, the deepest at approximately `1e-1000`; period-2 factorization is mathematically available, but **no factored 16,116-period operator / correct deep full frame has been demonstrated** | **Highest-upside first new algorithmic probe** |
| **#4: Exterior analytic fields** | Fit local analytic representations of Mandelbrot Green potential and gradient, replacing per-pixel orbit/derivative recurrences | Fast CPU field prototype: **2.065× vs independent scalar orbit/derivative** in a far exterior scene, **~1.007×** wide view and **~0.998×** at right tip; adaptive error-screened mode **1.512×** far exterior but *not* formally certified | **Conditional**: improve useful coverage and beat optimized control |
| **#5: GPU precision and work queues** | Run compressed dynamical operators efficiently on GPUs; conditional FP32x2 / FP64 and compact exceptional tails | Existing all-FP64 deep-zone OpenCL: **82–131 ms per 1080p frame**, ~**35× vs CPU BLA** (class/`nu` tested). New queue A/B source is **unmeasured on an OpenCL device**; previous naive FP32 substitution failed full frames | **Advance hardware probes**, keeping existing FP64 GPU as the opponent |
| **#6: Compiled mathematical film** | Reuse genuinely expensive parameter-dependent operators, analytic fields and proof objects across changed camera transforms | Atlas v0: **0.996×** relative to independent per-frame BLA on 750 frames, **plus 59.8 s compile**; no new matched same-math film-wide win. A cited **2.83×** example is *hypothetical* | **Defer large build** until reusable construction cost is substantial |

**Do not mix baselines.** Agent #1 beat per-frame CPU BLA on two selected targets. Agent #4 beat straightforward *CPU scalar orbit evaluation* mostly far from the boundary. Agent #5's ~35× is an *existing* GPU-versus-CPU result, not a new improvement from its study. Agent #6's 2.83× is arithmetic in an illustrative cost model, not a result. These are not six independently additive speedups.

## The technical opportunity

### A. Compress the period, not merely the pixel orbit (#3)

On the measured Misiurewicz ladder, the period is of the form

    P_n = q + r*n + L

where the repeated component has **r = 2**. Write `g_c = f_c^r`, let `a(c)` be the repelling cycle and `K_c` its Koenigs chart. On its valid chart domains,

    f_c^(q+r*n+L)(z)
      = f_c^L( a(c) + K_c( rho(c)^n * K_c^(-1)(f_c^q(z)-a(c)) ) ).

This is an exact analytic identity *where the charts and branch itinerary apply*. It is **not** automatically a stable numerical evaluator or a globally valid replacement for every return. The multiplier power needs only logarithmic exponentiation work, but high-precision source coordinates, inverse-chart conditioning, derivative propagation, transition patches and error guards may dominate.

The **new experiment** is to generate the entire bivariate return operator and its parameter derivatives from that factorization. That is different from the existing production zone pipeline, which already uses a Koenigs jump *after* a prepared minibrot return map. Measure whether high-period **operator construction itself** stops scaling linearly with `P`.

The existing direct measurements show roughly **4, 5 and 8** original-period returns at the v0 / ~`1e-100` / ~`1e-1000` rungs for the sampled bridge pixels. The online return count rises approximately as the logarithm of the ladder index, *not* as a flat constant. Do not resurrect the rejected one-lookup universal-landing conjecture.

### B. Make local acceleration discoverable (#1)

The research branch already detects a critical-orbit recurrence, Newton-refines a repelling cycle, builds parameter-dependent inverse Koenigs coefficients, and returns a **decline** for unpromising candidates. It is meaningful engineering progress: repeated net cold-frame wins and full-frame research comparisons exist.

The compiler currently handles a restricted finite-cycle search, uses preliminary cost heuristics and numerical chart checks, and relies on rendering an independent BLA frame for **research acceptance**. Do not equate that retrospective check with a production guard. Its Rust executor also serializes some high-precision absolute center data into doubles; retain reference-plus-offset or normalized representations through every relevant stage before generalizing to remote deep neighborhoods.

The intended production architecture is an *operator chooser*: classify candidates, generate and certify the most promising one, estimate the break-even cost, and deliberately abstain when BLA/direct iteration wins.

### C. After reducing the math, reduce hardware cost (#5)

The correct GPU opponent is the already optimized FP64 zone kernel. Profile what remains: map evaluations, Koenigs transformations, transcendental/phase work, finishing tails, divergence, transfers and shader output. The recent queue prototype merely splits the tail into an inline prefix and a compacted survivors kernel, with unchanged arithmetic. **Run it before adopting it**.

Compensated FP32 (float-float), FP32 plus explicit exponent, FP64, and CPU high precision address **different** failure modes. Explicit exponents solve range, not mantissa precision. Retain an FP64 checkpoint before any lossy path, and restart from there when a guard fails; casting damaged FP32 state to FP64 cannot recover the discarded information. The past all-FP32 tail failures are evidence against casual precision demotion, especially for DE/normal and low-expansion finish cycles.

### D. Reserve the other three for sharply falsifiable experiments

- **#2 near-parabolic:** valuable exactly where repelling Koenigs jumps are a bad model. First produce an accurate **complex** near-cusp viewport with derivatives and compete with the *same optimized reference/BLA baseline*. Include reference preparation. Extend to the period-2 root only after that passes.
- **#4 analytic fields:** valuable when many samples share a smooth certified exterior disk; simple large exterior regions have cheap ordinary orbits, so beating a scalar baseline is not sufficient. Test adaptive domains closer to boundary and use equivalent `nu`/Green-potential semantics and derivative/normal gates.
- **#6 film reuse:** defer until construction/validation is an appreciable fraction of the *same-math* independent renderer. The atlas must store an expensive-to-rebuild mathematical representation, not replay cheap per-frame prep under a new name. The 3 GiB bound and path-overlap economics still apply.

## Ordered Ishoo backlog — proposed filing and implementation sequence

**Issue IDs below are the repository's existing tracked IDs where named; “NEW” titles are *filing proposals*, not fabricated or created Ishoo records.** Ishoo, not this document, is the source of truth for actual issue status, ownership, and dependencies. An agent with local Ishoo access should reconcile the live ledger first and attach these deliverables to existing issues whenever they already exist.

The order below is a **critical-path research recommendation**, not a demand to suspend unrelated product work. Keep each experiment small, frozen against a fair opponent, and kill it quickly if its premise fails (DEC-14).

| Order | Priority | Existing issue or suggested NEW issue | Concrete deliverable / acceptance gate |
|---:|---|---|---|
| **0.1** | **P0 prerequisite** | **GATE-01** (existing) | Reject NaN/Inf and incorrect/deceptively equal deep widths in `fd compare` and camera metadata; demonstrate adversarial failing fixtures are caught. |
| **0.2** | **P0 prerequisite** | **GATE-02** (existing), align with **TRUT-01** | Whole-frame checks for **escaped/interior/unresolved class, smooth escape, DE and circular normal** plus high-precision adjudication of disagreeing pixels; one versioned truth pack usable by CPU and GPU. |
| **0.3** | **P0 prerequisite** | **FIX-35 → FIX-36** (existing) | Repair scaled-tier false interior declaration and `.fds` bounds/reader caps; regression corpus must include ambiguous boundary/nonfinite cases. |
| **0.4** | **P0 depth prerequisite** | **KERN-04** and **FILM-09** (existing) | Lossless high-precision deep center/offset representation; scales and film widths beyond the `f64` exponent range. Validate distinct camera samples at `1e-100` and `1e-1000`. |
| **1** | **P0 existing speed path** | **PROB-14 → PROB-10** (existing) | Whole-frame, same-output Rust measurement of co-moving mid-band jump and v0 deep-band handoff versus per-frame BLA. This preserves an already promising near-term film win; do not confuse sampled probes with frame gates. |
| **2** | **P0 research — highest upside** | **NEW: FACTOR-01 — Construct M(24,2) factored return maps**; link **PROB-17** | Build normalized, parameter-dependent return and derivative operator for **P=764/1,582/16,116** without `P` raw steps during each factored build. Compare cold build time, operator bytes, domain coverage, coefficient conditioning and high-precision state/`dz/dc` error against direct jet construction. Run 197/655 as hostile controls if extending beyond this ladder. |
| **3** | **P0 demonstration** | **PROB-17** (existing) + **NEW: FACTOR-02 — Validate complete ultradeep frames** | At ~`1e-100` and ~`1e-1000`, use correct coordinate widths, render nontrivial boundary/interior/exterior frames, enforce the **full** truth gate, report operator build + all pixel time, fallback and depth trend. Compare to a correct independent renderer even if a slower oracle is needed. **No “deep success” from six sampled pixels.** |
| **4** | **P1 generalization** | **PROB-12** (existing) + **NEW: AUTO-01 — Promote finite-cycle discoverer with guards** | Reuse the `research/auto-misiurewicz-discovery` branch: consume native `fd` reference/offset data, provide conservative state/parameter/derivative domain guards and a pre-render profitability decision; never run the truth frame to authorize a production jump. |
| **5** | **P1 breadth gate** | **NEW: AUTO-02 — Blind structural compiler corpus** | Freeze code then run **10–20 previously unused off-center views**, varied cycle periods, widths and mixed classes. Publish wins, honest abstentions, bad decisions, compilation cost, operator coverage, and every correctness failure against BLA and high precision. Integration depends on this gate, not on the two successful selected cameras alone. |
| **6** | **P1 hardware baseline** | **BENC-08 / BENC-09** (existing, reconcile actual Ishoo statuses) + **NEW: GPU-VALID-01 — Full-output GPU truth gate** | Freeze optimized FP64 OpenCL zone baseline on actual consumer GPU(s), add DE/normal and trustworthy nonfinite handling, time distinct stages and distributions of finishing steps; separate kernel-only from real frame wall time. |
| **7** | **P1 cheap hardware test** | **NEW: GPU-QUEUE-01 — Run the existing queue A/B** | Use the committed `gpu_queue_ab.py` on actual NVIDIA/AMD OpenCL drivers. Compare intact FP64 kernels at 1080p across widths and chunks 4/16/32/64, include launch/sync/queue traffic, require identical class/`nu` before speed claims. **Retain queueing only if repeatably >1.25×** on material frames and no serious regressions. |
| **8** | **P1 precision probe** | **NEW: GPU-PREC-01 — FP32x2 microkernels and guarded finish**; link existing **GPU-01 / GPU-02 / FILM-08** | Benchmark float-float complex steps *against native FP64 on the same cards*. Only then try checkpointed, precision-switched finishing tails. Predeclare **≥1.5× whole-frame over optimized GPU FP64**, all class/`nu`/DE/normal gates, with non-lossy FP64 fallback. If the cost of guards/registers erases gain, kill it. |
| **9** | **P2 dynamical expansion** | **NEW: PARA-01 — Complex near-parabolic transit** | Produce complete complex cusp frames, analytic derivatives, error budgeting and fast reference construction if needed; compete against optimized per-frame BLA **including setup**, then test near `c=-3/4`. Keep only if large bottleneck savings survive a fair frame test. |
| **10** | **P2 complementary accelerator** | **NEW: FIELD-01 — Boundary-proximate analytic fields** | Improve certified-exterior patch coverage with adaptive subdivision/Taylor models; compare same-output optimized CPU/GPU controls. Track fraction of total frame *time* saved, not just pixels covered. Park if wide/boundary scenes remain ~1×. |
| **11** | **P2 conditional film reuse** | **NEW: ATLAS-MATH-01 — Same-math film atlas challenge**; cross-reference parked atlas work | Trigger only after measurement shows shared validated operator/field build is materially expensive and reused by many frames. Compare independent same-math per-frame construction, profitability-aware independent construction, and one film-wide operator library on rotated/panning/deep paths. Goal **≥2× cold end-to-end**, correctness, **≤3 GiB**, plus truthful compile, hit-rate and fallback metrics. |

### Immediate 3-experiment packet to hand an agent

**Packet A — FACTOR-01:** make a small self-contained implementation for one M(24,2) return that evaluates the factored expression and its parameter derivative, compare it with a direct high-precision orbit at period 764. Only move to 1,582 and 16,116 once state and derivatives pass on real local inputs. Print build seconds, evaluation seconds, precision and domain failures. **Stop** if chart/transition construction hides the original `O(P)` cost or cannot cover real return-map inputs.

**Packet B — AUTO-01/AUTO-02:** use the already committed runtime compiler rather than rewrite it. First eliminate double-rounding of absolute deep centers and design independent guards, then freeze and test unseen off-center cameras. **Stop** on any escaped/interior error or unbounded derivative mismatch; preserve abstention as a valid result.

**Packet C — GPU-VALID-01/GPU-QUEUE-01:** instrument the existing FP64 GPU pipeline, then run the queue-only change unchanged on a real GPU. If the 1.25× scheduling target is not met, record the negative result and skip architectural queue complexity; try FP32x2 separately, with a fresh kernel microbenchmark.

The packets can run in parallel **after their relevant truth/addressing prerequisites**, with distinct owners and no overlapping production changes. Do not merge a research branch merely because the standalone benchmark passes.

## Dependencies, promotion rules and economics

### Rules shared by every issue

1. **Fair opponent:** for a new operator, include optimized independent BLA and an independent renderer given the same operator when measuring *reuse*. For a GPU implementation, compare against optimized GPU FP64 with the same mathematics. Report more than one hardware platform where conclusions are device-dependent.
2. **Complete outputs:** compare three-way class, finite smooth `nu`, pixel displacement under the project's `1e-3 px` criterion, and separate DE/normal tolerances. Treat NaN/Inf as errors, not as comparisons that happen to evaluate false. Use correctly represented deep camera widths and high-precision spot adjudication of worst or ambiguous points.
3. **Real costs:** separate discovery, arbitrary-precision orbit, operator/validity construction, per-pixel execution, fallback, I/O, GPU scheduling, shader/output; report **cold end-to-end** alongside warm and kernel-only results. At least three repeat runs for claimed moderate wins; record full range/median.
4. **Coverage and failure:** count how often a proposed accelerator declines, must fall back, or misses a frame; compute net film benefit on realistic paths, not just a narrow favorable scene.
5. **Numeric semantics:** explicit error contract for each chart/return/phase transition; high-precision centers with normalized local variables and stored scale exponent; parameter derivatives and stable jump-count branching must be guarded.
6. **Kill gates:** a failed prototype becomes a documented negative result. Retune once if a clear technical fix exists; do not build a large subsystem on top of an unexplained failure.

### Quantitative break-even questions

For an automatic one-frame compiler:

    T_auto = T_discover + T_validate_or_build + T_render_auto
    win iff T_auto < T_BLA_cold  (at the same correctness bar).

For factored return maps, ask *two* independent questions: can cold **construction** scale with short-cycle complexity rather than the raw period `P`, and can repeated per-pixel returns use the operator at sufficient validated coverage to beat the best full-frame fallback?

For a compiled film with shared mathematics:

    T_independent = sum_f(B_f + E_f)
    T_atlas = B_shared + sum_f(E'_f + L_f)

A genuine cross-frame gain requires saving repeated **B_f**, or reducing actual **E'_f** in a way a same-math independent renderer could not cheaply achieve. Original Atlas v0 cached ~18.3 ms/frame of reference/BLA construction against ~0.64 s/frame of pixel work, so sharing that prep could not be transformative. Do not restart a large atlas merely because its cached bytes have a high hit rate.

### Milestones that would change the project

- **Depth breakthrough:** a period-16,116, ~`1e-1000` *full frame* whose factored operator does not need 16,116 high-precision construction steps per build and whose costs/correctness are published next to the conventional construction.
- **General-location breakthrough:** automatic discovery consistently chooses correct shortcuts (or declines) across unseen viewpoints; the total wall clock is lower than BLA on a meaningful share of otherwise expensive frames.
- **Hardware breakthrough:** ≥1.5–2× over the *existing optimized FP64 GPU* under the full output/error contract on multiple scenes.
- **Film-architecture breakthrough:** ≥2× cold end-to-end against an independent renderer equipped with **the same** best operator family, including compilation, fallback and memory.

These are *research targets*, not promises or recorded outcomes.

## What not to spend time on first

- Rebuilding Atlas v0 with more reference/BLA cache entries or raster interpolation.
- Integrating the new cycle compiler directly into production without a numerical applicability gate.
- Assuming the ~2,400× direct-Python parabolic shortcut beats perturbation/BLA.
- Treating the far-exterior scalar-CPU field win as evidence of faster dense deep-boundary rendering.
- Blanket FP32 replacement, lossy remainders converted to FP64 afterward, or untested queue scheduling.
- Claiming `1e-1000` rendering from a discovered nucleus alone; precision-addressing and full-frame work are independent requirements.
- Assuming every long observed period is genuine nested polynomial-like renormalization. The 197/655 record returns, additive Misiurewicz ladders, and multiplicative tuning chains are different geometries.

## Source trail and reproduction starting points

**Agent reports, preserved verbatim or as research-branch report copies:**

- [#1 automatic dynamical compiler](../10-9-26/agent-investigations-10-10/01-automatic-dynamical-compiler-agent-report.md) and [native validation report](../10-9-26/agent-investigations-10-10/01-automatic-compiler-native-prototype-results.md); [research branch](https://github.com/junovhs/fractodactyl/tree/research/auto-misiurewicz-discovery); [three-run CI](https://github.com/junovhs/fractodactyl/actions/runs/38033469892).
- [#2 near-parabolic transit](../10-9-26/agent-investigations-10-10/02-near-parabolic-transit-agent-report.md).
- [#3 factored ladder-return study](../10-9-26/agent-investigations-10-10/03-factored-ladder-return-maps-agent-report.md); [existing ladder measurements](../10-9-26/misiurewicz-ladders.md) and [bridge correction](../10-9-26/universal-landing-claims-a-to-e.md).
- [#4 exterior analytic fields and benchmark](mandelbrot_exterior_field_research.md), [C++ prototypes](mandelbrot_field_benchmark.cpp) and [measurements](mandelbrot_benchmark_measurements.csv).
- [#5 GPU precision/work-queue assessment](gpu-precision-work-queues/REPORT.md), [queue A/B script](gpu-precision-work-queues/gpu_queue_ab.py).
- [#6 compiled mathematical film and same-math economics](../10-9-26/agent-investigations-10-10/06-compiled-mathematical-film-agent-report.md).

**Project constraints:** [current state and issue dependency order](../../spec/STATE.md), [benchmark rules and results log](../../spec/METHOD.md), [Atlas v0 Gate C failure](../../spec/BENCH.md), and [North Star](../../spec/NORTH-STAR.md).

## Final takeaway

The best path toward astonishing deep-zoom performance is **not** six separate specialty renderers or a huge cache. It is a *mathematical compiler plus execution engine*: automatically discover reusable structure; build short, valid, parameter-aware operators; evaluate them with rigorously budgeted precision on suitable hardware; and retain expensive constructions across frames *only if their reuse pays for itself*.

**Do the truth/addressing gates first. Then try to make a real period-16,116 return cheap. Generalize the discovery with Agent #1's working compiler. Finally attack remaining GPU cost with measured, reversible precision and scheduling experiments.** Let #2, #4 and #6 earn larger implementation budgets through their specific falsifiable tests.
