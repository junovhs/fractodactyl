# Deep-zoom Mandelbrot: certified multi-stage LA / AT investigation

**Research date (Pacific time): 2026-10-19.**  
**Status:** exploratory Python numerical study, **NOT a formally certified renderer** and **NOT a demonstrated 3x frame-level speedup**.

This folder archives the full executable prototypes, raw and aggregated numerical measurements, and comparison chart for an investigation into whether Imagina-style multi-stage linear approximations (LA), approximation transforms (AT), or a hybrid can beat plain perturbation by at least 3x in every zoom band while preserving Mandelbrot escape iteration, smooth value, and derivative. The user's current optimized Rust BLA benchmark is only 0.88–1.21x plain perturbation and leaves about 456–553 ordinary iterations per pixel.

## Executive finding

Second-order, composable, frame-preprocessed stages and a fourth-order initial parameter series substantially reduce work on selected stable/reference-return orbits. On **108 tested pixels** (6 centers x 6 widths x 3 offsets; maximum 640 iterations), the average ordinary perturbation counts were:

| Method | Mean ordinary steps/pixel | Correct escape indices vs mpmath | Illustrative work units/pixel |
| --- | ---: | ---: | ---: |
| Plain perturbation | 441.19 | 108/108 | 441.19 |
| Naive hierarchical BLA, radius factor 0.005 | 137.21 | 107/108 | 195.51 |
| Naive hierarchical BLA, radius factor 0.05 | 136.81 | 104/108 | 194.99 |
| First-step-quadratic LA ("LA_head") | 14.84 | 108/108 | 25.68 |
| Second-order staged LA, tol=1e-4 | 8.99 | 108/108 | 19.67 |
| Second-order staged LA, tol=1e-6 | 9.21 | 108/108 | 19.94 |
| Second-order staged LA, min skip 8 | 10.60 | 108/108 | 18.55 |
| Second-order staged LA, min skip 32 | 17.19 | 108/108 | 23.31 |
| Fourth-order parameter-series start, then plain perturbation | 7.03 | 108/108 | 8.36 * |

*The parameter-series operation proxy omits its substantial coefficient-construction cost. The cached implementation measures that independently. **None of these proxies predicts or proves native frame wall-clock speedup.**

The fourth-order series initializer was separately retested as a cached shared setup and again matched **108/108** escape indices, with mean 7.03 remaining ordinary steps and mean setup approximately **2.34 ms per (scene,width)** in the measured Python environment. Stage-2 setup constructs roughly 133,760 majorant-propagation iterations per (reference,width) in this particular prototype; those operations cannot be treated as free across a whole frame.

### Difficult cases: improvements are not universal

| Reference | Plain steps | Second-order staged LA steps | Second-order work units | Interpretation |
| --- | ---: | ---: | ---: | --- |
| Attracting c=-1 | 640 | 0 | 5.6 | Extremely favorable: nonescaping orbit |
| Parabolic c=-0.75 | 640 | 0 | 11.2 | Favorable period/return |
| Period-3 center | 640 | 1 | 17.8 | Favorable return |
| Seahorse-edge approximate center | 640 | 0 | 5.6 | Favorable for tested offsets, not proof for nearby boundary |
| Misiurewicz c=-2 | 26 | 26 | 33.86 | Too few iterations to amortize checking |
| Misiurewicz c=i | 61.17 | 28.28 | 45.58 | Modest advantage in work proxy; *slower in measured Python* |

On three reference cases at width 1e-20, medians/method timings (selected-case Python evaluator, preprocessing excluded) were:

| Reference | Plain | Second-order stage | Cached fourth-order series |
| --- | ---: | ---: | ---: |
| Attracting -1 | 235.8 µs | 8.0 µs | ~1.6 µs |
| Seahorse edge | 228.2 µs | 7.2 µs | ~1.6 µs |
| Misiurewicz i | 21.7 µs | 28.4 µs | ~7.5 µs |

The last column is from the cached-series addendum's evaluator microbenchmark, not the deliberately uncached "series4_direct" timings in fractal_la_python_timing.csv. These Python numbers are not optimized CPU or GPU wall-clock predictions.

## Experimental configuration and its limitations

