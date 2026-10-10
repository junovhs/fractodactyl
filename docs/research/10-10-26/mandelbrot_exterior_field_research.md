# Compiled analytic exterior fields for Mandelbrot rendering

Research prototype and technical notes, 10 October 2026.

## Executive result

An explicitly constructed local harmonic polynomial can replace escape-and-derivative recurrences for many exterior pixels. On a 1920 x 1440 CPU benchmark in the rectangular region Re(c) in [0.6,1.8], Im(c) in [0.55,1.45], a single-threaded fixed-degree-4, 12-sample local field renderer including construction and fallback took a median 95.62 ms versus 207.46 ms for independent orbit/derivative evaluations: 2.065x. A much more conservative version, using certified-exterior disks and analytic series-error screening with a *non-validated* assumed numerical sample error, took 130.28 ms versus 195.52 ms: 1.512x. The two implementations have different computational costs and degree choices; they are not a demonstration of fully rigorous floating-point certification.

This result applies to the specific all-exterior view, a scalar-double C++ implementation on an AMD EPYC 9V74, and the stated escape-threshold implementation. It does **not** demonstrate improvement over optimized SIMD/GPU orbit renderers, perturbation/BLA engines, or boundary-heavy scenes.

## Mathematical representation

Let z_0(c)=0, z_{n+1}(c)=z_n(c)^2+c. For c outside the Mandelbrot set M, the external uniformization Phi_M(c) exists and the Green potential satisfies

    G(c) = log |Phi_M(c)| = lim_{n->infinity} 2^{-(n-1)} log |z_n(c)|.

G is positive and harmonic on C\M. On any simply connected disk contained in the exterior, choose an analytic branch F=log Phi_M. Then

    G=Re F,   F'=G_x-i G_y.

Therefore a single approximation to F gives the potential, gradient, outward unit normal (Re F',-Im F')/|F'|, local potential/gradient ratio G/|F'|, and the conventional limiting Mandelbrot distance-estimator expression 2G/|F'|. The last two should not be confused with the exact Euclidean distance to the boundary. A standard smooth coloring coordinate is constant - log_2 G.

### Harmonic Fourier/Taylor patches

For a center c0 with a certified exterior disk of radius R, choose sample radius s<R and N equally spaced angular samples:

    u_j = G(c0+s exp(2pi i j/N)).

Define b_k approximately by (2/N) sum_j u_j exp(-2pi i jk/N) and b_0 by the sample mean. For delta=c-c0, a local field is

    P(delta) = b_0 + sum_{k=1}^p b_k (delta/s)^k,
    G(delta) ~= Re P(delta),   F'(delta) ~= P'(delta).

Construction is N reference orbit evaluations plus O(Np) Fourier arithmetic, while query is O(p) and supplies both value and derivative. The prototype uses this real-valued sampling technique, so no external-angle branch tracking is needed.

### Conditional analytic error bounds

If D(c0,R) is entirely outside M and G0=G(c0), positivity and harmonicity imply, for the Taylor coefficients of a local analytic F,

    |a_k| <= 2 G0 / R^k   (k>=1).

For all |delta|<=r<R, q=r/R, a degree-p true Taylor truncation satisfies

    |G-Re P_p| <= 2 G0 q^(p+1)/(1-q),

    |F'-P_p'| <= (2G0/R) [ (p+1)q^p/(1-q) + q^(p+1)/(1-q)^2 ].

The DFT representation must additionally account for (i) angular sampling aliases, (ii) error in the orbit-computed circle samples, and (iii) floating-point rounding when evaluating coefficients and pixel queries. If t=s/R, each mode k between 1 and p (p<N/2) has an alias coefficient bound

    <= 2 G0 [t^(N-k) + t^(N+k)]/(1-t^N)

in units of the sampling-radius coefficient. Multiply by (r/s)^k for field error and by k(r/s)^(k-1)/s for derivative error, then sum. The sample mean alias is at most 2G0 t^N/(1-t^N).

For example, q=1/4 and p=10 yield a Taylor potential-tail bound <=6.36e-7 * G0, plus alias and sample errors. A conservative derivative bound is 2.89e-5 * G0/R.

