# Toward a General-Purpose Mandelbrot Dynamical Compiler

## Executive assessment

A general-purpose dynamical compiler is mathematically plausible, but a universal system that always compresses most Mandelbrot iterations is not.

The promising approach is to treat Mandelbrot rendering as a problem of discovering, approximating, validating, and composing dynamical operators, rather than simply evaluating the same quadratic recurrence millions of times.

Given a camera center \\(c_0\\), width \\(W\\), a high-precision reference orbit, and an iteration budget, such a system could automatically:

1. Detect recurrent and near-periodic behavior in the reference trajectory.
2. Locate nearby repelling cycles, critical returns, hyperbolic component centers, and Misiurewicz parameters.
3. Construct local return maps using derivatives and Taylor coefficients.
4. Identify when those maps exhibit renormalization, expansion, contraction, or near-parabolic slowing.
5. Select accelerated operators, compose them, and validate their domains of applicability.
6. Revert to direct perturbation iteration wherever no safe shortcut exists.

The most important distinction is between recognizing a structure and turning that recognition into a cheaper computation. The former is often numerically feasible; the latter depends on a valid local model, its error bounds, its evaluation cost, and the number of pixels that can reuse it.

Existing algorithms already provide significant portions of this foundation. Modern deep-zoom renderers use perturbation, series approximation, rebasing, and bivariate linear approximation (BLA) to eliminate large amounts of repeated computation. Automatic period scans and Newton refinement can also locate hyperbolic centers. These methods establish that substantial automatic compilation is possible, although they do not collectively constitute the full proposed compiler.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://mathr.co.uk\&sz=32)

Mathr

+3



The key research opportunity is therefore not inventing a shortcut for every recognizable feature. It is creating a camera-conditioned compiler that discovers useful dynamical structure at runtime and chooses the cheapest validated representation.

One limitation matters immediately: a single reference orbit does not reveal every dynamically important periodic point. The compiler must sometimes probe neighboring parameters and phase-space regions. It must also tolerate unfamiliar orbits for which no compact representation is available.

## 1. What the compiler should actually compile

The underlying dynamical system is

\\[ f_c(z)=z^2+c,\qquad z_0=0. \\]

Let \\(Z_n\\) denote the high-precision reference orbit at \\(c_0\\), and let \\(\delta=c-c_0\\) be an individual pixel's parameter displacement.

Ordinary perturbation rendering advances

\\[ \epsilon\_{n+1}=2Z_n\epsilon_n+\epsilon_n^2+\delta. \\]

A dynamical compiler should go further. It should recognize that large intervals of iterations sometimes correspond to one recognizable mathematical operation.

For example, near a repelling periodic orbit of period \\(p\\), define

\\[ R_c(z)=f_c^p(z). \\]

Suppose \\(s\\) is a fixed point of this return map, with multiplier

