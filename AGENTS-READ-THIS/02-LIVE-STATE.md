# 02 · LIVE STATE — the only current scientific handoff

**Updated:** 2026-10-13 · **Purpose of update:** consolidate documentation obligations only. **NO NEW NUMERICAL EXPERIMENT** was run for this reorganization.

## Goal and live status

Find trustworthy, transferable mathematical shortcuts for expensive nonlinear/scientific computations by deriving and falsifying new consequences of existing theory. **No industrial simulator, industrial speedup, general nonlinear compiler, certified full time-parallel solver, or proven new algorithm has been demonstrated.**

**Active question:** fixed-initial-condition nonsequential seeding for unstable quadratic dynamics, building on [P-051](../PROMPTS/P-051.md) and the reported exploratory [R-010](../RESULTS/R-010.md). The original P-051 frozen 256–4096-step MPFR/performance experiment remains **UNTESTED**.

**Latest observation (R-010, REPORTED / EXPLORATORY / POD NO):** in one highly special real case c=-2+2^(-256), degree-32 repelling-chart seeding gave ~2.260e-39 absolute error at n=132 against a high-precision serial oracle without generating the intermediate prefix. Naively asking the chart for n=134 failed by ~10.79, while two direct steps from the accepted n=132 seed gave ~3.079e-38 error. A derived **ideal-arithmetic** residual admission budget distinguished the tested safe/unsafe chart evaluations. Floating arithmetic was not interval-enclosed, broad-domain usefulness and mathematical originality are unverified, correction still had serial dependency, and **no runtime savings were measured**. Read R-010 for exact equations, numbers, caveats and related primary literature.

## Next discriminating research step

**Priority 1:** construct outward-rounded enclosures for the repelling-chart fixed-z0 interface, including alpha/lambda, coefficient recurrence, chart inversion, truncation residual, entry rounding and direct-tail propagation. Compare those acceptance/rejection decisions to independent high-precision serial iteration. Include samples away from c≈-2 and varied repelling cycles/segment lengths; preserve explicit counterexamples. If useful intervals survive, investigate whether admitted independent seeds actually eliminate enough serial work to offset full precision, chart construction and correction costs.

**Alternative if rejected:** [P-040](../PROMPTS/P-040.md) certificate-admitted operator selection or [P-060](../PROMPTS/P-060.md) validated repelling-cycle composition/radius. Derive a *new* observable consequence before implementing a familiar algorithm.

**Important distinction:** a small interface residual, shadowing orbit or fast local warm evaluation cannot alone establish exact fixed-initial-condition certification or cold speedup.

## Condensed historical orientation (optional detail in records)

- [R-001](../RESULTS/R-001.md)–[R-004](../RESULTS/R-004.md): fitted/Koenigs ideas lost ground to stronger compiled or direct-composition baselines; broad-chart accuracy failed.
- [R-005](../RESULTS/R-005.md): an unrestricted global Newton scan diverged; motivates P-051's seeded short-segment question.
- [R-006](../RESULTS/R-006.md)–[R-008](../RESULTS/R-008.md): PWM degree-4 failed accuracy; degree-5 achieved a **reported** scoped 1.209x total synthetic repeated-use result, but related polynomial converter methods pre-existed the study. No claim of novelty or industrial gain.
- [R-009](../RESULTS/R-009.md): conditional warm-only known event-time-method measurement, no independent new mathematical method.
- [R-010](../RESULTS/R-010.md): exploratory chart admission/handoff, not certified/benchmarked.

## Recent handoff decisions (maximum three entries; full history remains in RESULTS)

- **2026-10-13 — ORGANIZATION ONLY:** compulsory full-read instructions centralized here with the checklist. New ordinary sessions write only an R-card plus this state document. All root research documents except AGENTS.md and CLAUDE.md are optional historical/reference snapshots and no longer routinely synchronized. **No experiment run, gate changed, or old result overwritten.**
- **2026-10-12 — R-010:** conditional accurate chart seed followed by an adaptive direct tail; catastrophic chart failure at n=134; next falsifier is arithmetic certification and wider-domain testing.

## Update rule for future agents

After a meaningful executed session, update **only this file and the new R-card**. Keep this document concise: revise active status, precise latest observation **with R-card link**, next falsifier, and up to three recent handoffs. Never paste full equations, tables, literature dossiers or a growing history here; those belong in RESULTS or on-demand references. Update an R-card's status transparently if evidence is later corrected. Do not silently change frozen old gates.
