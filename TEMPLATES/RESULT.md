# R-NNN · Precise experiment title

**Date:** YYYY-MM-DD · **Question:** [P-NNN](../PROMPTS/P-NNN.md)
**Original gate verdict:** PASS / FAIL / NOT TESTED
**Observed gain and current opportunity:** KEEP / CONDITIONAL / STOP / SUPERSEDED / BLOCKED (separate from gate)
**Failure category:** MATHEMATICS / ACCURACY / PERFORMANCE / BASELINE / COVERAGE / RESOURCE / INCONCLUSIVE
**Evidence:** SPECULATION / REPORTED / REPRODUCED / CERTIFIED
**Preregistered:** Yes (link/hash), or No (retrospective)

## Question and frozen gate

Quote the exact hypothesis, test and numerical pass/kill thresholds as preregistered. Record changes explicitly, never quietly move a goalpost.

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
- [ ] Exact equations, tested parameters, inputs, count and seed recorded
- [ ] Strongest fair opponent recorded, or first-gate skip justified
- [ ] Actually executed observation linked/archived (no invented data)
- [ ] Independent accuracy results/classes or transparent failure documented
- [ ] Cold and warm costs/repetitions, or `NOT RUN — accuracy failed`
- [ ] Binary gate outcome and honest decision with measured units
- [ ] Limits, environment, evidence level and next cheapest step recorded
- [ ] Parent/INDEX/MAP/frontier consistency + branch commit/read-back verified

**Only 9/9 = POD: YES.** See [proof policy](../PROOF_OF_DONE.md). POD is process completion, **not** a correctness theorem.

## Interpretation and next edge

State the **frozen gate result** and **any smaller positive effects independently** (including effects <1.10× or <3×), what the test does *not* prove, uncertainty, accuracy/certification, and whether the expected total saved compute justifies a new test. Never discard an accurate effect only for missing an old ambition; also never call a noisy or wrong result a gain. Link child cards, preserve failures and label reported-only evidence.