\\[ \lambda=R_c'(s),\qquad |\lambda|>1. \\]

For sufficiently small \\(u\\),

\\[ R_c^k(s+u) =s+\lambda^k u+O\\!\left((\lambda^k u)^2\right). \\]

This is already a compact computational program. A potentially long sequence of \\(kp\\) iterations becomes a multiplier power and a local error check.

Higher-order linearizing coordinates can improve the approximation. If a local coordinate \\(h\\) satisfies

\\[ h(R_c(z))=\lambda h(z), \\]

then

\\[ \boxed{ R_c^k(z)=h^{-1}\\!\left(\lambda^k h(z)\right) } \\]

where the local conjugacy and its inverse are valid.

Unlike simply extrapolating an orbit, this exploits the actual dynamical geometry.

The goal is to discover such coordinates without being told where the periodic orbit is.

## 2. Detecting dynamical structures automatically

The reference orbit provides a rich collection of numerical signals, but different structures require different detectors.

| Structure                | Automatic recognition                                    | Potential compiled operation                   | Assessment                       |
| ------------------------ | -------------------------------------------------------- | ---------------------------------------------- | -------------------------------- |
| Repelling cycle          | Near recurrence, Newton solve, multiplier test           | Local linearization and power jump             | Strong potential                 |
| Near-critical return     | Minima of \\(\lvert Z_n\rvert\\), return-time clustering | Quadratic return polynomial                    | Strong potential                 |
| Misiurewicz approach     | Detect \\(Z\_{q+p}\approx Z_q\\), refine parameter       | Transient map followed by repelling-cycle jump | Strong potential                 |
| Minibrot renormalization | Repeated critical returns, nested return domains         | Rescaled polynomial-like map                   | Promising, difficult             |
| Parabolic bottleneck     | Multipliers close to roots of unity, slow transit        | Fatou-coordinate translation                   | High upside, delicate numerics   |
| Nested repetition        | Hierarchies of return periods and scaling ratios         | Recursive composition of return maps           | Promising for structured regions |

### Recurrence and cycle detection

The first detector should look for two different phenomena:

\\[ d_p=|Z_p| \\]

measures returns of the critical orbit near zero, while

\\[ d\_{q,p}=|Z\_{q+p}-Z_q| \\]

measures near-periodicity after a transient of length \\(q\\).

The distinction is fundamental.

A near-critical return is evidence of a possible hyperbolic component, minibrot, or renormalization scale. A near-periodic return away from zero can indicate proximity to an attracting or repelling cycle.

A useful detector would maintain record minima, lagged recurrences, and candidate return periods, using camera width, numerical uncertainty, and estimated orbit sensitivity to determine whether the recurrence is significant.

For a promising period \\(p\\), solve

\\[ f_c^p(s)-s=0 \\]

with Newton iteration, starting from a nearby reference-orbit point.

Evaluate the multiplier

\\[ \lambda=\prod\_{j=0}^{p-1}2f_c^j(s). \\]

This separates attracting, repelling, and nearly neutral cycles. Fundamental-period checks are necessary to avoid mistaking a period-\\(p\\) cycle for a primitive cycle of period \\(2p\\).

There is already precedent for automatic period discovery. Claude Heiland-Allen's numerical work describes atom domains, period scans, Newton-based nucleus finding, and automated Misiurewicz-domain analysis.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://mathr.co.uk\&sz=32)

Mathr

+3



### Finding structures outside the reference orbit

A critical orbit may never pass close to a repelling cycle that nevertheless determines nearby geometry.

Therefore, reference-orbit recurrence cannot be the only discovery mechanism.

A more complete detector needs a secondary search over promising phase-space seeds, low-period cycles, nearby parameter samples, and continuations from previously discovered features.

The search should be camera-aware. If a candidate feature is dynamically important but its influence never intersects the viewport's pixel trajectories, compiling it wastes time.

This is one reason a general-purpose compiler needs active numerical probing, not merely a sophisticated classifier applied to one reference orbit.

## 3. Building return maps automatically

The most reusable abstraction is a local return map.

Suppose a candidate period \\(p\\) and relevant phase-space center \\(s\\) have been identified.

Construct a Taylor model

\\[ R\_{p,c_0+\delta}(s+u)-s = \sum\_{i+j\le d}a\_{ij}u^i\delta^j + E_d(u,\delta). \\]

The coefficients can be generated automatically through polynomial arithmetic or truncated automatic differentiation applied to the original recurrence.

For example, parameter derivatives along the reference orbit satisfy

\\[ D\_{n+1}=2Z_nD_n+1,\qquad D_0=0, \\]

\\[ D^{(2)}\_{n+1} = 2Z_nD^{(2)}\_n+2D_n^2. \\]

Higher-order derivatives can be propagated without symbolic expansion of the enormous polynomial \\(f_c^p\\).

This enables the compiler to construct a return map for a previously unrecognized period.

Near a critical return, a useful model often resembles

\\[ R(u,\delta) = a_0+b\delta+au^2 +O(u^3,u\delta,\delta^2), \\]

after appropriate local normalization.

Near a regular periodic point, the corresponding leading term is linear:

\\[ R(u,\delta)=\lambda u+b\delta+\cdots. \\]

The compiler can classify the type of return map from its derivatives instead of requiring a researcher to recognize the picture.

### Composition is the critical next step

One return map of period \\(p\\) reduces \\(p\\) direct iterations to one operator evaluation. That is useful, but does not yet deliver the strongest possible acceleration.

The compiler must also discover situations where repeated applications of the return map can be replaced by something cheaper:

- Linearization for repelling or attracting cycles.
- Power-series iteration or approximate conjugacy for regular maps.
- Fatou-coordinate translation for parabolic dynamics.
- Recursive renormalization for repeated critical returns.

The difficult case is a return map that remains strongly nonlinear and has no simple normal form over the relevant domain. It may still admit hierarchical Taylor approximation, but no enormous jump is guaranteed.

This distinction prevents the architecture from degenerating into an expensive structure detector that provides little rendering benefit.

## 4. The operator language

I would use a small intermediate representation, in which each operation carries a validity domain and an error estimate.

Proposed dynamical intermediate representation

Ordinary iteration

`STEP(n)` — one exact perturbation recurrence

AFFINE / JET

Skip a validated orbit block

RETURN

Apply an automatically fitted fᵖ

LINEARIZE

Jump k cycle returns

FATOU

Translate through a bottleneck

RENORMALIZE

Enter a rescaled dynamical copy

REBASE

Switch reference coordinates

VALIDATE / FALLBACK

Check error, domain containment, escape conditions and numerical stability. Revert to smaller blocks or direct iteration when necessary.

Conceptual operator system; not an existing Fractodactyl implementation.

These operators can be composed into a piecewise execution graph. The compiled result need not be one fixed sequence for every pixel: different pixels may pass through different dynamical regions.

This is analogous to speculative optimization in a conventional compiler, except that each guard represents a mathematical validity condition.

A crucial requirement is preserving first escape time. Even if a macro-operator predicts the correct final state, it cannot safely skip a block if an intermediate orbit might have crossed the bailout threshold. Intermediate orbit bounds or refinement checks are essential.

Existing BLA already demonstrates guarded block composition. Heiland-Allen describes combining local affine maps

\\[ T(u,\delta)=Au+B\delta \\]

into larger operators while propagating their applicability radii. The proposed compiler generalizes this idea to nonlinear returns and local dynamical conjugacies.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://www.mathr.co.uk\&sz=32)

Mathr

+1



## 5. Numerical experiments: can the discovery work?

I ran three small computational probes using the quadratic iteration. These are mechanism-level tests, not a renderer benchmark.

Experiment A: unfamiliar Misiurewicz approach

Numerical success

110-decimal-digit arithmetic, automatic recurrence scan over candidate preperiods and periods from 1 to 8.

I constructed a less-familiar strictly preperiodic parameter numerically:

\\[ c\_\*\approx 0.4196433776070805663 + 0.6062907292071993693\\,i. \\]

It is a root of

\\[ f_c^5(0)=f_c^3(0). \\]

I then displaced the camera center from that exact parameter by

\\[ \delta c=10^{-35}(1+0.6i). \\]

The recurrence detector, without being supplied the target preperiod or period, identified its strongest match at

\\[ \boxed{q=3,\qquad p=2.} \\]

The measured near-return residual was approximately \\(2.67\times10^{-34}\\).

Newton refinement of the relevant periodic orbit gave a repelling multiplier

\\[ \lambda\approx 5.67857351+2.42516292i, \\]

with \\(|\lambda|\approx6.17476\\).

I then compared direct iteration with the simple linearized operator that jumps twenty period-two returns.

Direct iterations replaced

# 40

Measured relative state error

## 9.71 × 10⁻²⁰

Absolute state error

3.19 × 10⁻³⁸

Operator

\\(s+\lambda^{20}(z-s)\\)

Errors are against high-precision direct iteration, with relative error normalized by the displacement from the periodic point. Discovery and Newton-refinement costs are excluded from the skip count.

This is meaningful evidence: a recurrence scan discovered the local cycle, and a mathematical operator accelerated it accurately without a hand-written location-specific rule.

However, the test parameter was deliberately constructed near a preperiodic point, rather than drawn from a blind random corpus. The first-order approximation is also valid only while the orbit remains in its local domain. Its success does not establish reliable behavior for arbitrary camera widths.

Experiment B: recognizing nested repetition

Structure detected

At the real period-doubling accumulation parameter,

\\[ c\approx-1.4011551890920506, \\]

the critical orbit exhibits near-zero returns at iteration counts

\\[ 2,\ 4,\ 8,\ 16,\ 32,\ 64,\ 128,\ldots \\]

For each power of two, I measured the absolute return distance and the ratio of successive distances.

Measured ratio |Z₂ⁿ| / |Z₂ⁿ⁺¹|

Convergence toward Feigenbaum spatial scaling

2.5029052.502911252.50291752.502923752.50293163264128256512

The ratios approach the known Feigenbaum scaling magnitude \\(\alpha\approx2.502907875\\).&#x20;

[image](https://www.google.com/s2/favicons?domain=https://mathworld.wolfram.com\&sz=32)

MathWorld

+1



An automatic detector could recognize the period-doubling hierarchy through the return times and scaling ratios, without a hard-coded classification of the location.

But recognition alone is not acceleration. To make this useful, the compiler must construct reliable return maps at periods \\(2^k\\), normalize them, and reuse those maps over neighboring pixels.

Infinite renormalizability also means that only finitely many levels can be resolved at finite precision and camera width.

Experiment C: parabolic bottleneck

Approximation needs refinement

Take

\\[ c=0.25000001. \\]

Writing \\(u=z-\frac12\\) gives the exact iteration

\\[ u\_{n+1}=u_n+u_n^2+10^{-8}. \\]

This is a discrete near-parabolic map with a long slow passage.

A first approximation comes from the differential equation

\\[ \frac{du}{dn}=u^2+\varepsilon, \\]

which yields

\\[ n_2-n_1\approx \frac{1}{\sqrt{\varepsilon}} \left[ \arctan\left(\frac{u_2}{\sqrt{\varepsilon}}\right) - \arctan\left(\frac{u_1}{\sqrt{\varepsilon}}\right) \right]. \\]

In the numerical test, passage from just above \\(u=-0.01\\) to \\(u=+0.01\\) required 31,216 iterations. The continuous approximation predicted a crossing time of approximately 31,215.56 iterations using the measured entrance state.

That is a surprisingly good transit-time estimate for a simple model.

But state reconstruction is less accurate. Using that continuous map to jump 25,000 iterations gave a state error of approximately \\(1.17\times10^{-7}\\).

This can be much too large for an extreme-zoom renderer, depending on the later orbit sensitivity.

The conclusion is important: near-parabolic behavior can be detected and its duration predicted, but precision-safe skipping requires more than the leading continuous approximation.

A proper compiler should construct higher-order Fatou coordinates satisfying the Abel equation

\\[ \Phi(R(z))=\Phi(z)+1 \\]

on an appropriate local domain, then use

\\[ R^k(z)=\Phi^{-1}(\Phi(z)+k). \\]

The computational complexity is not in recognizing slow transit; it is in constructing a sufficiently accurate inverse coordinate while respecting different petals, branch behavior, and parameter changes. The theory of Fatou coordinates and parabolic dynamics supports this approach.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://www.sciencedirect.com\&sz=32)

ScienceDirect

+1



## 6. Automatic minibrot renormalization

This is probably the most mathematically ambitious part of the compiler.

A near-zero return

\\[ |Z_p|\ll1 \\]

does not by itself establish that a minibrot has been found.

Genuine quadratic-like renormalization requires a suitable restriction

\\[ f_c^p:U'\longrightarrow U, \\]

where \\(U'\\) is compactly contained in \\(U\\) and the restricted map is a proper holomorphic map of degree two.

That domain structure is what makes a renormalized quadratic model meaningful. The literature on polynomial-like mappings and complex box mappings provides the mathematical foundation.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://link.springer.com\&sz=32)

Springer Nature Link

+1



I would separate recognition into two stages.

Candidate discovery: identify stable return periods, locate nearby nuclei using Newton's method, and construct local quadratic models. This is comparatively tractable.

Renormalization validation: construct plausible return domains, check the critical point and mapping degree, establish boundary containment, and test the resulting normalized map across the relevant camera region.

This is substantially harder.

For practical rendering, a numerical validity domain may suffice if every use of the map has strong error guards. Formal certification would require techniques such as interval arithmetic, validated boundary mappings, and argument-principle checks.

A correct renormalization detector would also provide something more powerful than an acceleration hint: a hierarchy of dynamically related maps.

Nested minibrots could then be represented as compositions of normalization and return operators, with repeated submaps shared rather than expanded.

## 7. Symbolic dynamics as a discovery aid

Numerical recurrence alone is sometimes ambiguous. Two regions can have similar return periods but different combinatorics.

External angles, kneading sequences, orbit portraits, and internal addresses offer another layer of information.

For the quadratic family, external-ray angles evolve under

\\[ \theta\mapsto 2\theta\pmod1. \\]

A symbolic itinerary can encode the relevant combinatorial structure. In particular, internal addresses provide a way to represent sequences of hyperbolic periods and renormalization ancestry. Schleicher's work establishes the mathematical connection between these symbolic descriptions and parameter-space structures.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://arxiv.org\&sz=32)

arXiv

+1



A compiler could use symbolic information to distinguish competing period hypotheses, identify satellite relationships, recognize repeated combinatorial patterns, and propose candidate return domains.

But I would not make symbolic dynamics the primary numerical engine.

Recovering external angles and puzzle partitions near arbitrary deep boundary locations can itself be expensive or numerically unstable. Symbolic information should guide the search and help explain discovered hierarchies; local numerical maps should provide the actual acceleration.

## 8. Adaptive compilation and algorithm selection

A useful compiler cannot apply every expensive detection method at every location.

The appropriate objective is

\\[ \min\_{\mathcal P} \left[ C\_{\mathrm{discover}}+ C\_{\mathrm{compile}}+ \sum\_{j\in\mathcal P}C\_{\mathrm{operator},j} \right] \\]

subject to accuracy, validity, and first-escape constraints.

A practical high-level algorithm would look like this:

```
compile(camera, width, reference, iteration_limit):

    analyze reference orbit
    find near-critical returns
    find periodic and preperiodic candidates
    identify possible nested recurrence hierarchies

    sample representative viewport trajectories
    rank candidates by dynamical relevance and expected savings

    for each promising candidate:
        refine cycles and parameters using Newton
        classify multipliers and local structure
        construct Taylor/return-map coefficients
        estimate validity domains and error bounds

        if useful:
            construct linearization, Fatou, or renormalization map
            validate on independent trajectories

    build a graph of possible operator transitions

    measure or estimate costs on representative pixels
    select profitable validated operators

    generate guarded execution paths
    retain perturbation and BLA as universal fallbacks

    return compiled renderer
```

### Operator selection should occur at two levels

At compile time, select candidates based on total estimated preprocessing cost and expected pixel coverage.

At runtime, select the largest valid jump for the current pixel and state.

This combines global amortization with local numerical safeguards.

For a frame of \\(M\\) pixels and maximum iteration count \\(N\\), the meaningful comparison is not merely direct iterations saved. It is total work:

\\[ T\_{\mathrm{compiled}} = T\_{\mathrm{reference}} + T\_{\mathrm{discovery}} + T\_{\mathrm{preparation}} + T\_{\mathrm{execution}}. \\]

A method that saves 90% of pixel iterations but takes longer to compile than the frame takes to render has failed.

The baseline must be state-of-the-art perturbation plus BLA and series approximation, not naïve high-precision per-pixel iteration. Fraktaler's documentation describes both BLA and automatic hardware/backend selection, illustrating how much of the general acceleration and algorithm-selection foundation already exists.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://mathr.co.uk\&sz=32)

Mathr

+1



## 9. The critical blind test

To determine whether this goes beyond location-specific techniques, I would freeze the compiler and evaluate it on parameter locations it has never previously encountered.

Proposed validation suite

Repelling-cycle approaches

Complex Misiurewicz neighborhoods and long transients

Near-critical returns

Primitive and satellite components across many periods

Nested minibrots

Multiple renormalization levels and mixed combinatorics

Parabolic bottlenecks

Cusp, root and near-root parameters

Adversarial locations

Chaotic orbits, weak recurrence and misleading near-periodicity

Ordinary locations

Exterior regions and short-iteration views where compilation should abstain

0 of 6 test families selected

Build test protocol&#x20;

The crucial tests are not just whether a feature gets labeled correctly. They must establish that the label generates a correct, profitable operator.

I would measure four independent outcomes:

| Metric                               | What it establishes                                                                     |
| ------------------------------------ | --------------------------------------------------------------------------------------- |
| Discovery recall and false positives | Does it recognize the relevant structures without hand guidance?                        |
| Numerical fidelity                   | Does it preserve escape times, smooth coloring, and required orbit accuracy?            |
| Net speedup                          | Are compilation and execution faster than an optimized baseline?                        |
| Generalization                       | Does it work on genuinely unseen periods, locations, zoom scales and nested structures? |

A reasonable initial research target would be a greater-than-2× end-to-end speedup over a strong perturbation/BLA baseline on a meaningful subset of difficult unseen views, with no unresolved incorrect pixels in the high-precision validation suite.

That is a proposed milestone, not an observed performance result.

Most importantly, failed candidate detections should not corrupt an image. A general-purpose compiler should be allowed to abstain and take the ordinary iteration path.

## 10. Fundamental limitations

There are several reasons not to expect universal compression.

Incomplete observability: one finite reference orbit cannot reveal every repelling cycle, branch transition, or nearby parameter-space structure.

Nonuniform complexity: some locations are governed by compact normal forms; others have long, sensitive itineraries without an inexpensive approximation over the relevant pixel domain.

Accuracy amplification: a small operator error near a repelling cycle can become significant after later iterations. A jump's safety cannot be judged solely by its immediate state error.

Renormalization complexity: identifying repeated scaling is easier than proving that a valid quadratic-like return map exists. Multiple possible nested structures may compete.

Parabolic sensitivity: extremely long bottlenecks are excellent acceleration targets, but small phase errors can affect subsequent escape times and detailed coloring.

Finite information: a finite orbit and precision budget can only reveal a finite portion of an infinite dynamical hierarchy.

There is also an important algebraic reason for caution: \\(f_c^N(z)\\), expanded generically as a polynomial in \\(z\\), has degree \\(2^N\\). Compact descriptions exist in terms of composition, but cheaply evaluating them at arbitrary states is a separate question.

Thus, a universal finite library of operators that compresses most iterations for every possible camera should not be assumed to exist.

That does not prevent a robust compiler from accepting arbitrary inputs and providing useful acceleration where structure permits.

## 11. Where this leaves Fractodactyl

I could not independently verify public technical documentation or source code for Fractodactyl, so I cannot claim to have benchmarked these ideas against its current manually developed acceleration zones.

However, the mathematical evidence and numerical probes do support a meaningful conclusion.

Current feasibility assessment

Automatic local cycle discovery

4.5 / 5

Automatic cycle-based acceleration

4.0 / 5

Automatic return-map construction

4.0 / 5

Nested hierarchy recognition

3.5 / 5

General minibrot renormalization

2.5 / 5

Reliable parabolic fast-forward

2.0 / 5

Universal majority-iteration elimination

1.0 / 5

Qualitative engineering feasibility estimates, not measured success probabilities.

The immediate path to something genuinely new is to build an automatic return-map discovery engine on top of perturbation/BLA, then add cycle linearization as the first new operator family.

That is the smallest implementation capable of demonstrating a qualitative advance over generic block skipping.

Only after proving end-to-end speedups would I prioritize automated polynomial-like renormalization and high-order parabolic coordinates.

## Final verdict

The critical question was whether a system could discover and accelerate structures that presently require a human researcher to recognize, including unfamiliar locations.

For finite-period recurrent and preperiodic structures, the answer is a qualified yes. The existing numerical techniques are sufficient to make automatic discovery plausible, and the prototype Misiurewicz experiment demonstrates both detection and a successful local jump.

For nested renormalizations and parabolic bottlenecks, the answer remains an open engineering and numerical-research question. The structures can be recognized in favorable cases, but automatic construction of profitable, accuracy-controlled operators is significantly harder.

For arbitrary cameras, no uniform speedup guarantee is justified. A compiler can be general-purpose in its inputs, recognition machinery, and fallback behavior without being universally successful at compression.

The proposed system is therefore a credible research direction: a dynamical just-in-time compiler whose mathematical models are discovered from orbit data, checked against the camera, reused across pixels, and replaced when they cease to be valid.

Its decisive demonstration would be a blind, end-to-end rendering benchmark in which it discovers unfamiliar cycles and return hierarchies, generates its own operators, and achieves speedups beyond BLA without hand-authored location rules. That experiment—not the existence of the individual mathematical techniques—would establish the breakthrough.