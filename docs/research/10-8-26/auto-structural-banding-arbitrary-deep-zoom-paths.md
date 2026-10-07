# Automatic Structural Banding for Arbitrary Deep Mandelbrot Zoom Paths

## Executive finding

There is **no published or publicly implemented end-to-end algorithm that does exactly what you describe**: take an arbitrary fixed-center Mandelbrot zoom path, automatically decompose it into depth bands, identify for every band a governing minibrot nucleus **and** a nearby Misiurewicz point **and** the corresponding repelling cycle, then compile those into a chain of exact polynomial-return/Koenigs/Taylor fast paths. The individual pieces exist, and several are quite mature, but they have not been assembled into that structural compiler. The closest existing implementation on the minibrot side is **NanoMB2 in Kalles Fraktaler**: it actually constructs a chain of local bivariate super-series approximations for successively relevant hyperbolic components and uses the deepest applicable approximation first, falling back toward shallower approximations and finally perturbation. On the Misiurewicz side, Claude Heiland-Allen's `mandelbrot-numerics`/Mandelbrot Perturbator and Imagina contain most of the numerical machinery for automatic feature detection and Newton refinement, but neither connects those features to a Koenigs accelerator. citeturn18search0turn22view0turn24view0 fileciteturn6file0L2-L2

My conclusion is that the strongest currently justified design is:

> **NanoMB2-style record-return discovery → reduced/high-precision nucleus Newton → Misiurewicz recurrence discovery → full-then-naive Misiurewicz Newton → continuation of the repelling cycle → certified return-map/Koenigs atlas → cost-based segmentation of the width schedule.**

The important departure from existing code is the middle and final parts: automatically deciding *which* Misiurewicz point dynamically explains a minibrot neighborhood, and automatically deciding the width interval on which the resulting compiled accelerator is both correct and cheaper than its parent zone. Those parts appear to be new engineering/research rather than a known algorithm that merely needs implementing. The existing numerical literature and software give unusually good building blocks for them. citeturn17view2turn17view3turn21view1turn24view2

For a **fixed center** \(c_\star\), there is also a useful simplification: the structures themselves do not change as the requested width \(w\) decreases. What changes is which structures become small enough, dominant enough, and computationally useful relative to the viewport. Therefore, I would not run a fresh two-dimensional feature search at every depth. I would do a largely one-time structural analysis around \(c_\star\), generating a hierarchy of candidate accelerators, and then solve a one-dimensional coverage/cost problem over the width schedule.

One conceptual correction is important. I would **not** define “governing structure” as “the Euclidean-nearest nucleus” or “the Euclidean-nearest Misiurewicz point.” Existing atom-domain and Misiurewicz-domain methods already suggest a better notion: a feature governs a region when its particular return relation is numerically dominant there. For your use case the final test should be still stronger: the feature governs the zone when its return map and repelling-cycle linearization give the largest *certified iteration skip per unit cost*. Atom domains are useful candidate generators, but Claude has an explicit period-18 counterexample showing that membership in the correct atom domain does **not** even guarantee Newton convergence to the intended nucleus. citeturn17view1

Your reported period-764 / \(M(24,2)\) example is exactly the kind of case where this distinction matters. The useful fact is not merely that the two parameter values are \(1.7\times10^{-25}\) apart; it is that the critical orbit has a short preperiod before entering a neighborhood governed by a repelling 2-cycle, so the very long visual/iteration behavior can be compressed through that low-period dynamical object. That is the structural information an automatic system should optimize for.

## Mathematical ground truth

Let

\[
f_c(z)=z^2+c,\qquad z_0=0,\qquad z_{n+1}=z_n^2+c.
\]

Alongside the orbit, propagate the parameter derivative

\[
u_0=0,\qquad u_{n+1}=2z_nu_n+1.
\]

These two recurrences are enough to drive essentially every Newton procedure discussed below.

### Hyperbolic nuclei

A period-\(p\) hyperbolic-component center, or nucleus, is a root of

\[
z_p(c)=f_c^p(0)=0.
\]

The basic Newton step is

\[
c\leftarrow c-\frac{z_p(c)}{u_p(c)}.
\]

This is exactly the method documented and implemented in Claude Heiland-Allen's Mandelbrot numerics work. citeturn17view0

Plain \(z_p=0\) also contains roots of periods dividing \(p\). A practical improvement used by Claude is to divide out the unwanted lower-period factors corresponding to proper divisors of \(p\) before applying Newton. In the implementation, as the orbit is generated, \(z_l\) is accumulated into the divisor whenever \(l<p\) and \(l\mid p\); Newton is applied to the resulting quotient. This materially enlarges useful basins in many examples, though it does not eliminate the possibility of convergence to the wrong root. citeturn21view0turn17view1

