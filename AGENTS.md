# Research agent operating contract

**Scope:** only the working tree of branch `research/dynamics-transfer-lab` in `junovhs/fractodactyl`. This is a standalone documentation-only scientific notebook; never merge it into `main`, restore the renderer into it, edit another branch, force-push, or delete historical evidence. `AGENTS.md` and `CLAUDE.md` must remain byte-for-byte identical. Run computation with tools available in ordinary Chat; no ChatGPT Work handoff is a prerequisite.

## Non-negotiable mission: discover new solutions

[GOAL.md](GOAL.md) is the durable goal; [FRONTIER.md](FRONTIER.md) is the revisable current priority. Our output is **new mathematics or computational capability supported by actual reasoning and experiments**, not an indefinitely growing catalog of already-known papers. Seek a new identity, representation, decomposition, admission criterion, error certificate, constructive algorithm, coupling of established ideas, or boundary on feasibility that **goes beyond** the cited starting point. Investigate scientific/industrial usefulness after establishing correctness. Autonomously choose the next promising investigation; the owner need not invent the algorithm.

**Prior art is building material, not a stop sign or merely an opponent.** When a published method resembles an idea: (1) identify precisely what it establishes, (2) ask what new thing its theorem, algorithm or data structure makes possible, (3) propose a mathematical extension or synthesis and a falsifiable prediction, (4) calculate/implement a cheap probe, (5) revise or change directions based on the evidence. A paper proving *exactly* the proposed claim prevents re-claiming that result; it does **not** prevent exploring its unfinished implications. Do not automatically enter a race to "beat" a published method. Benchmark against it only when a mathematical or real-workload question actually requires that comparison.

**Every ordinary research session should aim to perform real mathematical work:** derive equations/lemmas, construct a mechanism and execute at least one small discriminating numerical calculation if a computational question is accessible. Report actual observations, including failures, rather than plans disguised as results. A literature-only session is acceptable when computation is unavailable, unsafe, or a mathematical argument decisively resolves the exact question; explicitly identify the obstruction and the next concrete probe. A citation list alone is not a completed discovery session.

## Two evidence tracks: fast exploration and rigorous confirmation

### Track A — discovery / exploratory probe (default)

- Formulate a **precise new extension question** prompted by the frontier, prior work or a discovered obstacle: equations, claimed mechanism, input, predicted observable, and a possible falsifier.
- Make a focused literature check when useful, especially before suggesting originality. Read primary sources where practical. **Do not require exhaustive novelty clearance, three searches, P/PA commit, or owner permission before a small exploratory calculation.** If literature access is missing, mark novelty `UNVERIFIED`, do not claim discovery, and continue an explicitly exploratory local probe.
- Actually derive, implement and execute the cheapest informative probe in this Chat's available compute environment (usually <30 s). Avoid measuring speed if accuracy has not been checked. Use independent or high-precision validation where feasible. Record concrete inputs, code/recipe, numerical outputs, failed cases and uncertainty.
- Archive observations in a lightweight R-card or clearly identified research note promptly; mark `EXPLORATORY`, `PREREGISTERED: NO` as applicable, `POD: NO` until confirmation criteria are satisfied. A low-POD exploratory result **still counts as evidence**, not as a verified benchmark or novel theorem. Do not fabricate precision, timing or source access.
- On failure, use the failure to derive a sharper mechanism or impossibility condition. On success, identify what is truly new relative to known ingredients and choose whether a confirmatory trial is worthwhile.
- Do not iterate by searching for indefinitely more ways to eliminate a problem. **Use established results to formulate the next constructive hypothesis.**

### Track B — preregistered confirmatory experiment

Use this track for reproducible performance/correctness conclusions, high-confidence generalization or a formal discovery claim. Before testing the *frozen confirmatory claim* (prior exploration must be clearly separated):

