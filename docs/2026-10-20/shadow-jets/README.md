# Shadow jets, growing order, and cold-cost exploration (PROB-40)

2026-10-20, Pacific time. Research-only continuation of PROB-39. The user requested
another 20–30 minute exploratory sprint, fast tests, and a push of all results.

**The lead changed:** once we build a higher-order series to check a small rational
initializer, directly evaluating that higher-order polynomial is often better.
Growing its order during construction cuts setup work without discarding the
accumulated error. This is a useful lead, **not a FIX-09 renderer result**.

## What was tested

The same committed direct-orbit truth from PROB-38/39 supplies nine escaping cusp
training points and twelve mixed holdouts. The cusp uses c=1/4+epsilon, global
rho=1e-8, local center 0.5rho and radius 0.21rho. Eight fresh escaping points were
then declared after selecting the growing-order schedule; independent 100- and
150-digit direct computations agree there to 30 relative digits. None of these
corpora is a full image.

The fast sequence was:

1. Build order-32 and order-64 shadow series. Compare low-order polynomials,
   one-pole rational compressions, and the full shadow polynomial under fixed,
   reference-phase and full-disk phase budgets. The two initial policy sweeps
   give 54 method/policy records; a geometry-aware extension adds 36. Each build
   took about 3–4 seconds including high-precision reference construction; endpoint
   comparisons ran separately within the fast CLI invocation.
2. Port the real-reference cusp prepass to a standalone Rust research probe. It
   constructs its own centered reference, coefficients, truncation/derivative
   bounds and admission decisions. This removes the prior study's unmeasured
   native preprocessing cost. Test both rational and polynomial opponents.
3. Measure **actual cold batches**, rebuilding per run, with 9–4096 repeated known
   sampled points and three rotated rounds. This charges setup and evaluator work
   together. The final training batch sweep took 16.60 seconds; mixed holdout
   batches through 2048 took 11.86 seconds.
4. Try six schedules that start at order 8 and expand 8→16→32→64. Carry both
   remainder bounds through every expansion; do not reset uncertainty to zero.
   Compare coefficient work, prefix, endpoint error and build time.
5. Test the shadow idea at the repository's v0 center at widths 1e-14/1e-29, and
   at the difficult valley center. These Python cross-family probes took about
   2–2.5 seconds each and preserve the valley failures.
6. Isolate the reference-rounding component with an outward-rounded real reference
   interval, checked against 100-digit arithmetic at every one of 36,000 steps.
7. Falsify the chosen schedule on eight fresh points, with no prefix/radius/order
   retuning. Every native arm passed these eight as well as the original and
   mixed holdout corpora. Negative phase-budget and valley results remain saved.

## Main numerical results

On the original nine escaping cusp points, native fixed-budget results were:

| Build / per-pixel representation | Initial prefix | Approx. warm evaluator gain | Build time |
|---|---:|---:|---:|
| Order-32 shadow / one-pole order 16 | 31,104 | 3.0x | 24 ms |
| Order-32 shadow / polynomial order 32 | 31,968 | 3.2x | 24 ms |
| Order-64 shadow / one-pole order 24 | 33,184 | 3.5x | 87–91 ms |
| Order-64 shadow / polynomial order 64 | 34,720 | 4.0x | 87–92 ms |
| Growing shadow / polynomial order 64 | 34,720 | 4.0x | 45–51 ms |

The opponent is the same lean scalar centered ordinary iterator used in PROB-39.
It computes derivatives and first escape too. All rows pass the research state,
derivative, smooth-value, escape-index and class thresholds. Native builds exploit
real reference coefficients and symmetric convolution; general complex-reference
setup would have different costs. Python handles the complex-reference probes.

The actual cold-batch comparison, including a fresh prepass each run:

| Method | 4096 repeated escaping samples | 2048 repeated mixed holdouts |
|---|---:|---:|
| Order-32 rational initializer | 2.84x | 1.84x |
| Order-32 polynomial initializer | 3.00x | 1.90x |
| Order-64 polynomial initializer | 3.02x | 1.80x |
| Growing-order polynomial initializer | **3.40x** | **1.97x** |

On the eight fresh escaping points, an additional actual cold batch at 2048
repeated samples measured **3.04x** for the growing-order method, with all arms
passing accuracy. That fast sweep took 7.56 seconds.

