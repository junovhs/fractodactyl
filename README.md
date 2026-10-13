# Computational Structure Lab

**A research notebook, not a renderer fork.** This branch exists to discover whether mathematical structure can replace long, expensive computations in nonlinear systems. Fractodactyl is the originating testbed, not the content of this branch.

> **BRANCH-ONLY RESEARCH SNAPSHOT — DO NOT MERGE INTO MAIN.** The visible files on this branch are documentation only. The history still descends from the original repository; its working tree does not contain the renderer. Deleting files from this branch does **not** delete them from `main`.

## Standing objective and current frontier

Our **[standing goal](GOAL.md)** is self-directed discovery of useful mathematical shortcuts for expensive computation; **no particular domain or hypothesis is compulsory**. At the start of each session agents inspect the **[live research frontier](FRONTIER.md)**, extract useful mathematical ingredients from published work, derive extensions or syntheses, and **run a small falsifying calculation when accessible**. Finding similar prior art should prompt the question *how can we use it to go farther?*, not automatic abandonment or a race to beat the paper. The owner does not need to invent the technical plan.

**Exploratory discovery probes may run immediately**, with honest equations, outputs and uncertainty. Formal **confirmatory claims** require the [10/10 proof-of-done audit](PROOF_OF_DONE.md), including a pretest literature dossier; historical R-001–R-008 used 9/9. A failed probe that exposes a counterexample is useful evidence, while an attractive unverified benchmark is not certification. Formal proof follows the selective [Lean policy](LEAN.md).

## Instructions for research agents

Read [AGENTS.md](AGENTS.md) or identical [CLAUDE.md](CLAUDE.md). **Build on prior art to invent something farther-reaching.** Use a published theorem or algorithm as a component of a new construction, new capability or new proof—not simply a reason to pivot or an opponent to outrun. Cheap exploratory mathematics (symbolic or numerical) can precede formal P/PA preregistration if explicitly labeled exploratory. Before a new **confirmatory** claim, perform real literature searches and commit the [prior-art dossier](PRIOR_ART.md) plus P-card; disclose earlier experiments rather than pretending they were preregistered. Validate rigorously and preserve negative results. All computation can happen in ordinary Chat; no Work-mode handoff or manual runner is required. **Do not add Fractodactyl source code or merge this branch into main.**

## Start here

1. **[Research map](MAP.md)** — a branching graph of research questions, tests, overturned conclusions, and new leads.
2. **[Question and experiment index](INDEX.md)** — stable IDs, status, connections, and next action.
3. **[Prompts](PROMPTS/)** — individual research questions of **150 words or fewer**, each stating mechanism, fair opponent, test and kill gate.
4. **[Results](RESULTS/)** — what was measured, what failed, and what remains unknown.
5. **[Method](METHOD.md)** — short experimental protocol and standards for evidence.

## The question

Can we uncover *new, reusable, numerically trustworthy mathematical structure* for expensive nonlinear computation? Existing specialized methods are **reality checks for justified new questions**, not the default competitors in a race to optimize known ideas.

We are not trying to make every chaotic future computable, to sell a prettier fractal renderer, or to count acceleration against naive Python integration. We want a transferable reduction in the cost of real computation.

## Success definition

**No global speedup threshold applies to new discoveries.** A robust **1% improvement** may be valuable in a frequently run scientific calculation; 10%, 30%, 2× and larger improvements are also worth measuring and preserving. To choose research effort, assess accuracy, fastest fair opponent, timer noise, **absolute compute saved** under realistic one-shot or repeated usage, coverage, transferability and implementation cost. Do **not** substitute ≥3× or ≥1.10× for that assessment: 3× is an ambitious breakthrough label, not a minimum. Old preregistered numerical gates remain unchanged. Report the *original gate verdict* separately from *the smaller effect retained and its follow-up priority*.

Finding convincing prior art is useful **when it enables a new derived question or construction**. Cite exactly what is already solved, then test an unanswered implication rather than automatically pivoting. A failed calculation and a mathematically explained obstacle are valuable retained results. A conditional win superseded by a stronger baseline is not a confirmed gain.

## What has happened so far

