<!-- Owner-run research answer to question 1 of frontier-questions.md, pasted 2026-10-11 and filed unchanged. Verdict: global whole-orbit Newton scan KILLED; certified segment Newton (lead 1) remains open. The agent's artifact archive was not provided. -->

# Fractodactyl: Parallel-in-time reference orbit investigation

Kill: unrestricted global Newton scan

Result: An affine prefix scan can parallelize each Newton correction, but it does not make the high-precision Mandelbrot initial-value problem reliably solvable in logarithmically many Newton sweeps.

The decisive obstruction is nonlinear convergence, not associativity. A double-precision initial orbit can have extremely small residuals while being far outside the global Newton convergence basin.

I ran an MPFR arithmetic benchmark, a 1,600-bit nonlinear convergence experiment, and a mixed-precision correction experiment. The unrestricted proposal fails the accuracy gate. A full-mantissa scan arithmetic surrogate also exceeds the 4× work threshold at 10,000–100,000 bits.

Artifacts: Complete experimental package, code, timing logs and reproducibility notes

I reviewed the repository's research reports and `docs/spec/METHOD.md`, particularly the existing high-precision, compiler, and ladder investigations. This work does not duplicate those implementations.

## 1. The decisive experiment: tiny residual, divergent Newton

Predeclared kill: Reject the unrestricted scheme if a bounded reference orbit requires a Newton correction larger than 1, or if the maximum residual increases after a sweep.

I used the real Mandelbrot orbit at

\\[ c=-2+2^{-256},\qquad z_0=0, \\]

with \\(N=384\\) and 1,600-bit arithmetic.

The initial guess is the exact double-precision orbit obtained after rounding the parameter to \\(-2\\). That guess is

\\[ 0,-2,2,2,2,\ldots \\]

Its initial defects are only approximately \\(2^{-256}\\).

For a candidate orbit \\(x\\), define

\\[ r_n=x\_{n+1}-x_n^2-c. \\]

Newton solves

\\[ \delta\_{n+1}=2x_n\delta_n-r_n,\qquad \delta_0=0. \\]

After applying the correction, the new residual is exactly

\\[ \boxed{r_n^{\rm new}=-\delta_n^2} \\]

in exact arithmetic.

Thus Newton only improves the residual when its correction is sufficiently small.

### Measured nonlinear convergence

| Completed sweeps | log₂ maximum residual | log₂ maximum correction |
| ---------------- | --------------------- | ----------------------- |
| 0                | −256.0                | +509.4                  |
| 1                | +1,014.8              | +64,872.7               |
| 2                | +128,728.6            | +5,475,173.8            |
| 3                | +10,821,617.0         | +345,189,707.8          |

The original orbit remains bounded, but the proposed corrections grow catastrophically.

At iteration 128, the original guess has already accumulated an error of approximately \\(2^{-2.6}\\). Beyond that point, Newton is no longer making uniformly small corrections.

Decision: KILL the double-seeded, whole-orbit Newton formulation as a general reference generator.

This does not rule out Newton on short, accurately seeded segments.

## 2. How many guard bits does the scan need?

The affine composition itself is correct:

\\[ (A_2,B_2)\circ(A_1,B_1) =(A_2A_1,\\;A_2B_1+B_2). \\]

However, the numerical stability of the recurrence is controlled by the inverse of its triangular Jacobian.

A useful absolute-error amplification bound is

\\[ S_0=0,\qquad S\_{n+1}=|2x_n|S_n+1. \\]

If arithmetic contributes local absolute errors bounded by roughly \\(2^{-q}\\), the accumulated error is bounded, to first order, by approximately \\(S_N2^{-q}\\).

To achieve state accuracy \\(\tau\\), a sufficient precision budget is therefore approximately

\\[ \boxed{q\gtrsim\log_2(S_N/\tau)} \\]

plus summation and implementation guards.

This is a conservative absolute-error bound. Complex cancellation may produce a better actual result, but cannot be assumed for certification.

### Measured amplification along the bounded reference

| Iteration | log₂ amplification bound |
| --------- | ------------------------ |
| 64        | 126.4                    |
| 128       | 254.4                    |
| 160       | 287.5                    |
| 384       | 511.5                    |

At iteration 384, the scan may need approximately 512 extra bits above the desired absolute output accuracy.

There is no constant guard-bit budget independent of orbit length.

The parameter-to-pixel sensitivity and viewport scale must subsequently convert the orbit bound into Fractodactyl's required \\(10^{-3}\\)-pixel error bound. An orbit error alone is not a pixel certificate.

### Mixed-precision experiment

