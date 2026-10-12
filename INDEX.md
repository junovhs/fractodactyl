# Indexed research graph

P = short research-question prompt (≤150 words). R = experiment/result, retained even if negative. The text under **Status** is the **latest decision**, not necessarily the original hypothesis verdict.

| ID | Status | Question | Result / immediate dependency |
|---|---|---|---|
| [P-000](PROMPTS/P-000.md) | ONGOING | Can nonlinear computation be compiled automatically? | Root |
| [P-010](PROMPTS/P-010.md) | SUPERSEDED | Does a fitted return map beat direct simulation? | [R-001](RESULTS/R-001.md) |
| [P-020](PROMPTS/P-020.md) | KILLED BROAD | Koenigs on a driven scalar system | [R-002](RESULTS/R-002.md) |
| [P-021](PROMPTS/P-021.md) | CONDITIONAL | Shrink chart and amortize | [R-003](RESULTS/R-003.md), overturned by [R-004](RESULTS/R-004.md) |
| [P-022](PROMPTS/P-022.md) | CLOSED | Same-math direct polynomial competition | [R-004](RESULTS/R-004.md) |
| [P-030](PROMPTS/P-030.md) | UNTESTED · PRIORITY 1 | Parameter-dependent switching maps | Follows R-004 |
| [P-040](PROMPTS/P-040.md) | UNTESTED · PRIORITY 2 | Automatic cost/certificate selector | Follows R-004 |
| [P-050](PROMPTS/P-050.md) | KILLED GLOBAL | Million-step high-precision Newton scan | [R-005](RESULTS/R-005.md) |
| [P-051](PROMPTS/P-051.md) | UNTESTED | Certified segmented time-parallel solve | Follows R-005 |
| [P-060](PROMPTS/P-060.md) | UNTESTED | Direct composition at repelling Mandelbrot cycle | Follows R-004 |
| [P-070](PROMPTS/P-070.md) | UNTESTED · AFTER P-030 | Multistate validity/event extension | Depends on P-030 |

## Experiment results

| ID | Evidence | Accuracy | Cold-cost conclusion | Latest decision |
|---|---|---|---|---|
| [R-001](RESULTS/R-001.md) | REPORTED | Empirical only | Compiled RK4 wins over fitted map at 100k; old SciPy comparison unfair | **KILL apparent broad speed-up** |
| [R-002](RESULTS/R-002.md) | REPORTED | Broad-domain Koenigs fails | No valid timing claim | **KILL** |
| [R-003](RESULTS/R-003.md) | REPORTED | Local empirical pass | ~4.04× over repeated map at 8192 × 64; not fair final rival | **SUPERSEDED** |
| [R-004](RESULTS/R-004.md) | REPORTED | Empirical sampled comparison | Direct composition ~5.27× faster cold than Koenigs | **KILL Koenigs preference** |
| [R-005](RESULTS/R-005.md) | REPORTED | Global Newton diverges | Full-width scan too costly at high precision | **KILL global scan** |

No demonstrated novel industrial gain, no certified speed-up, no runnable artifacts committed.