These are measured batch ratios, **not frame speeds**. Points repeat cyclically
from the nine- or twelve-point corpora. The reference, coefficients, remainder
propagation and selection are inside each timed cold run; oracle generation,
truth-file parsing and compilation are outside. The mixed corpus has four
cap-reaching points, so the remaining tails dilute gains. An interior-aware
baseline could improve further; neither arm uses analytic interior tests here.

The selected growth threshold increases order when the carried state remainder
exceeds 1e-19 (1e-4 of the 1e-15 admission budget). Expansions occur at iterations
9,388, 21,137 and 21,138. Coefficient-pair multiplications fall from **74,880,000**
to **32,849,320**, a 56% reduction. The accepted prefix stays 34,720. On the mixed
holdout, maximum derivative relative error is about 4.1e-8; on the eight fresh
points it is about 6.7e-9. This schedule was selected on construction work and
training results; reused holdouts are validation, not a new blind trial. The fresh
points provide a separate later falsification attempt.

Expanding too late does not undo old uncertainty: waiting until the remainder
already reaches 1e-15 leaves only a 21,888-step prefix. Threshold effects are not
monotone because the reference contracts early, so an earlier expansion can reduce
new defect growth enough to delay the next expansion. The traces and work counts
are retained instead of inferring optimality from one wall-time measurement.

## Cross-family results and failures

With an order-64 shadow and fixed 1e-15 state budget:

| Stored reference / width | One-pole order-16 prefix | Polynomial order-64 prefix | Polynomial mean ordinary tail | Sample accuracy |
|---|---:|---:|---:|---|
| v0 / 1e-14 | 374 | 393 | 16.33 | 3/3 pass |
| v0 / 1e-29 | 893 | 975 | 31.67 | 3/3 pass |
| valley / 1e-14 | 2386 | 3027 | 1256.67 | polynomial fails 1/3 |

The v0 center comes from `bench/path-atlas-v0.txt`; it is **not the PROB-22/FIX-09
acceptance path**. These two depth samples show that the high-order lead is not
confined to the cusp, but give no native timing at those depths. The valley still
fails endpoint accuracy even though its prefix is longer. Prior reference/ordinary
arithmetic problems have not been solved by increasing truncation order.

A reference-only phase budget also fails: the eighth-order polynomial admitted at
16,736 steps fails two training endpoints. A full-disk phase denominator loses its
positive lower bound and limits long skips to about 26,016. A bound tailored to the
actual thin training rectangle admits longer skips and passes all its sampled
checks, but is valid only for that rectangle. It is not applied to broad holdouts.
No phase rule is promoted to a certified endpoint guarantee.

## Mathematics: checking a small chart with a shadow series

Let S_K(x)=sum(k=1..K)b[k]x^k be the normalized parameter series, |x|≤1. The usual
coefficient convolution evolves it. Exact-arithmetic truncation bounds E_K,F_K
cover state and normalized parameter derivative. With M=||S_K|| and D=||S_K'||,
and T,T' the omitted square and its derivative coefficient norms,

    E_next = 2(|Z|+M) E + E² + T
    F_next = 2(|Z|+M+E) F + 2 E D + T'.

For a low-order rational R=P/Q, form the **total difference polynomial**

    H = S_K Q - P,     q = |Q[0]| - sum(k>0)|Q[k]|.

When q>0, sufficient ideal-arithmetic bounds are

    |u-R| ≤ ||H||/q + E_K
    |u'-R'| ≤ ||H'||/q + ||H|| ||Q'||/q² + F_K.

This compares the total error at a checkpoint rather than summing the absolute
rational recurrence defects at every step. It preserves algebraic cancellation.
Check every intermediate nonescape envelope with the shadow series; evaluating
only the final state is insufficient. Candidate checkpoints are 32 iterations
apart in the cusp experiment. A high-order polynomial can use the same prepass
and simply take E_K,F_K as its error bounds, making it an essential fair opponent.

For growing order, replacing S_K with the same polynomial padded by zero higher
coefficients changes neither state nor derivative. Keep E_K,F_K, then resume the
recurrence at the larger order. Lost earlier coefficients remain covered by the
carried error; expansion is not retroactive reconstruction.

