# Normalized parameter-series boundary probe (PROB-38)

2026-10-20, Pacific time. Research only; no renderer change or promoted shortcut.

The question is whether a shared initial parameter polynomial remains useful on
**escaping boundary orbits**, after construction cost and derivative accuracy are
counted. This extends the October 19 study rather than benchmarking Rust.

## What changed in the experiment

The old sample had 36 escaping pixels out of 108 and a 640-iteration cap. Here there
are **19 escaping pixels out of 21**, seven scene/width pairs, and a **100,000** cap.
The two nonescaping samples reach that cap; this is not a proof of interior status.
The cusp exterior samples escape at 3,510–49,671 iterations. Other families include
the exact valley center from `bench/locations.txt`, the v0 center from
`bench/path-atlas-v0.txt`, and the exact i and -2 controls. Centers are parsed as
high-precision decimal strings before reference construction.

**These are not the FIX-09 acceptance frames.** FIX-09 specifies the PROB-22 path,
24 frames at 480×270, five three-decade bins over 1e-14..1e-29. The v0 path is a
separate existing repository path. This probe has three offsets per scene and two
sampled widths on v0, not a frame or a complete depth sweep.

Independent direct iteration (not the perturbation recurrence) freezes orbit state,
derivative, first escape index and escape status at 100 decimal digits. A separate
150-digit execution agrees on all 21 indices/classes and on state and derivative
to 30 relative decimal digits. This is empirical precision convergence, not a ball
certificate. Candidate timings read frozen truth; they do not time the oracle.

## Mathematics and the new error question

Write dc = rho x with |x| ≤ 1 and store b[k] = a[k] rho^k. Then

    P_n(x) = sum(k=1..p) b[k,n] x^k
    b[1,n+1] = 2 Z_n b[1,n] + rho
    b[k,n+1] = 2 Z_n b[k,n] + sum(h=1..k-1) b[h,n] b[k-h,n].

This is ordinary parameter-series approximation in normalized coordinates, not a
new algorithm. It avoids separately forming enormous a[k] and tiny rho^k. Horner
evaluation gives both P and Q = dP/dx = rho dz/dc. This prototype still requires
rho representable in binary64; it does **not** solve the extended-exponent problem
or claim support for 1e-1000. Reference rounding remains an error source.

For an ideal exact reference and exact coefficients define

    M = sum |b[k]|, D = sum k |b[k]|
    T = sum(a+b>p) |b[a]| |b[b]|
    T' = sum(a+b>p) (a+b) |b[a]| |b[b]|.

For state error E and normalized derivative error F, differentiating the omitted
residual gives the sufficient exact-arithmetic recurrences

    E_next = 2 (|Z| + M) E + E² + T
    F_next = 2 (|Z| + M + E) F + 2 E D + T'.

Require every intermediate |Z_next| + M_next + E_next < 2. Also require
E ≤ tolerance and F ≤ 1e-8 max(rho,D_next). Compare orders 2, 4 and 8 at state
tolerance 1e-12, plus orders 4 and 8 at 1e-18. **Neither threshold is a certified
pixel-space accuracy guarantee.** All arithmetic in this majorant is round-to-nearest,
and reference, coefficient, and evaluation rounding are omitted.

The important additional question is what happens *after* initialization. Along
each ordinary fallback step, the ideal truncation-only bounds transport as

    E_next = 2 |z_hat| E + E²
    F_next = 2 (|z_hat| + E) F + 2 E |q_hat|.

The CSV includes these final bounds. Transport is diagnostic and excluded from both
candidate and plain timings; a certified renderer would need to charge for its guard
and rounding treatment. A tiny initial error can be amplified by thousands of later
iterations. Prefix correctness alone does not settle endpoint/color correctness.

