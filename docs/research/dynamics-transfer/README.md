# Dynamics transfer lab

**Purpose:** test whether mathematical operators developed for Fractodactyl transfer to valuable non-fractal nonlinear computations. This is an experimental lab, **not** a production change or a claim of new industrial algorithms.

Source of truth: this branch `research/dynamics-transfer-lab`; `main` remains untouched.

## Reproducible protocol

1. Form a falsifiable mechanism, industrial use case, and strongest opponent *before* writing an expensive benchmark.
2. State numerical accuracy, work/cold time, memory, scope, and an explicit kill criterion **before running**.
3. First probe: usually <30 s; promote only on a pass, in accordance with [METHOD.md](../../spec/METHOD.md). Stop on the first accuracy failure.
4. Use a fair same-math rival: allow a conventional competitor to cache/compose exactly the same operators.
5. Separate sampled numeric comparisons from proofs; neither floating-point agreement nor (10^{-7}) state error is an interval certificate.
6. Record actual timing context, errors, construction overhead, compiler/JIT costs, and whether the hardware/opponent match industry practice.
7. Record negative experiments forever. A lead can be *narrowly positive* and still be **killed** when a better opponent is tested.
8. Before promoting externally, test a genuine switching electrical model with piecewise dynamics, derivatives, and automatic parameter domains, comparing with an optimized event-aware solver / periodic-shooting implementation and considering established SIMPLIS POP.

## Current experiment series

See [RESULTS.md](RESULTS.md) for the immutable observations and revised conclusion.
The Python benchmark archive created in the same investigation is included as `scripts/` and `results/` where available. The most important cross-check is EXP-004: a direct 64-return composed polynomial destroys EXP-003's apparent advantage.

## Ranking for next hypotheses (all unproven)

**Lead A — guarded parameter-dependent return composition.** Use `P(x,c)` on a switching or event-driven converter model, with automatically split validity boxes, event-sequence verification and interval bounds. Work unit: an entire *design parameter sweep*, not one stroboscopic fixed point. **Kill:** <3x cold against an optimized cached composition/event-aware solver, or any wrong event/class, or unbounded residual >1e-7 state on a broad holdout. **Cheapest test:** 100–1,000 parameter/state combinations, 16–64 cycles, Python/Numba with deterministic event branches. *Speculative upside*: repeated converter reliability and optimization sweeps.

**Lead B — optimal operator selection across horizons.** Choose between direct RK4, linear jump, direct polynomial composition, and Koenigs automatically based on *cost and certified domain*. **Kill:** added selection and certificates cost more than 10% without a >=3x global cold win over an oracle-selected same-math baseline. **Cheapest test:** recycle current scalar oscillator and sweep radii/return counts. Risk: the oracle-selected baseline leaves little headroom.

**Lead C — nonlinear multi-state charts (2D).** Transfer only after A passes. **Kill:** if dense multivariate coefficients exceed direct optimized integration or accurate local chart domains shrink too rapidly. **Cheapest test:** 2D Duffing Poincaré map, 3rd/5th-order jets, 100-point holdouts.

## Prior-art boundary

Commercial SIMPLIS already computes periodic operating points of switched power supplies without simulating full startup. Direct polynomial composition, Koenigs/Sternberg linearization, and local stroboscopic maps are established concepts. The project must show an **automatic, error-bounded, parameter-dependent, useful-domain and cold-cost advantage**, not just rediscover return maps.

References:
- https://www.simplis.com/documentation/simplis/simplis_am/topics/periodic_operating_point.htm
- https://www.simplis.com/documentation/simplis/ast_01/topics/1_0_5_pop_and_ac_analysis.htm

## How to rerun

From `scripts/` with Python packages `numpy scipy numba`:

```bash
OPENBLAS_NUM_THREADS=1 python experiment_001.py
OPENBLAS_NUM_THREADS=1 python experiment_002.py
OPENBLAS_NUM_THREADS=1 python experiment_003.py
OPENBLAS_NUM_THREADS=1 python experiment_004.py
```

Run in order. Numerical timing varies with CPU and warm-up; do not compare absolute timings from different machines. The experiments are scalar mathematical analogues, **not** calibrated physical circuits.
