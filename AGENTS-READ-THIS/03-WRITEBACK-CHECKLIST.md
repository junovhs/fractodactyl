# 03 · WRITEBACK CHECKLIST — one authoritative workflow

This is the **single persistent checklist**. READ THIS FILE FULLY each session. Do not edit the shared checklist to tick boxes; perform the applicable steps and report completion honestly in the result/response.

## Before working — compulsory reading and boundaries

- [ ] Confirm repo junovhs/fractodactyl and current branch research/dynamics-transfer-lab. Do not use main or any other branch.
- [ ] Read **all four Markdown files** in AGENTS-READ-THIS/ **fully**, in numbered order. No skimming, cached summary or partial excerpt.
- [ ] Identify the active gap in 02-LIVE-STATE.md; read relevant historical P/R papers and source materials **only as needed**. Historical files are not independently authoritative.
- [ ] Choose an explicit extension, prediction and falsifier; prefer real mathematical/computational work over a bibliography-only report.

## Exploratory run (default)

- [ ] Derive/test a concrete claim, using an independent correctness oracle when possible; record genuine inputs and outputs, counterexamples, domain, precision, limitations, executed provenance, known mathematical ingredients and novelty uncertainty.
- [ ] If executed, create **one new RESULTS/R-NNN.md** using the next unused ID (or an explicitly linked note/correction, without overwriting historical evidence). Include equations, reconstruction, actual observations, comparator only if appropriate, failure mode, next experiment, evidence labels, source references and status **EXPLORATORY / PREREGISTERED NO / POD NO**.
- [ ] Update **AGENTS-READ-THIS/02-LIVE-STATE.md** in the **same research-result commit** with the latest status, report link, next falsifier and concise handoff. Keep at most three recent entries.
- [ ] If no execution, do **not** create a fake R-card. State NOT RUN and reason; update 02-LIVE-STATE.md only if a substantive decision/obstruction changes the next research action. Otherwise no file update is required.

**No routine updates to README, INDEX, MAP, FRONTIER, GOAL, METHOD, SOURCES, PRIOR_ART, PROOF_OF_DONE, LEAN, parent P-cards, or either root entrypoint.** Link any newly consulted papers and exact searches in the R-card itself. A new P/PA pair is required only for prospective confirmation, not exploratory testing.

## Event-triggered exception — curated MAP.md research lineage

- [ ] **Evaluate after each research session:** Did evidence materially change the research dependency graph, rather than simply generate a new card or better measurement? Update MAP.md **if and only if** at least one of these triggers holds:
  1. **New direction:** an evidence-backed, independently meaningful mechanism or research question creates a genuinely distinct lineage branch (not just another P-number).
  2. **Decisive overturn/closure:** a counterexample, proof, or stronger rival substantially invalidates, closes or redirects an existing mapped direction.
  3. **Enabling milestone:** a sufficiently supported mathematical or computational capability changes what later research can build on, rather than merely tightening a bound within the same hypothesis.
  4. **Convergence:** a demonstrated relation or dependency connects previously independent research lines.
- [ ] **Not triggers:** a new P/R ID alone, another exploratory measurement within an unchanged direction, a small improvement to a previously mapped mechanism, a routine status/timestamp update, a literature citation without a material dependency change, or a workflow-only policy edit. **R-011 is an incremental P-051/R-010 refinement, not a new graph edge.**
- [ ] **Evidence and uncertainty:** Cite the supporting R/P/PA evidence or explicit mathematical obstruction in the revised map; mark reported or exploratory relationships as provisional and do not silently relabel them confirmed. **Formal preregistration/confirmation is not required** for a strongly supported structural change, but missing evidence is not a license to map speculative connections. The live state remains the authoritative scientific status.
- [ ] **Writeback:** Make the minimum affected node/edge changes to MAP.md and update 02-LIVE-STATE.md with the reason and evidence link, **in the same commit as the new R-card**. If an evidence-backed reinterpretation of existing records changes the graph with **no new experiment**, MAP.md and 02-LIVE-STATE.md alone may be committed with the reason stated; do not fabricate a new result. Do not update INDEX/FRONTIER/README as housekeeping. If no trigger fires, **leave MAP.md unchanged** and say so in the research handoff.

