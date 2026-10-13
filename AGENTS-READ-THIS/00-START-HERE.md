# 00 · READ ALL OF THIS FOLDER FIRST

**Mandatory full-read folder** for every agent working in the Dynamics Transfer Lab.

Read **each** file below completely, **in this order**, at the beginning of every session. Do not skim, truncate, skip passages, replace reading with a summary, or read only the files that sound relevant:

1. [00-START-HERE.md](00-START-HERE.md) — this document and authority boundaries.
2. [01-RULES-AND-METHOD.md](01-RULES-AND-METHOD.md) — scientific mission, evidence standards, and hard limits.
3. [02-LIVE-STATE.md](02-LIVE-STATE.md) — the one live scientific frontier, status, and handoff.
4. [03-WRITEBACK-CHECKLIST.md](03-WRITEBACK-CHECKLIST.md) — complete requirements before and after each research session.

**This directory is deliberately kept small.** All Markdown files directly in this folder are compulsory to read in full; no experiment logs, dossiers, scripts, or expanding archives belong in it. Add a new compulsory document only when a genuinely new permanent rule cannot fit an existing one.

## Single-source-of-truth rule

- These four files are **authoritative** for operations and the live scientific state. The root AGENTS.md and CLAUDE.md are identical entrypoint pointers, not alternative policy sources.
- Other root Markdown files (including README.md, GOAL.md, FRONTIER.md, METHOD.md, INDEX.md, MAP.md, SOURCES.md, PRIOR_ART.md, PROOF_OF_DONE.md and LEAN.md) are **optional, preserved historical/reference snapshots**. Consult them when genuinely useful; they are not mandatory to read and **never** require routine synchronization. If old process guidance conflicts, follow this folder.
- PROMPTS/, RESULTS/, PRIOR_ART/ and TEMPLATES/ are on-demand evidence, frozen experimental questions, preregistrations and samples. Historic gates, measurements and prior-art provenance cannot be altered merely to fit a new narrative.
- The full mathematical details and source links for a new experiment go **in its R-card**, not in this compulsory folder. Keep 02-LIVE-STATE.md concise with pointers to those records.

## Normal change surface

A normal executed exploratory session changes **exactly two files**:
- a **new immutable-style report** in RESULTS/R-NNN.md (or a linked correction to an existing report, clearly marked); and
- [02-LIVE-STATE.md](02-LIVE-STATE.md) for the latest finding, active direction, and next falsifier.

No other existing file should be updated for routine session housekeeping. Formal preregistration is an explicit exception detailed in the checklist. A session that produces no new evidence may have **no Git changes**; never invent work to fill the form.

## Non-negotiable scope

Only branch **research/dynamics-transfer-lab** in **junovhs/fractodactyl**. Documentation-only. Never touch, merge, rewrite or force-push other branches, and never add renderer code, executables, dependencies or generated binaries without a new explicit owner instruction. Computations may run in the normal Chat sandbox. If a mandatory document cannot be read in full, report the limitation rather than proceeding on a truncated view.

This organization was installed **2026-10-13** as a documentation-workflow change, not a scientific discovery.
