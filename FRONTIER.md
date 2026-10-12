# Active research frontier

**State as of 2026-10-12, after R-009 and owner-directed prior-art triage correction.** This file directs agent attention; it is not a mandate to pursue a fixed roadmap or a minimum speed ratio. [Standing goal](GOAL.md) · [Method](METHOD.md).

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

## Discovery-first literature leads — NOT approved experiments

These are **search targets**, not evidence of novel mathematics, approved P-cards, or a requirement to run tests. Search for the closest primary method *before coding*. If it already addresses the same substantive claim, cite it and choose a different mechanism. A new outside-domain candidate may outrank any row.

| Search priority | Different mechanism worth examining | What could eliminate it | First action |
|---:|---|---|---|
| 1 | Certificate- and cost-aware operator admission ([P-040](PROMPTS/P-040.md)) | Published certified adaptive solver/selector already realizes the proposed mechanism | Literature check; find an actual unanswered admission principle or PIVOT |
| 2 | Precision-aware segmentation of long-horizon computation ([P-051](PROMPTS/P-051.md)) | Existing multiple-shooting, time-parallel or validated integration settles the claimed shortcut | Literature check; isolate a genuinely different mathematical decomposition or PIVOT |
| 3 | Repelling-cycle structure ([P-060](PROMPTS/P-060.md)) or a new cross-domain mechanism | Classical polynomial composition/linearization already answers the specific question | Read the closest mathematics; if established, move on instead of racing the old approach |

The scoped measured gains in [R-008](RESULTS/R-008.md) and [R-009](RESULTS/R-009.md) are retained evidence. **Neither PWM polynomial approximation nor standard event-root warm starts is an automatic next research project.** A good session may finish with multiple literature eliminations and a fresher mathematical question, without any numerical run.

## Handoff discipline

At each session: propose multiple distinct hypotheses; **search first**; if the literature already contains one, cite the exact overlap and **PIVOT to another**. Only a substantive surviving open gap may be preregistered with a P/PA pair and tested. If an experiment is justified and actually run, preserve original accuracy/cost gates, all measured effects including genuine incremental benefits, proof status and uncertainty. Do not create an R-card just to prove that work happened, and do not treat beating known prior art as the lab's standing mission.

[Research index](INDEX.md) · [Lineage map](MAP.md) · [Proof of done](PROOF_OF_DONE.md).
