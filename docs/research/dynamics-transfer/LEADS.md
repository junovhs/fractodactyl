# Ranked leads and next tests

This document stores decisions, not scripts. **All proposed benefits are speculation until measured.** Establish the rival and kill gate *before* each probe.

| Rank | Candidate mechanism | Industrial payoff hypothesis | Cheapest next experiment | Predeclared kill criterion | Status |
|---|---|---|---|---|---|
| 1 | Automatically compile and validate **parameter-dependent, piecewise return maps** for a switched nonlinear system; split domains where event sequences differ | Fast design sweeps of converters and control systems | Two-state PWM / switching toy, 100–1,000 parameter/state combinations, 16–64 cycles; compare optimized Numba event-aware integrator, cached one-period map, polynomial composition, local charts | Reject if any event/branch error or worst state error >1e-7; then reject if <3x **cold** win against fastest accurate same-math method | **Not tested** |
| 2 | Build a **cost- and certificate-driven operator chooser** between polynomial composition, numerical integration, and Koenigs | Reuse compiled operators over multiple simulations; skip regions where no win | Sweep initial-state radii and horizon on the existing oscillator; include setup and validation costs | Reject if cold total <3x over a best available cached/composed alternative across the tested workload; reject any unbounded error | **Not tested** |
| 3 | **High-dimensional/2D nonlinear Poincaré jets** with independently bounded remainders | Broader applicability to control and electromechanics | Use 2D periodically forced Duffing map, compare 3rd vs 5th degree patching at fixed tolerances | Reject if jet construction/validity cost exceeds optimized direct integration or usable chart shrinks below practical region | **Not tested** |

## Important prior-art constraints

- Polynomial composition, stroboscopic maps, Koenigs/Sternberg linearization, Floquet analysis, multiple shooting and periodic operating-point solvers already exist.
- SIMPLIS/Plecs and other circuit solvers support special treatment of periodic operation and switching; a compelling claim must match a specialist baseline.
- Fractodactyl already has direct-return and factored operator research; test *same-math* direct composition before celebrating a Koenigs win.
- A single easy scalar oscillator is an algorithmic sanity check, not an industrial benchmark.

## Focus

The near-term research question is not whether a polynomial is faster than a Python solver. It is whether **automatic structural discovery plus rigorous validity domains yields a repeatable cold win** for realistically costly families of nonlinear simulations, against opponents equally able to cache and compose.
