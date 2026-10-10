# Fast-forwarding near-parabolic Mandelbrot dynamics

## Executive conclusion

Yes. Analytic coordinates can replace millions of near-parabolic iterations with a small number of evaluations. The most promising approach is a hybrid renderer that uses ordinary perturbation/BLA almost everywhere, but switches to a validated analytic transit map when an orbit enters a sufficiently well-understood parabolic bottleneck.

There are three important distinctions:

1. Mathematical feasibility: Fatou coordinates conjugate parabolic dynamics to translation, making arbitrarily long iteration jumps possible.
2. Numerical reliability: Stable, error-controlled jumps are achievable on restricted domains. A general complex-plane implementation requires considerably more machinery.
3. Competitive performance: A dramatic advantage over direct iteration is demonstrable. An advantage over optimized perturbation/BLA remains unproven and will depend strongly on the zoom geometry, reuse of reference orbits, and cost of constructing the analytic coordinates.

My investigation includes a numerical proof-of-concept for the real Mandelbrot cusp, with an explicit mathematical error bound for the analytic approximation.

REAL CUSP EXPERIMENT — c = 1/4 + 10⁻¹²

Direct iterations

# 3,141,591

Skipped analytically

# 3,141,191

Skipped fraction

# 99.987%

Both endpoints of the computed state enclosure escaped at iteration 3,141,591, agreeing with direct iteration. The mathematical enclosure does not yet account for floating-point rounding.

The Python experiment took approximately 0.141 seconds for direct iteration and 58 microseconds for the accelerated calculation, a roughly 2,400-fold reduction in elapsed time in this particular run. These are exploratory CPython timings, not a comparison against an optimized BLA implementation.

Across another 40 nearby real parameters, the interval-bracketed method reproduced all 40 direct escape counts without an ambiguous result.

Download the reproducible cusp experiment

The strongest finding is not the timing. It is that the approximation error can be bounded independently of the number of skipped iterations in this restricted case. That provides a concrete path toward acceleration without silently changing escape counts.

## 1. Which analytic transformations are worth implementing?

For the quadratic family

\\[ f_c(z)=z^2+c, \\]

the general idea is to replace a large iterate \\(f_c^{\circ N}\\) with a conjugacy to something that can be evaluated directly.

At an exactly parabolic parameter, a Fatou coordinate \\(\Phi\\) satisfies Abel's equation:

\\[ \Phi(f_c(z))=\Phi(z)+1. \\]

Consequently,

\\[ \boxed{f_c^{\circ N}(z)=\Phi^{-1}(\Phi(z)+N)} \\]

where the appropriate Fatou coordinate and inverse are defined.

This is an exact mathematical identity, not an approximation. The difficulty is evaluating both functions reliably for the particular orbit and parameter.

The modern foundations are well established, including Inou–Shishikura's work on parabolic and near-parabolic renormalization and Dudko–Sauzin's work on Fatou coordinates and Écalle–Voronin invariants.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://www.math.kyoto-u.ac.jp\&sz=32)

Kyoto Math Department

+2



### Practical assessment

| Method                                    | Potential acceleration                     | Main limitation                               | Priority  |
| ----------------------------------------- | ------------------------------------------ | --------------------------------------------- | --------- |
| Asymptotic Fatou coordinate               | Very high in parabolic petals              | Error, branches, convergence                  | High      |
| Near-parabolic transit coordinate         | Very high through bottlenecks              | Parameter dependence, inverse stability       | Highest   |
| Koenigs/Schröder linearization            | High near attracting or repelling cycles   | Domain shrinks near neutrality                | High      |
| High-order normal-form flow               | High for long slow passages                | Truncation and accumulated error              | High      |
| Validated polynomial/rational return maps | Moderate to high                           | Polynomial degree and domain size             | High      |
| Écalle horn maps                          | Potentially high across multiple petals    | Expensive construction and branch transitions | Research  |
| Full parabolic renormalization            | Potentially powerful for nested satellites | Implementation complexity                     | Long-term |

