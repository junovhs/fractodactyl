# Binary proof of done

**An experiment can be DONE when its hypothesis is killed.** “Done” means the scientific work was carried out and documented to a verifiable standard; it does **not** mean the hypothesis passed, a speed-up is real, the result is reproduced, or the mathematics is formally proved.

Every new `RESULTS/R-NNN.md` must contain the checklist below. The agent must write **`POD: YES` only if all 9 boxes are checked**; otherwise `POD: NO`, with missing evidence explained. A reported historical experiment can be useful while `POD: NO`. A pre-accuracy-gate failure may legitimately write `NOT RUN — accuracy gate failed` for timings and still be DONE, if all other requirements are met.

## Mandatory 9/9 completion checklist

- [ ] **P-ID and frozen gate:** the test refers to an existing ≤150-word hypothesis and identifies whether it was registered *before* execution. A retrospective report is labeled as such and **cannot** claim pre-registration.
- [ ] **Exact tested case:** equations, parameters/initial conditions, sample counts, seeds (or deterministic recipe), accuracy target, and relevant event/escape criteria are recorded.
- [ ] **Fair opponent:** identify the fastest relevant accurate same-math/specialist baseline, or explicitly state that comparison was not run and why (e.g. first accuracy gate failed).
- [ ] **Executed evidence:** actual observation or tool output is archived/referenced; simulation, extrapolation and absent data are labeled. An interrupted/failed run is described as such.
- [ ] **Accuracy evaluated first:** report measured independent error and class/event mismatches, or state explicitly why impossible; never silently treat non-finite or missing values as passes.
- [ ] **Cost accounted for:** cold time including build/preprocessing, appropriate warm time and repeats if promoted; or clearly mark `NOT RUN` because the predeclared accuracy gate failed.
- [ ] **Binary decision:** pass/fail each predeclared threshold with numbers; report **KEEP / KILL / CONDITIONAL / SUPERSEDED / BLOCKED** and distinguish empirical accuracy from an error certificate.
- [ ] **Limits and evidence level:** list hardware, tool versions and reproducibility artifacts or what is missing; use `REPORTED`, `REPRODUCED` or `CERTIFIED` honestly. Include the cheapest follow-up or an explicit reason to stop.
- [ ] **Graph and Git checked:** the result, parent P-card, `INDEX.md`, `MAP.md` and current `FRONTIER.md` if changed agree, links resolve, the repository contains only research documents, and the commit/ref is verified. If Git write access fails, this is **unchecked** until filing succeeds.

The 9th box may be checked only after committing and reading back the record. Final `POD: YES` is therefore an **audited assertion** by the agent, not a substitute for independent verification. On editing a file with a Git API that cannot atomically self-verify, finish the commit then perform a read-back, and mark POD via a subsequent commit if required.

## Distinct claims — do not collapse them

| Field | Question | Meaning |
|---|---|---|
| `POD: YES/NO` | Was this experiment properly completed, filed and reviewed? | Binary process status |
| `GATE: PASS/FAIL/NOT TESTED` | Did the candidate satisfy the predeclared numerical thresholds? | Measured hypothesis decision |
| `EVIDENCE` | Is it speculative, reported, reproduced, or certified? | Strength and reproducibility of support |
| `PROOF: NONE/INTERVAL/LEAN/OTHER` | What, if anything, was mathematically proved? | Only a claim within its explicit theorem/domain |

**No proof of novelty or industrial impact is conferred by any of these labels.** Performance is empirical, never a theorem merely because the underlying operator is correct.

## Formal proof is selective

Use [formal verification policy](LEAN.md). Lean is most useful after a promising reusable mathematical identity survives cheap tests; it is **not** required to complete or kill every <30-second experiment. A Lean theorem alone does not certify floating-point implementation error, branch/domain admission, or wall-clock speed.
