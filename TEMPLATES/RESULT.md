# R-NNN · Precise experiment title

**Date:** YYYY-MM-DD · **Question:** [P-NNN](../PROMPTS/P-NNN.md)
**Original gate verdict:** PASS / FAIL / NOT TESTED
**Observed gain and current opportunity:** KEEP / CONDITIONAL / STOP / SUPERSEDED / BLOCKED (separate from gate)
**Failure category:** MATHEMATICS / ACCURACY / PERFORMANCE / BASELINE / COVERAGE / RESOURCE / INCONCLUSIVE
**Evidence:** SPECULATION / REPORTED / REPRODUCED / CERTIFIED
**Preregistered:** Yes (link/hash), or No (retrospective)
**PRIOR ART:** PRETEST VERIFIED / BLOCKED / RETROSPECTIVE
**Search evidence:** [PA-P-NNN](../PRIOR_ART/PA-P-NNN.md) · pretest commit SHA
**Claim class:** KNOWN_METHOD_BENCHMARK / KNOWN_METHOD_NEW_APPLICATION / OPEN_GAP_HYPOTHESIS / NOVELTY_UNVERIFIED

## Question and frozen gate

Quote the exact hypothesis, test and numerical pass/kill thresholds as preregistered. Record changes explicitly, never quietly move a goalpost.

## Prior-art findings (must precede new experiments)

Exact executed queries, dated search provider and source/result links are in the linked committed PA file. Name closest known paper/method, specialist rival, overlapping mechanism/application, and *specific* unresolved gap. Say **KNOWN METHOD / ENGINEERING BENCHMARK** when literature already establishes the idea. If missing pretest evidence: **POD: NO**, not a new discovery.

## Setup and opponent

Mathematical equations, test domain, initial states/parameters, seeds, hardware, language, package/compiler versions, precision, input count, iterations/horizons. State the strongest accurate specialized or same-math competitor and its permitted preprocessing. Provide command/inline minimal reproduction or mark unarchived.

## Measurements

| Metric (with units) | Candidate | Best accurate rival |
|---|---:|---:|
| Cold end-to-end, median [min,max], n repeats | | |
| Setup/build/precompute | | |
| Warm execution | | |
| Realistic amortized total (if relevant) | | |
| Absolute wall time saved and full-workload speedup | | |
| Timing variability / uncertainty / sample count | | |
| Worst independent state error | | |
| Wrong event/class counts | | |
| Coverage / abstentions | | |

Include raw observed numbers and explicit measurement uncertainty. If unrun, write **NOT MEASURED** (never guess).

## Binary proof of done

**POD: NO** (change to YES only after every box is checked, including committed read-back)
**GATE:** PASS / FAIL / NOT TESTED
**PROOF:** NONE / INTERVAL / LEAN / OTHER

- [ ] P-ID, threshold, and honest preregistration status recorded
- [ ] **Pretest external PRIOR-ART SEARCH:** executed queries + date/provider + real results, nearest primary source, class/delta, dossier and P-card pretest Git commit read back
- [ ] Exact equations, tested parameters, inputs, count and seed recorded
- [ ] Strongest fair opponent recorded, or first-gate skip justified
- [ ] Actually executed observation linked/archived (no invented data)
- [ ] Independent accuracy results/classes or transparent failure documented
- [ ] Cold and warm costs/repetitions, or `NOT RUN — accuracy failed`
- [ ] Binary gate outcome and honest decision with measured units
- [ ] Limits, environment, **prior-art novelty class**, evidence level and next cheapest step recorded
- [ ] Parent/INDEX/MAP/frontier consistency + branch commit/read-back verified

**Only 10/10 = POD: YES for new R-cards.** See [proof policy](../PROOF_OF_DONE.md). POD is process completion, **not** a correctness theorem.

## Interpretation and next edge

State the **frozen gate result** and **any smaller positive effects independently** (including effects <1.10× or <3×), what the test does *not* prove, uncertainty, accuracy/certification, and whether the expected total saved compute justifies a new test. Never discard an accurate effect only for missing an old ambition; also never call a noisy or wrong result a gain. Link child cards, preserve failures and label reported-only evidence.
