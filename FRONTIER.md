# Active research frontier

**State as of 2026-10-12. This is a decision aid, not a command to pursue one domain.** Update it when new evidence changes priorities, and at the end of any substantial research session. [Standing goal](GOAL.md).

## Current evidence-based objective

**Find the first independently reproducible, potentially transferable algorithmic gain on a scientifically or industrially meaningful nonlinear computation, against a strong same-math baseline.**

This is an *evidence goal*, not a commitment to circuits, Koenigs, or a particular model. None of the recorded experiments has yet demonstrated a novel, competitive industrial gain; the existing R-001–R-005 observations are retrospective reports without frozen executable evidence.

## Candidate leads — provisional ranking

| Rank | Candidate | Why it is live | Cheapest decisive next test | What changes the rank |
|---:|---|---|---|---|
| 1 | [P-030](PROMPTS/P-030.md): automatically guarded state/parameter switching maps | Relevant to converter/control parameter sweeps; tests automatic validity rather than an already-known map fit | Tiny two-state event-aware model; compare cached/compiled/event-aware direct rivals; accuracy gate first | Drop if event partition is costly or fails, or specialist rival wins |
| 2 | [P-060](PROMPTS/P-060.md): polynomial composition at repelling Mandelbrot cycles | Directly tests whether the 5.27× contracting-toy result transfers back to the repelling regime | One already-known cycle, small orbit sample; strong same-math Koenigs rival | Advance only if valid domains and cold economics survive |
| 3 | [P-040](PROMPTS/P-040.md): cost/certificate-driven operator choice | Could automate deciding which local solver wins | Sweep chart radius/horizon on toy with cached competitors | Kill if selector cost removes gains |

**These are candidates, not obligations.** An agent may introduce and prioritize a better cross-domain hypothesis whenever it documents *why the new cheap experiment has higher expected information value*. Keep a small active portfolio (typically 1–3 leads), not a sprawling backlog of untested speculative prompts.

## Active preregistered probe (2026-10-12)\n\n[P-080](PROMPTS/P-080.md) tests the P-030 mechanism on a sampled-data, two-state PWM converter with clipping branches. This outranks P-060 for this session: same short numerical probe can falsify both the chart accuracy and the cold economics against a strong, exact analytical competitor, with immediate relevance to converter parameter sweeps. Its fixed-schedule switch is deliberately limited; success would not establish continuous state-triggered event admission. If it fails, the next avenue should target event-time sensitivity/automatic partitioning where the exact sampled-duty shortcut is unavailable.\n\n## Required per-session choice

1. State the highest-impact unanswered uncertainty and **why this experiment beats the alternatives**.
2. Choose or register the P-ID; freeze its accuracy and cost gates before computation.
3. Run the cheapest valid test with available compute; report measured rather than imagined evidence.
4. File R-ID; mark [binary proof of done](PROOF_OF_DONE.md), update [INDEX](INDEX.md) and [MAP](MAP.md).
5. Re-rank this file **only if evidence or clear opportunity-cost reasoning changes the frontier**; always leave one specific next cheap test.

**Do not repeatedly rerun a killed idea** unless a new premise has changed. This page describes what is *currently attractive*, not which industry agents must work on.
