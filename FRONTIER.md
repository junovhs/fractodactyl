# Active research frontier

**State as of 2026-10-12, after research-policy correction restoring experimental discovery.** This file directs agent attention; it is not a mandate to pursue a fixed roadmap or a minimum speed ratio. [Standing goal](GOAL.md) · [Method](METHOD.md).

## Active constructive research directive (supersedes the veto-style search policy below)

**Primary goal: derive and computationally test new solutions by building on known mathematics.** Reading a paper is step zero; its theorem/algorithm is raw material. Write the chain **published ingredient → specifically unproved consequence → constructed extension → predicted counterexample/observation → executed cheap probe**. A search-only literature triage is useful input, but does **not** close a direction just because its constituent technique exists. Do not reflexively race the published algorithm for speed. No new mathematical result or numerical experiment is asserted by this correction.

**Next actual research task (preferred, not compulsory):** revisit [P-051](PROMPTS/P-051.md) and the fixed-initial-condition unstable quadratic recurrence. Derive a concrete interface/seeding alternative that attempts to eliminate (not merely conceal) the serial high-precision prefix. Before tuning performance, execute a minimal controlled calculation measuring interface residual, correction convergence and exact initial-state discrepancy against independent high-precision serial iteration as segment length and instability change. Treat nearby-shadowed-orbit correctness as a distinct claim. A failure that identifies a mathematical obstruction counts as useful progress and should inform a new construction.

**Exploration can proceed in Chat without a complete pretest literature dossier** if explicitly marked `EXPLORATORY` and `NOVELTY_UNVERIFIED`; record actual outputs. Promote a surviving precise hypothesis to a searched, pretest committed P/PA pair **before a separate confirmatory run**. Keep existing historical ≥3× gates frozen and do not claim a new mechanism merely from a benchmark. Second-choice construction paths: P-040 global admission certificate; P-060 certified repelling-cycle radius/derivative composition. Re-rank from mathematical evidence, not bibliography length.

## Earlier 2026-10-12 search-only triage — retained historical evidence, NOT a current veto

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

**Earlier search-only conclusion (historical):** the agent selected the fixed-initial-condition sequential-prefix question but did not run a calculation. The above **active directive supersedes** the no-experiment conclusion: use established multiple-shooting and certification methods as ingredients for a derived extension and falsifiable exploratory calculation. The earlier literature findings remain valid at the level actually checked, not proof of impossibility.

## Research policy and decision distinction

**Discovery priority: USE PRIOR ART → DERIVE FURTHER → COMPUTE → CHECK.** A familiar published ingredient is not automatically an already-solved new application or derived mathematical construction. Identify exactly what is established, construct a useful farther-reaching claim and attempt to falsify it. If the *entire* proposed consequence is already proved or implemented for the same conditions, cite it and seek a different consequence rather than duplicating the claim. Routine "beat the literature" implementation races are low priority; a fair benchmark is a tool, not the primary ambition.

**Two research evidence tracks:** exploratory calculation with exact model, observed values and honest unverified novelty may run immediately; pretest literature dossier, linked P-card and [10/10 confirmation audit](PROOF_OF_DONE.md) remain necessary for a later frozen confirmatory claim. Lack of complete three-query provenance **blocks confirmation, not inexpensive exploration**.

**Preserve accurate incremental results:** even 1% can be meaningful. Historical numerical gates never change; accurate smaller effects and their limits remain on record. Consider actual total saved compute, trustworthiness, applicability and integration cost separately from mathematical originality.

## Verified-in-session findings and evidence limits

- [R-006](RESULTS/R-006.md): q=4 sampled-PWM kick failed independent accuracy; no performance claim.
- [R-007](RESULTS/R-007.md): q=5 corrected accuracy and gave ~2.027× **warm** kernel speed, but ~1.020× first-call cold, failing the frozen ≥3× goal. This was **not a universal KILL**.
- [R-008](RESULTS/R-008.md): new, preregistered **2,048-batch** synthetic reuse test measured **593.239 vs 717.221 ms** cold total (**1.209×**, ~17.29% wall-time saved), with independent sampled state error 1.176e-9 and zero wrong duty classes. **KEEP** this *scoped incremental positive result*; the older gate remains FAIL.

All these measurements are **REPORTED**, not independently REPRODUCED or CERTIFIED: the executable/raw packs are not archived in this documentation-only branch. No industrial converter acceleration, general simulator breakthrough or Fractodactyl renderer improvement has been demonstrated.

- [R-009](RESULTS/R-009.md): P-083 pretest-search-vetted known-method benchmark on a continuously state-triggered two-state hybrid. Quadratic event-time warm start gives **1.277× warm** over compiled midpoint safeguarded Newton at 262,144 repeated updates, **but cold is slower** (1,164.116 vs 1,122.827 ms medians). Empirical accuracy gate passed (max 9.619e-14 final-state, 0 wrong event classes), yet **all 2,048 events required fallback beyond two Newton steps**. **CONDITIONAL** historical warm result, but **STOP routine known-method follow-up** as a discovery direction: event localization is established and no substantive uncovered mechanism has been identified. Prior-art dossier [PA-P-083](PRIOR_ART/PA-P-083.md) was committed before execution; no novelty claim.

## Constructive discovery portfolio — candidates for actual exploratory computation

1. **P-051 (preferred):** Can a newly derived interface constraint use known certified segment methods while escaping the serial high-precision seed dependency for *the specified initial condition*? Write the algebra and falsifying residual/correction experiment first; do not simply benchmark old multiple shooting.
2. **P-040:** Can a certificate-aware global operator admission law provably outperform local choices on a constructed mathematical counterexample? Treat Petzold and Schupp–Ábrahám as useful ingredients, not full answers to this new law.
3. **P-060:** Can a verified radius/derivative sensitivity theorem for repelling-cycle polynomial composition be constructed from established fast formal-series iteration, with a falsifying hard domain case? Do not mistake composition itself for new mathematics.
4. **Cross-domain alternatives** remain welcome when a more consequential concrete extension is identified.

A small executed falsifying experiment and a new constructive inference are preferred over another four-citation elimination summary. Formal promotion requires a distinct confirmatory PA/P card.

## Handoff discipline

Each session should report: (1) published ingredient and what it truly covers, (2) proposed mathematical step *beyond* it, (3) actual derived equations and computation or a precise reason a run was impossible, (4) measured failure/success and independent accuracy caveats, and (5) next falsifying step. A literature-only outcome must be labeled as such and should not be mistaken for an experimental discovery. Run cheap exploration before spending sessions on additional searches. Preregister a new confirmatory test with P/PA and the strong relevant comparison when a clear claim survives. Preserve old R-cards and gates.

[Research index](INDEX.md) · [Lineage map](MAP.md) · [Proof of done](PROOF_OF_DONE.md).
