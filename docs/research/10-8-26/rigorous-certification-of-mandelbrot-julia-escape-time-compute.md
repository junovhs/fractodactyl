# Rigorous Certification of Mandelbrot and Julia Escape-Time Computations

## Executive assessment

There is a fairly sharp divide between what is already rigorous in computational complex dynamics and what high-performance deep-zoom renderers currently do.

**Proven and implemented:** whole-pixel/whole-cell certification of polynomial Julia sets using interval arithmetic and finite cell graphs; rigorous lower/upper area bounds derived from those cells; rigorous certification of enormous collections of Mandelbrot hyperbolic centers and Misiurewicz parameters using carefully designed complex-disk arithmetic; and general-purpose ball-arithmetic infrastructure capable of carrying rigorous complex power series and polynomial computations. The strongest published computations can be surprisingly tight: certified Mandelbrot root locations have radii of \(10^{-30}\)–\(10^{-35}\), and specialized disk arithmetic can accumulate almost no enclosure inflation over very long compositions when the starting disk is tiny. citeturn17view0turn19view6turn19view9

**Established rendering practice but not certified:** perturbation, reference rebasing, series approximation, BLA (“bivariate linear approximation”), and NanoMB-style minibrot approximations. The underlying perturbation identity is exact in exact arithmetic, but practical validity tests are engineering criteria: floating-point glitch thresholds, comparisons at probe points, hardware-\(\epsilon\)-scaled BLA radii, and fallbacks to new reference orbits. One of the main developers of these techniques explicitly states that there is still no complete proof with rigorous error bounds for series-approximation skipping; the BLA derivation itself describes its validity-radius step as involving “some handwaving.” citeturn18view4turn19view0turn19view3

**The mathematical gap is smaller than the software gap.** Cauchy bounds, coefficient majorants, interval/ball power series, polynomial-plus-remainder enclosures, validated disk arithmetic, and Newton/radii-polynomial arguments give all the ingredients needed to certify a perturbation/BLA/series skip. What is largely missing is an implementation that retains the aggressive performance characteristics of current renderers while propagating those bounds end-to-end. ACETAF experiments also show why this is nontrivial: naive Cauchy bounds can be many orders of magnitude too pessimistic, while sharper derivative- or polynomial-subtracted bounds can recover orders of magnitude at significant computational cost. citeturn17view4turn17view5turn17view6turn21search5

For repelling periodic points, the linearisation problem is especially favorable. For \(F=f_c^k\) and a repelling periodic point \(p\) with multiplier \(|\lambda|>1\), the normalized Poincaré linearizer \(L\) satisfying

\[
F(L(\zeta))=L(\lambda\zeta),\qquad L(0)=p,\qquad L'(0)=1
\]

exists; for a polynomial \(F\), it is entire. Its coefficient recursion has divisors \(\lambda^n-\lambda\), so there is **no small-divisor problem**—the denominators grow exponentially. The difficult part is not producing coefficients but certifying useful domains of injectivity and correspondingly sharp convergence radii and remainder bounds for the local inverse Schröder coordinate. I did not find a published quadratic-family package or paper that benchmarks such validated Koenigs/Poincaré computations the way the certified-Julia or tera-polynomial work does. citeturn22view0turn23view0

## Certified Mandelbrot and Julia computations that actually exist

### Whole-cell Julia images

The cleanest rigorous renderer I found is de Figueiredo, Nehab, Stolfi, and de Oliveira, *Rigorous Bounds for Polynomial Julia Sets*. Their method deliberately does **not** try to make ordinary escape-time iteration reliable point by point. Instead it covers the plane by rectangular cells, computes interval enclosures for the image of each complete cell, builds a directed cell graph, and reasons about all possible orbits by graph reachability and strongly connected components. citeturn15view0turn17view0

The resulting image has a literal certificate attached to every pixel/cell:

| Color/status | Certified statement |
|---|---|
| white | the entire cell is outside the filled Julia set |
| black | the entire cell is inside the filled Julia set |
| gray | unresolved; the union of gray cells is guaranteed to contain the Julia set |

That is much stronger than sampling a pixel center. It simultaneously addresses spatial sampling, arbitrary iteration cutoffs, and roundoff. Their quadratic implementation can even use exact dyadic fixed-point arithmetic, and the authors state that ordinary double precision is sufficient for guaranteed images as large as \(10^6\times10^6\) pixels because the arithmetic precision requirement is tied to spatial resolution rather than to blindly iterating a rounded point orbit. citeturn15view0

This illustrates an important distinction for “certified escape time.” For an individual point or ball, **escape is semidecidable in a very simple way**: once a rigorous enclosure of an iterate is wholly beyond a valid escape radius, exterior membership is proved. But “it did not escape during \(N\) iterations” is never by itself an interior certificate. The cell-map method obtains interior certificates by proving that complete sets of orbits become trapped in invariant graph components rather than by imposing an iteration limit. citeturn15view0

The price is conservatism near the boundary. A cell graph is an outer approximation: an edge is inserted whenever the interval image of one cell intersects another, so spurious transitions accumulate. The authors explicitly describe the cell graph as conservative and note that computations get harder when \(c\) lies near the Mandelbrot boundary. citeturn15view0turn16view0

### How expensive and how tight were those Julia certificates?

The published prototype was written in Lua, so it is not meaningful to quote its timing as an intrinsic “interval arithmetic slowdown.” Across their tests it took roughly **10 seconds to 5 minutes of CPU time** and **175 MB to 12 GB of memory**, with about 80% of time spent constructing the graph. They did not publish an apples-to-apples optimized escape-time-renderer baseline; therefore a defensible fixed “\(x\) times slower” number does not exist from this work. citeturn16view0turn17view1

