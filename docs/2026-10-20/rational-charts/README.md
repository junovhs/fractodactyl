# Rational local charts: fast research sprint (PROB-39)

Research date: 2026-10-20, Pacific time. **No production renderer change or promoted
shortcut.** This follows PROB-38's observation that one initial polynomial leaves
long boundary tails. The user authorized a 20–30 minute exploratory sprint and push.

The hunch: a polynomial is a poor representation of the sharp growth before escape.
A low-degree rational denominator might absorb that growth; local parameter charts
might extend its useful range. Test both ideas, then try to falsify the winner.

**Outcome:** a useful lead survived a small native evaluator test and holdout accuracy
checks. Several faster-looking variants failed. This does not establish FIX-09's 3x
renderer target. The successful cusp test is at width 1e-8, outside FIX-09's
1e-14..1e-29 acceptance band, and it is sparse rather than a rendered frame.

## Experiments, in order

Each comparison command took under 30 seconds (run JSONs record Python times).
Independent direct-orbit truth generation was separate from candidate timing.
Timings are snapshots, not assertions about reproducible exact performance ratios.

1. **Same coefficients, different representation.** Freeze 100 checkpoint states
   from four families: cusp, v0 at two depths, and valley. Compare degrees 4/8/12,
   initially 180 polynomial/balanced-rational records in 1.90 s. An expanded
   denominator-degree sweep produced 300 records in 2.31 s. At cusp iteration
   25,000, an eight-coefficient polynomial has ~0.030 relative derivative error;
   its [7/1] rational fit has ~7.5e-7. At 30,000 it fails. The apparent gain is
   worth following, but these manually chosen checkpoints are diagnostic—not
   a prefix-selection algorithm.
2. **Try to admit the jump without sampled truth.** Add a rational recurrence
   residual and denominator bound over the whole normalized disk. For cusp width
   1e-8, the eighth-order polynomial admits 8,935 iterations; the one-pole order-8
   chart admits 16,916; order 12 admits 21,199. Balanced fits suffer from poor
   conditioning and weak denominator margins. All these bounds omit rounding.
3. **Check the complete tail.** The global order-8 one-pole chart fails one of the
   three cusp endpoint checks despite its prefix guard. Order 12 passes. Global
   v0 samples pass, but rational is not universally better: at v0 width 1e-29,
   the order-8 polynomial admits 911 iterations, versus 877 for the one-pole fit.
   Valley endpoint accuracy still fails. Preserve all losses.
4. **Use a local chart and a fair polynomial opponent.** Nine predeclared complex
   cusp points have real normalized offsets 0.3/0.5/0.7 and imaginary offsets
   -1e-5/0/+1e-5, with global rho=1e-8. Direct escape indices span 37,547–57,356.
   Center the chart at C=0.25+0.5rho and give it radius 0.25rho. Compare both
   polynomial and rational fits on this same chart. All candidates in the final
   fair-local table pass the nine endpoint checks.
5. **Remove Python as an explanation.** Export the admitted coefficients and run
   a standalone scalar Rust evaluator. It uses the identical centered ordinary
   recurrence for every arm. Seven rounds rotate arm order, with 32 repetitions
   of the same points per arm per round. The native result follows the Python
   work reduction. Native setup/build cost is **not** measured by this evaluator.
6. **Falsify with holdouts and narrower regions.** Reuse the frozen radius-0.25
   charts on twelve holdout points, eight escaping and four cap-reaching. The
   rational charts pass all twelve. The weak order-8 polynomial fails two. The
   best rational's aggregate evaluator gain falls from about 3.2x on the original
   escaping set to about 2.0x on this broader set.
7. **A faster narrowing fails, and precision does not rescue it.** Radius 0.21rho
   admits longer prefixes, but each tested narrow polynomial/rational chart fails
   six of nine training endpoints. Replaying the exact same prefixes with
   64-bit-mantissa extended build arithmetic, including extended reference data,
   still fails six. Truncation transport is material here. Tightening state
   tolerance from 1e-12 to 1e-15 removes these failures; order-16 one-pole prefix
   falls from 33,141 to 31,059. That tighter chart also passes all twelve holdouts.
   Tightening to 1e-16 reduces it further to 29,597. Precision alone is not an
   endpoint error budget.

The initial endpoint timing accidentally allowed NumPy complex scalars into only
some ordinary loops. Those timings are archived under `endpoints-numpy-diagnostic`
and **must not be used as a fair performance result**. The corrected evaluator
converts all coefficients to built-in complex before timing. Native measurements
provide the main evaluator comparison. Some exploratory Python arms ran concurrently;
small Python timing differences should not be interpreted as wins.

