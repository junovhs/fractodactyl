I found the connected repository: [junovhs/fractodactyl](https://github.com/junovhs/fractodactyl), on the `main` branch.

The repository already contains substantial evidence directly relevant to the proposed research:

- Periods 197 and 655: `general_cycle_jump.py` verifies high-precision Koenigs jumps. Period 655 permits skipping its entire observed dwell, while period 197 remains limited by its small useful domain.
- The \\(10^{-100}\\) ladder: a period-1,582 minibrot nucleus was found using Newton refinement.
- The \\(10^{-1000}\\) ladder: a period-16,116 nucleus was found, reportedly in 12 seconds.
- Existing acceleration: Fractodactyl already implements reusable local-zone return maps and per-frame BLA, so a hierarchical method should extend these rather than replace them.

The crucial distinction is that finding the extremely deep nuclei has been demonstrated, but constant-cost full-frame rendering at those locations has not. The repository explicitly identifies rendering the new ladder rungs with the zone fast path as an outstanding test.

Sources: [General-period acceleration](https://github.com/junovhs/fractodactyl/blob/main/docs/research/10-9-26/general-period-cycle-acceleration.md), [Misiurewicz ladders](https://github.com/junovhs/fractodactyl/blob/main/docs/research/10-9-26/misiurewicz-ladders.md), and [mid-band handoff](https://github.com/junovhs/fractodactyl/blob/main/docs/research/10-9-26/HANDOFF-mid-band-jump.md).

## The most promising discovery: factor the period itself

The repository suggests a stronger method than composing increasingly large return polynomials.

For the Misiurewicz ladders, an enormous period can be expressed as a fixed preperiod, many repeats of a short repelling cycle, and a fixed exit:

\\[ P_n=q+rn+L. \\]

For Fractodactyl's ladder, \\(q=24\\), \\(r=2\\), \\(L=2\\), and \\(P_n=26+2n\\).

Let \\(g_c=f_c^r\\), let \\(a(c)\\) be the continued repelling cycle point, and let \\(K_c\\) be its forward Poincaré linearizer:

\\[ g_c(a(c)+K_c(w))=a(c)+K_c(\rho(c)w). \\]

Writing \\(\phi_c=K_c^{-1}\\), wherever the local inverse is valid,

\\[ \boxed{ f_c^{P_n}(z)= f_c^L\left( a(c)+K_c\left( \rho(c)^n\\, \phi_c(f_c^q(z)-a(c)) \right)\right) } \\]

This is an exact analytic factorization, not a degree-limited approximation.

It replaces \\(n\\) repetitions of the repelling cycle by one change of coordinates, a multiplier power, and one inverse coordinate change.

For the period-16,116 rung, approximately 8,000 two-cycle repetitions are represented by one operation. Computing \\(\rho^n\\) by exponentiation by squaring takes \\(O(\log n)\\) multiplications.

The hard part becomes evaluating the charts accurately over the necessary input and output domains, rather than storing thousands of orbit points.

This factorization is particularly attractive because Fractodactyl already implements most of the necessary machinery for its period-2 fast path.

### It may also eliminate linear-time nucleus construction

The existing `misiurewicz-ladders.md` derives the matching equation

\\[ W(c_n)=\rho(c_n)^{-n}V(c_n), \\]

where \\(W\\) describes entry into the repelling cycle and \\(V\\) describes the selected exit branch.

If \\(W,V,\rho\\) can be evaluated with bounded-cost coordinate patches, Newton refinement of a ladder nucleus need not evaluate all \\(P_n\\) iterates. This could replace the current \\(O(P_n)\\) orbit-based Newton evaluations with chart evaluations whose operation count depends polylogarithmically on \\(n\\), at growing arithmetic precision.

This is the highest-value untested hypothesis I found in the repository.

There are two separate benefits to test: using the factorization to locate a nucleus, and using it to build its parameter-dependent return operator. Success at one does not guarantee success at the other.

## 1. What constitutes genuine nesting?

Three different structures should not be conflated.

Actual nested renormalization

Successive polynomial-like restrictions contain smaller Julia sets, with return periods satisfying \\(P\_{i+1}=m_iP_i\\). This is the primary target for multilevel operator composition.

Misiurewicz ladders

Separate minibrot centers approach the same repelling-cycle landmark. Fractodactyl's periods \\(764,1582,16116\\) belong to this category. The periods increase additively, not by tuning multiplication. Their common low-period operator is reusable.

Record-return cycles

Periods 197 and 655 are observed return periods in different stretches of the Eye of the Universe orbit. They indicate near-recurrence and adjacent minibrot structure, but do not by themselves establish a polynomial-like nesting relationship.

Internal addresses and kneading sequences provide a principled combinatorial screening method. Schleicher's internal-address theory also supplies criteria distinguishing genuine renormalization from other hyperbolic-component relationships.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://armj.math.stonybrook.edu\&sz=32)

armj.math.stonybrook.edu

+1



## 2. Proposed implementation: a hierarchy of operators

I would extend `fd-kernel/src/zone.rs` with a reusable operator graph.

Proposed operator execution. A sample uses only nodes whose domains and error contracts permit the transition; otherwise it falls back to the existing perturbation/BLA machinery.

Each operator should store its coordinate charts, iteration count, parameter patch, forward value coefficients, derivative coefficients or jets, error bound, and successor conditions.

The four most useful operator types are:

- Critical-return map: high-order bivariate polynomial, preferably even in the critical coordinate.
- Repelling-cycle power: parameter-dependent \\(K_c(\rho(c)^n\phi_c(\cdot))\\).
- Chart transition: an automatic rescaling/recentering between different physical and normalized coordinates.
- Direct transfer: short perturbation/BLA block when no nonlinear chart is sufficiently accurate.

The graph should preserve compositions as a sequence of operators by default. Expanding a long composition into a single polynomial is likely to cause degree explosion and loss of numerical conditioning.

## 3. Discovering structure automatically

The repository's `auto-structural-banding-arbitrary-deep-zoom-paths.md` already recommends record-return discovery and pairing nuclei with repelling cycles. I would make this a concrete compiler pipeline.

1. Find candidate returns. Use a reference orbit with \\((z_j,\partial_c z_j)\\), multiscale recurrence hashing, and record minima near the critical point. Rank candidate period-\\(p\\) nuclei by the estimated Newton correction \\(|z_p/\partial_c z_p|\\).
2. Confirm exact structure. Newton-refine nuclei, check minimum periods, identify possible parent-child relations, and validate critical-return domains rather than assuming numerical recurrence proves renormalizability.
3. Identify reusable low-period cycles. For nearby Misiurewicz candidates, evaluate their preperiod, multiplier, chart-entry displacement, and expected usable jump length.
4. Compile selectively. Build low-order return jets first. Increase polynomial order, split the parameter patch, or construct a new chart only if the projected saving amortizes its construction and storage cost.
5. Record transitions and fallback rules. Domains must track both dynamical state and parameter; operator validity alone does not establish that a full orbit itinerary is valid.

For an actual nesting chain, the critical additional test is a degree-two polynomial-like first return \\(F:U\to V\\) with \\(U\Subset V\\), appropriate critical behavior, and a meaningful modulus separating the boundaries.

The straightening theorem supplies a hybrid conjugacy to a quadratic polynomial, not generally a globally holomorphic coordinate change that preserves external escape dynamics or derivatives. This is a major reason to retain explicit transitions and a fallback. Uniformly well-conditioned charts are not available for every infinite nesting: even rigorous complex-bounds results distinguish favorable bounded-combinatorics classes from problematic satellite geometries.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://link.springer.com\&sz=32)

Springer Nature Link

+1



## 4. Stable composition and derivative propagation

The central numerical choice is to normalize both variables independently:

\\[ \xi=\frac{z-a(c)}{s_z(c)},\qquad \eta=\frac{c-C}{s_c}. \\]

Store a local operator as

\\[ R(\xi,\eta)=\sum\_{i=0}^{d_z}\sum\_{j=0}^{d_c} a\_{ij}\xi^i\eta^j. \\]

In normalized coordinates, coefficient shells can be compared directly; tiny physical widths such as \\(10^{-1000}\\) do not turn into thousands of underflowing powers in ordinary floating point.

For a composed return, propagate state and parameter sensitivities:

\\[ D\_{\mathrm{out}}=R\_\xi D\_{\mathrm{in}}+R_c, \\]

where \\(D\\) is the derivative in the relevant normalized coordinates. If the chart center or scale moves with \\(c\\), their derivatives must also be included in the chart transformations.

For a repelling-cycle power, the potentially enormous factor is best differentiated analytically:

\\[ \frac{d}{dc}\rho(c)^n =n\rho(c)^n\frac{\rho'(c)}{\rho(c)}. \\]

It is unnecessary to propagate a derivative through \\(rn\\) explicit iterations.

Error propagation should be through a local bound, for example

\\[ e\_{i+1}\le L_i e_i+\epsilon_i, \\]

with \\(L_i\\) a bound on the derivative of the next operator, and \\(\epsilon_i\\) its approximation and rounding error. The bound must account for parameter uncertainty as well as state uncertainty.

Use complex ball arithmetic or Taylor models to validate the operators offline, then inexpensive inclusion tests at runtime. If errors or domains become unacceptable, split a patch or fall back. A Koenigs inverse becomes especially sensitive near critical points and chart boundaries; renormalization alone does not remove that conditioning problem.

## 5. Complexity: period versus depth

Let \\(P\\) be the original iteration period, \\(h\\) the number of genuine nesting levels, \\(M\approx(d_z+1)(d_c+1)\\) the number of coefficients in a rectangular bivariate patch, \\(k_i\\) the number of returns evaluated at level \\(i\\), and \\(T\\) the number of ordinary residual steps.

| Method                                     | Construction operations                                    | Stored operators                  | Per-pixel operations                                    |
| ------------------------------------------ | ---------------------------------------------------------- | --------------------------------- | ------------------------------------------------------- |
| Direct iteration                           | Minimal                                                    | \\(O(1)\\)                        | \\(O(P)\\) per full return                              |
| Single bivariate return map                | \\(O(PM^2)\\) naive jets                                   | \\(O(M)\\)                        | \\(O(kM+T)\\)                                           |
| Generic nested graph, built from raw orbit | Up to \\(O(PM^2)\\)                                        | \\(O(hM)\\), plus charts          | \\(O(M\sum k_i+T)\\)                                    |
| Nested graph with reusable child operators | Potentially \\(O(\sum m_i M^2+hC\_{\rm comp})\\)           | \\(O(hM)\\)                       | \\(O(M\sum k_i+T)\\)                                    |
| Exact short-cycle factorization            | \\(O(rM^2)\\) base chart, plus matching/patch construction | Base chart plus parameter patches | Chart evaluations plus \\(O(\log n)\\) for \\(\rho^n\\) |

These are arithmetic-operation models, not wall-clock predictions. The optimistic row for reusable child operators requires valid cross-level coordinate changes; otherwise building the child from the parent still involves substantial work. Dense series composition has its own \\(C\_{\rm comp}\\) cost.

If \\(m_i\\) is bounded and \\(P=\prod_i m_i\\), then \\(h=O(\log P)\\). With bounded per-level returns and polynomial degree, online cost can be \\(O(\log P)\\).

For Misiurewicz ladders, the stronger short-cycle factorization might give an operator whose evaluation count is almost independent of \\(P_n\\). Numerical precision and patch complexity still increase with depth, and the number of minibrot returns need not remain constant.

### An essential precision floor

At width \\(10^{-1000}\\), resolving pixel offsets requires approximately 1,000 decimal digits, or 3,322 bits, before safety margins.

The repository explicitly records that its current 512-bit fixed-point coordinate handling and `f64` film widths cannot support this case correctly. Those are blockers independently of return-map mathematics.

The right architecture stores the center at arbitrary precision while doing normalized pixel arithmetic in double or scaled-double where verified possible. It also retains exponents separately, rather than representing the physical width as an `f64`.

## 6. Critical test results and gaps

| Location              | Existing evidence                                                                                               | Remaining decisive test                                                         |
| --------------------- | --------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| Period 197            | 2 of 5.7 laps skippable with Koenigs; accuracy about \\(2\times10^{-55}\\) inside its usable domain             | Quadratic-like return map handling the outer laps, evaluated across real pixels |
| Period 655            | Observed dwell fully skippable with 32 terms; accuracy about \\(3\times10^{-71}\\)                              | Whole-frame cost and coverage with derivative outputs                           |
| \\(10^{-100}\\) rung  | Period 1,582; nucleus found in 0.7 s; roughly five original-period returns per sampled pixel                    | Build normalized zone and measure cold/warm rendering                           |
| \\(10^{-1000}\\) rung | Period 16,116; nucleus found in 12 s; 8.23–8.56 original-period equivalents before escape on six sampled pixels | Eliminate raw-period operator construction and render a fully validated frame   |

Existing measurements are from the repository's October 9 research scripts. They are not new runs of a multilevel renderer.

The ladder bridge measurement is especially informative. The mean return count increased by 0.96 between v0 and the \\(10^{-100}\\) rung, then by another 4.32 to the \\(10^{-1000}\\) rung. This closely follows \\(\log_2 n\\), as predicted by the repository's bridge analysis.

At the deepest rung, approximately eight return-map evaluations would stand in for more than 130,000 direct quadratic iterations for the sampled exterior pixels. That is an enormous potential reduction in online work, but it remains a projected implementation benefit rather than a measured renderer speedup.

## 7. The most valuable next experiment

I would not start by implementing a completely general renormalization framework. First, create a factored ladder-return prototype that uses the period-2 analytic chart already available in Fractodactyl.

Compare three versions of \\(f_c^{P_n}\\) at v0, rung 409, and rung 7676: direct multiprecision orbit iteration; the current bivariate-return construction; and the \\(q\\)-step / \\(\rho^n\\) / \\(L\\)-step factorization above.

The key benchmark must separately record:

| Measurement                                   | What it establishes                                       |
| --------------------------------------------- | --------------------------------------------------------- |
| Operator build time versus \\(P_n\\)          | Whether cold construction is genuinely compressed         |
| Operator bytes versus \\(P_n\\)               | Whether a high-period operator is structurally small      |
| Certified or validated state/parameter domain | Whether the operator is actually usable                   |
| Derivative error after composition            | Whether DE and normal shading survive                     |
| Evaluations per exterior pixel                | Whether pixel cost follows returns rather than raw period |
| Total warm-frame and cold-frame time          | Whether reuse outweighs construction                      |
| Fallback rate, especially near boundary       | Whether good results cover genuine frames                 |

Then run the whole-frame gate required by Fractodactyl's accepted DEC-17: every pixel, with class, smooth escape value, distance estimate, and normal checked. Correct the comparison path's nonfinite-value handling and width-underflow problem before trusting deep-frame pass counts.

The period-197 case should be included as an intentionally hostile control. A method that only works on the easy period-655 inner dwell has not solved general-period nesting.

## Conclusion

There is a plausible route to logarithmic or near-period-independent online cost on structured deep zooms. Fractodactyl already has the essential components: return maps, analytic cycle charts, parameter derivatives, and a path-aware reuse architecture.

The strongest finding from the connected repository is that an enormous original period can sometimes be represented not by a huge polynomial at all, but by a short-cycle operator raised symbolically to a large power. That deserves priority over attempting to straightening-map every nested minibrot into another generic quadratic.

The limitation is universality. Genuine polynomial-like nesting, repelling-cycle ladders, and arbitrary recurrent orbits are different structures and will need different operators. There is no current evidence that a compact hierarchy eliminates long histories at every location.

For Fractodactyl, the decisive next proof is concrete: construct the period-16,116 ladder return from its period-2 chart, without iterating 16,116 steps during each operator build, and then demonstrate correct whole-frame rendering. If that succeeds, the original period stops being the dominant online cost on this entire ladder family.

The most directly relevant repository materials are [zone.rs](https://github.com/junovhs/fractodactyl/blob/main/crates/fd-kernel/src/zone.rs), [ladder.py](https://github.com/junovhs/fractodactyl/blob/main/tools/research/misiurewicz/ladder.py), [general_cycle_jump.py](https://github.com/junovhs/fractodactyl/blob/main/tools/research/misiurewicz/general_cycle_jump.py), [the ladder bridge analysis](https://github.com/junovhs/fractodactyl/blob/main/docs/research/10-9-26/universal-landing-claims-a-to-e.md), and [the current project state](https://github.com/junovhs/fractodactyl/blob/main/docs/spec/STATE.md).