Kill criterion: Reject an unguarded lower-precision scan if its absolute correction error exceeds \\(2^{-40}\\) at the tested checkpoints.

Residuals were computed at high precision; only the linear correction recurrence was rounded.

| Scan precision | log₂ absolute correction error at \\(n=384\\) |
| -------------- | --------------------------------------------- |
| 53 bits        | +456.4                                        |
| 106 bits       | +402.4                                        |
| 212 bits       | +296.4                                        |
| 424 bits       | +84.4                                         |
| 848 bits       | Exact in this constructed case                |

The 848-bit exactness results from the specially chosen integer multipliers and dyadic residual. It is not evidence of general stability.

Decision: A uniformly low-precision global scan is rejected. Mixed precision remains plausible inside short segments with an independently calculated sensitivity bound.

## 3. MPFR wall-clock arithmetic benchmarks

The sandbox exposes five Intel Xeon Platinum 8573C virtual CPUs, rather than the project's Ryzen 3900X or either GPU.

I measured three arithmetic kernels:

- Sequential MPFR reference iteration, \\(z\gets z^2+c\\).
- Full-mantissa affine composition, using a generic multiprecision coefficient.
- Full-mantissa correction propagation.

The last two use a coefficient surrogate rather than a complete, varying-orbit parallel scan. Their combined cost is an arithmetic estimate for a sweep, not measured execution of a working Newton-scan solver.

Cost kill criterion: Reject a full-precision scan if one correction sweep plus residual evaluation exceeds 4× sequential arithmetic.

| N         | Precision    | Sequential MPFR | One-sweep arithmetic estimate | Ratio |
| --------- | ------------ | --------------- | ----------------------------- | ----- |
| 10,000    | 1,024 bits   | 2.29 ms         | 8.14 ms                       | 3.55× |
| 10,000    | 10,000 bits  | 38.4 ms         | 205.9 ms                      | 5.36× |
| 10,000    | 100,000 bits | 1.233 s         | 6.831 s                       | 5.54× |
| 1,000,000 | 1,024 bits   | 0.223 s         | 0.756 s                       | 3.39× |

The 100,000-bit measurement is a single run; the other displayed rows use the best of three runs.

The full-width arithmetic gate fails at the two higher precision levels even before accounting for multiple Newton sweeps, scan communication or orbit storage.

### Simulated 8, 64 and 4,096 lanes

For completeness, I modelled a work-efficient block scan with

\\[ K=\left\lceil\log_2(p/53)\right\rceil \\]

hypothetical Newton sweeps, all charged at final precision.

The optimistic lane model is

\\[ T_L= K\left[ \frac{T\_{\rm seq}+T\_{\rm affine}+T\_{\rm corr}}{L} +\frac{2\lceil\log_2L\rceil T\_{\rm affine}}{N} \right]. \\]

It excludes memory bandwidth, synchronization, guard-bit escalation and convergence failure.

Seconds. M = arithmetic calibration measured; E = partly or entirely extrapolated. Every parallel-lane value is simulated, not measured hardware execution.

| N    | Bits | 8 lanes | 64 lanes | 4,096 lanes | Calibration |
| ---- | ---- | ------- | -------- | ----------- | ----------- |
| 10k  | 1k   | 0.00510 | 0.000660 | 0.0000586   | M           |
| 10k  | 10k  | 0.206   | 0.0268   | 0.00254     | M           |
| 10k  | 100k | 9.42    | 1.22     | 0.116       | M           |
| 100k | 1k   | 0.0493  | 0.00618  | 0.000139    | M           |
| 100k | 10k  | 2.22    | 0.278    | 0.00659     | M           |
| 100k | 100k | 94.0    | 11.8     | 0.281       | E           |
| 1M   | 1k   | 0.472   | 0.0591   | 0.000965    | M           |
| 1M   | 10k  | 21.2    | 2.65     | 0.0437      | E           |
| 1M   | 100k | 939     | 118      | 1.93        | E           |

These numbers establish only the arithmetic concurrency theoretically available after solving the nonlinear convergence problem. They are not achievable speedups demonstrated by the prototype.

The 100k-bit, 100k–1M-step configurations exceeded the short probe budgets. Their estimates are extrapolations, not completed MPFR benchmarks.

Full numerical table and calibration flags

## 4. Does the asymptotic complexity work?

Conditionally, yes.

A work-efficient prefix scan has \\(O(N)\\) affine compositions and logarithmic parallel depth. If Newton converges quadratically from a globally close initial guess, precision doubling could give