## Native result and its limits

AMD Ryzen 9 3900X, one process/thread, Rust 1.90.0, `rustc -O`, default CPU target.
See `results/native.csv` for the exact measured times and ratios on this execution.
Approximate results from the nine escaping points:

| Method, same local region | Initial prefix | Warm scalar evaluator gain | Accuracy failures |
|---|---:|---:|---:|
| Centered ordinary iteration | 0 | 1.00x | 0/9 |
| Polynomial order 8 | 16,507 | 1.5x | 0/9 |
| Polynomial order 12 | 22,854 | 2.0x | 0/9 |
| One-pole order 12 | 30,336 | 2.9x | 0/9 |
| One-pole order 16 | 31,858 | 3.2x | 0/9 |

The last chart also passes 12/12 holdouts, with about **2.0x** aggregate native
warm evaluator gain there. Its original nine-point worst relative derivative
error is about 8.7e-8 and worst smooth-value error about 7.6e-8. The separate
radius-0.21, 1e-15 state-budget chart passes its training and holdout checks too;
it does not replace the original chart's holdout result.

Accuracy requires first escape index and escape status, state error ≤1e-6 times
max(1,|z|), relative derivative error ≤1e-6, and absolute smooth-value error ≤1e-6.
These are **research thresholds**, not the production equivalent-pixel contract.
Cap-reaching does not prove interior membership. The 100- versus 150-digit direct
truth audit agrees on all nine training states/derivatives to 30 relative digits.
Holdout truth was generated at 100 digits, without a second precision audit.

The fair opponent here is a lean scalar **centered ordinary evaluator**, not the
optimized production BLA path, SIMD/GPU renderer, or an interior-aware renderer.
The two holdout points at offset 0.32±0.00003i reach the cap, as do two broad
complex interior controls; their long tails substantially dilute the gain.
Analytic interior tests could benefit both opponents and are not included.

Python reference generation and guard construction cost seconds. The radius-0.25
order-16 chart costs about 3.5 s for guard/coefficients plus about 1.2 s for its
high-precision reference in this implementation. `local-timings.csv` charges these
costs and reports cold batch losses and break-even counts. The native benchmark
only consumes exported charts: **it establishes evaluator economics, not native
cold-frame economics**. Combining Python setup with Rust evaluation is a mixed
implementation estimate, not a renderer benchmark. No full frame, actual FIX-09
path, five depth-bin result, or 3x production speedup is claimed.

## Why the rational representation can help

Use dc=rho*x and normalized coefficients b[k]=a[k]rho^k. Taylor jets evolve exactly
as in PROB-38. A [p-1/1] approximant uses

    Q(x) = 1 - (b[p]/b[p-1]) x
    P(x) = Taylor coefficients through degree p-1 of Q(x)*sum b[k]x^k.

There is no dense solve for this one-pole form. It matches the original jet through
order p when the coefficient ratio is well defined. It is an approximation, not a
claim that a finite Mandelbrot iterate actually has a pole: that iterate is a huge
polynomial. Balanced Padé solves can fit excellent sample values while becoming
ill-conditioned or introducing problematic denominators. Their losses remain in
the sweep tables rather than being silently replaced by one-pole successes.