- High-precision reference orbit and per-pixel oracle use **mpmath with 100 decimal digits**; implemented in functions named \`hp_reference\` and \`hp_pixel\`.
- The fast perturbation and stage coefficients use native Python complex (binary64). Hence this is **not** a fully high-precision implementation of the accelerated paths.
- Reference centers: \`(-1,0)\`, \`(-0.75,0)\`, period-three \`(-0.1225611668766536,+0.7448617666197442)\`, Seahorse-edge \`(-0.743643887037151,+0.131825904205330)\`, \`(-2,0)\`, and \`(0,1)\`.
- Parameter displacement radii: \`1e-14\`, \`1e-17\`, \`1e-20\`, \`1e-23\`, \`1e-26\`, \`1e-29\`; three complex offsets \`0.4+0.2i\`, \`-0.6+0.45i\`, \`0.15-0.8i\`.
- Limit is **N=640** iterations; many attractive cases reached the limit without escaping. The Seahorse center is specified with only ~15 significant decimal digits; this is not a validated true 29-digit center of a zoom series.
- Escape agreement is a test outcome, **not** an interval/rounding certificate. Exact same escape index on 108 selected pixels cannot establish safety on all pixels or frames.
- The reported derivative comparison is empirical; derivative enclosures are **not** formally propagated by the prototype.
- Intermediate skipped states are bounded conservatively by a scalar majorant, but reference-error, floating-point rounding, and outgoing derivative uncertainty are not exhaustively enclosed.
- The stage-majorant table samples input radii on a discrete geometric grid; finer/adaptive radii and cost-aware selection need investigation.
- The main experiment's "preprocess_ops" and "cost" are explicitly illustrative counts/weights and not CPU instructions, latency, or frame times.
- The BLA implementations in this folder are pedagogical controls; **do not equate them with the user's optimized Rust BLA**.

## Mathematics: perturbation and composable approximations

For the Mandelbrot recurrence \(Z_{n+1}=Z_n^2+C\), let \(u_n=z_n-Z_n\) and \(\delta c=c-C\). Then

\[
u_{n+1}=2Z_n u_n+u_n^2+\delta c,\quad u_0=0.
\]

The parameter derivative \(d_n=\partial u_n/\partial(\delta c)\), with fixed \(C\), satisfies

\[
d_{n+1}=2(Z_n+u_n)d_n+1,\quad d_0=0.
\]

**Conventional hierarchical BLA** models a reference segment as an affine map
\(L_I(u,\delta c)=A_Iu+B_I\delta c\). For adjoining segments \(I,J\),

\[
A_{J\circ I}=A_J A_I,\qquad B_{J\circ I}=A_J B_I+B_J.
\]

It organizes many such maps in a binary hierarchy and accepts a map if its estimated validity radius covers the incoming state. The common propagated-radius test

\[
r_{J\circ I}=\min\left(r_I,\frac{r_J-|B_I|\rho}{|A_I|}\right)
\]

must be clamped to nonnegative values, accounts for \(A_I=0\) separately, and is only a *linear-map applicability condition*: it is **not by itself a bound on nonlinear truncation or rounding errors**. The naive \(\eta |2Z_n|\) radius caused incorrect escape indices here; the radius variant
\(r_n=\max(0,\eta |2Z_n|-\rho/|2Z_n|)\) with \(\eta=2^{-24}\)
achieved 108/108 observed escape-index matches in the addendum but averaged **186.86** ordinary steps. That is empirical agreement, not certification.

**Imagina-style multi-stage LA** is more than a binary BLA tree: reference-return/period-derived macro-maps compose into longer stages, and an **initial quadratic term** can be retained at stage boundaries:

\[
u_{n+k}\approx H_{n,k}(2Z_nu_n+u_n^2)+B_{n,k}\delta c,\qquad
H_{n,k}=\prod_{j=n+1}^{n+k-1}2Z_j.
\]

The first-step \(u_n^2\) matters when the reference visits a critical/near-zero region. Genuine approximation transforms (AT) change the iterative coordinates or initialize from a long-range analytic approximation; they are distinct from simply increasing a BLA block length. **The polynomial parameter-series initializer here is AT-like but is not a replica of Imagina's AT.**

**Second-order staged LA**, the primary experiment, uses a bivariate degree-2 map

\[
P_I(u,\delta c)=A_Iu+B_I\delta c+C_Iu^2+D_Iu\,\delta c+E_I(\delta c)^2.
\]

Its one-step coefficient update at reference \(Z_n\), writing \(a=2Z_n\), is

\[
A'=aA,\ B'=aB+1,\ C'=aC+A^2,\ D'=aD+2AB,\ E'=aE+B^2.
\]

For composition of segment \(I\) followed by segment \(J\), dropping terms of degree \(\geq3\),

\[
\begin{aligned}
A &= A_J A_I,\\
B &= A_J B_I+B_J,\\
C &= A_J C_I+C_J A_I^2,\\
D &= A_J D_I+2C_J A_I B_I+D_J A_I,\\
E &= A_J E_I+C_J B_I^2+D_J B_I+E_J.
\end{aligned}
\]

This is how higher-stage jets are composed, using macro-period sizes growing by a factor of four in the prototype. Use the largest *profitable* admissible macro-block rather than automatically selecting the largest geometrically possible one.

### Region-wide nonlinear error bound

Write \(P_j=L_j+Q_j\), with linear and homogeneous quadratic parts. Since the retained recurrence uses \(L_j^2\), if \(u_j=P_j+e_j\), then

\[
e_{j+1}=2(Z_j+P_j)e_j+e_j^2+H_j,\quad H_j=2L_jQ_j+Q_j^2.
\]

On \(|u|\le r\), \(|\delta c|\le\rho\), define

\[
\ell_j=|A_j|r+|B_j|\rho,\quad
q_j=|C_j|r^2+|D_j|r\rho+|E_j|\rho^2.
\]

A sufficient **truncation-only, exact-arithmetic** majorant is

\[
E_{j+1}=
2(|Z_j|+\ell_j+q_j)E_j+E_j^2+2\ell_jq_j+q_j^2.
\]

The code precomputes this for incoming radii and requires bounds on *all intermediate states*, such as \(|Z_{j+1}|+\ell_{j+1}+q_{j+1}+E_{j+1}<2\), to permit safe non-escape skipping in its idealized arithmetic model.

For a **production certificate**, incorporate outward-rounded interval/ball bounds on \(Z_j\), every complex coefficient, multiplication/addition, conversion to extended exponent, accumulated incoming state error, and derivative uncertainty. Any margin crossing the bailout boundary must trigger a more precise step or local recomputation, not a guessed decision.

### Derivative correctness

Differentiate the quadratic stage:

\[
\widehat d_{\rm out} =
(A+2Cu+D\delta c)\widehat d_{\rm in}
+B+Du+2E\delta c.
\]

This is a *predicted* derivative, not a certificate. The one-step error in the true derivative satisfies a bound of the form

\[
F_{n+1}\le2(|Z_n+\widehat u_n|+E_n)F_n
+2E_n|\widehat d_n|+H'_n+R'_n,
\]

where \(F_n\) bounds derivative error, \(H'_n\) bounds the derivative of the stage's omitted nonlinear residual, and \(R'_n\) bounds rounding. One may calculate \(H'_n\) from \(H_n=2L_nQ_n+Q_n^2\) via \(H'_n=2L'_nQ_n+2L_nQ'_n+2Q_nQ'_n\), propagating derivative majorants alongside the state. The prototype does *not* implement this complete enclosure.

### Escape iteration and smooth value

For a skipped segment it is insufficient to inspect only its endpoint: a bailout may occur at a skipped intermediate iteration. Either bound every intermediate orbit to \(|z|<2\) or explicitly resolve an ambiguous segment. After establishing the **correct first** escape iteration \(n\), smooth coloring is

\[
\nu=n+1-\log_2\!\big(\log|z_n|\big).
\]

For a certified radial enclosure \(r_-\le|z_n|\le r_+\), with \(r_->2\), monotonicity gives

\[
n+1-\log_2(\log r_+)\ \le\ \nu\ \le\
n+1-\log_2(\log r_-).
\]

A point estimate is not a proof of coloring correctness. Recheck near-\(2\) bailouts with higher precision or exact fallback.

### Fourth-order parameter-series initializer (AT-like)

Construct \(u_n(\delta c)\approx\sum_{k=1}^{4}a_{k,n}(\delta c)^k\) across the shared reference. Coefficients satisfy

\[
a_{1,n+1}=2Z_na_{1,n}+1,\qquad
a_{k,n+1}=2Z_na_{k,n}+
\sum_{j=1}^{k-1}a_{j,n}a_{k-j,n}
\quad(k\ge2).
\]

A frame-wide residual majorant defines the longest accepted initial prefix, then each pixel evaluates the polynomial once and continues with ordinary perturbation or stage maps. The original uncached main experiment charged only the pixel evaluation: this understated the true work. The cached addendum explicitly moves construction to a shared prepass. For production, use adaptive series order/region partitioning and a certified truncation-plus-rounding bound.

## Acceptance policy / cost model

The prototype accepts stage maps when a cached, radius-indexed majorant (i) says no intermediate escape, and (ii) reports \(E_{\rm out}\le \rho\,\mathrm{tol}\). It tries larger stages first and falls back to an ordinary perturbation step. This is **not** sufficient to certify IEEE arithmetic, derivative, or finite-precision high-precision reference transport.

Illustrative per-pixel work proxy for stage 2:

\[
W_{\rm stage}=N_{\rm ordinary}
+2.1\,N_{\rm accepted\ blocks}
+0.7\,N_{\rm candidates},
\]

with comparable separate weights for BLA and initial-series maps. The weights were selected for investigation only, not fitted to Rust assembly or GPU telemetry. Preprocessing is recorded separately; the complete model should be

\[
T_{\rm frame}=T_{\rm reference}+T_{\rm stages}+T_{\rm certificates}
+\sum_{p\in\mathrm{pixels}}\!\big(T_{\rm initialize,p}
+T_{\rm check,p}+T_{\rm map,p}+T_{\rm fallback,p}
+T_{\rm color,p}\big).
\]

Profiler-derived per-operation coefficients, instruction cache behavior, pixel divergence, data movement, SIMD lanes and reference reuse are still unknown. A stage can lower \(N_{\rm ordinary}\) yet lose wall time when its rejection/dispatch overhead is too high.

## Proposed Rust CPU -> extended exponent -> GPU sequence

1. Add a deterministic test harness around the **existing** Rust plain perturbation and optimized BLA baselines. Test exact reference centers as decimal strings. Report per-frame total time and every relevant phase. Preserve the current baseline; do not substitute this study's Python BLA measurements for it.
2. Build the **shared fourth-order parameter-series** initializer, with configurable region radii and measured setup time. Retain a plain-perturbation fallback.
3. Add second-order composable stage maps and cached envelope/error tables, and expose acceptance/rejection counters, lengths, and original iterations remaining. Tune a profitability threshold on measured CPU costs, not on skipped-iteration counts.
4. Add **outward-rounded ball/interval arithmetic** for state/derivative/reference/coefficients and handle bailout ambiguity using fallback or reference rebasing. Verify smooth-value enclosures.
5. Solve resistant areas: adaptive nearby reference selection, critical-point rebase/first-step quadratic retention, optional higher-order jets for high curvature, shorter-stage or no-stage paths for fast escape, and adaptive stage-radius granularity.
6. After deterministic correctness and profiler evidence on CPU, define a common extended-exponent state/coefficients/error representation and implement GPU block selection with bounded divergence, warp-friendly lookup layouts, and fallback paths.
7. Run the real acceptance set: **24 frames, 480x270**, covering five three-decade zoom bands. For **every band**, require ≥3.0x total wall-clock speedup against optimized plain perturbation, correct first escape index on every pixel, smooth-value tolerance, and derivative accuracy; measure warm/cold preprocessing and confirm no correctness failures. Report geometric means only in addition to, never instead of, worst-band figures.

### Next falsifiable hypothesis

**Cached fourth-order parameter-series initialization + second-order staged LA + adaptive reference rebasing** can sustain profitable long skips on difficult boundary reference families, where a standalone series terminates early and standalone staged LA has high dispatch cost. Test long (>640-step) genuinely escaping boundary pixels and full frame mosaics with native Rust counters and interval-certified fallbacks. If this does not attain 3x on the weakest band, examine interval-adaptive reference selection and periods/critical-point coordinate transformations before increasing jet order.

## Reproducing the numerical experiments

Requires Python 3.10+, \`mpmath\`, and \`matplotlib\` (matplotlib is optional if the plot is not required):

\`\`\`sh
python -m pip install mpmath matplotlib
mkdir -p /mnt/data
python fractal_la_experiment.py --output /mnt/data
python fractal_la_cached_series.py
python fractal_la_bla_radius_addendum.py
python fractal_la_python_timing.py
\`\`\`

The addenda import \`fractal_la_experiment.py\` and use **hard-coded /mnt/data output filenames**. This path was the original Python execution environment. To run in another filesystem, change those output paths to your local output directory (or provide a compatible path). The main script supports \`--quick\` for a smaller smoke test. Timings, CSV float formatting, and file hashes may differ after rerunning; the result tables here are **snapshots from the research execution**.

## Artifact manifest

- \`fractal_la_experiment.py\`: mpmath high-precision baseline, plain perturbation, hierarchical BLA, period-based staged quadratic jets, bounds, series initialization, CSV and plot generation.
- \`fractal_la_cached_series.py\`: cached fourth-order series evaluation and prep-cost measurements.
- \`fractal_la_bla_radius_addendum.py\`: comparisons of one-step acceptance-radius rules.
- \`fractal_la_python_timing.py\`: selected-case Python microbenchmarks.
- \`fractal_la_per_pixel.csv\`: 972 method/pixel records (108 test pixels x 9 original variants).
- \`fractal_la_summary.csv\`: grouped summary of original experiment.
- \`fractal_la_cached_series.csv\`: 108 cached fourth-order series accuracy records.
- \`fractal_la_bla_radius.csv\`: 324 one-step-radius comparison records.
- \`fractal_la_python_timing.csv\`: selected-case Python timing table.
- \`fractal_la_costs.png\`: operation-proxy plot (palette-optimized PNG for repository transport, visually based on the original chart, not a separately measured benchmark).

### Bottom line

**The 3x per-band acceptance target has NOT been demonstrated.** On selected reference orbits, the algorithmic workload reductions warrant a Rust prototype; however, neither the current Python error checks nor the observed escape-index agreement constitute an end-to-end numerical certification. The next experiment must be a full-frame native implementation on difficult escaping zoom bands, with preprocessing counted and correctness verified against high precision.