Geometrically, however, the enclosures can become quite tight. At quadtree level 19, their lower area bound—the area of the black cells—and upper area bound—the area of black plus gray cells—for quadratic filled Julia sets along \(-1.25\le c\le0.25\) are described as visually almost indistinguishable in their plot. They also report those bounds as substantially tighter in parts of the interval than a 100,000-term Gronwall/Milnor series upper bound. citeturn16view4turn17view2turn17view3

That should not be generalized to every Julia set: near parabolic, critical, or otherwise difficult boundary dynamics, the unresolved gray set is exactly where a rigorous method is expected to become conservative. The method's strength is that conservatism produces an explicit “unknown,” not a wrong pixel. citeturn15view0

### A rigorous Mandelbrot adaptation

Claude Heiland-Allen implemented a “Trustworthy Mandelbrot” adaptation of this idea. For a parameter cell \(C\), it carries out a certified Julia calculation over the parameter interval and asks whether the critical point \(0\) is certainly interior or certainly exterior. The Mandelbrot pixel is then certified correspondingly; otherwise it remains unknown. citeturn18view0turn24view0

The reported level-11 computation took approximately **five wall-clock days**, mostly using 16 threads of a Ryzen 7 2700X with 32 GB RAM. Its rigorous global area interval is

\[
1.416492462158203125
\le \operatorname{area}(\mathcal M)
\le
1.84781646728515625.
\]

The width is \(0.431324005126953125\), about **30.45% of the lower bound**. Thus this particular global Mandelbrot area certificate is mathematically strong but geometrically rather coarse compared with what one would call a high-precision area estimate. citeturn18view1turn24view0

That contrast is instructive: pixel certification can be extremely sharp for a particular hyperbolic Julia set, yet a global Mandelbrot certificate inherits the difficulty of resolving an enormous amount of parameter-space boundary.

### Certified hyperbolic centers and Misiurewicz parameters

The most impressive recent example of rigorous Mandelbrot computation is of a different kind. Mihalache and Vigneron certified all hyperbolic centers of periods up to 41 and all of their chosen Misiurewicz–Thurston families with preperiod plus period at most 35. Their largest defining polynomial has degree about \(1.1\times10^{12}\); overall they sifted through roughly \(3.3\times10^{12}\) roots and identified more than \(2.7\times10^{12}\) distinct parameters. citeturn19view7turn19view8

Here “certified hyperbolic” needs precise wording: a center is a parameter for which the critical point is periodic and the corresponding cycle is superattracting, so each listed center is rigorously tied to an exact root of the relevant polynomial. This is **not** a proof of the Mandelbrot Hyperbolicity Conjecture or of density of hyperbolicity. citeturn19view8turn19view9

Each published root is accompanied by a proof that an exact root lies within a specified disk and that Newton refinement converges to it. They certify errors no larger than \(10^{-30}\) for the hyperbolic-center polynomials and \(10^{-35}\) for the Misiurewicz polynomials. citeturn19view6turn19view9

This is also one of the clearest empirical answers to “how expensive is rigor?” Their aggregate figures were:

| Work | Raw root search | Rigorous proof phase | Proof fraction of total |
|---|---:|---:|---:|
| hyperbolic centers through period 41 | 229,000 core-hours | 96,400 core-hours | 29.6% |
| Misiurewicz families through total order 35 | 358,000 core-hours | 38,200 core-hours | 10.6% |

The authors report raw-search/certification ratios of 2.4 and 9.4 respectively. So in this highly specialized computation, rigorous certification was substantial but nowhere near an orders-of-magnitude penalty. citeturn20view1turn20view2

Even more strikingly, the independently certified high-precision lists agreed extremely closely with the fast hardware-FP80 search: up through period 33, the maximum reported discrepancy in corresponding hyperbolic-center coordinates was \(5.24\times10^{-19}\); for the lower-order Misiurewicz comparison it was \(3.25\times10^{-19}\). In this application the ordinary computation was empirically very accurate, while the validated pass supplied the proof. citeturn20view0

### Why complex disks can beat ordinary rectangular intervals

Mihalache and Vigneron's key numerical point is directly relevant to fractal iteration. Rectangular complex intervals tend to suffer badly from dependency and orientation effects under repeated conformal maps. Their complex **disk arithmetic** instead tracks a center and radius and exploits the fact that a sufficiently small disk is mapped by a holomorphic function to something very close to a disk determined by the derivative. citeturn19view6

They estimate the excess multiplicative inflation, relative to unavoidable first-order derivative growth, as roughly

