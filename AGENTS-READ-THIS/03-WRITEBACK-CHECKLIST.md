# 03 · WRITEBACK CHECKLIST — one authoritative workflow

This is the **single persistent checklist**. READ THIS FILE FULLY each session. Do not edit the shared checklist to tick boxes; perform the applicable steps and report completion honestly in the result/response.

## Before working — compulsory reading and boundaries

- [ ] Confirm repo junovhs/fractodactyl and current branch research/dynamics-transfer-lab. Do not use main or any other branch.
- [ ] Read **all four Markdown files** in AGENTS-READ-THIS/ **fully**, in numbered order. No skimming, cached summary or partial excerpt.
- [ ] Identify the active gap in 02-LIVE-STATE.md; read relevant historical P/R papers and source materials **only as needed**. Historical files are not independently authoritative.
- [ ] Choose an explicit extension, prediction and falsifier; specify task/output, accuracy layer (discrete versus continuous), input access, workload, strongest fair opponent, and why this question outranks the strongest other direction. Define what result changes CONTINUE / CHALLENGE / PARK / CLOSE.

## Exploratory run (default)

- [ ] Derive/test a concrete claim with independent correctness checks where possible. First seal the executed source/config/input/command and exit status using scripts/research_run.py (or equivalent). Save completed raw samples, interruptions and exact hash verification; **do not claim execution without an actual execution record**. Separate arithmetic/omission/discretization/model error.
- [ ] If executed, create **one new RESULTS/R-NNN.md** using the next unused ID (or an explicitly linked correction). Include equations, independent check, exact error scope, raw observations, source/config fingerprint, command/exit status, available durable evidence URI and hash, rival including rejection+fallback workload, strongest failure, challenger limitations, known prior work, and status **EXPLORATORY / PREREGISTERED NO / POD NO**. Never claim a missing bundle is retrievable.
- [ ] Update **AGENTS-READ-THIS/02-LIVE-STATE.md** in the **same research-result commit** with the latest status, report link, alternative direction, evidence-backed next priority and explicit CONTINUE / CHALLENGE / PARK / CLOSE decision. Keep at most three recent entries.
- [ ] If no execution, do **not** create a fake R-card. State NOT RUN and reason; update 02-LIVE-STATE.md only if a substantive decision/obstruction changes the next research action. Otherwise no file update is required.

**Owner-approved evidence exception:** commit small reproducible sources/configs/manifest/compact raw records under EVIDENCE/R-NNN/ with an executed R-card; reusable research-only guard scripts and tests under scripts/ when genuinely improved. Use stable external archives for larger outputs, checking retrieval. Never commit binaries, dependencies, renderer code, or tool-generated clutter. Verify archive SHA-256, manifest, source/input fingerprint, successful exit, and actual file presence. A checkpoint is reusable only when run ID and source/config/input fingerprint match; the runner refuses mixing. **No routine updates to README, INDEX, MAP, FRONTIER, GOAL, METHOD, SOURCES, PRIOR_ART, PROOF_OF_DONE, LEAN, parent P-cards, or either root entrypoint.** Link any newly consulted papers and exact searches in the R-card itself. A new P/PA pair is required only for prospective confirmation, not exploratory testing.

## Event-triggered exception — curated MAP.md research lineage

- [ ] **Evaluate after each research session:** Did evidence materially change the research dependency graph, rather than simply generate a new card or better measurement? Update MAP.md **if and only if** at least one of these triggers holds:
  1. **New direction:** an evidence-backed, independently meaningful mechanism or research question creates a genuinely distinct lineage branch (not just another P-number).
  2. **Decisive overturn/closure:** a counterexample, proof, or stronger rival substantially invalidates, closes or redirects an existing mapped direction.
  3. **Enabling milestone:** a sufficiently supported mathematical or computational capability changes what later research can build on, rather than merely tightening a bound within the same hypothesis.
  4. **Convergence:** a demonstrated relation or dependency connects previously independent research lines.