## Exception — before a confirmatory test

- [ ] Make at least three real external prior-art searches (mechanism, application, specialist rival); record UTC date, provider, exact query strings, returned identifiable results, primary abstract/paper actually inspected and access limitations. Distinguish established components from the **specific** proposed contribution.
- [ ] In **one pretest commit** create the new **PROMPTS/P-NNN.md** (<=150 words, frozen claim/gate, previous exploration disclosed) and matching **PRIOR_ART/PA-P-NNN.md** (source dossier and TRIAGE decision). Update 02-LIVE-STATE.md for a material active-priority change if needed. Read back both committed files **before** any confirmatory measurement. Never backdate or treat an exploratory test as pre-registered.
- [ ] Freeze exact model, domain/seed/precision/cases, independent accuracy/class gate, correct rival (if performance is being claimed), cold/warm/reuse measurement protocol and scientific verdict. Block **confirmation** if prerequisites fail; ordinary labeled exploration remains allowed.

## Confirmatory R-card: 10/10 evidence audit

The R-card must have individually verified answers to **all ten**:
1. Frozen pretest P-card and prior exploration honestly disclosed.
2. Actually searched, pretest-committed and read-back PA dossier with sources, overlap and classification.
3. Exact equations, domain, parameters, seeds, precision and cases.
4. Strong correct specialist comparator for a performance claim, or explicit justified not-applicable.
5. Actually executed and sufficiently archived/reconstructable observations; no fabricated timings or artifacts.
6. Independent error and discrete classifications, failures/abstentions before cost claims.
7. Cold/setup, warm and amortization where relevant, or justified NOT MEASURED with no performance claim.
8. Frozen-gate verdict separate from any smaller true effect, utility and decision.
9. Novelty, known ingredients, limits, proof scope and REPORTED/REPRODUCED/CERTIFIED provenance.
10. R-card and **02-LIVE-STATE.md** links/status consistent (and MAP.md only if its structural trigger fired), committed **only to the lab branch**, with read-back confirmed. This supersedes automatic INDEX/MAP/FRONTIER synchronization.

Only when all are real and checked is **POD: YES** permissible; otherwise **POD: NO**, even for a useful exploratory or failed trial. Lean is selective; never label a numerical output CERTIFIED without the actual linked proof/interval enclosure.

## Final Git audit — every write

- [ ] Confirm the target is only research/dynamics-transfer-lab, preserve previous history and frozen gates, add no code/dependencies/binaries/renderer and do not merge or force-push.
- [ ] For ordinary executed exploration, verify the changed-path list contains **only** RESULTS/R-NNN.md (or explicitly named correction) and AGENTS-READ-THIS/02-LIVE-STATE.md, **plus MAP.md only if the above evidence-backed structural trigger fired**. For a structural reassessment without execution, MAP.md + live state may be the only changed files. For confirmation, allow the explicitly required P/PA pair in its pretest commit. For an owner-authorized workflow change, only the necessary policy/entrypoint/map-description files may differ.
- [ ] Commit with a specific message; read the branch tip and every changed file back; check that evidence and live summary agree. If a write/read-back fails, say so. Never claim success prematurely.
- [ ] Hand off what mathematical construction was attempted, what actually happened, its strongest failure, what is and is not established, next falsifier and exact branch/commit.

**Changing the workflow itself** (editing these rules/entrypoints) is a rare, explicit owner-authorized exception. The 2026-10-13 consolidation and the separately owner-authorized event-driven MAP policy update are such exceptions, **not** scientific discoveries. Thereafter keep writeback small and localized.
