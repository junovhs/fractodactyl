# Indexed research graph

**Mission:** [GOAL](GOAL.md) · **Self-selected active frontier:** [FRONTIER](FRONTIER.md) · **Binary experiment completion:** [PROOF_OF_DONE](PROOF_OF_DONE.md).

P = short research-question prompt (≤150 words). R = experiment/result, retained even if negative. The text under **Status** is the **latest decision**, not necessarily the original hypothesis verdict.

| ID | Status | Question | Result / immediate dependency |
|---|---|---|---|
| [P-000](PROMPTS/P-000.md) | ONGOING | Can nonlinear computation be compiled automatically? | Root |
| [P-010](PROMPTS/P-010.md) | SUPERSEDED | Does a fitted return map beat direct simulation? | [R-001](RESULTS/R-001.md) |
| [P-020](PROMPTS/P-020.md) | KILLED BROAD | Koenigs on a driven scalar system | [R-002](RESULTS/R-002.md) |
| [P-021](PROMPTS/P-021.md) | CONDITIONAL | Shrink chart and amortize | [R-003](RESULTS/R-003.md), overturned by [R-004](RESULTS/R-004.md) |
| [P-022](PROMPTS/P-022.md) | CLOSED | Same-math direct polynomial competition | [R-004](RESULTS/R-004.md) |
| [P-030](PROMPTS/P-030.md) | UNTESTED · PRIORITY 1 | Parameter-dependent switching maps | Follows R-004 |
| [P-080](PROMPTS/P-080.md) | KILL | Degree-four guarded PWM map | [R-006](RESULTS/R-006.md): 2/128 exceed accuracy gate |\n| [P-081](PROMPTS/P-081.md) | IN PROGRESS · PRIORITY 1 | Change only Taylor order to five | Follows [R-006](RESULTS/R-006.md); preregistered, result pending |\n| [P-040](PROMPTS/P-040.md) | UNTESTED · PRIORITY 2 | Automatic cost/certificate selector | Follows R-004 |
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
| [R-005](RESULTS/R-005.md) | REPORTED | Global Newton diverges | Full-width scan too costly at high precision | **KILL global scan** |\n| [R-006](RESULTS/R-006.md) | REPORTED · new run | **1.056531374e-7 > 1e-7** | NOT RUN — accuracy failed | **KILL q=4** |

No demonstrated novel industrial gain or certified speed-up. R-006 has raw observations and a reconstruction recipe; scripts remain outside this documentation-only Git branch.