The recommended first implementation is near-parabolic transit coordinates with validated local normal forms, not a general-purpose Écalle renormalization engine.

## 2. A concrete construction with bounded error

The cusp \\(c=1/4\\) provides an unusually clean test.

Set

\\[ c=\frac14+\varepsilon,\qquad w=z-\frac12. \\]

The quadratic iteration becomes exactly

\\[ w\_{n+1}=w_n+w_n^2+\varepsilon. \\]

For real positive \\(\varepsilon\\), the orbit moves slowly through a bottleneck near \\(w=0\\). The transit time grows approximately like

\\[ N\sim\frac{\pi}{\sqrt{\varepsilon}}. \\]

A continuous approximation is

\\[ \frac{dw}{dn}=w^2+\varepsilon, \\]

whose solution suggests the phase

\\[ \Phi_0(w,\varepsilon) = \frac{1}{\sqrt{\varepsilon}} \arctan\left(\frac{w}{\sqrt{\varepsilon}}\right). \\]

But this coordinate alone omits an important discrete-iteration correction.

A better approximation is

\\[ \boxed{ \Phi(w,\varepsilon)= \frac{\arctan(w/\sqrt{\varepsilon})}{\sqrt{\varepsilon}} +\frac12\log(w^2+\varepsilon) } \\]

It is not an exact Fatou coordinate. However, its one-step error can be bounded explicitly.

### Deriving a non-accumulating error bound

Write \\(u=w^2+\varepsilon\\). Since

\\[ \Phi_w=\frac{1+w}{u}, \\]

the Abel defect is

\\[ r(w,\varepsilon) = \Phi(w+u,\varepsilon)-\Phi(w,\varepsilon)-1. \\]

Using the integral representation of this difference, for real \\(|w|\le b<1/2\\), one obtains

\\[ \boxed{ |r(w,\varepsilon)| \le \frac{7/6+b/2}{1-2b}\\,(w^2+\varepsilon) } \\]

The important observation is

\\[ w\_{n+1}-w_n=w_n^2+\varepsilon. \\]

Therefore the accumulated defect telescopes into a bound proportional to the total change in position:

\\[ \left| \Phi(w_N)-\Phi(w_0)-N \right| \le C_b(w_N-w_0), \\]

where

\\[ C_b=\frac{7/6+b/2}{1-2b}. \\]

This bound does not grow with \\(N\\).

For the experiment, I used \\(b=0.005\\) and a conservative accumulated phase-error budget of \\(E=0.02\\) iterations.

After a safe skip of \\(N\\) iterations, the unknown exact state is enclosed mathematically by

\\[ \boxed{ w_N\in \left[ \Phi^{-1}(\Phi(w_0)+N-E), \Phi^{-1}(\Phi(w_0)+N+E) \right]. } \\]

The implementation advances both endpoints through the remaining ordinary iterations. If they escape on the same iteration, the escape count is determined without reconstructing the skipped orbit.

For positive real \\(\varepsilon\\), the relevant map is monotone, which makes this particularly manageable.

### Experimental results

| \\(\varepsilon\\) | Direct escape count | Analytic skipped iterations | Escape count agrees |
| ----------------- | ------------------- | --------------------------- | ------------------- |
| \\(10^{-8}\\)     | 31,414              | 31,014                      | Yes                 |
| \\(10^{-10}\\)    | 314,157             | 313,758                     | Yes                 |
| \\(10^{-12}\\)    | 3,141,591           | 3,141,191                   | Yes                 |
| \\(10^{-14}\\)    | 31,415,925          | 31,415,525                  | Yes                 |

This is a meaningful existence demonstration: for this restricted real family, the slow dynamics can be replaced by a small number of transcendental evaluations and roughly 400 ordinary iterations.

Scope of the result: The error inequality is mathematical, but the demonstration uses ordinary floating-point arithmetic. It does not provide directed-rounding enclosures, does not handle general complex parameters, and does not certify derivatives. Those are necessary before claiming zero erroneous pixels in a production renderer.

## 3. Generalizing to parabolic roots and satellite minibrots