For an **exact-period certificate**, numerical convergence is not enough. After finding \(c_H\), verify, at sufficient precision, that

\[
z_p(c_H)=0
\]

while \(z_d(c_H)\ne0\) for every proper divisor \(d\mid p\). A rigorous implementation would enclose the root and these nonzero quantities with ball/interval arithmetic rather than infer exact period from a tiny floating-point residual. The algebraic justification for removing lower-period solutions is the same generalized-dynatomic framework used to define exact critical portraits. Hutz and Towsley give explicit algebraic formulae for unicritical maps with specified exact period and preperiod. citeturn20view2

### Misiurewicz points

Using the convention in Claude's code, a Misiurewicz parameter with preperiod \(q\) and eventual period \(r\) satisfies

\[
H_{q,r}(c)
 =
z_{q+r}(c)-z_q(c)
 =
0,
\]

with no corresponding equality at an earlier preperiod, and with \(r\) reduced to its exact period. The naive Newton step is therefore

\[
c\leftarrow
c-
\frac{z_{q+r}-z_q}
     {u_{q+r}-u_q}.
\]

Claude's “full” Misiurewicz Newton divides \(H_{q,r}\) by the factors corresponding to lower preperiods,

\[
G_{q,r}(c)=
\frac{z_{q+r}-z_q}
{\displaystyle\prod_{j=0}^{q-1}
 \left(z_{j+r}-z_j\right)},
\]

which gives a substantially larger attraction basin in practice. His recommended strategy is particularly relevant here: use the full version for acquisition, then polish with the naive equation, because the factored expression is often less accurate close to the root. citeturn17view2

There is an asymmetry worth preserving in an automatic system: the full procedure aggressively rejects **lower preperiod**, while exact eventual period still needs explicit checking. Once a candidate has converged, test every proper divisor \(d\mid r\),

\[
z_{q+d}\ne z_q,
\]