\\[ \sum\_{j=0}^{\lceil\log_2p\rceil}N M(2^j) =O(NM(p)) \\]

total arithmetic work for conventional multiplication-cost functions.

The parallel depth could then approach \\(O(\log N\log p)\\) scan stages when multiprecision arithmetic is treated as a primitive.

But the required assumption is exactly what fails: an initial double orbit is not globally accurate after a long chaotic or repelling transient.

There is also a major working-set problem. Simply storing one complex orbit requires:

| N         | Precision      | Raw orbit storage |
| --------- | -------------- | ----------------- |
| 1 million | 100,000 bits   | 25 GB             |
| 1 million | 1 million bits | 250 GB            |

Newton requires additional residuals, corrections and scan state. Streaming can reduce storage, but reintroduces segment dependencies and communication.

## 5. Ranked next leads

| Rank                                  | Mechanism and evidence                                                                                                                                                                                                       | Projected speed-up relative to sequential MPFR                                                                                                        | Cheapest confirmation or kill                                                                                                                                                                                                  |
| ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1. Certified segment Newton           | Break the orbit into accurately seeded blocks and accept corrections only when a computable nonlinear bound passes. The global failure experiment identifies why bounded blocks are necessary; no segment speed-up measured. | Speculative: 2–8× if neighboring keyframes or compiled maps provide accurate segment seeds cheaply. Otherwise no win.                                 | Test 256–4096-step segments on existing certified references, including the \\(c=-2+2^{-256}\\) hostile case. Require corrections small enough for a Newton enclosure, and beat the same-operator sequential baseline.         |
| 2. Periodic-closure multiple shooting | Solve a cyclic boundary-value system rather than the fixed-\\(z_0\\) IVP. A repelling periodic orbit permits an adjustable initial point, unlike the critical reference. No performance measurement yet.                     | Speculative: 1–4× for repeated expensive cycle refinement, compared with direct MPFR refinement; must also beat the existing factored cycle compiler. | Implement a 16-block cyclic solve for an existing long period and compare precision, build time and branch correctness with current factored methods. Kill if the compiler is faster or the wrong periodic branch is selected. |
| 3. Global Newton prefix scan          | Bounded-orbit experiment diverges; full-mantissa arithmetic surrogate costs 5.36–5.54× sequential at higher precision for one sweep.                                                                                         | No valid speed-up established. Killed as the general reference engine.                                                                                | No larger benchmark justified unless a new globally convergent preconditioner first passes the small hostile probe.                                                                                                            |

### A useful certification test for lead 1

For a candidate segment, calculate

\\[ \beta=\\|J^{-1}r\\|\_\infty,\qquad \kappa=\\|J^{-1}\\|\_\infty. \\]

A sufficient Newton fixed-point enclosure of radius \\(R\\) is

\\[ \boxed{\beta+\kappa R^2\le R,\qquad 2\kappa R<1.} \\]

This follows from expressing the exact correction as the Newton correction plus the inverse-Jacobian image of its quadratic remainder.

It gives a cheap, falsifiable admission gate. Segments for which the inequality fails are not parallelized.

The remaining research problem is obtaining accurate segment entry states without reconstructing the entire preceding orbit. Reusing already certified frame references or dynamically compiled operators is the promising possibility; producing those seeds sequentially would eliminate the benefit.

## 6. What the prior-art raid changes

The directly relevant recent literature agrees with the experimental obstruction. Work on parallel-in-time chaotic systems identifies coarse-grid instability as a central obstacle; shadowing formulations become better conditioned partly by relaxing the fixed initial-value constraint. That relaxation is unavailable when computing Fractodactyl's prescribed critical orbit.

FractalShark's current approach reinforces a different direction: keep the reference recurrence sequential, but execute its expensive multiprecision arithmetic more efficiently using a fused GPU kernel and NTT multiplication. Its reported substantial gains are on much more powerful NVIDIA hardware and at considerably higher precision than several of these probes. They cannot be transferred directly to the RX 5700 XT or RTX 3050 Ti.

## Final decision

Do not build a production global Newton-scan reference engine. The central hypothesis fails its first correctness gate, and the full-mantissa arithmetic surrogate fails the stated work threshold at higher precision.

The genuinely interesting remainder is certified, parallel correction of short orbit segments whose initial states are already cheaply available. That changes the experiment from trying to solve a chaotic million-step IVP all at once to asking whether Fractodactyl's existing keyframes and mathematical operators contain enough information to seed many stable blocks concurrently.

The archive contains the runnable C and Python probes, measured timing logs, mixed-precision results, and complete lane-model predictions. No production repository files were changed.