Let \\(c_0\\) be a parabolic parameter and \\(\zeta\\) a periodic point of period \\(p\\).

Define the return map

\\[ F_c=f_c^{\circ p}. \\]

If the multiplier at \\(c_0\\) is a primitive \\(q\\)-th root of unity, study

\\[ G_c=F_c^{\circ q}=f_c^{\circ pq}. \\]

For a generic nondegenerate satellite root, its parabolic expansion has the form

\\[ G\_{c_0}(\zeta+w) = \zeta+w+a w^{q+1}+O(w^{q+2}). \\]

A leading translation coordinate is

\\[ U=-\frac{1}{qa w^q}. \\]

Then

\\[ U(G\_{c_0}(z))=U(z)+1+\text{lower-order corrections}. \\]

Higher-order normal-form terms, including logarithmic corrections, remove much of the residual drift.

This is the natural generalization of the cusp calculation. However, a \\(q\\)-petal map has multiple sectorial branches, and the near-parabolic unfolding may contain several lower-order terms. A single arctangent formula cannot generally describe the entire transit.

The rigorous near-parabolic theory explicitly addresses maps with multipliers near roots of unity, not just near \\(1\\).&#x20;

[image](https://www.google.com/s2/favicons?domain=https://arxiv.org\&sz=32)

arXiv

+1



### Computing the maps numerically

A practical approach would be:

1. Locate and certify the periodic point and multiplier.
2. Expand the return map in a local coordinate using high-precision Taylor arithmetic.
3. Construct an asymptotic phase coordinate from its normal form.
4. Correct the residual Abel equation numerically.
5. Construct a validated inverse map on the intended transit domain.

Two possible corrections are attractive.

Asymptotic correction: Solve the Abel equation coefficient by coefficient, truncate according to an estimated or bounded remainder, and use the resulting coordinate only where the bound is adequate. Formal expansions can be divergent, so adding more terms indefinitely is not necessarily beneficial.

Spectral correction: Solve for a correction function on suitable complex domains using Chebyshev, Taylor, or rational approximations. This may achieve substantially higher accuracy, but the residual still needs a posteriori bounds and the inverse must remain single-valued.

Écalle's theory explains why sectorial solutions and exponentially small differences between them matter. Horn maps encode precisely the discrepancies that simple formal normal forms cannot capture.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://comptes-rendus.academie-sciences.fr\&sz=32)

Comptes Rendus

+1



## 4. Domain determination is essential

The method must never jump simply because a multiplier happens to be close to a root of unity.

A safe fast-forward requires a certified channel.

### Entry conditions

A candidate orbit should satisfy all of the following:

- It is in a neighborhood of a certified periodic point, with a known period and multiplier.
- The return-map approximation has a verified remainder bound throughout the proposed domain.
- The current point belongs to a selected attracting, repelling, or transit chart with a consistent analytic branch.
- The proposed trajectory remains inside the validated channel.
- No intermediate iterate can reach the escape boundary or violate another observable the renderer promises to compute.

This final condition matters. A correct endpoint does not automatically imply a correct first escape iteration, because the orbit might have escaped somewhere inside the skipped block.

### Determining safe complex domains

For a simple parabolic point, sectorial domains become approximately half-planes in the reciprocal normal-form coordinate.

For higher-order points, analogous regions occur in the different petals.

These provide mathematically motivated candidate domains, but a numerical implementation should determine their admissibility with validated calculations:

- Interval Newton or Krawczyk methods for periodic-point certification.
- Complex ball arithmetic and Taylor-model bounds for return-map errors.
- Sector or tube containment tests for the entire jump.
- Bounds on coordinate inversion and its derivative.
- Branch-cut exclusion and explicit continuation between charts.

One useful engineering design is to precompute domains in a local state–parameter space \\((w,c)\\), rather than certify every pixel independently. Certified parameter tiles can share local expansions and inverse-coordinate approximants.

When a domain cannot be certified, the renderer declines the shortcut.

## 5. Propagating derivatives correctly

Fast-forwarding only the state \\(z\\) is insufficient for distance estimation, normal-map shading, or derivative-sensitive rendering.

Ordinary iteration propagates

\\[ D\_{n+1}=2z_nD_n+1, \qquad D_n=\frac{dz_n}{dc}. \\]

Suppose an exact coordinate for a fixed parameter family satisfies

\\[ \Phi(H(z,c),c)=\Phi(z,c)+N, \\]

where \\(H=f_c^{\circ N}\\) is the skipped map.

Differentiate implicitly with respect to \\(c\\). If the input state has derivative \\(D\_{\rm in}\\),

\\[ \boxed{ D\_{\rm out} = \frac{ \Phi_z(z,c)D\_{\rm in} +\Phi_c(z,c) -\Phi_c(z\_{\rm out},c) }{ \Phi_z(z\_{\rm out},c) }. } \\]

Thus, one analytic jump can propagate both position and derivative.

This also extends to higher derivatives and Taylor jets through automatic differentiation of the coordinate and its inverse.

There is an important qualification:

A small position-error bound does not automatically imply a small derivative-error bound.

If the numerical coordinate has an Abel defect \\(R\\), the derivative formula also acquires a term involving the parameter derivative of the accumulated defect. Bounding \\(R\\) alone is not enough.

A production implementation therefore needs either:

- Certified derivative bounds for the approximate transit map, including the defect correction.
- Joint Taylor-model enclosures for the state and parameter derivatives.
- Uniform analytic error bounds on enlarged complex parameter domains, allowing Cauchy derivative estimates.

The third option may be conservative near singularities, but is mathematically clean.

Ordinary perturbation and BLA already have useful derivative propagation mechanisms, so the analytic accelerator must match their capabilities rather than discard them.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://mathr.co.uk\&sz=32)

Mathr

+1



### Smooth escape counts

After a valid jump, preserve the integer iteration index:

\\[ n\_{\rm new}=n\_{\rm old}+N. \\]

For a return-map jump, multiply by the return period.

Continue through the final escape using the actual quadratic iteration, or a separately validated alternative.

A standard normalized smooth count is

\\[ \boxed{ \nu=n+1-\log_2\left(\frac{\log|z_n|}{\log2}\right). } \\]

The final \\(z_n\\) should be computed accurately at the actual escape index, preferably with additional outward iterations if higher-accuracy potential estimates are desired.

Do not obtain the smooth count by treating the approximate Fatou phase as a fractional escape time. Fatou transit phase and exterior escape potential are different objects.

For distance estimation, use the propagated \\(D_n\\), with the usual exterior approximation

\\[ d(c)\approx \frac{|z_n|\log|z_n|}{2|D_n|}. \\]

Both the smooth count and distance estimate should inherit explicit numerical error tolerances.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://en.wikipedia.org\&sz=32)

Wikipedia

+1



## 6. Handling transitions between parabolic and ordinary dynamics

A renderer should not operate entirely in parabolic coordinates.

The better design is a state machine:

Ordinary perturbation / BLA

Default rendering path

Certified parabolic entry detector

Period, multiplier, chart, error budget, and escape safety

Eligible

Validated analytic jump

Ineligible

Continue BLA or ordinary steps

Validate exit and resume

Transfer state, derivative, and iteration count; escape-test or fall back

Important transition rules include:

Near but not exactly parabolic: Use a parameter-dependent transit coordinate. An exact-parabolic Fatou coordinate generally cannot be reused unchanged at nearby parameters.

Inside an attracting component: A certified invariant region or attracting-cycle test may eliminate the need for escape iteration entirely. Koenigs linearization can accelerate the approach when the corresponding conjugacy domain is known.

Crossing between petals: Use explicit chart transitions, or the appropriate horn-map information. Never silently change a complex logarithm branch.

Near a complex neutral multiplier: Local linearization may exist, but its useful domain can be extremely small. A multiplier close to the unit circle does not by itself guarantee a practical shortcut.

Leaving the parabolic region: Switch back before the phase coordinate becomes poorly conditioned. Use distinct entry and exit thresholds to avoid repeated switching.

For high-precision perturbation, the state transfer must preserve the high-precision reference-plus-delta representation. Reconstructing a full state in ordinary floating point can destroy deep-zoom information.

Finally, the high-precision reference orbit itself must benefit from the new scheme, or its millions of original iterations may become the remaining bottleneck.

## 7. Can this outperform optimized perturbation/BLA?

This is the hardest question.

Modern BLA implementations are much more competitive than simple perturbation. They can skip large iteration blocks, reuse periodic reference orbits, and use higher-order approximations. Published implementation reports include substantial improvements, including a reported 21-fold speedup with 96.9% of iterations skipped at one deep-zoom location.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub

+1



Furthermore, minibrot-specific bivariate series methods already permit repeated jumps over whole periods. These should be part of a serious baseline.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://www.mathr.co.uk\&sz=32)

Mathr



### Where analytic skipping should win

The favorable situation is a long parabolic bottleneck where:

- The number of required iterations grows dramatically as a parameter approaches a root.
- A relatively small collection of analytic charts covers many pixels.
- Pixel parameters differ enough that a shared linear or low-order BLA ceases to be effective.
- The analytic transit can be evaluated and inverted with a small, bounded amount of work.
- Few pixels require expensive fallback.

In this regime, the analytic cost may remain approximately constant while ordinary iteration cost diverges.

### Where BLA may remain superior

At an extremely deep zoom centered near a particular parameter, the pixel offsets can be tiny compared with the scale of the parabolic unfolding.

Then a single high-precision reference orbit may closely approximate all pixels, and BLA may skip almost the same enormous bottleneck.

For instance, near the real cusp, if the viewport parameter width is much smaller than \\(\varepsilon\\), the shared-reference approximation can remain favorable across much of the slow passage.

In that case, the analytic approach competes not against millions of steps per pixel, but against a few BLA evaluations. Its logarithms, inversions, certificates, and branch management could make it slower.

### Expected regimes

| Situation                                            | Likely preferred method                      |
| ---------------------------------------------------- | -------------------------------------------- |
| Wide near-cusp transit region                        | Analytic transit                             |
| Very deep narrow zoom with excellent reference reuse | BLA or higher-order BLA                      |
| Long isolated near-parabolic slowdown                | Analytic transit                             |
| Short periodic orbit, strongly attracting            | Cycle-based or Koenigs acceleration          |
| Complicated satellite with frequent petal changes    | BLA initially; analytic methods experimental |
| Outside validated analytic domains                   | Perturbation/BLA fallback                    |

These are predictions, not measured winners.

The real-cusp benchmark establishes a major speedup over direct iteration but does not establish a speedup over an optimized perturbation/BLA renderer. That comparison still needs to be performed.

## 8. The critical correctness and performance test

I would structure the validation in three stages.

### Stage A — Prove correctness in a tractable family

Extend the real-cusp prototype to use directed-rounding interval arithmetic, retaining the bounded-error phase coordinate.

The target is an end-to-end certificate that both endpoints of the exact-state enclosure first escape on the same iteration, not merely agreement between two floating-point implementations.

Then propagate first derivatives through the jump and verify their enclosures against high-precision direct differentiation.

This is achievable without implementing the full Écalle machinery.

### Stage B — Generalize to genuinely complex parabolic dynamics

Use a benchmark suite with progressively harder geometry:

| Test case                              | What it establishes                                |
| -------------------------------------- | -------------------------------------------------- |
| \\(c=1/4+\varepsilon\\), positive real | Long single-channel transit                        |
| Complex parameters near \\(c=1/4\\)    | Branches, complex phase, transit boundaries        |
| Near \\(c=-3/4\\)                      | Root-of-unity multiplier and multiple petals       |
| Small-period satellite roots           | Periodic return-map construction                   |
| High-period satellite minibrots        | Setup cost and reusable charts                     |
| Nested near-parabolic structures       | Repeated chart switching and renormalization       |
| Ordinary deep-zoom regions             | Whether the dispatcher avoids unnecessary overhead |

In particular, the \\(c=-3/4\\) region is valuable because it exercises a multiplier of \\(-1\\), rather than the simpler multiplier-\\(1\\) cusp.

### Stage C — Compare fairly against BLA

The optimized baseline should include high-precision reference orbits, glitch detection and rebasing, a BLA hierarchy, second-order BLA where available, minibrot-specific series approximation, and exponent-range handling.

Measure whole-frame time rather than just the inner iteration kernel.

Record reference-orbit computation, BLA preparation, analytic-chart construction, per-pixel evaluation, fallback, and final coloring costs.

The benchmark should also compare cold renders against renders reusing existing reference or analytic data.

For correctness, use high-precision independent reference rendering and mathematical interval certification wherever possible. Report at least integer escape-index disagreements, smooth-count error, derivative error, unresolved intervals, and fallback rate.

An approximate interval that straddles two possible escape iterations must trigger refinement or fallback. It must never be assigned an arbitrary winning iteration.

I would use the following engineering decision criterion:

Advance the method only if it delivers a repeatable, end-to-end speedup of at least 2× on several difficult near-parabolic views, with zero accepted incorrectly classified pixels in the validated test set.

That threshold is a proposed practical target, not an established result.

## 9. Implementation priorities

First: certified cusp transit

Convert the demonstrated real-cusp error enclosure into a directed-rounding implementation. Add parameter derivatives, robust inverse evaluation, and exact bailout verification.

Second: complex local transit charts

Build parameter-dependent, high-order normal-form coordinates for simple parabolic periodic points. Use guarded inversions and explicit domain certification.

Third: integration with perturbation/BLA

Introduce a dispatcher that uses analytic jumps only when a validated cost estimate predicts a gain, and retain reference-plus-delta precision across transitions.

Fourth: satellite and Écalle extensions

Handle multiple petals and chart-transition maps. Attempt reusable parabolic-renormalization machinery only after local transit maps have demonstrated real performance gains.

## Final assessment

The investigation supports a qualified positive answer.

- Replacing millions of iterations: Demonstrated in a restricted real-cusp experiment, with an analytic error bound independent of the number skipped.
- Numerical stability: Plausible and mathematically controllable on certified local domains, but not yet established for a general complex renderer.
- Derivatives and smooth coloring: There is a well-defined analytic propagation framework. It needs separate error certification.
- Transitions and satellites: Supported by substantial mathematical theory, but technically demanding to implement efficiently.
- Zero incorrect pixels: Requires enclosure-based acceptance, branch control, and fallback; heuristic agreement alone is insufficient.
- Beating optimized BLA: Not yet established. The strongest opportunity is in long near-parabolic passages where reference-based approximations lose effectiveness or constructing the reference orbit is itself expensive.

The central engineering insight is that the accelerator should certify the whole slow passage rather than approximate individual steps. That changes the error-analysis problem from controlling millions of iterations separately to verifying one local transit map.

A hybrid parabolic/BLA renderer is therefore worth developing experimentally. A general Écalle renormalization engine is not the right starting point.

### Principal references

The most relevant foundations and existing implementation work are:

- [Inou and Shishikura — The renormalization for parabolic fixed points and their perturbation](https://www.math.kyoto-u.ac.jp/~mitsu/pararenorm/ParabolicRenormalization.pdf).
- [Dudko and Sauzin — The resurgent character of the Fatou coordinates of a simple parabolic germ](https://arxiv.org/abs/1307.8093).
- [Kapiamba — Non-degenerate near-parabolic renormalization](https://arxiv.org/abs/2210.06647).
- [Claude Heiland-Allen — Deep zoom theory and practice](https://www.mathr.co.uk/blog/2021-05-14_deep_zoom_theory_and_practice.html).
- [Phil Thompson — Faster Mandelbrot Set Rendering with BLA](https://philthompson.me/2023/Faster-Mandelbrot-Set-Rendering-with-BLA-Bivariate-Linear-Approximation.html).

These establish the mathematical and renderer background. The real-cusp approximation, error-bound application, and numerical experiment described above are the investigation's own preliminary results, not benchmarks reported by those sources.