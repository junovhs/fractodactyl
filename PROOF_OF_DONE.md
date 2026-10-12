# Binary proof of done

**NEW RULE (effective 2026-10-12 after the original R-008): the prior-art search is a blocking tenth check.** See [PRIOR_ART.md](PRIOR_ART.md). Old R-001–R-008 were filed under 9/9 and remain historical; they must not be misrepresented as having met the new standard.

**An experiment can be DONE when its hypothesis is killed.** “Done” means the scientific work was carried out and documented to a verifiable standard; it does **not** mean the hypothesis passed, a speed-up is real, the result is reproduced, or the mathematics is formally proved.

Every **new** `RESULTS/R-NNN.md` must contain the **10/10 checklist** below. The agent must write **`POD: YES` only if all ten boxes are checked**; otherwise `POD: NO`, with missing evidence explained. A completed search must have actual external queries/results and a pre-experiment Git commit; internal knowledge, retroactive citations or no-access placeholders do **not** qualify. A reported historical experiment can be useful while `POD: NO`. A pre-accuracy-gate failure may legitimately write `NOT RUN — accuracy gate failed` for timings and still be DONE, if all other requirements are met.

## Mandatory 10/10 completion checklist (new R-cards)

- [ ] **P-ID and frozen gate:** the test refers to an existing ≤150-word hypothesis and identifies whether it was registered *before* execution. A retrospective report is labeled as such and **cannot** claim pre-registration.
- [ ] **PRIOR ART EXECUTED BEFORE TESTING:** new P links a read-back, pretest-committed `PRIOR_ART/PA-P-NNN.md`, containing >=3 actual executed search queries, search date and provider, linked identifiable search results, at least one reviewed primary-source abstract/paper, closest known method, explicit novelty class/delta and **TRIAGE: PROCEED or explicit owner-requested exception**; if results fewer than three the true shortfall is recorded. Search unavailable or dossier merely retrospective => **UNCHECKED; POD: NO**.
- [ ] **Exact tested case:** equations, parameters/initial conditions, sample counts, seeds (or deterministic recipe), accuracy target, and relevant event/escape criteria are recorded.
- [ ] **Fair opponent:** identify the fastest relevant accurate same-math/specialist baseline **from the prior-art search**, or explicitly state that comparison was not run and why (e.g. first accuracy gate failed).
- [ ] **Executed evidence:** actual observation or tool output is archived/referenced; simulation, extrapolation and absent data are labeled. An interrupted/failed run is described as such.
- [ ] **Accuracy evaluated first:** report measured independent error and class/event mismatches, or state explicitly why impossible; never silently treat non-finite or missing values as passes.
- [ ] **Cost accounted for:** cold time including build/preprocessing, appropriate warm time and repeats if promoted; or clearly mark `NOT RUN` because the predeclared accuracy gate failed.
- [ ] **Two-part decision:** pass/fail each **historically frozen** numerical gate with numbers, then **separately** record any smaller credible gain and whether further pursuit merits KEEP / CONDITIONAL / STOP / SUPERSEDED. `KILL` applies only to the tested claim. Distinguish empirical accuracy from a certificate.
- [ ] **Limits, prior-art novelty class and evidence level:** distinguish an already established technique from a new claimed delta, list hardware/tool versions/reproducibility artifacts or what is missing; use `REPORTED`, `REPRODUCED` or `CERTIFIED` honestly. Include the cheapest follow-up or reason to stop.
- [ ] **Graph and Git checked:** the result, parent P-card, `INDEX.md`, `MAP.md` and current `FRONTIER.md` if changed agree, links resolve, the repository contains only research documents, and the commit/ref is verified. If Git write access fails, this is **unchecked** until filing succeeds.

The 10th box may be checked only after committing and reading back the record. Final `POD: YES` is therefore an **audited assertion** by the agent, not a substitute for independent verification. On editing a file with a Git API that cannot atomically self-verify, finish the commit then perform a read-back, and mark POD via a subsequent commit if required.

## Distinct claims — do not collapse them

| Field | Question | Meaning |
|---|---|---|
| `POD: YES/NO` | Was this experiment properly completed, filed and reviewed? | Binary process status |
| `GATE: PASS/FAIL/NOT TESTED` | Did the candidate satisfy the predeclared numerical thresholds? | Measured hypothesis decision |
| `EVIDENCE` | Is it speculative, reported, reproduced, or certified? | Strength and reproducibility of support |
| `PROOF: NONE/INTERVAL/LEAN/OTHER` | What, if anything, was mathematically proved? | Only a claim within its explicit theorem/domain |

**POD is not approval to study an already-known idea.** Literature matches should ordinarily result in a **cited PIVOT and no numerical experiment**; search-only pivots have no R-card or POD. New known-method benchmarks require an **explicit owner request**, not an agent's decision to "try to beat" the published work. R-009's historical process audit remains intact; it did not justify further known-method optimization.

**No proof of novelty or industrial impact is conferred by any of these labels.** Performance is empirical, never a theorem merely because the underlying operator is correct.

**Hard veto on claiming novel mathematics:** A verified performance gain using a known method is an optimization/replication, not necessarily a scientific discovery. The prior-art audit is necessary to file new experiments as done; it never by itself proves global novelty. No external search or no committed query/result record = `PRIOR_ART: BLOCKED` before testing and `POD: NO` if a run has already been attempted.

**Original gate versus follow-up opportunity:** An old preregistered ≥3× cold gate remains FAIL even if the run measured an accurate 2× warm improvement; a new P-card may test real reuse, **without introducing a universal 1.10× gate either**. Retain all credible positive findings, including 1% or 5% effects when clearly above measurement noise. Decide separately whether the absolute saving is worth more work. Do not erase failures, overstate `REPRODUCED`, or equate a gate failure with "worthless." POD audits process, not scientific or economic importance.

## Formal proof is selective

Use [formal verification policy](LEAN.md). Lean is most useful after a promising reusable mathematical identity survives cheap tests; it is **not** required to complete or kill every <30-second experiment. A Lean theorem alone does not certify floating-point implementation error, branch/domain admission, or wall-clock speed.