Prior art: mathr's [Series Approximation](https://mathr.co.uk/web/m-series-approximation.html)
and [Simpler series approximation](https://mathr.co.uk/blog/2016-03-06_simpler_series_approximation.html)
describe the shared coefficients and general-order convolution recurrence. The
normalization and majorants above are algebraic derivations for this probe.

## Measured outcomes

The full frozen-truth comparison ran in **15.42 seconds**, using Python 3.10.12 and
mpmath 1.4.1, one process, median of three setup/evaluation repetitions. Timings are
noisy Python measurements; small gains should be treated as noise.

At tolerance 1e-12, prefixes and setup costs were:

| Scene | Escape indices (three pixels) | Order 4 prefix | Order 8 prefix | Order 4 / 8 setup ms | Accuracy failures order 4 / 8 |
|---|---|---:|---:|---:|---:|
| cusp, rho 1e-6 | 4965, 3510, cap | 267 | 888 | 2.64 / 19.54 | 0 / 0 |
| cusp, rho 1e-8 | 49671, 35122, cap | 2726 | 8935 | 26.56 / 196.25 | 0 / 0 |
| valley, rho 1e-14 | 4045, 5417, 3389 | 1967 | 2035 | 20.40 / 48.13 | 3 / 2 |
| v0, rho 1e-14 | 415, 407, 406 | 322 | 356 | 3.26 / 8.17 | 0 / 0 |
| v0, rho 1e-29 | 1016, 999, 1005 | 813 | 911 | 8.16 / 20.49 | 0 / 0 |
| i, rho 1e-29 | 81, 79, 81 | 73 | 77 | 0.76 / 1.85 | 0 / 0 |
| -2, rho 1e-29 | 52, 1, 50 | 0 | 0 | 0.01 / 0.02 | 1 / 1 |

Accuracy requires matching first escape and status, relative state error ≤1e-6
(normalized by max(1,|z|)), relative derivative error ≤1e-6, and, when escaped,
absolute smooth-value error ≤1e-6. These are probe thresholds, not fd's equivalent
pixel-displacement contract. All 126 method/pixel records are supported by the
reference; **17 fail accuracy**, including baseline failures. No failures are removed.

The valley's indices match in every method, yet order 4 has a derivative error up
to about 2.8e-4 and smooth error about 3.0e-4. Even plain perturbation fails the
chosen derivative/smooth thresholds on one valley pixel. Tightening initialization
state tolerance from 1e-12 to 1e-18 leaves two failures at order 4 and one at order 8.
This rules out promoting any valley candidate. It does not identify a unique error
source: truncation transport, reference conversion, evaluation and fallback rounding
need separate attribution.

At -2, the point with negative real dc escapes at iteration 1 in high precision.
The reconstructed binary64 sum Z+u rounds back onto radius 2 and the prototype
reports iteration 24. A diagnostic check of

    |Z+u|² - 4 = (|Z|² - 4) + 2 Re(conj(Z) u) + |u|²

retains the crossing for this exact-reference control. That suggests a compensated
bailout or higher-precision ambiguity fallback; it is not a general reference-error
certificate. All methods decline the initial series here (prefix zero), so increasing
order cannot fix this failure.

## What the timing does and does not say

`summary.csv` records reference setup, series setup, per-pixel evaluation, and
break-even pixels = ceil(series_setup / (plain_pixel - candidate_pixel)). It reports
`never` if the candidate pixel evaluator is slower. The batch score includes shared
reference + setup + all three sampled pixels and is zeroed on any accuracy failure.
Most three-pixel cold batches do not amortize construction.

For successful v0 samples, order 4 breaks even after approximately 23–24 pixels;
order 8 after 50–53. An extrapolation repeating those **same sampled costs** across
129,600 pixels gives order 4 about 3.35x / 4.49x and order 8 about 4.61x / 7.62x at
the two widths, including one reference and one series setup. **No such frame was
rendered.** Spatial diversity, real BLA competition, shading, CPU/GPU dispatch and
native costs can overturn that extrapolation. Valley extrapolations fail correctness
and must never be credited as wins.

For the cusp, order 8 skips only 18–25% of the escaping orbit; order 4 only 5–8%.
The whole mixed three-pixel batch extrapolates to roughly 1.03x at rho 1e-6 and
1.18x at rho 1e-8 for order 8. This is evidence against standalone fixed-order
initialization being a universal 3x solution. The cusp widths also lie outside
FIX-09's target depth band, so this is a stress test, not a failing FIX-09 bin.

## Next hypothesis and the meaning of 3x

The production target is **same frame, at most one-third the total elapsed time**
versus plain perturbation, in every tested three-decade bin, with correctness checked.
It includes setup; fewer ordinary steps are not the goal by themselves. This study
cannot say what fraction of that target is complete. Under DEC-14/17 it is a sparse
ranking/rejection probe, not promotion, and it does not time the optimized Rust BLA
rival. It makes no atlas or film speed claim.

The next useful experiment is a **condition-aware initializer plus segmented
continuation**, rather than blindly raising degree: budget initial error using later
sensitivity, resolve bailout ambiguity, and investigate multiple local series or
staged quadratic maps through long bottlenecks. Test smaller regional radii only
with the additional reference/build work charged. First resolve the valley endpoint
errors against high precision. Only then compare a native candidate with the best
existing Rust path on the actual PROB-22 frames. Higher degree alone has diminishing
returns and does not cure a weak fallback.

## Reproduce

From repository root (mpmath is the sole dependency):

```sh
python3 -m venv .venv
.venv/bin/pip install mpmath==1.4.1
# Routine probe reads committed frozen truth; no oracle in the timing.
.venv/bin/python docs/2026-10-20/series_probe.py
.venv/bin/python docs/2026-10-20/check_probe.py
# Optional independent precision audit / truth rebuilding (not part of probe budget).
.venv/bin/python docs/2026-10-20/series_probe.py --freeze --dps 150 --truth /tmp/prob38-truth150.json
.venv/bin/python docs/2026-10-20/check_probe.py /tmp/prob38-truth150.json
```

`--freeze` defaults to 100 digits and 100,000 iterations. `--truth` and `--output`
accept other paths. Regenerating truth or changing the scene definitions is a research
change; ordinary candidate comparisons should use the committed truth. Timing CSVs
are snapshots and will vary on rerun. `check_probe.py` checks both successful samples
and the deliberate negative controls, so its passing does not mean every candidate
passed numerical accuracy. All documented commands above exited 0 in this study.