Prior art: [mpmath's Padé documentation](https://mpmath.org/doc/current/calculus/approximation.html)
describes rational approximation from Taylor coefficients;
[mathr's general-order series recurrence](https://mathr.co.uk/blog/2016-03-06_simpler_series_approximation.html)
provides the parameter-jet starting point. This study tests an application and
admission/error behavior; it makes no priority claim for rational orbit skipping.

## Residual guard derivation

At adjacent iterations write R=P/Q and R_next=A/B. The exact-arithmetic recurrence
defect is

    H(x) = [(2 Z P Q + P² + rho*x*Q²) B - A Q²] / (Q² B).

For a polynomial V, let ||V||=sum |v[k]| and ||V'||=sum k|v[k]|. If
q=|Q[0]|-sum(k>0)|Q[k]|>0, then |Q(x)|≥q for |x|≤1. Define b similarly for B.
With D the numerator of H, sufficient bounds are

    h = ||D|| / (q² b)
    h' = ||D'||/(q² b) + h (2||Q'||/q + ||B'||/b)
    M = ||P||/q
    L = ||P'||/q + ||P|| ||Q'||/q².

State error E and normalized derivative error F therefore satisfy

    E_next ≤ 2(|Z|+M) E + E² + h
    F_next ≤ 2(|Z|+M+E) F + 2 E L + h'.

Check every intermediate nonescape envelope |Z_next|+M_next+E_next<2. Reject on a
nonpositive denominator margin, excessive error, ambiguity, or nonfinite arithmetic.
`check_math.py` checks 800 complex residual/derivative cases and the one-pole jet
identity at four orders.

This is an **ideal-arithmetic derivation**. The implementation uses ordinary
round-to-nearest norms, coefficients, reference values and residual subtraction.
It does not enclose reference/rounding uncertainty. The derivative criterion is
relative to a region-wide bound, not a lower bound for each pixel's derivative.
It is not certified admission, as the retained endpoint failures demonstrate.

## Two different numerical failure sources

The valley diagnosis fixes the final index from frozen truth to isolate error:
using high-precision arithmetic with a rounded binary64 reference still leaves
about 1.3e-6 relative derivative error on the worst sampled pixel. Exact-reference
or split-reference high-precision arithmetic agrees with direct truth to its saved
45-digit precision; extended reference and arithmetic reduce the worst relative
derivative error to about 1.4e-9. Reference conversion and ordinary arithmetic both
matter. This experiment does not time a practical extended-precision kernel.

Conversely, rebuilding the rejected narrow cusp charts with extended arithmetic
and reference data leaves their endpoint failures unchanged. Their approximation
error is amplified by the long ordinary continuation. The two diagnoses should
not be conflated into “just use more precision” or “just increase polynomial order.”

A useful next hunch is an error budget in **iteration phase** near a bottleneck.
For w=z-1/2 and epsilon=c-1/4, w_next=w+w²+epsilon; a local continuous-time analogy
converts a state error roughly to E/|w²+epsilon| units of transit time. That is a
conditioning heuristic, not an implemented certificate. A defensible initializer
should budget the *final* state/derivative/color error, with fallback for ambiguity.

## Reproduce and inspect

From repository root:

```sh
python3 -m venv .venv
.venv/bin/pip install mpmath==1.4.1 numpy==2.2.6
export OPENBLAS_NUM_THREADS=1
P=docs/2026-10-20/rational-charts
.venv/bin/python "$P/tournament.py" screen --out /tmp/rational-sweep
.venv/bin/python "$P/guard_probe.py" --order 8 --denom 1 --out /tmp/rational-guard.json
.venv/bin/python "$P/endpoint_probe.py" --out /tmp/rational-endpoints
.venv/bin/python "$P/local_probe.py" run --family fair-local --out /tmp/rational-local
rustc -O "$P/native_probe.rs" -o /tmp/fractodactyl-rational-probe
/tmp/fractodactyl-rational-probe "$P/results/fair-local/charts.txt"
.venv/bin/python "$P/check_math.py"
```

Frozen truth is committed. Optional `freeze` commands regenerate snapshot, local,
and holdout direct truth; local `--dps 150 --truth /tmp/prob39-local150.json` and
`check_math.py --audit /tmp/prob39-local150.json` repeat the independent audit.
`local_probe.py --family radius` reproduces rejected narrowing;
`--family budget --tol 1e-15` reproduces the repaired error-budget probe.
`chart_precision.py` and `precision_probe.py` reproduce error attribution.

`blind_probe.py export` reuses original exported charts on holdout truth. Native
execution on `results/holdout-charts.txt` **exits 101 intentionally** because the
order-8 polynomial fails two holdouts; its full comparison CSV is retained.
Execution on `results/holdout-budget15-charts.txt` exits 0 with all arms passing.
`check_math.py` verifies that both passing and rejected results remain present.
A passing research check does not mean all approximants passed accuracy.

`results/*run.json`, CSVs, frozen chart exports, direct truth JSONs, and
`native-meta.json` record the numerical environment and raw outcomes. Guard
JSONs from concurrently dispatched exploratory runs should not be used for tight
cross-arm timing comparisons. Regeneration changes wall times; it should preserve
mathematical conclusions and disclose any changed accuracy outcomes.

## Handoff

Follow the **single-pole local parameter chart** lead, with final-error conditioning,
reference precision and native preprocessing charged. A second-order staged LA or
multiple local charts could handle the remaining tail. First compare against the
best existing kernel on actual FIX-09 frames; sparse cusp success is insufficient
to promote this or claim the target has been met. Stop increasing degree when the
extra setup and small prefix extension lose economically. The negative tables are
part of the result.
