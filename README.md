# Computational Structure Lab

**A research notebook, not a renderer fork.** This branch exists to discover whether mathematical structure can replace long, expensive computations in nonlinear systems. Fractodactyl is the originating testbed, not the content of this branch.

> **BRANCH-ONLY RESEARCH SNAPSHOT — DO NOT MERGE INTO MAIN.** The visible files on this branch are documentation only. The history still descends from the original repository; its working tree does not contain the renderer. Deleting files from this branch does **not** delete them from `main`.

## Standing objective and current frontier

Our **[standing goal](GOAL.md)** is self-directed discovery of useful mathematical shortcuts for expensive computation; **no particular domain or hypothesis is compulsory**. At the start of each session agents inspect the **[live research frontier](FRONTIER.md)**, challenge its ranking, choose the cheapest decisive experiment, and change direction when the evidence warrants. The owner does not need to invent a technical plan.

Each experiment also has a **[binary proof of done](PROOF_OF_DONE.md)**; formal proof, when warranted, follows the **[Lean policy](LEAN.md)**. A completed failed experiment is valuable. A good-looking benchmark without fair comparison or documentation is not complete.

## Instructions for research agents

**Start with [AGENTS.md](AGENTS.md) or [CLAUDE.md](CLAUDE.md)**—the two files contain exactly the same operating instructions. They define the mandatory sequence: pre-register a ≤150-word question, freeze fair baseline and kill criteria, run a quick Python probe, file every result, update the lineage graph and index, verify the Git commit. Use [question](TEMPLATES/QUESTION.md) and [result](TEMPLATES/RESULT.md) templates. **Do not add Fractodactyl source code to this branch.**

## Start here

1. **[Research map](MAP.md)** — a branching graph of research questions, tests, overturned conclusions, and new leads.
2. **[Question and experiment index](INDEX.md)** — stable IDs, status, connections, and next action.
3. **[Prompts](PROMPTS/)** — individual research questions of **150 words or fewer**, each stating mechanism, fair opponent, test and kill gate.
4. **[Results](RESULTS/)** — what was measured, what failed, and what remains unknown.
5. **[Method](METHOD.md)** — short experimental protocol and standards for evidence.

## The question

Can we automatically discover *reusable, numerically trustworthy representations* of selected nonlinear dynamics that make large collections of simulations much cheaper **than the strongest existing specialized methods**?

We are not trying to make every chaotic future computable, to sell a prettier fractal renderer, or to count acceleration against naive Python integration. We want a transferable reduction in the cost of real computation.

## Success definition

**A credible 1.10x improvement is worth considering** if it is accurate, consistently better than a strong same-math rival, and saves meaningful end-to-end compute on a real workload. Even smaller gains can matter at scale. Assess total time saved, amortization, numerical coverage, implementation risk, and research cost. **>=3x is a major-breakthrough tier, not a universal pass/fail gate.** Old preregistered experimental thresholds cannot be retroactively changed.

A **failed** hypothesis is a valid, permanently retained result. **A conditional win superseded by a stronger baseline is not a win.**

## What has happened so far

- A fitted one-period map looked fast versus SciPy; a compiled-integrator baseline overturned the cold speed-up ([R-001](RESULTS/R-001.md)).
- Broad-domain Koenigs interpolation failed the accuracy gate ([R-002](RESULTS/R-002.md)).
- Shrinking the domain made a local jump accurate; large batches showed a win **only against a weaker repeated-map competitor** ([R-003](RESULTS/R-003.md)).
- A fairer **direct composition of the same polynomial** was about **5.27× faster than Koenigs** in that toy test, killing the preferred Koenigs option ([R-004](RESULTS/R-004.md)).
- Earlier parallel-in-time Newton-scan work found severe global convergence failure and unfavorable high-precision arithmetic ([R-005](RESULTS/R-005.md)).
- A newly executed degree-four PWM switched-return test missed its predeclared 1e-7 accuracy gate on 2/128 trajectories, so no speed timing was run ([R-006](RESULTS/R-006.md)). Degree five repaired the accuracy issue but measured **only ~1.020× cold** versus compiled exact propagation, failing the ≥3× gate ([P-081](PROMPTS/P-081.md), [R-007](RESULTS/R-007.md)).
- **No industrial speed-up, no certified industrial simulator, no general-purpose nonlinear compiler has been demonstrated.**

Older results are retrospective conversation records; R-006/R-007 are new reported sandbox runs. Executable files, full raw samples and environment snapshots are **not committed to this documentation-only branch**; do not describe these records as independent reproducibility packs.

## Current frontier

The [current frontier](FRONTIER.md) has **killed the fixed-schedule PWM Taylor-kick speed claim** ([R-006](RESULTS/R-006.md), [R-007](RESULTS/R-007.md)) and now favors a *new, continuous state-triggered event-time sensitivity* probe under [P-030](PROMPTS/P-030.md). **[P-060: repelling-cycle polynomial composition](PROMPTS/P-060.md)** remains an untested alternative. Neither has a measured result. **These are starting candidates, not an assigned roadmap**—an agent should replace them when a better cheap, discriminating experiment is identified.

**Operator choices should be competed against each other.** Koenigs, direct polynomial composition, conventional integration and established domain-specific solvers are all potential winners depending on regime.

## How to use this notebook

Every new research question gets a short, stable **P-ID** file and an edge in the [map](MAP.md). Pre-register a benchmark and kill gate before the run. Every test gets a stable **R-ID**, including negative results. Update its parent prompt, the map, the index and any subsequent forked prompts. **Do not overwrite the old decision when later evidence overturns it: link the superseding result.**

Experiments run in a temporary compute sandbox; research notes are committed here. The human does not need to run scripts. No executable code or production files are stored here unless explicitly requested.

**Navigation:** [Goal](GOAL.md) · [Frontier](FRONTIER.md) · [Map](MAP.md) · [Index](INDEX.md) · [Method](METHOD.md) · [Proof of done](PROOF_OF_DONE.md) · [Lean](LEAN.md) · [Source boundaries](SOURCES.md)

**Small-gain follow-up:** [P-082](PROMPTS/P-082.md) preregisters 2,048 repeated-use PWM batches and a >=1.10x total-cost threshold, without revising R-007's historical 3x failure.

**New incremental win (2026-10-12):** A separately preregistered reuse workload, [P-082](PROMPTS/P-082.md), passes its >=1.10x cold-total gate: [R-008](RESULTS/R-008.md) measures **1.209x acceleration** (17.3% less wall time) over the compiled exact map across 2,048 synthetic batches, with independent empirical accuracy passing. This preserves rather than rewrites P-081's original failed >=3x single-cold gate. **Not an industrial or certified result.**
