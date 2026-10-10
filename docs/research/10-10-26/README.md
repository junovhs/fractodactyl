# Research — October 10, 2026

This directory collects the six-agent accelerator investigations and their follow-up implementations. Read the final synthesis first, then the individual reports.

## Decision memo and issue filing order

**[Six-agent final takeaways and ordered Ishoo issue roadmap](six-agent-final-takeaways-and-issue-roadmap.md)** — measured versus speculative results; prioritizes factored returns, automatic dynamical compilation and GPU optimization; lists existing blockers, proposed new issues, dependencies, acceptance tests and stop conditions.

## Reports and runnable experiments in this directory

- **Agent #4 — Analytic exterior fields:** [research report](mandelbrot_exterior_field_research.md), [fast C++ prototype](mandelbrot_field_benchmark.cpp), [more conservative C++ prototype](mandelbrot_field_adaptive.cpp), [raw timing CSV](mandelbrot_benchmark_measurements.csv). A standalone far-exterior CPU win, **not** a matched win over optimized deep-zoom BLA/GPU.
- **Agent #5 — GPU precision and work queues:** [architecture report](gpu-precision-work-queues/REPORT.md), [setup/readme](gpu-precision-work-queues/README.md), [queue A/B prototype](gpu-precision-work-queues/gpu_queue_ab.py). **No new GPU speedup measured**; hardware execution is the next gate.

## Agent source reports filed under the requested October 9 archive

- [Agent #1 — automatic compiler](../10-9-26/agent-investigations-10-10/01-automatic-dynamical-compiler-agent-report.md), [native research results](../10-9-26/agent-investigations-10-10/01-automatic-compiler-native-prototype-results.md), [implementation branch](https://github.com/junovhs/fractodactyl/tree/research/auto-misiurewicz-discovery).
- [Agent #2 — near-parabolic acceleration](../10-9-26/agent-investigations-10-10/02-near-parabolic-transit-agent-report.md).
- [Agent #3 — factored ladder-return operators](../10-9-26/agent-investigations-10-10/03-factored-ladder-return-maps-agent-report.md).
- [Agent #6 — cross-frame compiled mathematical film](../10-9-26/agent-investigations-10-10/06-compiled-mathematical-film-agent-report.md).

Proposed issue titles in the synthesis are **not** automatically filed in Ishoo. Reconcile the live local Ishoo ledger before creating or reprioritizing issues. Evidence and approval rules remain in [METHOD.md](../../spec/METHOD.md) and [STATE.md](../../spec/STATE.md).
