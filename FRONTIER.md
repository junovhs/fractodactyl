# Active research frontier

**State as of 2026-10-12.** This file directs agent attention; it is not a mandate to pursue a fixed roadmap or a minimum speed ratio. [Standing goal](GOAL.md) · [Method](METHOD.md).

## Research policy and decision distinction

**Preserve correct improvements of any magnitude.** Even a repeatable 1% gain can be useful at high call counts. Accuracy and fair comparison determine whether it is credible; estimated **absolute saved compute across actual deployments**, confidence, transfer and research/integration cost determine whether to invest more. **Neither ≥3× nor ≥1.10× is a universal promotion floor.** The original result's frozen gate and the current portfolio opportunity are separate decisions.

**BEFORE ANY NEW TEST:** [PRIOR_ART.md](PRIOR_ART.md) requires an actually executed external literature search and **pretest committed query/results dossier** with a new linked P-card, nearest prior mechanism/specialist rival and a claim class. No search, no experiment. The retrospective PWM audit identified a directly overlapping 1990 polynomial buck-converter simulator: [PA-PWM-RETRO](PRIOR_ART/PA-PWM-RETRO.md). Its exact executed queries and actual results are archived **after** the measurements; they do not count as pretest diligence. Thus the 1.209× synthetic gain is **engineering evidence for a known mathematical family**, not algorithmic originality. Every listed untested lead requires new pretest search evidence before numerical execution.

Older [P-030](PROMPTS/P-030.md), [P-040](PROMPTS/P-040.md), [P-060](PROMPTS/P-060.md), and [P-070](PROMPTS/P-070.md) encode historical ≥3× *question-specific* tests. **Do not use those thresholds to discard accurate sub-3× findings** or impose them on newly registered experiments. Keep old gates for the old tests; make a linked new P-card if the workload/economic criterion changes.

## Verified-in-session findings and evidence limits

- [R-006](RESULTS/R-006.md): q=4 sampled-PWM kick failed independent accuracy; no performance claim.
- [R-007](RESULTS/R-007.md): q=5 corrected accuracy and gave ~2.027× **warm** kernel speed, but ~1.020× first-call cold, failing the frozen ≥3× goal. This was **not a universal KILL**.
- [R-008](RESULTS/R-008.md): new, preregistered **2,048-batch** synthetic reuse test measured **593.239 vs 717.221 ms** cold total (**1.209×**, ~17.29% wall-time saved), with independent sampled state error 1.176e-9 and zero wrong duty classes. **KEEP** this *scoped incremental positive result*; the older gate remains FAIL.

All these measurements are **REPORTED**, not independently REPRODUCED or CERTIFIED: the executable/raw packs are not archived in this documentation-only branch. No industrial converter acceleration, general simulator breakthrough or Fractodactyl renderer improvement has been demonstrated.

## Candidate leads and next discriminating tests

| Rank | Candidate | Why it matters | Smallest useful next check |
|---:|---|---|---|
| 1 | Continuous state-triggered event-time map, child of [P-030](PROMPTS/P-030.md) | Scheduled-duty toy leaves implicit event times untested; may expose a reusable nonlinear shortcut | New P-card with an accuracy/event itinerary gate and **contextual** economic target; benchmark guard and event sensitivity against compiled exact/event solver |
| 2 | Representative workload for [R-008](RESULTS/R-008.md) | Test whether a measured incremental toy gain survives real input distributions, domain coverage and full lifecycle | One relevant converter workload; match exact specialist baseline, count setup, report saved seconds and fallbacks |
| 3 | Repelling-cycle direct composition, child of [P-060](PROMPTS/P-060.md) | Compare operator construction with the existing guarded Koenigs accelerator | One known repelling chart, fair composition/Koenigs competitor, independent MPFR holdout and realistic build/reuse lifecycle |

[P-040](PROMPTS/P-040.md) remains a lower-priority operator-choice candidate. Choose a different lead when a cheaper, more consequential experiment appears; don't confuse a large ratio on an irrelevant workload with valuable science.

## Handoff discipline

Every session should (1) state the uncertainty and its expected information **and real compute savings**; (2) pre-register a P-card with fair opponent and accuracy gate (and a workload-specific cost threshold if appropriate); (3) run a small decisive test; (4) file an R-card even after a negative gate; (5) report **original GATE**, **measured effect**, **current opportunity decision**, provenance and next test separately. Stop wasting effort on trivial or noisy follow-ups, **not** on a correct gain merely because it is smaller than a preferred headline number.

[Research index](INDEX.md) · [Lineage map](MAP.md) · [Proof of done](PROOF_OF_DONE.md).