- A fitted one-period map looked fast versus SciPy; a compiled-integrator baseline overturned the cold speed-up ([R-001](RESULTS/R-001.md)).
- Broad-domain Koenigs interpolation failed the accuracy gate ([R-002](RESULTS/R-002.md)).
- Shrinking the domain made a local jump accurate; large batches showed a win **only against a weaker repeated-map competitor** ([R-003](RESULTS/R-003.md)).
- A fairer **direct composition of the same polynomial** was about **5.27× faster than Koenigs** in that toy test, killing the preferred Koenigs option ([R-004](RESULTS/R-004.md)).
- Earlier parallel-in-time Newton-scan work found severe global convergence failure and unfavorable high-precision arithmetic ([R-005](RESULTS/R-005.md)).
- The fourth-degree PWM return missed its independent error gate ([R-006](RESULTS/R-006.md)); degree five passed accuracy, showed **2.027× warm**, but failed its **historical ≥3× one-shot cold** gate ([R-007](RESULTS/R-007.md)). A **separately preregistered repeated-use test** found **1.209× end-to-end** savings (17.29% less wall time) on a synthetic workload versus the strong exact operator ([R-008](RESULTS/R-008.md)). **This is a retained scoped incremental win**, not a new industrial theorem or renderer improvement.
- **The PWM polynomial method was already known in closely related work:** Luciano & Strollo's [1990 fast switched-converter algorithm](https://doi.org/10.1109/63.56528) used polynomial transition approximations including a buck converter. This was identified *only after* our experiments. The explicit [retrospective search log](PRIOR_ART/PA-PWM-RETRO.md) corrects the interpretation: R-008 is a **known-method performance measurement**, not a new algorithm.
- **No industrial speed-up, no certified industrial simulator, no general-purpose nonlinear compiler and no novel PWM mathematical method have been demonstrated.**

Older results are retrospective conversation records; R-006/R-007 are new reported sandbox runs. Executable files, full raw samples and environment snapshots are **not committed to this documentation-only branch**; do not describe these records as independent reproducibility packs.

- **P-083/R-009** tested a continuous state-triggered event model with a preregistered [prior-art dossier](PRIOR_ART/PA-P-083.md). The quadratic warm start had **1.277× warm** speed over a compiled safeguarded Newton opponent, with empirical accuracy pass and no event-class mistakes, but **cold was slower** and every event still needed additional root corrections. [R-009](RESULTS/R-009.md) retains its **CONDITIONAL** warm measurement, but the discovery line is now **CLOSED/PIVOT**: relevant prior art had already been found, so running an additional standard-method speed contest was a research-priority mistake.
- **P-051/R-010** explored a fixed-initial-condition trajectory shortcut using a repelling-point polynomial chart, a proposed nonsequential error-admission bound, and direct continuation. The reported q=32 seed at step 132 had ~2.26e-39 empirical error for c=-2+2^(-256); direct chart evaluation at step 134 was catastrophically wrong (~10.79 error), whereas two ordinary recurrence steps from the accepted seed had ~3.08e-38 error. [R-010](RESULTS/R-010.md) is **EXPLORATORY / POD: NO**: no interval certificate, speed benchmark, proven originality, or general solution. The original P-051 frozen test remains untested.

## Current frontier

**2026-10-12 research update:** the earlier literature-first search-only elimination of P-040, P-051 and P-060 was premature as a **prohibition on experiments**. All three original P-cards retain their untested frozen gates; known techniques are now being used as ingredients for new falsifiable extensions. The first [P-051](PROMPTS/P-051.md) extension is documented in [R-010](RESULTS/R-010.md): a reported numerical chart-seeding/handoff probe with a conditional analytic error budget and a sharp failure case. That exploratory observation is **not** a certified method or measured computational savings. [FRONTIER](FRONTIER.md) now prioritizes rigorous finite-arithmetic enclosures, out-of-domain counterexamples, and fair total-cost accounting before any stronger claim.

Prior results remain unchanged: [R-008](RESULTS/R-008.md) records a reported 1.209× **synthetic** reuse gain from a known polynomial technique, and [R-009](RESULTS/R-009.md) a conditional warm-only known-method observation. Their evidence does not imply a new mathematical method or industrial benefit. Different extensions may use these known ideas without hiding their provenance.

## How to use this notebook

Use the [map](MAP.md) and [index](INDEX.md) to choose an unanswered mathematical question, then derive and **execute an exploratory calculation** in Chat. Log even a negative run with equations, exact observed values, and its novelty limitations; `POD: NO` is honest for exploration. Escalate credible results to a pretest-registered confirmatory P/PA experiment and [10/10 audit](PROOF_OF_DONE.md), using a fair rival only when relevant. Never overwrite old gates or results; link superseding evidence. Research records are committed here, while executable code stays in the compute sandbox unless the owner changes that rule.

**Navigation:** [Goal](GOAL.md) · [Frontier](FRONTIER.md) · [Map](MAP.md) · [Index](INDEX.md) · [Method](METHOD.md) · [Proof of done](PROOF_OF_DONE.md) · [Lean](LEAN.md) · [Source boundaries](SOURCES.md)