A rigorous implementation must compute enclosed G0, orbit samples, and coefficients using directed rounding or ball arithmetic, and propagate all error terms. The adaptive benchmark has a *heuristic 2e-13 absolute node-error allowance*, not a floating-point proof. Its tests are analytic error-screening experiments and empirical accuracy validation, **not end-to-end formal certificates**.

### Exterior disk proof

Given |c-c0|<=R, track z_n(c0) and a uniform disk radius e_n, where e_0=0 and

    e_{n+1} = 2 |z_n(c0)| e_n + e_n^2 + R.

This encloses every z_n(c) for c in the parameter disk (in exact arithmetic). If for some n,

    |z_n(c0)|-e_n > max(2, |c0|+R+2),

then every orbit in the disk has reached a sufficient escape radius, proving that the entire disk lies outside M. Interval rounding is needed to make the computer certificate rigorous; overestimation can cause false *rejections* but not mathematically false acceptances. This simple ball recurrence proves only a minority of large exterior disks near boundary structures.

### Sample orbit tail

For a sample parameter c and iterate z_n large enough that all later |c/z_k^2|<=eta<1, the exact telescoping identity is

    G(c) = 2^{-(n-1)} log |z_n| + sum_{k=n}^infty 2^{-k} log |1+c/z_k^2|.

Hence an elementary absolute remainder bound is

    <= 2^{-(n-1)} eta/(1-eta).

It can be combined with validated interval logarithms for source-value certificates. For gradient derivatives, one can propagate complex derivative/tail enclosures or use analytic bounds on the harmonic patch error. Exact classification of every point on the fractal boundary is not implied by a finite iteration budget.

## Experimental setup

- CPU: AMD EPYC 9V74 80-Core Processor, single-threaded C++ test, g++ 14.2.0, `-O3 -std=c++17 -ffp-contract=off`.
- Baseline: independent `z=z*z+c` and derivative `d=2*z*d+1` until |z| >= 1e8, or maximum 400 iterations; output `2^{-(n-1)} log|z|` and `2^{-(n-1)} d/z`. Both field and direct baseline calculate *potential plus gradient*, not only escape counts.
- Field: patches of 48 x 48 at 1920 x 1440 or 24 x 24 at 960 x 720. Disc certification, Fourier samples, all query cost and fallback cost included. No pixel from a rejected patch is approximated.
- Fast configuration: 12 circular samples, degree 4. Its error is empirical, not a priori certified. Accuracy checked against independent orbit evaluation at more than 305,000 held-out pixel centers for the large exterior view.
- Error-screened configuration: 48 samples, adaptive polynomial degree up to 20, conservative coefficient/alias/tail tests, and a separately assumed 2e-13 sample-error allowance. Fully rigorous rounded interval arithmetic is not implemented.
- Reported wall times are medians of five executions, while reported speed factors are medians of the five *paired within-run ratios* (and therefore do not exactly equal the quotient of separately reported time medians). Process and cache scheduling make small timing differences noisy. Two modes have separately timed baselines; do not compare their baseline numbers as if they were from the same process.

| View, pixel count | Field coverage | Fast: baseline / hybrid / speed | Screened: baseline / hybrid / speed |
|---|---:|---:|---:|
| Far exterior, 1920x1440 | 99.58% | 207.46 / 95.62 ms / 2.065x | 195.52 / 130.28 ms / 1.512x |
| Exterior mix, 960x720 | 37.17% | 103.15 / 91.90 ms / 1.109x | 102.00 / 104.86 ms / 0.973x |
| Wide Mandelbrot, 960x720 | 11.17% | 212.52 / 211.39 ms / 1.007x | 203.17 / 206.67 ms / 0.988x |
| Right-tip region, 960x720 | 2.17% | 553.76 / 554.65 ms / 0.998x | 505.05 / 506.78 ms / 0.997x |

In the far-exterior view, the largest relative discrepancy to held-out high-threshold orbit values was ~1.73e-7 in G and ~1.06e-5 in F' for the fast configuration, versus ~8.43e-12 and ~5.84e-9 for the screened configuration. Held-out testing is not a proof. The error-screened configuration's analytic truncation tests are more conservative than these measured discrepancies.

As a further sensitivity test, reducing the direct orbit bailout from |z| >= 1e8 to |z| >= 1e3 (squared norm 1e6) reduced the baseline cost but a field test still recorded about 1.9x acceleration in the exterior-dominated view. Reducing to |z| >= 100 gave about 1.7x in one run but also increased the observed maximum relative F' discrepancy to roughly 3e-4, a different accuracy setting.

