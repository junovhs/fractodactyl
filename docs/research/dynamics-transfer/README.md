# Dynamics transfer research notebook

**Purpose:** Investigate whether Fractodactyl's nonlinear dynamics techniques transfer to useful scientific and industrial computations. This branch is a **research notebook only**. No Fractodactyl production source or build configuration is intentionally modified.

> Git branches inherit their base repository's tracked files. This branch was created from `main`; the research-specific additions live only under `docs/research/dynamics-transfer/`.

## Files

- [EXPERIMENT_LOG.md](EXPERIMENT_LOG.md) — hypotheses, preregistered kill criteria, measurements, failures, baselines, and decisions.
- [LEADS.md](LEADS.md) — candidate experiments and cheapest decisive probes.

## Method and constraints

Follow [Fractodactyl METHOD](../../spec/METHOD.md):
1. State a measurable goal and choose the best **same-math** rival before running anything.
2. Write the numeric acceptance/kill conditions before experimenting.
3. Run a cheap, local Python probe first, ideally under 30 seconds. Expand only if the gate passes.
4. Measure **cold** end-to-end time, including compilation/setup, as well as warm iteration time.
5. Mark every result as (a) empirical with sample counts, (b) certified with a proof, or (c) speculative. Do not treat sample accuracy as an error guarantee.
6. Report complete hardware/runtime and avoid attributing SciPy-interpreter overhead to mathematics.
7. Preserve negative results, especially overturned leads. Benchmark against opponents that can adopt the same optimization.
8. Work in ephemeral local scripts when useful; **only research prose/tables are committed to this notebook branch**, unless explicitly requested otherwise.

## Current status

**No demonstrated novel industrial speedup.** On a scalar forced-oscillator analogue, a local Koenigs jump beat repeated polynomial evaluation in one benchmark but lost decisively to direct polynomial composition when the competitor was allowed to compile the *same map*. The current research decision is to seek parameter-dependent, guarded, event-aware operators, not another narrow jump benchmark.

No fully certified physical-device results have been obtained. The previously discussed prototype scripts and experiments were executed in a sandbox; they are not committed here. The log records those reported observations and identifies their missing reproducibility artifacts. Future experiments should save enough reproducibility metadata to revisit the decision.

The notebook deliberately does **not** include production code, installed packages, executable scripts, or generated benchmark binaries.