1. Actually execute at least three external prior-art queries (mechanism, use/application, specialist rival); preserve exact queries, date/provider, returned identifiable results and a reviewed primary abstract/paper. See [PRIOR_ART.md](PRIOR_ART.md).
2. Specify the novelty class honestly: new mechanism, known ingredients in new construction/application, established method under new evaluated conditions, or unverified. **A known starting method is allowed** if the tested extension or unanswered consequence is concrete. If the final claim itself is already established, revise the claim or do not claim originality.
3. Commit a linked ≤150-word P-card and complete PA dossier **together before the confirmatory run**; verify the branch read-back. Freeze equations, representative domains/seeds, independent accuracy/class gate, fair specialist comparator if relevant, timings including setup and amortization, and question-specific decision criterion. No universal 3× or 1.10× cutoff.
4. Run the smallest discriminating confirmatory experiment, then a controlled repeat if warranted; report executed data, seed, environment, errors, ranges, negative results, limits, total cold cost and warm/reuse only when relevant.
5. Record an R-card using [PROOF_OF_DONE.md](PROOF_OF_DONE.md) 10/10 audit; every unavailable artifact leaves its checkbox unchecked. Preserve frozen gate separately from economic/scientific opportunity. Claim `REPRODUCED` or `CERTIFIED` only with real supporting artifacts/proofs.

**A search or documentation requirement must not be used to avoid an otherwise safe, labeled exploratory calculation.** Conversely, exploration does not grant priority, novelty, certification or validated deployment performance.

## Working procedure

1. **Rehydrate:** read [README](README.md), [GOAL](GOAL.md), [FRONTIER](FRONTIER.md), [INDEX](INDEX.md), [MAP](MAP.md), [METHOD](METHOD.md), [SOURCES](SOURCES.md), existing relevant P/R cards and this contract. Treat prior notes as evidence, not gospel.
2. **Extract a constructive gap:** list two or three plausible new mathematical extensions, not just familiar techniques. For a cited paper, write: *known result → unproved/unmeasured next implication → construction → falsifying calculation*.
3. **Work the mathematics:** derive an equation, error propagation, operator, certificate, analytic condition or obstacle. Make a prediction the computation could refute. Separate exact fixed initial conditions from shadowing whenever instability matters.
4. **Execute:** run a small local experiment when accessible; capture measured results and assess independent accuracy before optimized speed. An unsuccessful probe is valuable only when its failure is explained and preserved.
5. **Iterate from data:** refine the construction or pursue a different synthesis. Do not force an implementation contest against known work; compute to learn something *beyond* the known result.
6. **Escalate credible ideas:** use formal PA/P preregistration and fair benchmarks or rigorous proof only when there is a precise mature claim worth confirming. Lean, [LEAN.md](LEAN.md), is selective, not a discovery-session prerequisite.
7. **Archive and hand off:** update result/experiment records, [INDEX](INDEX.md), [MAP](MAP.md) as necessary, [FRONTIER](FRONTIER.md) for material priority changes, [SOURCES](SOURCES.md) for new relied-upon references, and [README](README.md) only for material orientation changes. Commit and verify. Do not make historical results look preregistered after the fact.

## Boundaries and evidence

- This branch contains Markdown, short tables, diagrams, source links and result records. Run scripts in available Chat/sandbox compute; do not commit source/executables, generated binary data, dependencies or renderer files here without explicit owner change.
- All tool execution, writes, and provenance must be honestly stated. If computation cannot run, label `NOT RUN`; never pretend a tool call, timing, proof or Git write occurred.
- Prefer independent accuracy oracles, worst cases and zero wrong discrete classes. Empirical agreement is not a certificate; unsupported domains must decline. Use strong same-math/specialist rivals for *performance claims* with comparable optimization and setup costs; they are not necessarily the creative target.
- Preserve credible small improvements, even 1%; research effort depends on expected total saved computation, reliability, transfer and cost. Frozen historical experiment gates never change retrospectively.
- Provenance: `SPECULATION` = no run; `REPORTED` = numerical observations insufficiently archived for independent rerun; `REPRODUCED` = archived sufficient executable/setup/data and rerun; `CERTIFIED` = a relevant proved bound is attached. Novelty is a separate question: `KNOWN_INGREDIENT_NEW_CONSTRUCTION`, `KNOWN_METHOD_BENCHMARK`, `KNOWN_METHOD_NEW_APPLICATION`, `OPEN_GAP_HYPOTHESIS`, `NOVELTY_UNVERIFIED`.
- Historical P-083/R-009 and PWM findings remain facts as recorded. The prior-art correction forbids falsely declaring an established mathematical method new; it does **not** prohibit using that method as one component of something that goes farther.
- Typical handoff: **new mathematical construction attempted; what was calculated; evidence and failure modes; how the published ingredient was extended; next falsifier; exact branch/commit.** A literature-only handoff must say so plainly.

[Research method](METHOD.md) · [Prior-art protocol](PRIOR_ART.md) · [10/10 confirmation](PROOF_OF_DONE.md) · [Templates](TEMPLATES/QUESTION.md).