## Break-even model

Suppose a patch provides Q pixel queries, requires construction cost B, has query cost Tfield, while independent orbit evaluation costs Torbit per pixel. Ignoring shared fallback, break-even is

    B + Q Tfield < Q Torbit,
    Q > B/(Torbit-Tfield),  provided Tfield<Torbit.

For screen-spanning pixel query counts, a patch of 48x48 answers 2,304 pixels at one zoom level, amortizing tens of circular orbit samples. For a feature closer than approximately the pixel spacing to the Mandelbrot boundary, the size of a provably exterior patch shrinks, Q falls, and fallback becomes preferable.

## What these experiments do and do not establish

**Established in the limited benchmark:** A local field representation can amortize construction and beat straightforward independent double-precision orbit-plus-gradient evaluation on a substantial exterior region; low-order polynomial query quality can be very high on well-separated exterior patches.

**Not established:** A win over GPU/SIMD orbit iteration, deep-zoom perturbation, BLA/series acceleration, exterior adaptive rendering with an optimized spatial hierarchy, or mathematical error certification of IEEE computations. Wide views and near-boundary windows in this prototype show little or no acceleration.

## Next research directions

1. Replace naive disk-radius propagation with interval Taylor-model propagation or subdivision to certify useful patches much closer to M.
2. Adapt tile dimensions and degree to per-patch exterior clearance and projected pixel count; reject candidate patches with negative predicted amortization.
3. Compile F and its derivative directly from Taylor jets of a post-escape orbit, eliminating 12–48 separate circle orbits where advantageous.
4. Use boundary residual checking: both the true G and the harmonic polynomial are harmonic inside a certified disk, so a continuous boundary error bound controls the entire interior by the maximum principle. Bound derivative error on smaller concentric disks.
5. Benchmark adaptive Chebyshev and AAA rational approximants (with pole-free region checks); compare nodes, coefficients, query cost, error bounds, and peak memory.
6. Exploit coherent pan/zoom and repeated frames; cache valid field patches across camera movements and supersampling passes.
7. Compare against tuned CPU SIMD/GPU kernels and perturbation/BLA baselines at matched smooth-color and gradient errors. Include build costs and failed patch costs.
8. Add directed-rounding/ball-arithmetic evaluation to turn the conditional tail and alias inequalities into actual rigorous floating-point guarantees.

## References

- Douady–Hubbard external uniformization summarized in the Princeton Companion to Mathematics: https://sites.math.rutgers.edu/~zeilberg/akherim/PCM.pdf
- Bielefeld, Fisher, von Haeseler, *Computing the Laurent Series of the Map Psi*, Advances in Applied Mathematics 14 (1993): https://www.sciencedirect.com/science/article/pii/S019688588371002X
- Jungreis inverse expansion noted in the MathOverflow discussion: https://mathoverflow.net/questions/48642/parametrization-of-the-boundary-of-the-mandelbrot-set
- Nakatsukasa, Sete, Trefethen, *The AAA Algorithm for Rational Approximation*, SIAM J. Sci. Comput. 40 (2018): https://epubs.siam.org/doi/10.1137/16M1106122
- Claude Heiland-Allen, *Deep Zoom* (perturbation and BLA reference): https://mathr.co.uk/web/deep-zoom.html
- Phil Thompson, BLA benchmarking and setup tradeoffs: https://philthompson.me/2023/Faster-Mandelbrot-Set-Rendering-with-BLA-Bivariate-Linear-Approximation.html

## Reproduce

```bash
# Empirical fast configuration (12 samples, degree 4)
g++ -O3 -std=c++17 -ffp-contract=off -DNUM_SAMPLES=12 -DDEGREE=4 mandelbrot_field_benchmark.cpp -o field_fast
./field_fast 48 1920 1440 faroutside

# More conservative analytical screening (not directed-rounding certified)
g++ -O3 -std=c++17 -ffp-contract=off -DNUM_SAMPLES=48 -DDEGREE=20 mandelbrot_field_adaptive.cpp -o field_screened
./field_screened 48 1920 1440 faroutside
```

`mandelbrot_benchmark_measurements.csv` contains the unaggregated measurements from five executions per method and view.