\[
1+\frac{r}{|f'(z)|}
\]

per step for a disk of radius \(r\). As a simple scale example, with \(r=10^{-20}\), \(10^5\) iterations, and derivatives bounded below by 1, they estimate cumulative artificial inflation around

\[
(1+10^{-20})^{100000}\approx1+10^{-15}.
\]

That is extraordinarily tight. They also report that close transits near zero can make their certification essentially impossible with conventional interval arithmetic while disk arithmetic remains usable. citeturn19view6turn20view1

This is probably the most relevant published precedent for a future rigorous deep-zoom Mandelbrot engine: **do not equate rigor with naively iterating axis-aligned complex boxes**.

### Where Arb/FLINT fits

Johansson's Arb provides midpoint-radius real balls, Cartesian complex balls (`acb`), and rigorous real/complex polynomial and power-series arithmetic. Its design target is narrow enclosures and automatic tracking of numerical error. At high precision, Johansson argues that midpoint-radius storage can have asymptotic cost close to a single high-precision floating value, rather than the roughly two-endpoint cost of conventional interval storage. citeturn24view1

Arb also explicitly supports complex polynomial/power-series calculations and adaptive precision: a standard pattern is to run at a chosen precision, inspect the resulting ball accuracy, and repeat with more guard bits if necessary. citeturn18view8turn24view1

What I did **not** find in the primary literature is a widely used Mandelbrot/Julia renderer whose published certification story is specifically “we render deep zooms with Arb/acb.” The prominent rigorous fractal examples above instead use custom cell intervals, exact dyadic arithmetic, MPFR-backed complex disks, or related validated machinery. Arb nevertheless supplies almost exactly the low-level primitives one would want for validated reference orbits, power-series coefficients, Koenigs functions, and a slow fallback/certificate path. citeturn24view1turn19view6

## Deep-zoom acceleration: what is exact and what is heuristic

### Perturbation itself is an exact algebraic identity

For the Mandelbrot recurrence

\[
Z_{n+1}=Z_n^2+C,
\]

take a nearby parameter \(C+c\) and write its orbit as \(Z_n+z_n\). Then exactly,

\[
z_{n+1}=2Z_nz_n+z_n^2+c.
\]

Nothing approximate has happened yet. The main deep-zoom win is that \(Z_n\) can be computed once at high precision while \(z_n\) is computed cheaply for millions of nearby pixels. citeturn18view4

Likewise, “rebasing” is fundamentally a change of representation. In current Fraktaler-style documentation, when

\[
|Z_m+z_n|<|z_n|,
\]

the renderer replaces the delta with \(Z_m+z_n\) and restarts at the beginning of a reference orbit. This is intended to keep the perturbation small and avoid glitches. The algebraic recentering can be exact; **the inequality is not itself an error certificate for the finite-precision values used by the program**. citeturn18view4

A rigorous implementation would therefore separate two questions that ordinary renderers combine:

1. Is the coordinate transformation/rebase algebraically valid?
2. Does the computed floating value provably enclose the true transformed delta?

The first is easy. The second requires balls, directed rounding, or an explicit forward-error recurrence.

### A useful exact formulation when the reference orbit is numerical

There is a particularly clean route to validating perturbation. Let \(\widetilde Z_n\) be the *computed* reference rather than pretending it is exact, and define the reference defect

\[
r_n=\widetilde Z_n^2+C-\widetilde Z_{n+1}.
\]

For the true pixel orbit \(x_n=\widetilde Z_n+\delta_n\), the exact relation becomes

\[
\boxed{
\delta_{n+1}
=
2\widetilde Z_n\delta_n+\delta_n^2+c+r_n.
}
\]

If \(C,\widetilde Z_n,r_n,\delta_n,c\) are enclosed in complex balls, this recurrence automatically incorporates both reference-orbit error and pixel arithmetic error. This is a synthesis of the exact perturbation identity with standard ball arithmetic; I did not find it deployed as an end-to-end certificate in a mainstream deep-zoom renderer. The relevant arithmetic infrastructure and long-iteration precedent do exist. citeturn18view4turn24view1turn19view6

This formulation also makes certified rebasing nearly trivial: replace one enclosing ball representation by another enclosing ball representation and prove containment. No separate philosophical notion of a “glitch” is needed for correctness; glitch detection becomes only a performance heuristic telling the renderer when the current enclosure is getting inefficient.

### What current glitch detection actually tests

Practical perturbation is known to fail when nearby pixel orbits become indistinguishable in the low-precision delta representation. The commonly documented Pauldelbrot-style condition monitors a quantity of the form

\[
|Z+z|^2 < G|Z|^2,
\]

where \(G\) is an empirical threshold; the renderer can then recompute using a better reference. Heiland-Allen explicitly notes that the choice of \(G\) remains an open issue. citeturn19view2turn19view0

That is qualitatively different from a validated inequality such as

\[
\operatorname{dist}(B_n,\{|z|=R_{\rm esc}\})
>
\text{rigorous error radius},
\]

which would prove an escape decision insensitive to all accumulated error. Current glitch tests try to predict numerical trouble; they do not generally enclose the true orbit.

### Series approximation and NanoMB

A common acceleration expands the perturbation as a polynomial in the pixel offset,

\[
z_n(c)=\sum_{k\ge1}A_{n,k}c^k,
\]

using the coefficient recurrence obtained by substituting the series into

\[
z_{n+1}=2Z_nz_n+z_n^2+c.
\]

A low-degree truncation then jumps many initial iterations. citeturn19view1

Documented practice has traditionally selected the jump length empirically: evaluate the truncated series and ordinary perturbed iteration at a collection of probe points, stop when their discrepancy becomes too large, then back up an iteration. That gives strong practical evidence but no supremum-norm bound over every point in the pixel disk. citeturn19view1

NanoMB-style schemes extend this idea to a bivariate series near a periodic reference/minibrot center. A biseries in state displacement and parameter displacement can skip one complete period, and repeated applications skip many periods; the documented “NanoMB1” scheme uses an escape/validity radius around the periodic reference. Chaining approximations associated with successively nested minibrots was documented as “NanoMB2”; the published implementation notes describe that variant as experimental and capable of failing at some locations. citeturn19view1

Most importantly for your question, the same practitioner literature states explicitly that robust series-skipping lacked a complete mathematical proof with rigorous error bounds. citeturn19view0

### BLA validity is an error-budget heuristic, not an enclosure

BLA drops the nonlinear term from one or more perturbation steps and represents a skip by

\[
z_{n+\ell}\approx A_{n,\ell}z_n+B_{n,\ell}c.
\]

The current documentation says the approximation should be used when the omitted nonlinear contribution is small enough that dropping it causes less trouble than ordinary low-precision rounding. For a one-step approximation it begins from

\[
|z_n^2|\ll |2Z_nz_n+c|
\]

and reduces this to a radius condition on \(|z_n|\). citeturn18view4

The newer engineering formula scales the allowable radius by the hardware precision \(\epsilon\); its derivation is explicitly described as using “some handwaving,” and an extra factor of two is used in practice “to be extra safe.” citeturn19view3turn19view4

Multi-step BLAs are composed algebraically. If

\[
T_x(z)=A_xz+B_xc,\qquad
T_y(z)=A_yz+B_yc,
\]

then

\[
A_z=A_yA_x,\qquad
B_z=A_yB_x+B_y,
\]

with a recursively propagated validity radius such as

\[
R_z=
\max\!\left(
0,
\min\!\left(
R_x,
\frac{R_y-|B_x||c|}{|A_x|}
\right)
\right).
\]

At render time the implementation chooses the longest BLA whose stored radius contains the current \(z\); otherwise it performs an ordinary perturbation step. citeturn18view5turn19view5

That radius is a **practical admissibility radius**, not a theorem saying “the true skipped orbit lies within \(10^{-q}\) of the BLA result.” A rigorous BLA would carry one additional quantity: a validated nonlinear remainder.

For example, instead of storing merely

\[
z_{\rm out}\approx Az+Bc,
\]

store

\[
\boxed{
z_{\rm out}\in Az+Bc+\mathbb D(0,E)
}
\]

for all \(|z|\le R_z,\ |c|\le R_c\). The merge operation would propagate both \(A,B\) and \(E\), with interval bounds for the effect of each remainder through the next block. This is precisely the sort of polynomial-plus-remainder calculation for which Taylor models, majorant series, or ball arithmetic are designed. The missing part is a production implementation whose \(E\) remains tight enough to preserve the enormous BLA speed advantage. citeturn17view4turn24view1

## Rigorous bounds for truncated iterated-map series

### Cauchy bounds: simple, universal, often pessimistic

Suppose

\[
f(z)=\sum_{k=0}^\infty a_kz^k
\]

is analytic on \(|z|\le R\), and

\[
M_R=\max_{|z|=R}|f(z)|.
\]

Cauchy's estimate gives

\[
|a_k|\le \frac{M_R}{R^k}.
\]

Therefore, for \(|z|\le\rho<R\),

\[
\boxed{
\left|
f(z)-\sum_{k=0}^{N}a_kz^k
\right|
\le
M_R
\frac{(\rho/R)^{N+1}}
{1-\rho/R}.
}
\]

This is completely rigorous once \(M_R\) itself has a rigorous upper bound. ACETAF implements exactly this style of validated contour bounding with interval arithmetic. citeturn16view7turn17view4

The weakness is easy to see. If

\[
f(z)=A+\varepsilon z^{N+1},
\qquad |A|\gg|\varepsilon|R^{N+1},
\]

then \(M_R\approx |A|\), even though the actual Taylor tail after degree \(N\) is only \(|\varepsilon z^{N+1}|\). The ratio between the Cauchy bound and true remainder can therefore be made arbitrarily large. There is no universal “Cauchy is within a factor of \(K\)” result.

For long iterates of a quadratic map this problem can be severe. The iterate \(f_c^n\) is an entire polynomial, so analyticity itself is never the limitation, but its maximum modulus on a moderately larger contour can become enormous. A generic \(M_R\) bound may then tell you almost nothing about the small local disk relevant to a deep zoom.

### Subtract what you already know before applying Cauchy

A major practical improvement is to apply Cauchy not to \(f\) but to its residual after subtracting a computed Taylor polynomial \(T_\ell\):

\[
N(R,\ell)
=
\max_{|z|=R}|f(z)-T_\ell(z)|.
\]

Then for coefficients beyond \(\ell\),

\[
|a_j|\le \frac{N(R,\ell)}{R^j}.
\]

ACETAF's authors explicitly report that this residual norm can be **several powers of ten smaller** than the original maximum-modulus bound, and that the corresponding improvement carries directly to remainder estimates. citeturn16view7turn17view5

Their \(e^z\) examples make the tradeoff concrete. At radius \(R=10\), the raw Cauchy constant is about \(2.7\times10^4\), while subtracting an optimized degree-28 Taylor polynomial reduces the relevant constant to about \(7.5\times10^{-1}\). Derivative-enhanced estimates reduce high-order coefficient bounds still further: the reported \(a_{100}\) bound can move from roughly \(10^{-96}\) under the simplest treatment to roughly \(10^{-140}\)–\(10^{-144}\) with the stronger methods. citeturn17view6

The cost can be substantial: on the authors' historical test machine, simple bounds took a few seconds or less while some combined high-order methods took hundreds of seconds. The precise timings are obsolete, but the structural lesson is not: **tight validated tails usually require more than plugging a large \(M_R\) into Cauchy.** citeturn17view5turn17view6

### Derivative Cauchy bounds

ACETAF also applies Cauchy estimates to \(f^{(m)}\). Because

\[
f^{(m)}(z)
=
\sum_{j\ge m}
\frac{j!}{(j-m)!}a_jz^{j-m},
\]

a rigorous bound

\[
U(R,m)=\max_{|z|=R}|f^{(m)}(z)|
\]

yields an extra factorial/product suppression in the coefficient bounds. The authors note that the resulting remainder majorant converges faster than a plain geometric series. citeturn16view7turn16view8

For fractal series this is attractive because derivatives of iterates can be propagated alongside the orbit. But one should not assume derivatives automatically help: near critical or nearly singular inverse geometry, derivative ranges themselves can be expensive to enclose tightly.

### Direct majorant propagation is often better for quadratic iteration

For Mandelbrot perturbation, there is enough algebraic structure to avoid a black-box Cauchy bound.

Suppose a degree-\(d\) polynomial \(P_n(c)\) approximates the perturbation \(z_n(c)\) for every \(|c|\le\rho\), with certified error

\[
z_n(c)=P_n(c)+e_n(c),
\qquad
|e_n(c)|\le E_n.
\]

Substitute into the exact recurrence:

\[
z_{n+1}
=
2Z_n(P_n+e_n)+(P_n+e_n)^2+c.
\]

Let

\[
P_{n+1}
=
\operatorname{trunc}_d
\left(2Z_nP_n+P_n^2+c\right)
\]

and let \(T_n\) rigorously bound the discarded high-degree polynomial terms on \(|c|\le\rho\). Then

\[
e_{n+1}
=
(2Z_n+2P_n)e_n+e_n^2+\tau_n,
\]

where \(|\tau_n|\le T_n\). Consequently,

\[
\boxed{
E_{n+1}
\le
\bigl(2|Z_n|+2\,\|P_n\|_\rho\bigr)E_n
+
E_n^2
+
T_n.
}
\]

Replacing the scalar quantities by outward-rounded ball bounds gives a rigorous recurrence.

This has several advantages over a generic Cauchy estimate. It uses the actual orbit \(Z_n\), it isolates precisely the terms thrown away by truncation, and it can exploit a tiny parameter disk. Its drawback is familiar from interval numerics: replacing a complex function by separate absolute values loses cancellation. Disk arithmetic and polynomial remainder representations can reduce that loss substantially, just as the tera-polynomial computation avoids the cumulative inflation of rectangular intervals. citeturn18view4turn19view6

### Taylor-model style propagation

The most natural validated object for a deep zoom is therefore not merely a ball

\[
z_n(c)\in B_n
\]

but a **polynomial plus a remainder ball**

\[
z_n(c)
\in
P_n(c)+\mathbb D(0,E_n),
\qquad |c|\le\rho.
\]

The polynomial retains dependency on \(c\); only the unresolved tail is intervalized. This is the core idea behind Taylor-model and interval-polynomial methods. ACETAF demonstrates the same basic principle for analytic functions—keep an explicit Taylor approximation and rigorously bound only the residual—and Arb provides rigorous ball-coefficient polynomial arithmetic suitable for implementing it. citeturn17view4turn17view5turn24view1

For a BLA, use degree one. For ordinary series approximation, use degree perhaps 4–32. For NanoMB, use a bivariate polynomial in state and parameter. A renderer can then choose the cheapest representation whose certified remainder remains below a user-defined accuracy budget.

The important change from current practice is the stopping criterion. Instead of

\[
\text{“omitted term} < \epsilon_{\rm hardware}\times\text{scale,”}
\]

use

\[
\boxed{
\text{certified propagated remainder}
<
\text{available classification margin}.
}
\]

For escape-time coloring, for example, an iterate whose entire enclosing ball lies outside the escape circle is certified escaped. If the ball overlaps the escape boundary, refine the approximation, increase precision, shorten the skip, or fall back to direct validated iteration.

### A posteriori residual certificates

A second approach is to compute aggressively first and validate afterward. Suppose \(P\) is a numerically generated polynomial approximation to some functional relation \(F(P)=0\). Instead of tracking every rounding error that created \(P\), evaluate the **residual** \(F(P)\) rigorously, bound an approximate inverse to the linearized operator, and use a Newton–Kantorovich or radii-polynomial theorem to prove there is a true solution near \(P\).

The radii-polynomial method formalizes exactly this structure: finite calculations supply bounds on the residual, approximate inverse, derivative error, and nonlinear variation; a scalar polynomial inequality then certifies existence and uniqueness of an exact infinite-dimensional coefficient solution in a known norm ball. A 2026 Lean formalization goes further by reducing coefficient tails to finite rational inequalities and machine-checking them. citeturn21search5

For a renderer, this technique is probably more naturally suited to reusable objects—periodic-point expansions, Koenigs functions, minibrot models, long BLA blocks—than to every single pixel.

## Rigorous Koenigs, Schröder, and Poincaré linearisations

### The convention matters

Let

\[
f_c(z)=z^2+c
\]

and let \(p\) be a repelling point of exact period \(k\). Define

\[
F=f_c^k,\qquad F(p)=p,\qquad
\lambda=F'(p),\qquad |\lambda|>1.
\]

There are two inverse ways of writing the linearisation.

The **Poincaré linearizer**

\[
L(0)=p,\qquad L'(0)=1
\]

satisfies

\[
\boxed{
F(L(\zeta))=L(\lambda\zeta).
}
\]

The local **Schröder/Koenigs coordinate**

\[
h(p)=0,\qquad h'(p)=1
\]

satisfies

\[
\boxed{
h(F(z))=\lambda h(z).
}
\]

Locally,

\[
h=L^{-1}.
\]

For rational maps, normalized linearizers at repelling fixed points are standard; replacing the map by its \(k\)-th iterate gives the periodic-point version. Fletcher and MacClure state this formulation explicitly. citeturn22view0turn23view0

### For a polynomial, the forward Poincaré series has infinite radius

A useful fact is sometimes obscured by the phrase “radius of convergence of the Koenigs series.” For a polynomial \(F\), the normalized **Poincaré linearizer \(L\)** at a repelling fixed point extends to an entire function. Thus its Taylor series around \(0\) has radius

\[
\boxed{R_L=\infty.}
\]

The interesting finite radius is normally that of the **local inverse** \(h=L^{-1}\), or of a chosen univalent branch of \(L\). Fletcher and MacClure formulate the global linearizer for repelling rational-map points and note that poles of the linearizer arise when the rational map has finite poles; a polynomial has no such finite poles. citeturn22view0

So for quadratic polynomials, “certifying the radius” divides into two very different problems:

\[
L:\quad \text{entire, radius infinite;}
\]

\[
h=L^{-1}:\quad
\text{finite local inverse radius determined by loss of invertibility/univalence.}
\]

### Coefficients are unusually easy to validate at a repeller

Write local coordinates

\[
G(w)=F(p+w)-p
=
\lambda w+\sum_{m=2}^{D}g_mw^m,
\qquad D=2^k,
\]

and

\[
L(\zeta)-p
=
\zeta+\sum_{n=2}^{\infty}a_n\zeta^n.
\]

Substitution into

\[
G(L(\zeta)-p)=L(\lambda\zeta)-p
\]

gives, at order \(n\),

\[
(\lambda^n-\lambda)a_n
=
[\zeta^n]
\sum_{m=2}^{D}
g_m
\left(
\zeta+\sum_{j=2}^{n-1}a_j\zeta^j
\right)^m.
\]

Hence

\[
\boxed{
a_n=
\frac{\text{known polynomial in }a_2,\ldots,a_{n-1}}
{\lambda^n-\lambda}.
}
\]

Because \(|\lambda|>1\),

\[
|\lambda^n-\lambda|
\ge
|\lambda|\bigl(|\lambda|^{n-1}-1\bigr).
\]

So the divisors move **away from zero exponentially fast**. This is the opposite of the small-divisor difficulties in Siegel/Cremer linearisation.

That makes rigorous coefficient production particularly friendly to ball arithmetic: first isolate \(p\) rigorously; compute balls for \(\lambda\) and \(g_m\); then perform the recurrence in `acb`-style complex balls. Arb supplies exactly the needed rigorous complex-polynomial and power-series operations, while the Mandelbrot root-certification work demonstrates how tightly periodic/root data can be isolated using MPFR-backed disks. citeturn24view1turn19view6

### Certifying a truncation of the entire linearizer

Even though \(L\) is entire, a finite polynomial

\[
L_N(\zeta)=p+\zeta+\sum_{n=2}^{N}a_n\zeta^n
\]

still needs a remainder bound on \(|\zeta|\le\rho\).

There are at least three practical rigorous routes.

**Coefficient-majorant route.** Construct a scalar positive-coefficient majorant for the recursion above. Once the tail coefficients satisfy something like

\[
|a_n|\le A R^{-n},
\]

the remaining series is bounded geometrically for \(\rho<R\).

**Cauchy/residual route.** Obtain a validated bound for \(L\) on \(|\zeta|=R\), or better for \(L-L_N\), and apply the Cauchy-tail machinery described above. ACETAF's results strongly suggest applying Cauchy to the residual rather than the full function whenever possible. citeturn17view4turn17view5

**Functional-equation/radii-polynomial route.** Treat the unknown coefficient tail as an element of a weighted \(\ell^1\) algebra and solve

\[
\mathcal F(L):=F\circ L-L\circ(\lambda\,\cdot)=0.
\]

Starting from the computed coefficients, a radii-polynomial certificate proves that an exact coefficient sequence lies within an explicit norm radius. Weighted coefficient spaces are specifically suited to turning such a norm certificate into a uniform analytic-function bound on a disk. citeturn21search5

The third method is the cleanest if the goal is a reusable, machine-checkable linearisation certificate rather than merely a safe plot.

### Certifying a domain for the inverse Schröder coordinate

The local inverse requires injectivity. A simple sufficient certificate is particularly easy to implement.

Suppose ball arithmetic proves on \(|\zeta|\le R\) that

\[
|L'(\zeta)-1|\le q<1.
\]

For any \(z,w\) in the disk, integrating \(L'\) along the line segment gives

\[
|L(z)-L(w)|
\ge
(1-q)|z-w|,
\]

so \(L\) is injective on that disk.

Next rigorously bound

\[
m_R
=
\min_{|\zeta|=R}|L(\zeta)-p|.
\]

Then the image \(L(D_R)\) contains at least the disk

\[
D(p,m_R),
\]

and the inverse Schröder coordinate \(h=L^{-1}\) is analytic there. Consequently its Taylor series at \(p\) has a certified convergence radius at least

\[
\boxed{R_h\ge m_R.}
\]

All ingredients—\(L\), \(L'\), the tail, and the minimum over the boundary circle—can be bounded with ball arithmetic plus subdivision.

This certificate is likely conservative. The true inverse branch can remain univalent far beyond the domain on which \(|L'-1|<1\); the derivative-closeness condition deliberately sacrifices sharpness for a one-line injectivity proof. A sharper computation would use a validated argument principle, interval Newton tests, or direct pair-separation/univalence methods.

### Remainder bounds for the inverse

Once a disk \(D(p,r)\) of analyticity for \(h\) is certified, the same Cauchy machinery applies. If

\[
M_r=\max_{|w|=r}|h(p+w)|,
\]

then for \(|w|\le\rho<r\), a degree-\(N\) inverse series has the standard geometric Cauchy tail bound. Better yet, compute the inverse coefficients by rigorous series reversion and bound only the residual \(h-h_N\), rather than \(h\) itself. citeturn17view4turn24view1

Another attractive a posteriori certificate uses the conjugacy residual

\[
R_N(w)
=
h_N(G(w))-\lambda h_N(w).
\]

Because the linearized coefficient operator has denominators separated from zero by the repelling multiplier, a validated Newton correction in a weighted series norm should be well conditioned compared with neutral linearisation problems. The general radii-polynomial machinery provides the existence/uniqueness framework for exactly this style of validation. citeturn21search5

### What appears to be missing

I found standard theory for repelling Poincaré linearizers, generic rigorous power-series machinery, rigorous root isolation, and modern radii-polynomial technology, but I did **not** find a published project that reports, for large collections of quadratic repelling cycles:

- certified \(N\)-term Koenigs/Poincaré coefficients;
- certified optimal or near-optimal inverse radii;
- certified truncation errors compared numerically with true high-precision errors;
- timings versus ordinary nonvalidated coefficient generation; or
- use of those certificates as an acceleration primitive for Mandelbrot deep zooms.

That looks like a genuine practical research niche rather than a missing piece of fundamental theory. citeturn22view0turn24view1turn21search5

## How tight are rigorous certificates in practice?

There is no single answer because four rather different sources of pessimism are often conflated: arithmetic roundoff, set-valued dependency, approximation truncation, and dynamical uncertainty.

### Roundoff-only balls can be extremely tight

When the input is essentially a point and the job is only to contain floating-point rounding error, ball arithmetic can be very cheap and sharp. Arb was designed around this “narrow interval” regime; its examples repeatedly increase precision until the output ball achieves the requested accuracy. citeturn18view8turn24view1

Mihalache–Vigneron's tiny-disk computations are the strongest complex-dynamics example: the excess enclosure inflation can remain essentially negligible over \(10^5\) holomorphic iterations at sufficiently tiny initial radii. citeturn19view6

### Wide boxes under chaotic iteration can be awful

A pixel is not a point. Applying the same nonlinear formula repeatedly to a rectangle reuses correlated variables as though they were independent; the enclosure can therefore grow far faster than the actual image. Johansson explicitly identifies catastrophic interval blowup as a generic limitation, and the Mandelbrot root-certification work reports situations where conventional interval arithmetic becomes unusable while circular disks succeed. citeturn24view1turn20view1

This is why a serious rigorous deep-zoom renderer should not simply replace every `double complex` with a rectangular interval type.

### Polynomial-plus-remainder models can be much tighter

Keeping low-order dependence symbolically before intervalizing the remainder can recover many orders of magnitude. ACETAF's examples show several orders of magnitude from subtracting a Taylor polynomial before Cauchy bounding, and still larger improvements for high-order coefficients after derivative information is used. citeturn17view5turn17view6

There is nevertheless no universal tightness factor. Absolute-value majorants deliberately erase cancellation, and arbitrarily large overestimation is mathematically possible. Tightness is an algorithmic property of the local dynamics, chosen domain, expansion center, order, and subdivision scheme.

### Dynamical boundary uncertainty is irreducible in a different sense

Even perfect arithmetic does not turn a finite computation into a decision procedure for every boundary point. A pixel that genuinely intersects both sides of a Julia boundary should remain mixed; points extremely close to the boundary can require arbitrarily long escape times; and proving boundedness is fundamentally different from observing a finite nonescaping orbit. The certified Julia method makes this explicit through its gray cells instead of hiding the ambiguity behind an iteration cutoff. citeturn15view0

The practical spectrum therefore looks roughly like this:

| Calculation | Observed certificate quality |
|---|---|
| isolated Mandelbrot roots / periodic parameters | spectacularly tight: \(10^{-30}\)–\(10^{-35}\) localization feasible citeturn19view9 |
| tiny disks under holomorphic maps | near-first-order-optimal disk inflation can persist for very long compositions citeturn19view6 |
| hyperbolic Julia-set cell images | gray boundary can become extremely thin under refinement; published level-19 area bounds nearly coincide visually in tested real slice citeturn17view2turn17view3 |
| whole Mandelbrot set by cell certification | much coarser; level-11 area interval has 0.4313 width, about 30.45% of lower bound citeturn24view0 |
| naive rectangular interval iteration | potentially catastrophic overestimation citeturn24view1turn20view1 |
| raw Cauchy tail estimates | can be arbitrarily pessimistic; published refinements improve examples by many orders of magnitude citeturn17view4turn17view5 |

## Proven results, established practice, and the main open problems

### Proven results

The exact perturbation recurrence is algebraically valid; escape beyond a rigorous escape-radius enclosure is a finite certificate of exterior membership; whole Julia-set cells can be certified inside/outside through interval cell mapping; their unresolved union rigorously contains the Julia boundary. citeturn18view4turn15view0

Mandelbrot parameter cells can likewise be certified by applying such critical-orbit/Julia-set reasoning over a parameter interval, producing mathematically proved interior, exterior, and unresolved regions and therefore genuine area lower and upper bounds. citeturn24view0

Enormous finite sets of special Mandelbrot parameters can be certified much more tightly: MPFR-backed disk arithmetic has given \(10^{-30}\)–\(10^{-35}\) root localizations and Newton-basin certificates for tera-scale defining polynomials. citeturn19view6turn19view9

For power-series approximation, Cauchy estimates, residual Cauchy estimates, derivative estimates, interval range bounds, and recursively derived geometric/derivative-geometric tails give mathematically rigorous truncation certificates. citeturn16view7turn16view8

For a repelling periodic point, a normalized Poincaré linearizer exists; for polynomial maps it extends globally as an entire function, while its inverse is local. Coefficient recursions are free of small divisors because \(|\lambda|>1\). citeturn22view0

### Established high-performance practice

Modern deep-zoom renderers compute one high-precision reference, perturb nearby pixels in low precision, rebase when the current reference becomes poor, and use glitch detection/fallbacks when numerical distinguishability degrades. citeturn18view4turn19view2

Series approximation skips early perturbation iterations by truncating a parameter power series. Implementations have historically established a useful skip length through probe points rather than a disk-wide supremum certificate. citeturn19view1

BLA skips blocks using \(Az+Bc\), with validity radii derived from an engineering comparison between neglected quadratic effects and hardware floating-point precision; merged BLA tables allow \(O(M)\)-sized preprocessing of a reference orbit and choose the longest currently admissible skip. citeturn18view5turn19view3turn19view5

NanoMB-style approximations exploit the extra periodic structure of minibrot centers and can skip complete periods using bivariate series. At least in the published implementation notes, validity is controlled by practical radii and fallback behavior rather than a rigorous tail certificate. citeturn19view1

### Open engineering and mathematical problems

The most conspicuous open problem is an **end-to-end certified deep-zoom renderer whose speed remains in the same regime as perturbation+BLA/NanoMB renderers**. The literature I found contains rigorous renderers and very fast deep renderers, but not a system that fully combines the two. Heiland-Allen's own notes explicitly identify missing rigorous error bounds for series skipping. citeturn19view0

A particularly plausible route is **validated perturbation with defect tracking**: high-precision ball reference orbit, low-precision midpoint deltas, a cheap radius recurrence, and rebasing whenever the error radius—not a glitch heuristic—becomes inefficient. The disk-arithmetic tera-polynomial work suggests that circular complex bounds rather than Cartesian boxes are likely essential for keeping this competitive. citeturn19view6turn24view1

For BLA, the obvious missing object is a **certified block remainder** \(E\) accompanying \(A,B,R\). A useful research benchmark would measure how much smaller the rigorous BLA radius becomes than today's heuristic radius and how frequently the nonlinear remainder forces a fallback.

For series/NanoMB, the analogous problem is a **fast bivariate Taylor model** with a tight disk-wide remainder. Naive Cauchy will often be far too conservative; residual Cauchy estimates, coefficient majorants tailored to the quadratic recurrence, adaptive recentering, and subdivision are more promising. ACETAF's experiments provide quantitative warning that choosing the bounding formulation can change quality by many orders of magnitude. citeturn17view5turn17view6

A second open benchmarking problem is that rigorous and nonrigorous renderers generally report different things. The certified-Julia paper gives absolute runtime and memory but no optimized escape-time baseline; the Trustworthy Mandelbrot calculation gives five days but no matched ordinary-render timing; deep-zoom programs report spectacular skip/speed behavior without a rigorous mode. Consequently, there is currently no authoritative answer such as “a rigorous Mandelbrot deep zoom costs 1.8× ordinary BLA.” citeturn17view1turn18view1turn19view1

For Koenigs/Schröder coordinates, the underlying case is unusually tractable. What seems missing is a practical validated library that combines periodic-point isolation, ball coefficient recurrences, an a posteriori tail proof, a sharp univalence/inverse-radius computation, and fast evaluation/inversion—and then asks whether those certified linear models can outperform or reinforce NanoMB/BLA near repelling cycles.

## References and overall conclusion

The most directly relevant primary sources are de Figueiredo, Nehab, Stolfi, and de Oliveira, *Rigorous Bounds for Polynomial Julia Sets*, for certified cell images, quadtree area bounds, and the limitations of ordinary escape-time rendering; Heiland-Allen's *Trustworthy Mandelbrot* implementation for a whole-parameter-space adaptation; and Mihalache and Vigneron, *How to Split a Tera-Polynomial*, for exceptionally tight and scalable Mandelbrot parameter certification using MPFR-backed complex disks. citeturn17view0turn24view0turn19view6

For the deep-zoom side, Heiland-Allen's current technical documentation gives the perturbation and rebasing recurrences, BLA composition and radius rules, while his development notes are unusually explicit about which parts are engineering approximations rather than proven error estimates and about the still-missing rigorous theory for series skipping. citeturn18view4turn18view5turn19view0turn19view3

For validated analytic approximation, Eble and Neher's ACETAF work is especially useful because it does not merely restate Cauchy's theorem: it measures how pessimistic plain Cauchy can be and demonstrates residual-polynomial and derivative methods that improve coefficient and tail bounds by several orders of magnitude. Johansson's Arb paper supplies the natural arbitrary-precision ball/power-series substrate, while modern radii-polynomial work supplies an attractive a posteriori route to validating infinite coefficient sequences. citeturn17view4turn17view5turn17view6turn24view1turn21search5

For repelling linearisation, Fletcher and MacClure provide the standard Poincaré-linearizer framework at repelling periodic points. Combined with ball coefficient arithmetic and a residual or majorant-series proof, quadratic repellers are arguably one of the easiest places in complex dynamics to build very tight rigorous analytic approximations: the forward linearizer is entire and the coefficient equations have exponentially improving denominators. The hard—and interesting—numerical question is how tightly one can certify the *inverse* domain of univalence. citeturn22view0turn23view0

The clearest overall conclusion is therefore:

**Rigorous Mandelbrot/Julia computation itself is mature enough to produce nontrivial images, area bounds, and astonishingly tight special-parameter databases. Rigorous analytic approximation is likewise mature. What is not mature is the connection between those techniques and the acceleration stack used by state-of-the-art deep-zoom renderers.** Perturbation is an excellent starting point because its core recurrence is exact; BLA, series approximation, and NanoMB can all be reformulated naturally as polynomial-plus-remainder enclosures. The central research question is no longer whether one *can* attach a proof to those approximations, but whether the proof radii can be kept tight and cheap enough that a certified renderer retains most of their practical speed. citeturn19view0turn19view6turn17view5turn24view1