and test all relevant \(q'<q\) to certify the minimal preperiod. The generalized dynatomic-polynomial theory gives the exact algebraic framework, but constructing giant symbolic polynomials is obviously the wrong numerical strategy at the periods relevant to deep rendering; orbit-level Newton plus exact-period checks is the practical analogue. citeturn20view2turn17view2

### The repelling cycle comes almost for free

Once \(c_M\) and \((q,r)\) are known,

\[
a_0=z_q(c_M)
\]

lies on the eventual \(r\)-cycle:

\[
f_{c_M}^{r}(a_0)=a_0.
\]

Because a Misiurewicz critical orbit is strictly preperiodic rather than attracted to a cycle, the eventual cycle is repelling. This is precisely why Misiurewicz neighborhoods generate the expanding/self-similar dynamics that your Koenigs jump exploits. Claude's Misiurewicz-domain description explicitly contrasts these repelling periodic orbits with the attracting cycles of hyperbolic components. citeturn21view1

For a nearby parameter \(c\), do **not** solve blindly for an arbitrary root of

\[
f_c^r(a)-a=0.
\]

Continue the desired cycle from \(c_M\). For one continuation step, Newton in the dynamical variable is

\[
a\leftarrow a-
\frac{f_c^r(a)-a}
     {(f_c^r)'(a)-1}.
\]

Claude tested essentially this continuation strategy for periodic cycles: start where the desired cycle is known, move \(c\) in small increments, and use the preceding periodic point as the Newton seed. He found it considerably more stable than independently solving the periodic-point polynomial at the final parameter. citeturn17view3

Let

\[
g_c=f_c^r,\qquad
\lambda(c)=g_c'(a(c)).
\]

Locally around the repelling fixed point \(a(c)\), the Koenigs coordinate \(\phi_c\) conjugates the return to multiplication,

\[
\phi_c(g_c(z))=\lambda(c)\phi_c(z).
\]

That local linearization is the mathematically clean reason a low-period eventual cycle can collapse hundreds or thousands of explicit iterations. What is **not** supplied by the theorem is the size of a renderer-safe numerical patch, the appropriate truncation degree, or how far in parameter space the same atlas remains useful; those are computational questions.

### What is actually proven about the Misiurewicz visual regime

The strongest theorem relevant to “this area starts looking like a Julia set and then repeats under zoom” is Tan Lei's asymptotic self-similarity result at Misiurewicz parameters. Calegari's modern proof explicitly states that it proves the asymptotic self-similarity of the Mandelbrot and Julia sets at Misiurewicz points. Thus the existence of a genuine asymptotic self-similar regime near a Misiurewicz point is theorem-level mathematics, not merely a renderer observation. citeturn20view1

What is **not** a theorem in that statement is that an arbitrary artistic fixed-center zoom can be decomposed into a unique succession of Misiurewicz-controlled bands, or that the periods of the minibrots encountered must double. Those are much stronger claims.

## What current implementations actually do

The existing software forms a surprisingly clear progression toward the architecture you have in mind.

| System | What it already automates | What is missing for your proposed chain |
|---|---|---|
| **mathr / mandelbrot-numerics / Mandelbrot Perturbator** | Atom-domain period discovery, box-period scans, nucleus Newton, reduced nucleus Newton, Misiurewicz domains, full and naive Misiurewicz Newton, periodic-cycle continuation. The GUI can find the lowest-period nucleus in a selected circle and the lowest-(pre)period Misiurewicz point in a selected square. citeturn17view4turn21view0turn17view2turn22view0 | No automatic association of a nucleus with a dynamically governing Misiurewicz point; no Koenigs/Taylor render accelerator; no automatic band scheduler. |
| **Kalles Fraktaler** | Newton zoom calculates period; its default “Ball Method” replaced/augmented the earlier box method for robust deep feature finding. It exposes atom-domain and embedded-Julia-oriented zoom modes. citeturn18search0 | General Newton zoom remains an explicitly selected operation; the ordinary perturbation path does not compile Misiurewicz/Koenigs zones. |
| **NanoMB1/2** | Builds local bivariate super-series approximations around minis. NanoMB2 has a literal **chain of minibrots**; the public implementation stores several shifted approximations and tries the deepest applicable one first before eventually falling back to perturbation. citeturn18search0 fileciteturn6file0L2-L2 | No Misiurewicz identification, no eventual-cycle/Koenigs jump, and its validity radius is heuristic rather than exact: the manual warns that increasing `RadiusScale` can cause visible distortion. |
| **Imagina** | Its public feature finder distinguishes `PeriodicPoint` from `MisiurewiczPoint`, reports preperiod and period, discovers the relation using its LA/perturbation evaluator, and runs Newton correction on the discovered feature. citeturn24view0turn24view1turn24view2 | The public code does not turn those Misiurewicz detections into a persistent hierarchy of render accelerators. Its precise-locating API is also more developed for periodic points than Misiurewicz points. citeturn24view3 |
| **Fraktaler 3** | Current public version 3.1 uses perturbation+BLA, performs periodicity checking on references, and has Newton Zooming that can automatically center on minis or embedded Julia sets after a click. citeturn22view3turn22view4turn22view5 | It does not expose a multi-zone Misiurewicz/Koenigs execution chain; its documentation still treats known reference period as a single optimization and warns that a wrong period can corrupt images. citeturn22view3 |

### NanoMB2 is particularly close to the minibrot half of your idea

Its implementation is more interesting than the manual alone suggests. During reference construction, NanoMB2 follows the critical orbit and notices unusually small returns to zero. The code keeps a `smallest_ref_rad_so_far`; when a new return is sufficiently smaller, it extracts the parameter-only series polynomial, finds its nearby root with Newton, interprets that root as a hyperbolic-component center, shifts the bivariate approximation there, records the current iteration count as the period, assigns a usable radius, and appends the resulting SSA to its chain. During pixel iteration it explicitly tries the stored SSAs from “latest (deepest) to first,” then works back toward shallower approximations, and finally uses perturbation. fileciteturn6file0L2-L2

Even more tellingly, comments in the source identify almost exactly the unsolved problem in your question. They propose finding “upper level hyperbolic component centres,” no longer assuming that the reference itself is near the deepest center, and warn that the polynomial approximation can lose accuracy before a useful component is found. The implementation therefore “suppose[s] we have a good deepest HCC.” That is essentially an unfinished automatic structural hierarchy. fileciteturn6file0L2-L2

NanoMB2 is nevertheless **not an exact precedent** for your system. Kalles' manual calls NanoMB2 experimental, disables normal glitch detection/correction in that mode, and says that enlarging the `RadiusScale` may improve speed while causing visible distortion. Thus its chain is excellent evidence that multi-minibrot acceleration works, but not evidence that a renderer already has certified per-zone exactness. citeturn18search0

### Period detection is already good enough; root identity is the harder problem

Claude's older automatic period scanner successively subdivides a region, uses a Jordan-curve/box-period method to detect a nucleus in a box, Newton-refines its center, and then verifies the result with a much smaller box. That combination is much safer than “find a small \(z_p\), run Newton, believe whatever came back.” citeturn17view4

Kalles subsequently made its interval/ball method the default because it works better than the box method in difficult, skewed locations. For a new compiler I would use record-return detection as the fast **candidate generator**, and box/ball/root enclosure only as necessary for verification or to fill gaps. Running a full two-dimensional period scan at every zoom depth would throw away the chief advantage of a fixed-center path. citeturn18search0

Imagina demonstrates another useful direction. Its feature finder carries period detection through the existing linear-approximation/perturbation machinery rather than switching to a completely separate slow direct iterator. It records `Preperiod` and `Period`, forms a Newton correction from the dynamically accumulated difference and parameter derivative, and iterates the correction up to a fixed number of steps. That is a good model for making structure discovery cheap enough to run automatically. citeturn24view1turn24view2

## Recommended automatic structural compiler

This section is **my synthesis** rather than an algorithm I found described as a whole in the literature. Its components are established practice; the way they are joined is the speculative/new part.

### Discover the minibrot hierarchy from one long orbit

For the fixed center \(c_\star\), compute a high-quality reference orbit

\[
(z_n,u_n),\qquad n=0,\ldots,N,
\]

where \(N\) is at least as large as the structural periods you expect to exploit. During that orbit, maintain record or near-record returns of \(|z_n|\). An atom domain is based precisely on the iteration index at which \(|z_n|\) attains the relevant minimum; around a period-\(p\) component that index tends to be \(p\). Claude uses this definition explicitly, and NanoMB2 effectively turns strong record returns into a hierarchy of SSA candidates. citeturn17view1turn21view1 fileciteturn6file0L2-L2

For every promising record at \(p\), the first Newton correction

\[
\Delta c_H\approx -\frac{z_p}{u_p}
\]

is not just a Newton step; its magnitude is a first-order estimate of how far the relevant period-\(p\) root lies from \(c_\star\). That gives an extremely cheap initial ranking. Refine only candidates whose estimated component/domain scale is relevant to some requested viewport width.

Run reduced arbitrary-precision Newton on each retained \(p\), then certify exact period. Cluster duplicates. If a large logarithmic width interval has no usable candidate, fall back to a Kalles-style ball-period or Claude-style box-period search in one or a few strategically chosen viewports. This hybrid approach gets the \(O(N)\)-ish candidate stream of NanoMB2 without assuming the record-minimum heuristic is exhaustive. Atom-domain Newton is explicitly known not to be guaranteed, so the fallback/certification stage is necessary. citeturn21view0turn17view4turn17view1

I would preserve all successful centers

\[
H_i=(c_{H_i},p_i,\text{domain estimate},\text{return jet metadata})
\]

rather than immediately deciding which one “owns” a depth. Ownership should be decided later by validity and cost.

### Discover nearby Misiurewicz relations numerically, not combinatorially

This is the least solved part of the problem.

The straightforward way to test a proposed \((q,r)\) at \(c_\star\) is

\[
H_{q,r}=z_{q+r}-z_q,\qquad
H'_{q,r}=u_{q+r}-u_q.
\]

The quantity

\[
D_{q,r}
=
\left|
\frac{H_{q,r}}{H'_{q,r}}
\right|
\]

is especially valuable: to first order it is the parameter-space Newton distance from \(c_\star\) to a root of that preperiod/period equation. For automatic discovery, I would rank candidate Misiurewicz relations by \(D_{q,r}\), not by \(|H_{q,r}|\) alone.

This is a natural quantitative extension of Claude's Misiurewicz-domain idea. His domains fix \(r\) and ask which \(q\) minimizes

\[
|z_{q+r}-z_q|,
\]

and he observes that the resulting domains surround Misiurewicz points and are much larger/easier to detect than the points themselves. The Newton-normalized quantity incorporates the conditioning needed to estimate actual parameter distance. The normalization is my proposed extension, not something claimed in his published domain algorithm. citeturn21view1

You do not want to enumerate every \((q,r)\) pair up to \(N\), which is \(O(N^2)\). Instead, use orbit recurrences as candidate generators. Maintain a spatial index or multiscale hash of the dynamical orbit values \(z_n\); when \(z_j\) falls close to an earlier \(z_i\), emit

\[
q=i,\qquad r=j-i,
\]

then evaluate the proper Newton-normalized residual. Also explicitly test small \(r\), because short eventual cycles are disproportionately valuable to a Koenigs accelerator: your \(M(24,2)\) case is the ideal example.

There is an important caveat. A nearest-neighbor search in the \(z\)-plane can miss a nearby parameter root when \(H'\) is enormous: \(H\) need not be spectacularly small for \(H/H'\) to be tiny. Therefore a robust implementation should combine recurrence hashing with sparse direct scans over likely periods \(r\) and record minima of \(D_{q,r}\).

For every retained \((q,r)\), start at \(c_\star\) or at the associated nucleus and run **full Misiurewicz Newton**, then naive Newton for polishing. Certify minimal \(q\) and exact \(r\). Claude's graphical tool already demonstrates that “find the lowest-(pre)period Misiurewicz point in this square” can be automated, so this is not a fundamentally new root-finding capability; what is new is running it systematically as part of accelerator compilation. citeturn22view0turn17view2

### Do not pair nucleus and Misiurewicz point by distance alone

Suppose nucleus \(H_i\) has several nearby Misiurewicz candidates \(M_j\). For each \(M_j=(c_M,q,r)\):

1. Continue its eventual \(r\)-cycle from \(c_M\) to a representative parameter in the proposed zone.
2. Compute its multiplier \(\lambda\).
3. Build a preliminary Koenigs coordinate.
4. Map the post-preperiod critical state into that coordinate.
5. Estimate how many \(r\)-returns remain inside the usable linearization domain.

If

\[
\eta(c)=\phi_c\!\left(z_q(c)\right)
\]

is the displacement from the continued repelling fixed point in Koenigs coordinates, and the trusted dynamic patch has radius \(R_\phi\), then a first estimate of the number of repeat cycles that can be jumped is

\[
K(c)\approx
\frac{\log(R_\phi/|\eta(c)|)}
     {\log|\lambda(c)|},
\]

clamped to the valid range.

That quantity is far more relevant than \(|c_H-c_M|\). A Misiurewicz point that is slightly farther away but gives \(K=500\) cheap period-2 returns is a better governing structure than a closer one whose critical orbit only shadows a period-17 cycle for three repeats.

This also gives a clean automatic explanation of your period-764 / \(M(24,2)\) speedup: the high period identifies the minibrot return structure, while the low eventual period gives an exceptionally cheap repeated local dynamical operation. The useful object is the **pair** \((H,M)\), not either feature in isolation.

### Continue rather than repeatedly rediscover the cycle

At the exact Misiurewicz root, the eventual cycle is known directly from the critical orbit. Continue it to the nucleus and to whatever parameter anchors you use for Taylor patches. Do this with adaptive parameter steps; if Newton begins to need many corrections, reduce the continuation step.

This is directly supported by Claude's periodic-cycle experiments: continuation from a known nucleus/cycle was much more stable than asking Newton to select the desired cycle from all roots at the final parameter. citeturn17view3

For your particular architecture, I would store a zone roughly as

\[
Z=
(c_H,p;\;c_M,q,r;\;a,\lambda;
\;P_{\rm return};
\;\Phi\text{-atlas};
\;\text{certified domains};
\;\text{cost model}).
\]

The key is that \(p\) and \(r\) play quite different roles: \(p\) describes the local minibrot/critical return, while \(r\) describes the low-period repelling dynamics that allows the dramatic jump.

### Build the fast path only after the structural pair passes an economics test

Before paying for high-order return jets and a dense Taylor-patch atlas, benchmark a cheap prototype. Estimate

\[
\text{saved iterations}
-
\text{return-map cost}
-
\text{Koenigs-map cost}
-
\text{patch-navigation cost}.
\]

Discard zones that cannot plausibly beat BLA/perturbation. This matters because “interesting mathematical structure” and “useful accelerator” are not equivalent.

This would keep the preprocessing tractable at extreme periods: a candidate nucleus can be identified with orbit/Newton work before you incur the much larger cost of constructing its complete exact fast path.

## Depth bands, nesting, and handover

### “Zoom doubling” is real practice but not a universal period law

The artistic observation is well established in Claude's tools and writing. Current Fraktaler 3 Newton Zooming explicitly offers power presets corresponding to doubling \(2,4,8,\ldots\) for a quadratic formula when approaching a miniset. Kalles describes relative folding where, for the power-2 Mandelbrot set, particular fractional approaches to the mini produce doubled and quadrupled versions of the visible pattern. citeturn22view3turn18search0

Embedded Julia sets also have striking combinatorial period structures. Claude gives an example around a period-88 island where successive subdivisions contain one period-88 island, two of period 91, four of period 94, eight of period 97, and so on. Crucially, he explicitly labels the general binary-subdivision statement a **conjecture**, and notes complications from doubly embedded Julia sets and special filament directions. citeturn21view2

So an automatic compiler should exploit period-doubling-like behavior as a **prediction heuristic**, never as the discovery rule. Arbitrary artistic paths can pass through tuned copies, filaments, embedded Julia regimes, doubly influenced regions, near-parabolic bottlenecks, and ordinary hair where the next useful period does not obey a clean recurrence. Renormalization does explain why a small copy locally reproduces Mandelbrot dynamics with multiplied periods, but it does not imply a universal “next band period = \(2p\)” sequence for arbitrary fixed centers. citeturn21view2

### Atom-domain scale is a useful first approximation to when a mini becomes visually relevant

Kalles' Newton navigation is revealing here. Its documentation says that zooming to roughly the **atom-domain** scale is typically around an embedded Julia set, with a somewhat deeper power sometimes landing in the Julia-morph regime. That is practitioner evidence that an atom-domain size is a useful predictor of a structural transition in an artistic zoom. citeturn18search0

Likewise, Mandelbrot Perturbator says Misiurewicz domains are typically around the scale at which spiralling starts to dominate. Thus two naturally occurring parameter-domain scales already bracket much of the behavior you want to detect: the minibrot's atom domain and the nearby Misiurewicz domain. citeturn22view0

But I would **not** make either domain boundary the actual handover. They are discovery/visual scales, not performance or correctness boundaries.

### A band should be defined by certified coverage and runtime

Let the viewport at width \(w\) be \(V(w)\), with maximum parameter-space radius from its fixed center

\[
\rho(w)=\gamma w,
\]

where \(\gamma\) accounts for aspect ratio and whatever width convention you use.

For each compiled zone \(Z_i\), derive a parameter-space region \(D_i\) in which its local return map and Koenigs/Taylor machinery meet your accuracy requirement. If a conservative circular certificate has radius \(R_i\) around anchor \(a_i\), then the simple sufficient condition for the **entire viewport** to be eligible is

\[
|c_\star-a_i|+\rho(w)\le R_i.
\]

For a noncircular atlas, test the actual viewport enclosure or subdivide it into tiles.

That gives a correctness eligibility interval in \(w\). Separately measure or model the rendering cost

\[
C_i(w)
=
C_{\rm map}
+
C_{\rm Koenigs}
+
C_{\rm atlas}
+
P_{\rm fallback}(w)C_{\rm base}.
\]

The correct handover from zone \(i\) to a deeper zone \(j\) is the first width for which

\[
V(w)\subseteq D_j
\]

**and**

\[
C_j(w)<C_i(w)
\]

by enough margin to cover setup/cache costs.

That is the answer I would use to “where does one band hand over to the next?”: **not at a particular atom boundary, Misiurewicz-domain boundary, period change, or nominal zoom exponent, but at the intersection of certified validity and measured break-even cost.**

For a predetermined sequence \(w_0,\ldots,w_m\), this becomes a trivial optimization problem. If switching/setup is negligible, select

\[
Z(w_k)=
\arg\min_{i:\;V(w_k)\subseteq D_i} C_i(w_k).
\]

If construction, GPU upload, or cache switching matters, use dynamic programming:

\[
D[k,i]
=
C_i(w_k)
+
\min_j
\left(D[k-1,j]+S_{j,i}\right),
\]

setting \(C_i(w_k)=+\infty\) when zone \(i\) is invalid. This gives the globally cheapest band decomposition for the actual requested frames rather than an arbitrary geometric decomposition.

I would add modest hysteresis so that noisy benchmarking does not cause a zone to alternate every few frames.

### Prefer a hierarchy over mutually exclusive independent zones

There is a deeper optimization opportunity. A “band” need not throw away its parent. The best implementation is probably a **stack of compiled transformations**:

\[
\text{generic perturbation/BLA}
\rightarrow
\text{parent return map}
\rightarrow
\text{child return map}
\rightarrow
\text{Misiurewicz preperiod}
\rightarrow
\text{Koenigs jump}.
\]

A deeper band selects a deeper entry point in this hierarchy while reusing common preambles and constants. NanoMB2 already does something analogous for its minibrot SSAs by keeping several approximations rather than replacing one with the next. fileciteturn6file0L2-L2

That hierarchical interpretation also handles “doubly embedded” or overlapping visual influences more gracefully than assigning every zoom level a single philosophical owner. The renderer only cares which composition is cheapest and certified.

## Cost, failure modes, and exactness

### Precomputation cost is mostly linear in the periods you actually refine

One orbit evaluation for nucleus Newton costs \(O(p)\) Mandelbrot steps per Newton iteration, times the cost of the chosen multiprecision arithmetic. A Misiurewicz Newton evaluation costs \(O(q+r)\) orbit steps before factoring/checking overhead. Repelling-cycle Newton costs \(O(r)\) map steps per correction. Jet construction is roughly linear in the underlying iterate count multiplied by the cost of the chosen truncated-polynomial algebra; for ordinary dense degree-\(d\) jets, direct coefficient convolution introduces an approximately quadratic dependence on \(d\). These are algorithmic consequences of the corresponding recurrences rather than properties unique to a specific renderer.

That means your favorable case \(p=764,\,(q,r)=(24,2)\) is structurally cheap to analyze: the high-period nucleus setup is modest, and every operation associated with the Misiurewicz cycle is tiny. At much deeper paths, nucleus periods may become the dominant setup expense, but the work is still one-time and amortizable over all frames and pixels in the band.

A major advantage of the proposed record-return hierarchy is that you do **not** run full Newton for every integer period. One \(N\)-step reference orbit can emit a sparse list of structural period candidates, just as atom-domain logic and NanoMB2 do already. citeturn17view1 fileciteturn6file0L2-L2

### The main failure modes are identifiable

**Wrong Newton basin.** This is a real, demonstrated failure, not a theoretical nit. An atom domain can cross a Newton-basin boundary, and Claude's period-18 example remains problematic even after lower-period roots are divided out. Therefore every nucleus result needs independent period/root verification, and difficult gaps need box/ball isolation rather than repeated blind Newton. citeturn17view1

**Lower-period aliasing.** A root of \(z_p=0\) may really have period dividing \(p\); likewise a solution of \(z_{q+r}=z_q\) may have a smaller eventual period or preperiod. Reduced Newton improves acquisition, but explicit divisor/minimality checks are still required. citeturn21view0turn20view2

**Ambiguous Misiurewicz pairing.** A minibrot may have several nearby preperiodic points, and “closest” need not mean “responsible for the useful orbit shadowing.” This is precisely why I recommend scoring the candidates by actual post-\(q\) entry into the continued repelling cycle and by predicted Koenigs skip length. This selection rule is speculative engineering, not an existing theorem.

**Near-parabolic conditioning.** If the relevant multiplier approaches the unit circle, a repelling-cycle linearization can remain mathematically valid while becoming a poor numerical accelerator: departure from the fixed point is slow, usable coordinate neighborhoods can be awkward, and long transition regions arise. Such zones should lose the cost comparison and revert to a different accelerator rather than be forced through Koenigs.

**Approximation-radius optimism.** NanoMB2 is the cautionary example. Its `RadiusScale` trades speed against visible distortion, demonstrating what happens if a numerically convenient local-map radius is treated as a hard validity theorem. citeturn18search0

**Precision underestimation.** At \(10^{-50}\) parameter scales, “50 decimal digits” is not automatically enough; the needed precision depends on conditioning, derivatives, cancellation, and the desired root enclosure. Use adaptive precision driven by Newton correction size, residuals, and interval separation rather than a fixed `digits = zoom_digits + constant` rule.

**Multiple structures across one viewport.** At the shallow side of a band, a large frame can intersect different atom/Misiurewicz domains even though its center is already strongly associated with one feature. A global accelerator can therefore be profitable for only part of the image. The exact implementation should permit per-tile or per-pixel fallback instead of delaying the zone until every pixel fits.

### “Every pixel exact” changes the engineering standard

A finite Taylor approximation or finite Koenigs-series approximation is not literally identical to the infinite analytic function. To preserve an exact renderer result, there must be one of two things:

\[
\text{rigorous remainder bound}
\]

that is strong enough to prove the same iteration/bailout decision, or

\[
\text{fallback}
\]

whenever the bounded approximation is not decisive.

This is where I would make your implementation more conservative than NanoMB2. Use ball arithmetic, Taylor models, or explicit complex-disk remainder estimates during **offline zone compilation**. The resulting renderer need not run heavy interval arithmetic for every pixel: it can store pre-certified patch domains and error bounds and take the fast branch only when a cheap inclusion test succeeds. General interval-arithmetic packages formalize exactly this distinction between an approximate value and a guaranteed enclosure. citeturn16search1

For a Taylor atlas patch \(T_k\), store at least a domain radius and a remainder bound,

\[
|F(z)-T_k(z)|\le\epsilon_k
\qquad (z\in D_k).
\]

Do the analogous thing for the inverse/forward Koenigs patches. If the accumulated enclosure remains comfortably inside the next patch or on one side of an escape/iteration decision, proceed. If not, return the state to your ordinary exact perturbation/BLA path.

That scheme allows the fast path itself to use inexpensive floating-point or `floatexp` arithmetic while retaining an **exactness proof by construction** for every accepted jump.

### Separate the evidence levels explicitly

**Proven:** exact-period/preperiod conditions can be encoded algebraically with generalized dynatomic constructions; a Misiurewicz critical orbit lands on an eventual periodic cycle; local dynamics around the repelling cycle are analytically linearizable; and the Mandelbrot/Julia geometry has asymptotic self-similarity at Misiurewicz parameters. citeturn20view2turn20view1

**Established numerical practice:** atom domains and Misiurewicz domains are excellent feature-finding regions; reduced Newton improves root acquisition; full-then-naive Misiurewicz Newton is effective; periodic-cycle continuation enlarges practical basins; box/ball period detection verifies candidate minis; NanoMB2 can maintain and execute a hierarchy of minibrot return approximations; Imagina can automatically infer periodic versus Misiurewicz features and their period data. citeturn17view4turn21view0turn17view2turn17view3turn18search0turn24view2

**Speculation / proposed research:** Newton-normalized recurrence mining to discover the relevant \((q,r)\) automatically; scoring nucleus–Misiurewicz pairs by predicted Koenigs residence time; compiling certified parameter/dynamical Taylor atlases; and choosing band boundaries by global cost optimization. I found no primary source or public renderer implementing that combined procedure.

## Verdict and primary references

The central result of this research is that the problem naturally splits into two halves of very different maturity.

The **minibrot hierarchy problem is largely solved in practice**. For a fixed-center path, a NanoMB2-like stream of record critical-orbit returns is probably the best first-pass detector. For each emitted period, use reduced arbitrary-precision Newton, then ball/box or interval verification. NanoMB2's source is especially important evidence because it already constructs multiple hyperbolic-component-centered approximations in one reference pass and chains them at execution time. citeturn21view0turn17view4 fileciteturn6file0L2-L2

The **Misiurewicz association problem is the missing algorithmic link**. Claude's Misiurewicz domains and full Newton give a strong candidate-acquisition method, and Imagina demonstrates that preperiod/period feature recognition can be embedded in a fast deep evaluator. What does not yet exist publicly is an algorithm that says: “this period-764 mini is best explained, for rendering purposes, by this \(M(24,2)\); therefore continue its 2-cycle, construct this Koenigs coordinate, and compile this band.” citeturn21view1turn17view2turn24view1

The most promising new numerical primitive is therefore

\[
\boxed{
D_{q,r}(c)=
\left|
\frac{z_{q+r}(c)-z_q(c)}
     {u_{q+r}(c)-u_q(c)}
\right|
}
\]

used over a sparse recurrence-generated set of \((q,r)\). It estimates *parameter distance to the corresponding preperiodic equation*, can be computed from exactly the orbit data you already maintain, and naturally produces high-quality seeds for full Misiurewicz Newton. Then choose among the converged roots by the thing that actually matters to your renderer: certified Koenigs residence/skip length and total predicted cost.

Finally, I would make **band segmentation the last step**, not feature discovery. Compile candidate structures first; for each structure determine the width interval over which its viewport is certified and measure its speed. Then solve the one-dimensional minimum-cost scheduling problem. This avoids trying to infer computational handovers from visually seductive but non-universal patterns such as zoom doubling.

On the question “does any renderer already chain zones like this?” the strongest answer I can support from public code as of October 7, 2026 is **no**. NanoMB2 chains multiple minibrot-centered polynomial approximations, which is a genuine antecedent; Imagina and mathr tools automatically find periodic/Misiurewicz structure; Fraktaler 3 uses Newton-discovered period information and BLA. But I found no public system that chains

\[
\boxed{
\text{minibrot return}
\rightarrow
\text{Misiurewicz preperiod}
\rightarrow
\text{repelling-cycle Koenigs jump}
\rightarrow
\text{certified Taylor atlas}
}
\]

across automatically selected depth bands. Kalles' existing multi-minibrot chain is the closest architectural relative, and its source comments effectively document the still-open problem of robustly discovering and transitioning among the relevant hyperbolic centers. citeturn18search0turn24view0turn22view3 fileciteturn6file0L2-L2

The most useful primary references are Heiland-Allen's **Nucleus** implementation for the base Newton recurrence; **Newton's method for periodic points** for reduced-period Newton; **Atom domains and Newton basins** for the crucial counterexample to naïve automatic acquisition; **Misiurewicz Point (“Full”)** and **Misiurewicz domains** for preperiodic discovery and refinement; and **Newton's method for periodic cycles** for continuation of the eventual cycle. citeturn17view0turn21view0turn17view1turn17view2turn21view1turn17view3

For implemented renderer practice, the primary references are the **Kalles Fraktaler manual and NanoMB2 source**, **Imagina's `FeatureFinder.cpp`**, and the current **Fraktaler 3** documentation. citeturn18search0turn24view0turn22view3turn22view5 fileciteturn6file0L2-L2

For the theorem-level background, Hutz and Towsley's **Misiurewicz Points for Polynomial Maps and Transversality** supplies generalized dynatomic formulae for exact critical portraits, while Calegari's **Surgery sequences and self-similarity of the Mandelbrot set** gives a modern proof of Tan Lei's theorem on asymptotic self-similarity at Misiurewicz parameters. These support the algebraic and asymptotic pieces of the architecture, but neither supplies the renderer-level zone decomposition described here. citeturn20view2turn20view1