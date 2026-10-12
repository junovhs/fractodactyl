# Experiment log

**Scope:** preliminary Fractodactyl → industrial nonlinear dynamics transfer. Negative findings are preserved. The first four records are transcribed from the October 12, 2026 investigation; their runnable files and raw logs were **not committed to GitHub**. Treat timings as exploratory until reproduced with a frozen harness. These are not interval-certified results or demonstrated industrial wins.

| ID | Hypothesis / target | Predeclared gate | Observed outcome | Decision |
|---|---|---|---|---|
| DT-001 | A degree-8 Koenigs jump can replace periodic integration on a broad state domain of a forced oscillator | max absolute state error ≤1e-7 before timing | 1-period max jump error ~4.09e-4, fitted map ~3.05e-9, direct RK4-vs-DOP853 ~1.87e-9 (32 states) | **Kill broad-domain jump** |
| DT-002 | Shrink the local chart until accurate; beat repeated period-map iteration at a modest workload | error ≤1e-7 and cold speed ≥3x | radius 0.09 failed (~7.17e-7); radius 0.045 passed sampled tests (~1.68e-9). 512 states ×64 periods: 0.002821s jump vs 0.003520s repeated map, **1.25x** | **Kill for 512 states** |
| DT-003 | Larger batched workload amortizes the Koenigs chart | error ≤1e-7 and cold speed ≥3x vs repeated map | 8192 states ×64 periods: 0.003738s jump vs 0.015087s repeated map (**4.04x**); max difference vs repeated map ~1.59e-11 | **Conditional** win against restricted opponent; superseded by DT-004 |
| DT-004 | Directly compile the same one-period polynomial into a 64-period polynomial; compare fairly to Koenigs | accuracy ≤1e-7; if composed polynomial erases ≥3x jump win, kill Koenigs preferred | 8192 states: cold direct composition+evaluation **0.001265s**, Koenigs build+evaluation **0.006672s**; composed vs repeated map ~1.59e-11 max difference | **Kill Koenigs preference**; composed polynomial **5.27x faster** cold in this toy test |

## Model and important caveats

- Scalar periodically forced dissipative toy ODE: `x'=-0.006*x-0.15*x^3+0.18*cos(t)`; sampled every (2\pi).
- The experiment used Numba-compiled RK4 / independently checked SciPy DOP853, polynomial fits, one-dimensional Koenigs charts, and direct polynomial composition.
- Timings: sandbox virtual CPU, warm JIT, no owner's GPU/CPU. Sample precision gates are *not proofs*.
- The chart and composed map were validated on sampled input intervals only; no exhaustive domain or `≤1e-3 px` equivalence is claimed.
- The toy does not include the switching events, stiffness, parameter changes, or higher-dimensional physics of real power converters.
- DT-003 cannot be presented as a general win because DT-004 gives the fairer competitor.
- The earlier `38.6x` comparison against batched SciPy was not a fair industrial baseline. A compiled RK4 rerun at 100k samples gave roughly **0.838 s** integrator vs **1.306 s** fitted map **including construction**: the map was **1.56x slower** cold.

## Current conclusion

**No industrial gain established.** Direct composition of an existing one-period map is simpler and faster than the Koenigs-based candidate on this problem, and may fail in repelling or less-contracting dynamics due to shrinking validity domains. Test automatic parameter/state domains and harder piecewise systems next, preserving a strict same-math competitor.

## Provenance checklist for future records

Record date, git commit/hash of any code used, exact command, Python/Numba/Numpy versions, machine, seeds, baseline details, distribution and count of test cases, maximum/quantile errors, cold and warm wall-times (median and range), construction cost, proof status, and decision. If any field is missing, mark it explicitly—not as an established production result.