This is an application of polynomial-plus-remainder reasoning, not a claim of new
Taylor-model theory. Background: [TaylorModels.jl](https://juliaintervals.github.io/TaylorModels.jl/dev/)
describes polynomial models with remainder enclosures; [mathr's series notes](https://www.mathr.co.uk/web/m-series-approximation.html)
describe the Mandelbrot coefficient recurrence. The implementations here use
round-to-nearest arithmetic and **do not** enclose every rounding error.

## Phase and region geometry

For the cusp, w=z-1/2 satisfies w_next=w+w²+epsilon. The continuous-flow analogy
suggests a state error measured in clock units E/|w²+epsilon|. That motivates a
budget but does not prove a discrete Abel/Fatou coordinate error bound.

The full-disk lower bound comes from the coefficient norm of
G(x)=(w_ref+S_K(x))²+epsilon_center+rho_local*x, subtracting the shadow error's effect.
The rho_local*x term is essential and has an algebra regression check.

For the thin rectangle, real normalized global offsets lie in [0.3,0.7], imaginary
offsets have magnitude ≤1e-5. Along its real axis, the approximating W is real, so
W²+epsilon≥3e-9. With local imaginary half-width eta and ||W||,||W'|| over the disk,

    |G| ≥ 3e-9 - eta (2||W|| ||W'|| + rho_local)
                   - 2||W|| E_K - E_K².

This retains the narrow geometry that a circular enclosure discards. The sampled
strip policies pass, but the clock analogy and unrounded arithmetic remain separate
limitations. Broad complex parameters outside the strip must not use this bound.

## What remains unproved

The native prepass uses centered binary64 arithmetic and passes sampled truth; it
is not a high-precision arbitrary-center renderer. The reference-only outward
interval has radius about 6.4e-16 at iteration 34,720, comparable to the 1e-15 state
budget. This shows why reference and coefficient rounding cannot be ignored in a
complete certificate. The real-reference interval encloses 100-digit truth at all
36,000 checked steps, but does not cover coefficient or pixel arithmetic.

Accuracy thresholds are the same research thresholds as PROB-39, not fd's final
pixel-displacement contract. There is no optimized production BLA/SIMD/GPU timing,
whole-frame comparison, five-band result, or proof of production 3x. Cusp width
1e-8 lies outside FIX-09's target band; the v0 checks at target depths are sparse
Python accuracy tests only. Growing order is a lead to compare with the best
kernel, not a promoted shortcut or an atlas win.

## Reproduce

From repository root, with Python 3.10+, mpmath 1.4.1 and NumPy 2.2.6:

```sh
export OPENBLAS_NUM_THREADS=1
P=docs/2026-10-20/shadow-jets
.venv/bin/python "$P/shadow_probe.py" --K 64 --out /tmp/shadow-K64
.venv/bin/python "$P/band_probe.py" --scene v0-29 --out /tmp/shadow-band.csv
rustc -O "$P/native_shadow.rs" -o /tmp/fractodactyl-shadow-probe
/tmp/fractodactyl-shadow-probe docs/2026-10-20/rational-charts/results/fair-local/charts.txt
/tmp/fractodactyl-shadow-probe docs/2026-10-20/rational-charts/results/fair-local/charts.txt 36000 cold
/tmp/fractodactyl-shadow-probe docs/2026-10-20/rational-charts/results/holdout-charts.txt 36000 cold 2048
.venv/bin/python "$P/check_math.py"
.venv/bin/python "$P/reference_interval.py"
```

`fresh_probe.py freeze` regenerates fresh direct truth. `--dps 150 --truth
/tmp/prob40-fresh150.json` plus `fresh_probe.py audit --other /tmp/prob40-fresh150.json`
repeats its precision audit; `export` writes the native point input. Normal probes
use committed frozen truth, without an oracle in candidate timing. All final
verification commands exited 0; numerical negative controls are retained in CSVs
and explicitly checked by `check_math.py`.

`native-*.csv` reports warm evaluation and build cost; its cold columns are simple
setup+N*per-point extrapolations. **Only `cold-*.csv` contains measured cold batches.**
Logs record elapsed time and checksums. Early static/growth tables are archived as
snapshots; the final native CLI includes the complete growth sweep. Regeneration
changes timings and may expose numerical failures on other platforms—report them.

## Next lead

Prefer a **growing-order polynomial prepass with a measured batch-size choice**
over automatically compressing everything into a rational chart. Keep rational
compression for cases where evaluation cost or coefficient storage actually pays.
Next, use native general complex-reference construction and an endpoint/rounding
budget, then test actual FIX-09 frames against the optimized kernel. Region geometry
and analytic interior handling should be available to both opponents.
