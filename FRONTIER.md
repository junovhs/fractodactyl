# Active research frontier

**State as of 2026-10-12, after R-009 and a new, search-only literature triage.** This file directs agent attention; it is not a mandate to pursue a fixed roadmap or a minimum speed ratio. [Standing goal](GOAL.md) · [Method](METHOD.md).

## 2026-10-12 literature triage — no numerical experiments

**Disposition: four candidate mechanisms were eliminated as autonomous discovery directions before testing.** These are **search-only** decisions: none is an experimental failure, a reproduction, a theorem about nonexistence of improvements, or a claim that an exact specialist application has been exhaustively studied. No P-card, R-card, POD score or numerical speed claim was created. Read the individual citations in [SOURCES.md](SOURCES.md).

| Candidate | Nearest primary prior art and substantive overlap | Decision / important distinction |
|---|---|---|
| [P-040](PROMPTS/P-040.md), generic cost-aware, certificate-admitted operator chooser | [Petzold 1983](https://doi.org/10.1137/0904010) switches ODE integration algorithms on predicted efficiency; [Schupp & Ábrahám 2018](https://doi.org/10.1007/978-3-319-89963-3_17) adapt computation precision/configuration in verified hybrid reachability and reuse intermediate analysis. | **PIVOT as discovery lead; legacy question stays UNTESTED.** Their specific solver/representation menus differ from integration vs composition vs Koenigs; that difference alone is not a substantive new mathematical principle. |
| [P-060](PROMPTS/P-060.md), polynomial iterate powering | [Brent & Traub 1980](https://doi.org/10.1137/0209004) explicitly analyzes computing iterated formal power series via the composition analogue of repeated squaring, and related coordinate transforms. | **PIVOT as discovery lead; UNTESTED.** Repelling-cycle domain/derivative/validated radii are not established by this abstract, but repeated composition itself is not a new mechanism. |
| [P-051](PROMPTS/P-051.md), certified short-segment parallel correction | [Parallel multiple shooting (1994)](https://doi.org/10.1016/S0167-8191(06)80013-X), [Chaudhry et al. 2016](https://doi.org/10.1137/16M1079014) on posterior error for time-parallel methods, and [van der Hoeven's 2025 manuscript](https://www.texmacs.org/joris/lagrange/lagrange.html) on high-precision **fixed-initial-condition** trajectories, local segment certificates and parallel global certification. The 2025 manuscript explicitly recognizes essentially sequential initial trajectory generation. | **PIVOT as discovery lead; UNTESTED.** The latter work concerns analytic ODEs, not the exact quadratic-map/MPFR benchmark; no demonstrated alternative to serial seed construction, and no basis yet for a novel test. |
| Fresh cross-domain suggestion: reuse verified hybrid search work under dynamic refinement | [Schupp & Ábrahám 2018](https://doi.org/10.1007/978-3-319-89963-3_17) already formalizes selective precision/configuration refinement and storage/reuse of earlier search results. | **PIVOT immediately**, without creating a new P-number or racing this published algorithm. |

**Search provenance:** actual external web searches executed 2026-10-12; provider: web search, then publisher/author primary abstracts and accessible text. Representative **exact queries executed** (not a pretest experimental dossier):

- `validated numerical ODE solver automatic method selection interval error bounds switching Taylor models adaptive method selection primary paper`
- `certified adaptive choice between multiple validated numerical integration methods Taylor interval algorithm selection solver validated dynamic switching`
- `power series composition fast iteration repeated squaring polynomial map repelling periodic point Koenigs complex dynamics`
- `analytic iteration of power series repelling fixed point local truncated composition Brent Traub Koenigs linearization`
- `parallel multiple shooting interval Newton rigorous orbit high precision complex dynamics Mandelbrot validated segmented computations`
- `fixed initial condition rigorous validated time parallel multiple shooting highly unstable chaotic trajectory certified interface enclosures`
- `rigorous parareal validated trajectory identical initial condition chaotic dynamics interval Newton`
- `certified hybrid systems parametric reachability event sequence regions reuse flowpipe guard transition cache parameter sweep`
- `parameter space partition certificate same event sequence hybrid dynamical systems flowpipe reuse`

**Evidence limits:** Petzold and Brent–Traub publisher abstracts were checked; Schupp–Ábrahám open-access full-text introduction/method was reviewed; van der Hoeven's author manuscript sections 2–3 were reviewed. The 1994/2016 sources were reviewed at abstract level. This is targeted triage, **not exhaustive novelty clearance**, and does not imply equivalence of ODE flow to a discrete polynomial recurrence.

**New highest-information SEARCH QUESTION (not an approved experiment):** Can any mathematically *distinct* interface construction for fixed-initial-condition, highly expanding nonlinear iterates avoid the sequential high-precision prefix/seed dependency **without** substituting a shadowing orbit with a different initial value? First investigate the closest lower-bound, divide-and-conquer and certified fixed-seed literature; identify a rigorous delta and plausible scientific workload **before** proposing any P/PA pair. A mere new batching workload for an established certification method is **not** eligible for autonomous benchmarking.

## Research policy and decision distinction

**New discovery priority: FIND PRIOR ART -> CITE -> PIVOT.** A known published mechanism in a substantially similar application is a successful early elimination, **not permission to test if we can beat it**. Record the actual citation and overlapping claim, then independently search another idea. Only a meaningful unsolved mathematical/new-application delta (or explicitly owner-requested non-novel engineering task) proceeds to experiment. Search-only eliminations require neither P-card nor R-card. [PRIOR_ART.md](PRIOR_ART.md) is the binding gate.
**Preserve correct improvements of any magnitude.** Even a repeatable 1% gain can be useful at high call counts. Accuracy and fair comparison determine whether it is credible; estimated **absolute saved compute across actual deployments**, confidence, transfer and research/integration cost determine whether to invest more. **Neither ≥3× nor ≥1.10× is a universal promotion floor.** The original result's frozen gate and the current portfolio opportunity are separate decisions.

**BEFORE ANY NEW TEST:** [PRIOR_ART.md](PRIOR_ART.md) requires an actually executed external literature search and **pretest committed query/results dossier** with a new linked P-card, nearest prior mechanism/specialist rival and a claim class. No search, no experiment. The retrospective PWM audit identified a directly overlapping 1990 polynomial buck-converter simulator: [PA-PWM-RETRO](PRIOR_ART/PA-PWM-RETRO.md). Its exact executed queries and actual results are archived **after** the measurements; they do not count as pretest diligence. Thus the 1.209× synthetic gain is **engineering evidence for a known mathematical family**, not algorithmic originality. Every listed untested lead requires new pretest search evidence before numerical execution.

Older [P-030](PROMPTS/P-030.md), [P-040](PROMPTS/P-040.md), [P-060](PROMPTS/P-060.md), and [P-070](PROMPTS/P-070.md) encode historical ≥3× *question-specific* tests. **Do not use those thresholds to discard accurate sub-3× findings** or impose them on newly registered experiments. Keep old gates for the old tests; make a linked new P-card if the workload/economic criterion changes.

## Verified-in-session findings and evidence limits

- [R-006](RESULTS/R-006.md): q=4 sampled-PWM kick failed independent accuracy; no performance claim.
- [R-007](RESULTS/R-007.md): q=5 corrected accuracy and gave ~2.027× **warm** kernel speed, but ~1.020× first-call cold, failing the frozen ≥3× goal. This was **not a universal KILL**.
- [R-008](RESULTS/R-008.md): new, preregistered **2,048-batch** synthetic reuse test measured **593.239 vs 717.221 ms** cold total (**1.209×**, ~17.29% wall-time saved), with independent sampled state error 1.176e-9 and zero wrong duty classes. **KEEP** this *scoped incremental positive result*; the older gate remains FAIL.

All these measurements are **REPORTED**, not independently REPRODUCED or CERTIFIED: the executable/raw packs are not archived in this documentation-only branch. No industrial converter acceleration, general simulator breakthrough or Fractodactyl renderer improvement has been demonstrated.

- [R-009](RESULTS/R-009.md): P-083 pretest-search-vetted known-method benchmark on a continuously state-triggered two-state hybrid. Quadratic event-time warm start gives **1.277× warm** over compiled midpoint safeguarded Newton at 262,144 repeated updates, **but cold is slower** (1,164.116 vs 1,122.827 ms medians). Empirical accuracy gate passed (max 9.619e-14 final-state, 0 wrong event classes), yet **all 2,048 events required fallback beyond two Newton steps**. **CONDITIONAL** historical warm result, but **STOP routine known-method follow-up** as a discovery direction: event localization is established and no substantive uncovered mechanism has been identified. Prior-art dossier [PA-P-083](PRIOR_ART/PA-P-083.md) was committed before execution; no novelty claim.

## Discovery portfolio after search triage — not approved experiments

1. **Investigate the fixed-initial-condition sequential-prefix obstruction** as a mathematical literature question, distinguishing exact initial data from shadowing a nearby trajectory, discrete polynomial recurrences from ODEs and total certification from the cost of generating the starting numeric orbit. Check [van der Hoeven 2025](https://www.texmacs.org/joris/lagrange/lagrange.html) and adjacent divide-and-conquer complexity results first; if no substantive gap emerges, pivot outside the legacy lineage.
2. **Seek a different mechanism across other scientific or industrial computations**, with a concrete real-world workload and cheap falsifiable theoretical criterion; search primary literature before any model run.
3. **Retain [P-040](PROMPTS/P-040.md), [P-051](PROMPTS/P-051.md), [P-060](PROMPTS/P-060.md) as untested historical questions, not default experiments.** Each generic mechanism has close published predecessors. A new P/PA pair is justified only by a materially new mathematical delta; ordinary parameter sweeps, another operator menu, and faster known-method implementation do not suffice.

## Handoff discipline

At each session: propose multiple distinct hypotheses; **search first**; if the literature already contains one, cite the exact overlap and **PIVOT to another**. Only a substantive surviving open gap may be preregistered with a P/PA pair and tested. If an experiment is justified and actually run, preserve original accuracy/cost gates, all measured effects including genuine incremental benefits, proof status and uncertainty. Do not create an R-card just to prove that work happened, and do not treat beating known prior art as the lab's standing mission.

[Research index](INDEX.md) · [Lineage map](MAP.md) · [Proof of done](PROOF_OF_DONE.md).
