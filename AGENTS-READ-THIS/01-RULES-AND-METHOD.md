# 01 · Mission, scientific method, and integrity rules

## Standing objective

Discover **new mathematical or computational capabilities** that replace expensive scientific/industrial computation by exploiting structure. Seek an identity, decomposition, representation, validated admission/error bound, coupling of known ideas or feasibility boundary. Fractodactyl inspired the laboratory but rendering is not its aim. The agent independently chooses the most informative next experiment; the owner need not invent its mechanism.

Use prior art **as building material**. Precisely state: published result and assumptions → unresolved implication → proposed mathematical extension → falsifiable prediction and counterexample. If a source establishes exactly the intended claim, do not present it as novel; identify an unfinished implication instead. Do not endlessly eliminate ideas because their components are published. Do not turn every session into a speed race against an existing paper.

## Exploration: default research track

1. Read the active question and latest evidence via 02-LIVE-STATE.md; open historical source, P and R cards **as needed**, especially those directly supporting or contradicting the new claim.
2. Define equations, inputs/domain, proposed mechanism, predicted observable and a result that would falsify it. Derive a mathematical relation, error estimate, interface, or explicit obstruction.
3. Actually run the cheapest discriminating symbolic or numerical test in the available environment when feasible (often less than 30 seconds). Prefer an independent, high-precision oracle, hostile cases and correctness **before** timing. Distinguish exact-fixed-initial-condition predictions from shadowing of a nearby trajectory in unstable systems.
4. Explain observations and failures quantitatively. Iterate constructively from evidence; a negative probe with an understood mechanism is valuable. If computation is unavailable or a proof settles the question, say explicitly **NOT RUN**, why, and the next calculation.
5. File the evidence under RESULTS/R-NNN.md with reconstruction details and citations. Label **EXPLORATORY: YES, PREREGISTERED: NO, POD: NO, NOVELTY UNVERIFIED** as applicable. Exploration can occur before exhaustive literature clearance. Never claim certification, priority or reproduced performance on that basis.

## Confirmation: separate, deliberate track

Before a **new frozen confirmatory claim**: (a) execute at least **three real external literature searches**—closest mathematical mechanism, use/application and strongest specialist rival; preserve dated exact queries/provider/results, at least one primary abstract/paper actually reviewed, access limitations and novelty overlap; (b) create a **<=150-word P-card in PROMPTS/** linked to a **PA dossier in PRIOR_ART/** together in the **same pretest commit**, and read both back; (c) explicitly separate all preceding exploration; (d) freeze equations, parameters, seeds, sample/domain, independent accuracy or class gate, specialist rival where germane, setup/timing plan and a question-specific verdict before running confirmation. No universal minimum speedup exists; historical numerical gates remain frozen.

Use TRIAGE EXTEND / PROCEED / EVALUATE / PIVOT / BLOCKED truthfully; an established ingredient is not automatically a settled new construction. Missing search or Git prerequisites block **confirmation only**, not labeled exploration.

For an executed confirmatory result, audit **all ten** items in 03-WRITEBACK-CHECKLIST.md in the R-card. POD: YES only if every check is genuinely satisfied, including pretest provenance and post-commit read-back. Mark absent artifacts unchecked. Relevant rigorous interval/ball proof can support CERTIFIED only when the bound and its relation to actual finite-precision output are auditable; an unrun Lean snippet is not a proof. Lean is optional, not a discovery prerequisite.

## Research-lineage stewardship

Maintain MAP.md as an **intentionally incomplete, evidence-backed graph of important research dependencies**, not as a chronological log. At the end of a session ask whether a new direction, decisive overturn/closure, enabling milestone, or connection between previously separate lines has materially changed that graph. Only then follow the MAP.md exception in 03-WRITEBACK-CHECKLIST.md. A well-supported exploratory finding may qualify but must retain its provisional evidence/novelty labels; formal confirmation is not automatically necessary. Do not add an edge just because a P/R card was created or an existing method was incrementally tightened. Current scientific status always lives in 02-LIVE-STATE.md.

## Mathematical, numerical and performance discipline

- Report exact problem, inputs, precision, seed, oracle, max errors/failed classes and uncertainties. A small residual or empirical fit alone is **not** a certificate. Unsupported domains should decline.
- Measure speed only after the accuracy criterion is met. Use a correct same-math/specialist competitor with comparable compilation, caching, batching and reuse. Report cold/setup, warm, amortized cost and absolute savings when relevant. A credible 1% improvement can matter; do not impose a universal 3x or 1.10x cutoff.
- Preserve negative findings, original gates, changed interpretations and subsequent smaller true gains without rewriting history. Historic PWM (R-008) was a known-method synthetic gain; R-009 was a conditional warm-only known-method observation.
- Provenance is independent of novelty: **SPECULATION** = no executed evidence; **REPORTED** = observations without full independent-run artifacts; **REPRODUCED** = archived sufficient setup/artifacts plus an actual independent rerun; **CERTIFIED** = relevant auditable proof/enclosure. Claim class: KNOWN_INGREDIENT_NEW_CONSTRUCTION / KNOWN_METHOD_NEW_APPLICATION / KNOWN_METHOD_BENCHMARK / OPEN_GAP_HYPOTHESIS / NOVELTY_UNVERIFIED. Never conflate these.
- Never fabricate a numerical run, literature consultation, timings, pretest commit, proof or Git write. Include exact executed tool access and limits. Keep all new findings linked in the live state; do not update optional historical documents as an exercise in bookkeeping.
