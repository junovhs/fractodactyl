# Prior-art protocol: use known mathematics to go farther

**This policy separates discovery exploration from formal confirmatory claims.** Historical PA/P/R records remain intact. Relevant links: [operating contract](AGENTS.md), [method](METHOD.md), [P template](TEMPLATES/QUESTION.md), [R template](TEMPLATES/RESULT.md), [proof of done](PROOF_OF_DONE.md).

## What a search is for

A literature search should answer **"What powerful ingredient can this work provide, and what can I do with it that is not already established?"** It is not primarily a filter that sends every overlapping idea to PIVOT, nor an instruction to race the published implementation.

1. Identify the nearest theorem, algorithm, domain, assumptions, accuracy/cost claims and failure mode.
2. Write a **bridge from prior art to a new question**: known result → useful ingredient → proposed extension/synthesis → exact measurable or provable new consequence. Consider changing an assumption, coupling separate algorithms, strengthening a certificate, crossing a domain boundary, or eliminating a specific obstacle.
3. If a paper **already proves the exact proposed statement**, cite it and do not claim to have invented it. Try a new consequence of that result or select another problem. Mere familiarity of a component is **not** sufficient to eliminate a new construction.
4. A fair baseline is important for performance and capability claims, but a benchmark race is not the default goal. The objective is a new solution to a question, not a faster implementation of published mathematics for its own sake.

## Fast exploratory probes — allowed before complete dossier

A cheap numerical or symbolic discovery probe (usually under 30 seconds) may be performed with a focused source check or even without an available search, **provided** it is honestly labeled `EXPLORATORY` and `NOVELTY_UNVERIFIED` until checked. It must state equations, parameters, predicted observable and falsifier and record actual output or failure. It can use a published algorithm as a component or oracle.

A full three-query prior-art search, P/PA preregistration and pretest Git commit are **not required for these exploratory calculations**. Their results are hypotheses/evidence, not confirmed originality, certified correctness, or reproducible speedup claims. Record negative runs. Do not label a later write-up pretest if the probe already occurred.

## Formal confirmation — pretest evidence required

Before a **new confirmatory experiment** intended to support a frozen accuracy/performance/novelty claim, execute and preserve at least three external searches: mechanism/closest mathematics, the targeted application and the strongest specialist rival. Preserve date in UTC, provider, exact literal queries, actually returned identifiable titles/URLs/DOIs, at least one checked relevant primary abstract or paper (and access limits), and specific overlap and remaining extension. Search results are not proof of novelty.

Create a linked `PRIOR_ART/PA-P-NNN.md` and ≤150-word `PROMPTS/P-NNN.md` in the **same commit before the confirmatory test**, verify read-back and branch, and state:

- `TRIAGE: EXTEND` — known work is a component; precise new construction/claim is stated.
- `TRIAGE: PROCEED` — credible unanswered mechanism/question not already settled by cited work.
- `TRIAGE: EVALUATE` — clearly scoped non-novel scientific/application correctness, coverage or economic question (not a novelty claim), justified by useful information.
- `TRIAGE: PIVOT` — the **entire proposed conclusion** is already established or no meaningful extension survives; explicitly identify what is settled, not simply that a component is old.
- `TRIAGE: BLOCKED` — pretest confirmation prerequisites unavailable. **Only confirmation is blocked; labeled exploratory computation remains allowed.**

Preserve `KNOWN_INGREDIENT_NEW_CONSTRUCTION`, `KNOWN_METHOD_NEW_APPLICATION`, `OPEN_GAP_HYPOTHESIS`, `KNOWN_METHOD_BENCHMARK`, `NOVELTY_UNVERIFIED` as honest claim classes. Neither a `KNOWN_METHOD_BENCHMARK` nor any other class implies that a numerical gain constitutes new mathematics. Routine implementation races without a discovery/evaluation question are low priority. Do not require owner permission for a scientifically justified non-novel exploratory or evaluation experiment.

## Interpret and record responsibly

- A pretest dossier is required for a **confirmatory** `POD: YES` claim, not for discovering which hypothesis merits confirmation. Prior exploration must be declared rather than hidden.
- The original PWM polynomial experiment did not search pretest and the 1990 converter precedent later surfaced: [retrospective dossier](PRIOR_ART/PA-PWM-RETRO.md). Keep its 1.209× **reported** synthetic speed finding, but do not claim the polynomial method was invented here.
- [P-083/R-009](RESULTS/R-009.md) reports a known event-time technique. Preserve its warm-only measurements and limitations. The lesson is **do not confuse reimplementing prior art with discovering new mathematics**, not "never run an experiment when the method has been published."
- If a search or experiment changes the novelty interpretation, correct the classification in the frontier and later record. Never retroactively claim preregistration, formal proof or a speedup that was not measured.
- All historical confirmatory gates remain historical facts. New exploratory or formal questions require their own explicit claims and records. A full PA dossier may follow *discovery exploration*, but must precede the **new** confirmatory run.

[PA template](PRIOR_ART/TEMPLATE.md) · [Current frontier](FRONTIER.md).
