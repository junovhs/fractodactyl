# Computational Structure Lab

**A research notebook, not a renderer fork.** This branch exists to discover whether mathematical structure can replace long, expensive computations in nonlinear systems. Fractodactyl is the originating testbed, not the content of this branch.

> **BRANCH-ONLY RESEARCH SNAPSHOT — DO NOT MERGE INTO MAIN.** The visible files on this branch are documentation only. The history still descends from the original repository; its working tree does not contain the renderer. Deleting files from this branch does **not** delete them from `main`.

## Standing objective and current frontier

Our **[standing goal](GOAL.md)** is self-directed discovery of useful mathematical shortcuts for expensive computation; **no particular domain or hypothesis is compulsory**. At the start of each session agents inspect the **[live research frontier](FRONTIER.md)**, propose competing ideas and **search for existing solutions before choosing an experiment**. A published overlap causes a cited **PIVOT** to another hypothesis. The owner does not need to invent a technical plan.

Each *new* experiment requires a **[10/10 binary proof of done](PROOF_OF_DONE.md)**, including prior-art search; historical R-001–R-008 used 9/9. Formal proof, when warranted, follows the **[Lean policy](LEAN.md)**. A completed failed experiment is valuable. A good-looking benchmark without fair comparison or documentation is not complete.

## Instructions for research agents

**Start with [AGENTS.md](AGENTS.md) or [CLAUDE.md](CLAUDE.md)**—the files are identical and define the mandatory research workflow. **Prior-art HARD GATE:** Before every new numerical experiment, **actually search external literature and commit exact queries, dated results and source URLs in a [prior-art dossier](PRIOR_ART.md) alongside the new P-card**. No documented pretest search → **do not run**. **If the mathematical idea is already in prior art, document it and look for something else; do not automatically try to beat its implementation.** A faster implementation of known mathematics is not a new algorithm. Our original PWM experiment skipped this step; see the [retrospective record](PRIOR_ART/PA-PWM-RETRO.md). **Only if a substantive new mechanism/application gap survives search**, register a ≤150-word hypothesis, freeze fair baselines and accuracy/cost gates, run the smallest justified probe, file every result and verify its Git commit. An explicitly owner-requested non-novel engineering benchmark is the sole known-method exception. Use the [question](TEMPLATES/QUESTION.md), [prior-art](PRIOR_ART/TEMPLATE.md) and [result](TEMPLATES/RESULT.md) templates. **Do not add Fractodactyl source code to this branch.**

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

Finding convincing prior art *before* running is also progress: cite and pivot, with **no experiment or R-card required**. A **failed** hypothesis is a valid, permanently retained result. **A conditional win superseded by a stronger baseline is not a win.**

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

## Current frontier

**2026-10-12 literature-first update:** a search-only triage found close primary prior art for P-040's generic automatic operator selection, P-060's fast polynomial iteration and P-051's seeded parallel high-precision certification. A fourth suggestion—reusing verified work under adaptive hybrid analysis—also overlapped 2018 literature. **These historic P-cards remain UNTESTED; their generic discovery directions are PIVOT, not experimentally falsified.** The next action is a literature question about fixed-initial-condition, nonsequential high-precision interface construction, not permission for a benchmark. [FRONTIER](FRONTIER.md) records exact queries and citations. No R-card was created.

The [current frontier](FRONTIER.md) **retains** [R-008](RESULTS/R-008.md)'s measured ~1.21× synthetic repeated-use gain and has closed routine *continuous state-triggered event-time* warm-start benchmarking under [P-083](PROMPTS/P-083.md) and is searching for **different unresolved mathematical mechanisms**, not a better variant of a known event-root finder. [P-060](PROMPTS/P-060.md)'s repelling-cycle case and [P-040](PROMPTS/P-040.md)'s operator selector remain untested. **Their old ≥3× P-card thresholds are historical, not universal policy; changed-scope incremental tests require new linked cards with appropriate gates.** The frontier is re-ranked by expected payoff and test cost, not speedup spectacle.

**Compare operators only when evaluating a substantively new research question** or performing an explicitly owner-requested engineering study. Koenigs, direct polynomial composition and established domain-specific solvers supply prior-art context; their existence is a reason to search for different mathematics, not an automatic invitation to a speed contest.

## How to use this notebook

A **search-only** collision with published mathematics is recorded with a citation and a frontier pivot; it does not require a P/R card or a manufactured experiment. A *surviving experiment-worthy* question gets a short, stable **P-ID** and an edge in the [map](MAP.md). Pre-register a benchmark and kill gate before the run. Every test gets a stable **R-ID**, including negative results. Update its parent prompt, the map, the index and any subsequent forked prompts. **Do not overwrite the old decision when later evidence overturns it: link the superseding result.**

Experiments run in a temporary compute sandbox; research notes are committed here. The human does not need to run scripts. No executable code or production files are stored here unless explicitly requested.

**Navigation:** [Goal](GOAL.md) · [Frontier](FRONTIER.md) · [Map](MAP.md) · [Index](INDEX.md) · [Method](METHOD.md) · [Proof of done](PROOF_OF_DONE.md) · [Lean](LEAN.md) · [Source boundaries](SOURCES.md)

