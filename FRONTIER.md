# Active research frontier

**State as of 2026-10-12. This is a decision aid, not a command to pursue one domain.** Update it when new evidence changes priorities, and at the end of any substantial research session. [Standing goal](GOAL.md).

## Current evidence-based objective

**Accumulate trustworthy, materially useful improvements in scientific/industrial computation—small or large—ranked by total realistic compute savings per research effort against a strong same-math baseline.**

This is an *evidence goal*, not a commitment to circuits, Koenigs, or a particular model. None of the recorded experiments has yet demonstrated a novel, competitive industrial gain; the existing R-001–R-005 observations are retrospective reports without frozen executable evidence.

## Candidate leads — provisional ranking

| Rank | Candidate | Why it is live | Cheapest decisive next test | What changes the rank |
|---:|---|---|---|---|
| 1 | [P-030](PROMPTS/P-030.md): automatically guarded state/parameter switching maps | Relevant to converter/control parameter sweeps; tests automatic validity rather than an already-known map fit | Tiny two-state event-aware model; compare cached/compiled/event-aware direct rivals; accuracy gate first | Drop if event partition is costly or fails, or specialist rival wins |
| 2 | [P-060](PROMPTS/P-060.md): polynomial composition at repelling Mandelbrot cycles | Directly tests whether the 5.27× contracting-toy result transfers back to the repelling regime | One already-known cycle, small orbit sample; strong same-math Koenigs rival | Advance only if valid domains and cold economics survive |
| 3 | [P-040](PROMPTS/P-040.md): cost/certificate-driven operator choice | Could automate deciding which local solver wins | Sweep chart radius/horizon on toy with cached competitors | Kill if selector cost removes gains |

**These are candidates, not obligations.** An agent may introduce and prioritize a better cross-domain hypothesis whenever it documents *why the new cheap experiment has higher expected information value*. Keep a small active portfolio (typically 1–3 leads), not a sprawling backlog of untested speculative prompts.

## New evidence and active preregistration (2026-10-12)

[P-080](PROMPTS/P-080.md) tested a 4th-order guarded PWM input kick against an exact specialist. [R-006](RESULTS/R-006.md) measured **1.056531374e-7** maximum final-state error at 128 × 32 versus the **1e-7** limit, 2 failures and zero duty-class errors: **KILL q=4; no timing attempted.** This is a new sandbox run, but its executable is outside the docs-only tree.

[P-081](PROMPTS/P-081.md) completed the one changed-assumption retest ([R-007](RESULTS/R-007.md)). Degree five passes accuracy (9.367e-10, zero branch errors) but provides **only 1.020× cold speed over the JIT exact analytical operator**, below the frozen ≥3× gate; **KILL the sampled-duty polynomial acceleration claim**. The **2.027x warm ratio is a genuine research signal**, though not evidence of a deployed end-to-end win. Under the revised incremental-gain policy it merits **one cheap predeclared amortization probe** before dismissal; the original >=3x cold gate stays failed. Do not increase q again without a discriminating new premise.

**Next selected frontier:** true *continuous state-triggered threshold events*, where the event time is an implicit nonlinear function of state/parameter, unlike the known scheduled switching time tested here. First cheap test: 2-state affine flow with a threshold guard, event root and analytic sensitivity, exact-branch/crossing classification versus a high-accuracy event solve and a compiled specialist baseline. Require a quantitative crossing-transversality guard; record abstentions. No speed prediction. Alternatives P-060 and P-040 remain on the portfolio.

## Required per-session choice

1. State the highest-impact unanswered uncertainty and **why this experiment beats the alternatives**.
2. Choose or register the P-ID; freeze its accuracy and cost gates before computation.
3. Run the cheapest valid test with available compute; report measured rather than imagined evidence.
4. File R-ID; mark [binary proof of done](PROOF_OF_DONE.md), update [INDEX](INDEX.md) and [MAP](MAP.md).
5. Re-rank this file **only if evidence or clear opportunity-cost reasoning changes the frontier**; always leave one specific next cheap test.

**Do not repeatedly rerun a killed idea** unless a new premise has changed. This page describes what is *currently attractive*, not which industry agents must work on.

**Incremental-gain assessment (owner direction, 2026-10-12):** Record and consider repeatable 1.10x gains, or smaller gains with large absolute savings; do not apply a blanket 3x veto. A useful, cheap follow-up on R-007 is 2,048 repeated 128x32 batches with amortized JIT/chart build against compiled exact propagation. This is a changed workload premise, not a post-hoc rewrite of R-007. Stronger scientific candidates remain continuous state-triggered event-time maps and P-060. Prefer the largest expected cumulative end-to-end time saved *per research hour*, and do not multiply separate speedups without a combined measurement.