- [ ] **Not triggers:** a new P/R ID alone, another exploratory measurement within an unchanged direction, a small improvement to a previously mapped mechanism, a routine status/timestamp update, a literature citation without a material dependency change, or a workflow-only policy edit. **R-011 is an incremental P-051/R-010 refinement, not a new graph edge.**
- [ ] **Evidence and uncertainty:** Cite the supporting R/P/PA evidence or explicit mathematical obstruction in the revised map; mark reported or exploratory relationships as provisional and do not silently relabel them confirmed. **Formal preregistration/confirmation is not required** for a strongly supported structural change, but missing evidence is not a license to map speculative connections. The live state remains the authoritative scientific status.
- [ ] **Writeback:** Make the minimum affected node/edge changes to MAP.md and update 02-LIVE-STATE.md with the reason and evidence link, **in the same commit as the new R-card**. If an evidence-backed reinterpretation of existing records changes the graph with **no new experiment**, MAP.md and 02-LIVE-STATE.md alone may be committed with the reason stated; do not fabricate a new result. Do not update INDEX/FRONTIER/README as housekeeping. If no trigger fires, **leave MAP.md unchanged** and say so in the research handoff.

## Promotion and challenge gate (before substantial engineering or confirmation)

- [ ] State the claim against the closest known specialist method, not merely an implemented weak baseline; identify exactly which capability, assumption or workload is new. Do focused primary-source reading before escalating novelty claims.
- [ ] Freeze a held-out challenge specification, strongest feasible guarantee-matched rival, and budget for all queries (successful, rejected, fallback, cold setup and reuse). Demand a second challenge pass distinct from the implementation/replay, retaining critiques; if no independent reviewer is available, label that limitation and do not claim independent reproduction.
- [ ] Choose CONTINUE/CHALLENGE/PARK/CLOSE with a one-paragraph opportunity-cost argument comparing another research direction. The next task cannot be justified solely by an unresolved subproblem or an improved toy ratio. One cheap falsifier may precede formal searches, but never silently promote exploration to confirmation.

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
5. Actually executed, source-fingerprint-linked, retrieved and independently replayable observations (record limitations if not); no fabricated timings or artifacts.
6. Independent error and discrete classifications, failures/abstentions before cost claims.
7. Cold/setup, warm, amortization, abstention/fallback and realistic workload where relevant, or justified NOT MEASURED with no performance claim.
8. Frozen-gate verdict separate from any smaller true effect, utility and decision.
9. Novelty, known ingredients, limits, proof scope and REPORTED/REPRODUCED/CERTIFIED provenance.
10. R-card and **02-LIVE-STATE.md** links/status consistent (and MAP.md only if its structural trigger fired), committed **only to the lab branch**, with read-back confirmed. This supersedes automatic INDEX/MAP/FRONTIER synchronization.

Only when all are real and checked is **POD: YES** permissible; otherwise **POD: NO**, even for a useful exploratory or failed trial. Lean is selective; never label a numerical output CERTIFIED without the actual linked proof/interval enclosure.

## Final Git audit — every write

- [ ] Confirm the target is only research/dynamics-transfer-lab, preserve previous history and frozen gates, add no code/dependencies/binaries/renderer and do not merge or force-push.
- [ ] For ordinary executed exploration, verify the changed-path list contains RESULTS/R-NNN.md (or linked correction), AGENTS-READ-THIS/02-LIVE-STATE.md, **bounded EVIDENCE/R-NNN/** files for actual execution, and MAP.md **only** on the evidence-backed structural trigger. No research-source artifacts are required for NOT RUN sessions. For a structural reassessment without execution, MAP.md + live state may be the only changed files. For confirmation, allow the explicitly required P/PA pair in its pretest commit. For an owner-authorized workflow change, only the necessary policy/entrypoint/map-description files may differ.
- [ ] Commit with a specific message; read the branch tip and every changed file back; check that evidence and live summary agree. If a write/read-back fails, say so. Never claim success prematurely.
- [ ] Hand off what mathematical construction was attempted, what actually happened, its strongest failure, what is and is not established, next falsifier and exact branch/commit.

**Changing the workflow itself** (editing rules/entrypoints or adding an evidence harness) is a rare explicit owner-authorized exception. The owner approved the 2026-10-13 durability, run-fingerprint, challenger, and decision-gate changes as workflow-only, as well as the limited research source/evidence exception. These are **not** scientific discoveries. The original 2026-10-13 consolidation and MAP policy changes remain workflow-only. Thereafter keep writeback small and localized.
