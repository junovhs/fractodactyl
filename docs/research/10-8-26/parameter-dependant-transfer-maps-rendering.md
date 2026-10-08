# Parameter-Dependent Transfer Maps for Deep Mandelbrot Rendering

## Executive conclusion

The closest analogue to your problem is not a parametric reduced-order model. It is **differential-algebraic jet transport with the parameter promoted to an independent variable**, combined with the astrodynamics practice of **splitting or re-expanding a map when one polynomial ceases to be a good local representation**.

For your quadratic family,
\[
f_c(z)=z^2+c,\qquad c=C+\delta,
\]
the direct extension of your fixed-\(C\) atlas is to store, for every existing \(w\)-patch,
\[
Z_n(w,\delta)
=
f_{C+\delta}^{\,n}\!\bigl(A+\psi_C(w)\bigr)
\approx
\sum_{j=0}^{q}\delta^j P_{n,j}(w),
\]
where every \(P_{n,j}\) is itself a Taylor polynomial in \(w\). This is exactly the kind of state-plus-parameter Taylor map called a differential-algebra expansion, jet transport map, or state-transition tensor in accelerator and astrodynamics work. Modern formulations explicitly combine initial-state perturbations and parameter perturbations in one multivariate Taylor map. citeturn17search4turn23view2

The recurrence is especially cheap for \(z^2+c\). If
\[
Z_k(w,\delta)=\sum_{j=0}^{q}P_{k,j}(w)\delta^j,
\]
then, truncating in \(w\) and \(\delta\) after every step,
\[
\begin{aligned}
P_{k+1,0}&=P_{k,0}^2+C,\\
P_{k+1,1}&=2P_{k,0}P_{k,1}+1,\\
P_{k+1,j}&=2P_{k,0}P_{k,j}
+\sum_{a=1}^{j-1}P_{k,a}P_{k,j-a},
\qquad 2\le j\le q.
\end{aligned}
\]
This is simply truncated-polynomial arithmetic; it is what DA software automates. COSY INFINITY was designed to compute high-order maps while retaining system parameters in the maps, and modern astrodynamics writes the same construction explicitly as a Taylor polynomial in \((\delta x_0,\delta q)\). citeturn25view3turn23view2

The crucial lesson from accelerator and astrodynamics practice is, however, **not to force one monolithic polynomial to survive arbitrarily long nonlinear evolution**. Accelerators normally compute a one-turn or one-cell map and then iterate or normal-form that map; astrodynamics explicitly subdivides a domain when the Taylor representation loses convergence. In Wittig et al.'s orbital example, even increasing one global flow map from sixth to fourteenth order did not cure the loss of accuracy; automatic domain splitting did. citeturn18view3turn24view2

For your renderer I would therefore build the following, in this order:

| Layer | What I would implement | Status |
|---|---|---|
| Parameter jet | Replace every fixed-\(C\) coefficient by a short array of coefficients in normalized \(\delta\) | **Established practice / algebraically exact up to truncation** |
| Independent degrees | Keep \(w\)-order \(p\) and parameter order \(q\) separate rather than one total degree | **Established and particularly appropriate here** |
| Error indicator | Store scaled coefficient-shell bounds and, preferably, a Taylor-model remainder | **Established practice** |
| Adaptive parameter subdivision | Split/recenter in \(c\) where the highest parameter shells stop decreasing sufficiently | **Established practice adapted directly from astrodynamics** |
| Tail segmentation | Store maps for blocks of iterations rather than insisting on one polynomial through all 400–2000 iterations | **Established transfer-map principle; renderer-specific implementation** |
| Co-moving Poincaré chart \(L_c\) | Precompute the linearizer jointly in \(w,c\), and exploit \(F_c^m(L_c(w))=L_c(\lambda(c)^m w)\) | **Mathematically justified; likely the highest-leverage renderer-specific idea, but not established rendering practice** |

One numerical observation explains immediately why first order is not enough at your scale. If \(|\delta|\sim10^{-15}\), then
\[
|\delta|^2\sim10^{-30}.
\]
So even a completely unexceptional \(O(1)\) quadratic coefficient in parameter already contributes at the scale of the features you are trying to resolve. Long-term dynamics can amplify the quadratic and higher coefficients enormously. **A first-order correction was never capable, in principle, of uniformly supporting \(10^{-30}\)-scale fidelity over a \(10^{-15}\) parameter radius.** Iteration merely makes the failure visible.

The converse is also important: **2000 iterations does not imply that you need 2000 parameter orders, or even dozens.** Transfer-map communities choose polynomial order by scaled coefficient decay/error bounds and split or re-center when that decay deteriorates. Published accelerator work describes roughly tenth-order maps in six phase-space variables as typical for demanding weakly nonlinear problems; COSY examples include ninth-order maps, and FFAG studies have used eleventh-order maps. These are *total state-map orders*, not evidence that your single parameter variable needs order ten. citeturn25view0turn25view3turn17search4

My engineering expectation for your very small parameter disk is that **parameter order \(q=3\)–\(6\) should cover a large fraction of non-parabolic patches if you allow splitting/re-expansion; \(q=8\)–\(12\) is a reasonable ceiling before declaring that the patch or coordinate system should be changed rather than raising order further**. That is speculation, not a result from the literature. The correct \(q\) should be selected patch-by-patch from coefficient/remainder tests, not from iteration count.

## How the transfer-map fields represent state and parameter together

### Differential algebra is exactly the required data structure

Berz's differential-algebra approach replaces numbers by truncated multivariate Taylor polynomials and then runs essentially the original dynamics on those objects. COSY's early descriptions explicitly emphasize arbitrary-order maps, dependence on system parameters, and normal-form analysis; a historical COSY example generated a ninth-order map of a complicated bending magnet in about a second on 1991 hardware. citeturn23view0turn25view3

The corresponding modern astrodynamics formulation is transparent. Acciarini et al. write
\[
\delta x_f^i
=
\mathcal P_i^k(\delta x_0,\delta q)+O(k),
\]
where the vector of Taylor variables explicitly concatenates initial-state perturbations and parameter perturbations. They note that the same object appears in the literature as a DA expansion, jet transport, state-transition tensors, or truncated-Taylor-polynomial flow expansion. citeturn23view2

For your holomorphic problem there is a substantial simplification that generic DA packages often do not exploit automatically: **use two complex variables \(w,\delta\), not four real variables \(\Re w,\Im w,\Re\delta,\Im\delta\)**. Holomorphy means there are no \(\bar w\) or \(\bar\delta\) monomials. A rectangular truncation
\[
Z(w,\delta)=
\sum_{i=0}^{p}\sum_{j=0}^{q}a_{ij}w^i\delta^j
\]
therefore contains only
\[
(p+1)(q+1)
\]
complex coefficients. A generic total-degree polynomial of degree \(d\) in two complex variables has
\[
\frac{(d+1)(d+2)}2
\]
coefficients, while treating the problem as four unrelated real variables would grow far more quickly.

More importantly, **rectangular order is preferable to total degree in your application**. There is no reason for the \(w\)-order demanded by an existing spatial patch to equal the parameter order demanded by a \(10^{-15}\) perturbation. This point has independently become explicit in recent parameter-dependent invariant-manifold work: Jain and Li contrast the traditional trick of treating parameters as zero-dynamics state variables—which mixes state and parameter orders—with an approach based on \(\delta=\mu-\mu_0\) that permits independent control of expansion order in manifold coordinates and parameters. citeturn18view0

They use precisely the bivariate ansatz
\[
W(p,\mu)=
\sum_{m,\ell}w_{m,\ell}\,p^m\delta^\ell
\]
together with an analogous parameter-dependent reduced map, and determine the coefficients by a nested recursion in state/manifold degree and parameter degree. citeturn18view2

That is almost exactly the architecture I would use for your atlas.

### Normalize both independent variables before storing coefficients

I would not store powers of the physical
\[
\delta=c-C
\]
directly. Give each parameter patch a certified radius \(\rho_c\) and write
\[
\delta=\rho_c\,\eta,\qquad |\eta|\le1.
\]
Likewise, if an existing \(w\)-patch has center \(w_0\) and radius \(\rho_w\), use
\[
w=w_0+\rho_w\,\xi,\qquad |\xi|\le1.
\]

Then store
\[
Z(\xi,\eta)=
\sum_{i,j}b_{ij}\xi^i\eta^j.
\]

This is not cosmetic. The magnitude of a coefficient shell
\[
S_j=\sum_i |b_{ij}|
\]
is now approximately the largest possible contribution from parameter order \(j\) over the normalized patch. You can use \(S_j\) directly to decide whether parameter order \(j\) matters. It also avoids coefficients differing by factors such as \(10^{15j}\).

For the quadratic recurrence, normalization merely changes the \(+1\) in the \(j=1\) equation to \(+\rho_c\):
\[
P_{k+1,1}=2P_{k,0}P_{k,1}+\rho_c.
\]

### Storage and runtime cost are benign for one parameter

Suppose your current fixed-\(C\) patch stores \(p+1\) complex coefficients. Adding parameter order \(q\) multiplies storage almost exactly by \(q+1\):

\[
\begin{array}{c|c}
q&\text{storage relative to fixed } C\\
\hline
1&2\times\\
2&3\times\\
4&5\times\\
6&7\times\\
8&9\times
\end{array}
\]

Evaluation can be nested Horner:
\[
Z(w,\delta)
=
P_0(w)+
\delta\left[P_1(w)+
\delta\left(P_2(w)+\cdots\right)\right].
\]
If every \(P_j\) has \(w\)-degree \(p\), a straightforward implementation costs about \((p+1)(q+1)\) complex multiply-adds plus the outer Horner operations. In other words, a \(q=4\) atlas is roughly five fixed-\(C\) polynomial evaluations, not 400–2000 orbit iterations.

FLINT/Arb already provides rigorous complex balls, truncated complex polynomial multiplication, composition and Horner/rectangular-splitting evaluation, while AuDi/pyaudi implements sparse truncated Taylor polynomial algebra, complex coefficients, vectorized evaluation and Taylor models. citeturn21search0turn21search2turn23view3

Offline propagation is somewhat more expensive. For each parameter degree \(j\), the quadratic recurrence contains a convolution
\[
\sum_{a+b=j}P_aP_b,
\]
so naïve offline work is \(O(q^2)\) polynomial products per iteration, with each dense \(w\)-product naïvely \(O(p^2)\). Symmetry nearly halves the parameter convolution, and fast polynomial multiplication is available if \(p\) is sufficiently large. For the small \(q\) likely here, the offline cost is unlikely to be the limiting factor.

### An extremely useful diagnostic requires no bivariate \(w\)-map at first

At the center of every existing patch, you can cheaply propagate the parameter coefficients alone:
\[
z_k(\delta)=
a_{0,k}+a_{1,k}\delta+a_{2,k}\delta^2+\cdots.
\]
For the first few orders,
\[
\begin{aligned}
a_{0,k+1}&=a_{0,k}^2+C,\\
a_{1,k+1}&=2a_{0,k}a_{1,k}+1,\\
a_{2,k+1}&=2a_{0,k}a_{2,k}+a_{1,k}^2,\\
a_{3,k+1}&=2a_{0,k}a_{3,k}+2a_{1,k}a_{2,k},\\
a_{4,k+1}&=2a_{0,k}a_{4,k}+2a_{1,k}a_{3,k}+a_{2,k}^2.
\end{aligned}
\]

Multiply \(a_j\) by \(\rho_c^j\). This tells you where second, third, fourth, … parameter orders become important along the tail. Running this inexpensive scalar probe over every atlas-patch center would give you an empirical map of the required \(q\) before you enlarge the atlas format at all.

That is the experiment I would do first.

## What the literature says about order, long iteration counts, and splitting

There is a surprisingly consistent message across accelerator optics and astrodynamics: **higher order helps until it does not; once the domain has become too distorted, subdivision/re-expansion beats simply increasing polynomial degree.**

Accelerator map work routinely goes beyond first or second order. Makino's long-term-stability work states that derivatives through roughly order ten in six phase-space variables are typically needed for weakly nonlinear beam dynamics. The same work combines DA maps with rigorous remainder bounds for repetitive systems. citeturn25view0 COSY's early demonstration computed a ninth-order map of a complicated magnet, and later FFAG work explicitly discusses arbitrary-order parameter-dependent maps and compares lower- and eleventh-order tracking. citeturn25view3turn17search4

But accelerators generally **do not form one polynomial approximating the millionth iterate**. The economical object is the one-turn or one-cell map. Once that map is available, it is repeatedly evaluated, composed, symplectified, or transformed to normal form. The ICAP FFAG presentation states this rationale directly: for a repetitive system only one cell transfer map must be computed, making subsequent tracking much faster than ray-by-ray integration. citeturn17search4

That distinction matters for your design. There is no requirement that a 2000-iteration tail become one enormous \(Z_{2000}(w,\delta)\). You can instead have, for example,
\[
H_1,\ H_2,\ldots,H_s,
\]
where each \(H_r\) represents 25–100 original iterations in local state and parameter coordinates. At runtime a pixel passes through perhaps 10–40 inexpensive polynomial maps instead of 2000 high-precision iterations.

Astrodynamics supplies the strongest evidence for doing this adaptively. Wittig, Di Lizia, Armellin, Makino, Bernelli-Zazzera and Berz found that when nonlinearities cause the current Taylor representation to lose accuracy, splitting the domain and giving each child its own expansion restores accuracy; their algorithm triggers a split when an estimate of truncation error reaches a threshold. citeturn18view3

Their particularly relevant observation is that **merely raising polynomial order can fail**. In one orbital example, a sixth-order global expansion had become inaccurate after about two nominal revolutions, and a single fourteenth-order map still did not fix the problem. Splitting the initial domain produced a collection of local Taylor maps that did. citeturn24view2 This is almost a controlled experiment demonstrating why a fixed global order is the wrong knob once nonlinear stretching dominates.

Recent astrodynamics also treats convergence diagnostically rather than assigning an order from elapsed time. Acciarini et al. compare ratio and multivariate Cauchy-Hadamard estimates of convergence radius; in their orbital cases, through order five the Cauchy-Hadamard estimate was the more stable diagnostic. They also give the symmetry-reduced number of high-order variational equations for a six-state problem through orders one to five as 42, 168, 504, 1260 and 2772. citeturn23view2

For your one-complex-parameter problem the combinatorics are much milder, but the methodological lesson is identical: **look at coefficient shells and error bounds, not \(n\).**

### A practical parameter-order policy for your atlas

I would use an adaptive rule roughly like this.

For each normalized patch compute parameter shells
\[
S_j=\sup_{|\xi|\le1}
\left|P_j(\xi)\right|
\]
or a safe bound on them. The actual order-\(j\) contribution is already scaled if \(\eta=\delta/\rho_c\).

Compute two or three **guard orders** beyond the order you intend to store. Thus, if considering \(q=4\), actually propagate through six or seven offline. Accept a truncation at \(q\) only when the guard shells demonstrate that the omitted tail is beneath your required absolute error.

A simple non-rigorous criterion is
\[
r_j=\frac{S_j}{S_{j-1}},
\]
and, if the last few ratios are safely below one,
\[
E_{\rm tail}
\approx
\frac{S_{q+1}}{1-r}
\]
as a heuristic. This should not be used as a proof because Taylor coefficients can have nonmonotone shells.

For certification, replace it with a Taylor-model bound or a Cauchy bound on a slightly larger complex parameter disk.

For your scale, I would initially generate through **parameter order six** everywhere. Store only the smallest \(q\) certified for each patch, probably two through six in ordinary regions. Allow eight through twelve as an exceptional range. Once order twelve still does not give rapid shell decay, split the parameter patch or shorten the transfer-map segment instead of going to order twenty.

That order policy is an engineering recommendation, not something established in the accelerator literature. What *is* established is the strategy of balancing polynomial order against subdivision rather than treating order as the sole accuracy control. citeturn18view3turn17search2

### Why Chebyshev is probably not the main answer

Parametric reduced-basis methods contribute two valuable ideas: an expensive **offline** construction followed by a very small **online** evaluation, and an error indicator that adaptively enriches the parameter representation. Grepl and Patera's reduced-basis method, for example, constructs its parameter sample set greedily offline and produces an online output and rigorous error bound whose cost depends on the reduced dimension and parameter complexity, not the original full model. citeturn23view1

That is exactly the architecture your atlas should have.

I do **not**, however, think replacing your parameter Taylor series by Chebyshev polynomials is the primary fix. Your dependence on one complex parameter is holomorphic, your parameter radius is tiny, and derivatives are essentially free through jet transport. A centered complex Taylor expansion exploits all of that. Tensor Chebyshev in \((\Re c,\Im c)\) would throw away the holomorphic structure and turn one complex variable into two real variables.

Chebyshev becomes attractive if you want a relatively wide, preassigned parameter rectangle, cannot conveniently differentiate the generating code, or want one interpolation representation spanning several Taylor neighborhoods. For the \(10^{-15}\)-scale neighborhood you describe, **Taylor plus adaptive recentering is the natural representation; the useful ROM lesson is the offline/online and certification architecture, not the polynomial basis.**

## Certification, re-expansion, and a renderer-ready Taylor-model design

Taylor models provide the cleanest answer to “how do I know this patch is still valid?” They represent a function as
\[
F(x)\in P(x)+I,
\]
where \(P\) is a high-order polynomial and \(I\) is an interval or ball rigorously enclosing everything omitted: truncation, arithmetic error and other enclosed uncertainty. Berz and Makino developed this specifically to retain high-order functional dependence over long validated integration while avoiding the catastrophic dependency growth of ordinary interval arithmetic. citeturn17search2turn25view0 TaylorModels.jl implements the same polynomial-plus-interval concept, and FLINT/Arb supplies rigorous real and complex ball arithmetic at arbitrary precision. citeturn23view4turn21search2turn21search12

Your quadratic recurrence makes the rigorous remainder particularly simple.

Suppose over a normalized \((w,\delta)\)-patch you know
\[
Z=P+e,\qquad |e|\le r,
\]
and let
\[
B\ge \sup |P|.
\]
Let
\[
Q=\operatorname{Trunc}\!\left(P^2+C+\delta\right)
\]
be the polynomial retained at the next iteration, and let
\[
\tau\ge
\left|
P^2+C+\delta-Q
\right|
\]
bound discarded polynomial terms plus coefficient-rounding errors. Then
\[
\begin{aligned}
Z^2+C+\delta-Q
&=2Pe+e^2+(P^2+C+\delta-Q),
\end{aligned}
\]
so a valid new scalar remainder is
\[
\boxed{
r_{\rm new}
\le
2Br+r^2+\tau.
}
\]

That formula is exact elementary error propagation. It gives you a tailor-made Taylor-model recurrence requiring no generic ODE machinery.

For a normalized bidisk \(|\xi|,|\eta|\le1\), the extremely cheap coefficient majorant
\[
B\le\sum_{i,j}|b_{ij}|
\]
is rigorous, though often conservative. Tighter polynomial range bounds can use subdivision or Bernstein representations; pyaudi's Taylor-model implementation specifically uses Bernstein polynomials for fast multivariate polynomial bounding. citeturn23view3

### Re-expansion should be a first-class atlas operation

When a polynomial gets poorly conditioned, do not discard everything and recompute its orbit from scratch. Taylor shifting is exact for the stored polynomial.

For a parameter shift
\[
\delta=\delta_0+\hat\delta,
\]
\[
\sum_j P_j(w)\delta^j
=
\sum_k \hat P_k(w)\hat\delta^k,
\]
with
\[
\hat P_k(w)
=
\sum_{j=k}^{q}
{j\choose k}
P_j(w)\delta_0^{j-k}.
\]

The same applies to \(w\). Thus every atlas record can support:

\[
(w_0,C,\rho_w,\rho_c)
\longrightarrow
(w_0',C',\rho_w',\rho_c')
\]
without repeating the original 400–2000 map iterations, provided the parent polynomial/remainder is valid over the new domain.

This is the computational counterpart of automatic domain splitting. Astrodynamics literally maintains a list of different local polynomial expansions, each covering a subset of the original uncertainty domain. citeturn18view3turn24view2

### A concrete atlas record

I would make every patch conceptually contain:

```text
center_w
center_c
radius_w
radius_c

w_degree = p
parameter_degree = q

coeff[j][i]          # coefficient of eta^j xi^i
remainder_radius     # certified complex absolute error

segment_length       # number of original f_c iterations represented
next_patch / segment metadata

quality:
    last_parameter_shells
    last_w_shells
    supnorm_bound
    condition flags
```

The important design choice is that **\(q\) need not be uniform**. A patch that certifies at quadratic parameter order should not pay for eighth order merely because another patch passes near a near-parabolic bottleneck.

### Offline rigorous, online fast

You do not necessarily have to carry interval/ball arithmetic per pixel.

A practical arrangement is:

1. Generate coefficients offline with complex ball arithmetic at sufficiently high precision.
2. Prove a validity radius and remainder for each atlas record.
3. Store ordinary midpoint coefficients plus the certified metadata.
4. At runtime, ordinary high-precision/SIMD polynomial evaluation is allowed whenever the pixel lies inside the certified domain.
5. Fall back or choose a finer patch when it does not.

Arb's contract is precisely inclusion arithmetic: a result ball encloses the exact result for all points in the input balls. It supports arbitrary-precision complex balls and truncated complex polynomial arithmetic. citeturn21search2turn21search8turn21search12

Given the \(\sim10^{-30}\) absolute spatial scale, about 100 binary bits correspond merely to representing that scale; cancellation and long-tail amplification call for guard precision beyond that. Something like 160–256-bit offline coefficient generation would be a sensible starting experiment, but that bit count is an engineering recommendation rather than a literature-derived requirement.

## Near-parabolic dynamics and critical-point passages

These two regimes should not be treated alike.

### Near a critical point, forward jets are regular

For
\[
f_c(z)=z^2+c,
\]
a passage near \(z=0\) is **not a singularity of your forward transfer map**. Indeed
\[
\frac{\partial z_{k+1}}{\partial z_k}=2z_k,
\]
so state perturbations are locally *contracted* as \(z_k\to0\).

Parameter sensitivity obeys
\[
u_{k+1}=2z_k u_k+1,
\qquad
u_k=\partial_cz_k.
\]
Thus the dependence on \(c\) does not disappear at the critical point; the additive \(+1\) effectively resets first-order parameter sensitivity. At second order,
\[
a_{2,k+1}=2z_k a_{2,k}+a_{1,k}^2,
\]
so curvature with respect to \(c\) can remain important even while state sensitivity contracts.

Therefore I would **not automatically split a forward Taylor patch merely because an orbit approaches the critical point**. Split when a genuine remainder/coefficient criterion says the patch is deteriorating.

Critical points become troublesome when your algorithm asks for an **inverse** map or a locally invertible coordinate, because \(f'_c=0\) there. This is one reason a transfer-map chain should preferably propagate forward and reserve inverse-coordinate operations for neighborhoods where their Jacobian is safely separated from zero.

### Near a parabolic cycle, the coordinate system really does become ill-conditioned

A near-parabolic multiplier is much more serious.

For a local conjugacy
\[
F(h(w))=h(\lambda w),
\]
the coefficient equations contain divisors of the form
\[
\lambda^m-\lambda
=
\lambda(\lambda^{m-1}-1).
\]
These become small when \(\lambda\) approaches one or another resonance. This is the familiar homological-equation obstruction behind normal-form resonance and small-divisor problems. Standard linearization theory cleanly distinguishes the hyperbolic \(|\lambda|\ne1\) case from neutral multipliers, where small divisors enter. citeturn17search3

Accelerator normal-form algorithms handle the analogous situation by **not trying to eliminate resonant terms**. Resonant monomials survive in the normal form rather than being divided by a small homological denominator; high-order DA normal forms are used precisely to expose such nonlinear/resonant structure. citeturn17search1turn17search5

For your quadratic 2-cycle there is an unusually helpful exact simplification. The exact period-two points, excluding fixed points, satisfy
\[
z^2+z+c+1=0,
\]
so
\[
p_\pm(c)
=
\frac{-1\pm\sqrt{-3-4c}}{2}.
\]
The two points have product
\[
p_+(c)p_-(c)=c+1,
\]
and therefore the multiplier of
\[
F_c=f_c^2
\]
on the 2-cycle is exactly
\[
\boxed{\lambda(c)=4(c+1).}
\]

Thus the parabolic degeneration \(\lambda=1\) occurs at
\[
c=-\frac34,
\]
where the two period-two points coalesce. This also makes transparent why analytic continuation of the cycle becomes badly conditioned there: the square-root branch degenerates, while the fixed-point equation for \(F_c\) loses the nondegeneracy condition \(1-\lambda\ne0\).

Consequently, for near-parabolic patches I would use three escalating responses:

**First:** shorten your transfer-map segments and tighten the parameter radius.

**Second:** abandon a parameter-dependent Koenigs chart before its homological denominators become badly conditioned, while continuing to use direct \((w,\delta)\) forward maps, which remain analytic for every finite iterate.

**Third:** for patches that truly live in a parabolic bottleneck, use a parabolic/Fatou-coordinate treatment rather than trying to make a high-order hyperbolic Koenigs expansion survive \(\lambda\to1\).

The first two are direct adaptations of established transfer-map practice. The third is mathematically natural complex-dynamics machinery but would be a substantial renderer-specific development.

An important conceptual point follows. For your **fixed \(C\) initial chart**,
\[
f_{C+\delta}^{\,n}\!\left(A+\psi_C(w)\right)
\]
has no finite-\(\delta\) analytic singularity for finite \(n\): \(f_c^n(z)\) is a polynomial in \(c\), and composition with an entire Poincaré function in \(w\) remains entire in \(w\). The problem is therefore not a finite radius of analyticity of the raw iterate. It is **coefficient growth, numerical conditioning, and the inefficiency of a low-degree local polynomial over the chosen domain**.

That is exactly the situation in which splitting and re-expansion are effective.

## Parameter-dependent Poincaré linearizers may be the strongest solution

There are two different questions here:

1. **Does a repelling cycle and its linearizer depend analytically on the parameter?**
2. **Do practitioners routinely compute that parameter-dependent linearizer as a joint Taylor map?**

The answer to the first is essentially yes away from degeneracy. The answer to the second is “analogous calculations are routine, but this particular complex-dynamics use is not standard practice.”

### What is proven

For analytic families, a periodic point satisfying the nondegeneracy condition
\[
(f_c^n)'(p)\ne1
\]
continues analytically with the parameter by the implicit-function theorem. Mañé, Sad and Sullivan's stability theory makes analytic parameter motion of periodic points and holomorphic motions central to analytic families of rational maps; their construction gives parameter-analytic motion on stable Julia sets. citeturn19view1turn20view0

For a polynomial with a repelling fixed point, the normalized Poincaré linearizer extends from its local Koenigs germ to an **entire function** satisfying
\[
F(L(w))=L(\lambda w).
\]
This is standard Poincaré-function theory. citeturn9search0

Separately, the parameterization-method literature proves regularity with respect to parameters for invariant manifolds and formulates the invariance problem as
\[
F\circ K=K\circ R.
\]
Cabré, Fontich and de la Llave's parameterization-method papers explicitly study parameter regularity, and more recent computational work implements bivariate Taylor expansions in manifold coordinates and parameters. citeturn4search4turn7search7turn18view2

These facts strongly support a local joint expansion
\[
L(w,\delta)=L_{C+\delta}(w).
\]

### The concrete coefficient algorithm

Let
\[
F_c=f_c^2,
\]
let \(p(c)\) be your selected 2-cycle point, and let
\[
\lambda(c)=F_c'(p(c))=4(c+1).
\]

Normalize
\[
L_c(0)=p(c),\qquad
\partial_w L_c(0)=1
\]
and write
\[
L_c(w)
=
p(c)+
w+
\sum_{m=2}^{M}\ell_m(c)w^m.
\]

Then expand each coefficient as
\[
\ell_m(C+\delta)
=
\sum_{j=0}^{q}\ell_{m,j}\delta^j.
\]

Substitute into
\[
F_{C+\delta}\!\left(L_{C+\delta}(w)\right)
=
L_{C+\delta}\!\left(\lambda(C+\delta)w\right).
\]

At each \(w\)-degree \(m\), all nonlinear terms built from lower \(w\)-degrees are already known. The unknown \(\ell_m(\delta)\) enters a linear homological equation whose leading divisor is essentially
\[
\lambda(\delta)^m-\lambda(\delta).
\]
You then perform the division in the truncated parameter-series algebra. This gives a triangular recursion in \(m\), with a secondary truncation at parameter degree \(q\).

This is mathematically the same pattern as the modern parameter-dependent invariant-manifold algorithms: compute the parameter-dependent anchor/fixed point first, then solve nested linear problems for successive manifold-coordinate and parameter orders. Jain and Li explicitly compute a parameter-dependent fixed point as
\[
x^\star(\mu)=\sum_\ell x^\star_\ell\delta^\ell
\]
and obtain each new coefficient by a linear solve whose right-hand side depends only on lower orders; they then apply a bivariate ansatz to the manifold and reduced dynamics. citeturn18view1turn18view2

For your quadratic 2-cycle this is easier still because \(p(c)\) and \(\lambda(c)\) are available algebraically rather than through an implicit numerical solve.

### The exact long-tail collapse

Here is the potentially transformative part.

If
\[
L_c(w)=A(c)+\psi_c(w)
\]
is the Poincaré function for
\[
F_c=f_c^2,
\]
then
\[
F_c(L_c(w))=L_c(\lambda(c)w)
\]
immediately implies
\[
\boxed{
F_c^m(L_c(w))
=
L_c\!\left(\lambda(c)^m w\right).
}
\]

This is not an approximation. It follows exactly by induction from the functional equation.

Therefore a 2000-iteration even tail, when represented in the *correct parameter-dependent Poincaré coordinate*, does not intrinsically require propagation through 2000 bivariate jet steps. It becomes one multiplication of the linearizing coordinate by \(\lambda(c)^{1000}\), followed by one evaluation of \(L_c\). An odd final iterate adds one application of \(f_c\).

Moreover, in your quadratic 2-cycle case,
\[
\lambda(c)^m
=
\lambda(C)^m
\left(
1+\frac{4\delta}{\lambda(C)}
\right)^m,
\]
so its parameter Taylor coefficients are known explicitly:
\[
\lambda(c)^m
=
\lambda(C)^m
\sum_{j=0}^{m}
{m\choose j}
\left(\frac{4\delta}{\lambda(C)}\right)^j.
\]

For \(|\delta|\sim10^{-15}\) and \(m\lesssim1000\), the nominal expansion parameter arising *just from the multiplier* is on the order of
\[
m\,\frac{4|\delta|}{|\lambda|}
\sim10^{-12}
\]
when \(|\lambda|=O(1)\). Thus **large iteration count by itself does not make the parameter dependence of the multiplier high order**. The difficult parameter dependence must come from the coordinate transformation \(L_c\), from evaluating it at a highly amplified argument, or from portions of the orbit that are not usefully represented in that chart.

This is, in my view, the most important structural opportunity in your problem.

### You need a coordinate-change map because your current chart is fixed at \(C\)

A pixel begins in your existing fixed-\(C\) coordinate,
\[
z=L_C(w),
\]
not generally in
\[
L_c(w).
\]

So introduce a near-identity parameter-dependent coordinate change
\[
T(w,\delta)
=
L_{C+\delta}^{-1}\!\left(L_C(w)\right)
\]
on the local neighborhood where the inverse is univalent. Then
\[
L_C(w)
=
L_c(T(w,\delta)),
\]
and hence
\[
\boxed{
F_c^m(L_C(w))
=
L_c\!\left(
\lambda(c)^m T(w,\delta)
\right).
}
\]

Both \(L_c\) and \(T\) can be stored as bivariate jets. \(T\) can be obtained either by power-series reversion/composition or by solving
\[
L_c(T)=L_C
\]
coefficient by coefficient. FLINT's complex polynomial API already includes truncated composition and series reversion primitives useful for exactly this sort of calculation. citeturn21search0turn21search4

There are two caveats.

First, the inverse \(L_c^{-1}\) is only locally single-valued even though the Poincaré function itself is entire. You therefore want to perform this chart transition while the initial point is safely in the local linearization neighborhood.

Second, near \(\lambda=1\), both continuation of the cycle and homological equations become ill-conditioned, so this is precisely where a co-moving Koenigs chart is least attractive.

### Is this done “in practice”?

Parameter-dependent invariant manifolds: **yes**. They are explicitly computed as joint polynomial expansions in internal coordinates and parameters, including in current work. citeturn18view0turn18view2

Parameter-dependent accelerator normal forms/maps: **yes in the broader sense**. COSY has long retained machine/system parameters inside high-order maps, and DA normal-form algorithms operate on those maps. citeturn25view3turn17search4

Holomorphic motion of repelling Julia dynamics: **well-established theory**, not principally a high-performance numerical representation. Mañé-Sad-Sullivan establish analytic parameter motion in stable families; it is valuable theoretical justification for continuation, but it is not the algorithm I would use to evaluate millions of pixels. citeturn19view1turn20view0

A jointly computed \(L_c(w)\) specifically as a Mandelbrot deep-zoom transfer-map primitive: I did not find evidence that this is established computational practice. The mathematics and the computational analogies are strong, but **the claim that it will materially outperform your direct bivariate atlas is a renderer-specific hypothesis that needs benchmarking**.

## Recommended architecture for your renderer

The literature points quite strongly to a design that is more flexible than “add second order in \(c\) everywhere.”

### Core representation

For each existing spatial atlas patch, store
\[
Z(\xi,\eta)
=
\sum_{j=0}^{q}
\eta^j
\left(
\sum_{i=0}^{p}a_{ij}\xi^i
\right),
\]
with
\[
\xi=\frac{w-w_0}{\rho_w},
\qquad
\eta=\frac{c-C_{\rm patch}}{\rho_c}.
\]

Use **independent \(p,q\)** and exploit complex holomorphy.

Generate through a few guard parameter orders and retain only the smallest certified \(q\).

### Two complementary atlas types

I would maintain two map types.

**Direct transfer patches** cover arbitrary dynamical regions:
\[
(w,c)\mapsto f_c^L(z(w))
\]
for an adaptively chosen segment length \(L\). These are the robust fallback and continue to work near critical points and near parameter bifurcations because finite forward iterates themselves are entire.

**Poincaré-coordinate patches** cover regions tied to the repelling 2-cycle:
\[
(w,c)
\mapsto
L_c\!\left(\lambda(c)^m T(w,c)\right).
\]
These should replace hundreds or thousands of raw iterations whenever their conditioning test is favorable.

That is analogous to accelerator practice: ordinary transfer maps do the robust transport; normal forms supply a much more compressed representation in regions where the normal-form coordinate is well conditioned. citeturn17search4turn17search5

### Adaptive decisions should be based on dimensionless shell sizes

For every patch record
\[
S_{w,i},\qquad S_{c,j},
\]
the contribution bounds of the final few \(w\)- and \(c\)-degree shells on the normalized unit domains.

Use them to choose among four operations:

\[
\text{raise order}
\;\longleftrightarrow\;
\text{split in }w
\;\longleftrightarrow\;
\text{split in }c
\;\longleftrightarrow\;
\text{shorten the segment}.
\]

If parameter shells deteriorate while \(w\)-shells remain excellent, split/recenter \(c\), not \(w\). If both deteriorate after a particular iterate, shorten the transfer segment. If the Poincaré homological denominators become small, switch to direct maps.

This anisotropic adaptation is the major advantage of keeping parameter degree separate.

### A sensible prototype sequence

I would implement the research prototype in the following progression:

1. **Scalar parameter jets at patch centers through \(q=8\).** This costs almost nothing and tells you empirically where every parameter order turns on.
2. **Full \((w,\delta)\) jets through \(q=4\) or \(6\)** using your existing polynomial engine.
3. **Normalized parameter variables and guard orders**, so the atlas builder can choose \(q\) automatically.
4. **Offline Taylor-model remainder propagation** using
   \[
   r'\le2Br+r^2+\tau.
   \]
5. **Parameter splitting/recentering** for patches that fail.
6. **Segmented tails** for patches whose monolithic maps still require excessive \(q\).
7. **Joint \(L_c(w)\) and \(T(w,c)\)** around the 2-cycle and benchmark the exact linearized-tail factorization against direct segmented maps.
8. Only after those tests, consider specialized parabolic coordinates.

The specific prototype orders in this sequence are my recommendation, not a published standard.

A particularly informative benchmark would compare, patch by patch:

\[
\begin{array}{l}
\text{direct monolithic bivariate map},\\
\text{segmented bivariate transfer maps},\\
\text{parameter-dependent Poincaré factorization}.
\end{array}
\]

For each, record storage bytes, offline construction time, pixel evaluation time, maximum certified \(\rho_c\), and smallest \(q\) needed for a fixed \(10^{-30}\)-scale absolute error budget.

My expectation is that ordinary hyperbolic regions will favor the Poincaré factorization, awkward transient regions will favor segmented direct maps, and near-parabolic regions will force smaller parameter domains regardless of representation.

## Proven results, established practice, and speculation

The distinction matters here because some of the strongest ideas are mathematically sound but not yet demonstrated in your exact computational setting.

| Statement | Status |
|---|---|
| Finite iterates \(f_c^n(z)\) are polynomial in \(c\); with your fixed entire Poincaré chart the finite-\(n\) map is entire in \(w\) and \(c\) | **Proven directly from the algebra** |
| Promoting \(c-C\) to an independent Taylor/DA variable gives all state-parameter cross derivatives in one propagation | **Established practice** in accelerator physics and astrodynamics. citeturn25view3turn23view2 |
| Independent state and parameter expansion orders are computationally advantageous | **Established modern practice**, explicitly motivated in parameter-dependent manifold computation. citeturn18view0turn18view2 |
| High polynomial order alone does not guarantee long-domain accuracy; subdivision can outperform further order increase | **Established experimentally and algorithmically** in astrodynamics. citeturn18view3turn24view2 |
| Taylor polynomial plus a rigorous interval/ball remainder can certify a whole domain | **Established Taylor-model theory and software practice**. citeturn25view0turn17search2turn23view4 |
| Repelling periodic points move analytically with parameter away from multiplier \(1\); stable Julia dynamics admits holomorphic motion | **Proven complex-dynamics theory**. citeturn19view1turn20view0 |
| A repelling polynomial fixed point has an entire Poincaré linearizer | **Proven classical complex dynamics**. citeturn9search0 |
| Parameter-dependent invariant manifolds can be represented and computed as bivariate Taylor series | **Proven theoretically and established computationally**. citeturn4search4turn7search7turn18view2 |
| For the quadratic 2-cycle, \(\lambda(c)=4(c+1)\) and \(F_c^m(L_c(w))=L_c(\lambda(c)^m w)\) | **Exact algebraic consequence** |
| A close pass to \(z=0\) is not by itself a pathology for forward jets | **Exact consequence** of \(f'_c(z)=2z\); inverse representations are the problematic ones |
| A parameter-dependent Poincaré chart will substantially lower the parameter order required by your actual atlas | **Strongly motivated speculation** |
| \(q\approx3\)–\(6\) will usually suffice away from parabolic regions if splitting is allowed | **Engineering speculation requiring your data** |
| Patches still needing \(q>8\)–\(12\) should generally be split/resegmented rather than extended to ever higher order | **Engineering recommendation informed by transfer-map practice**, not a theorem |

The deepest transferable lesson is therefore not a magic polynomial order. It is an architecture:

\[
\boxed{
\text{holomorphic bivariate jets}
+
\text{independent parameter order}
+
\text{normalized domains}
+
\text{certified remainder}
+
\text{adaptive splitting/re-expansion}
+
\text{a normal-form/Poincaré shortcut where well-conditioned}.
}
\]

That combination directly addresses why your linear-in-\(c-C\) scheme dies after roughly 300 iterations without requiring a separate high-precision orbit for every pixel.

## References and software

**Berz / COSY INFINITY.** Berz's COSY work is the foundational accelerator implementation of arbitrary-order differential-algebra maps. The original COSY paper explicitly describes arbitrary-order maps including system-parameter dependence and normal-form tools; its ninth-order magnet example also gives a useful historical indication of how inexpensive map construction can be relative to point tracking. citeturn25view3turn23view0

**Makino and Berz, remainder-enhanced DA / Taylor models.** Makino's accelerator-stability work combines high-order transfer maps with rigorous interval remainder bounds and reports order-ten derivatives in six variables as typical for demanding weak nonlinearities. Berz and Makino's Taylor-model validated-integration work formalizes the polynomial-plus-remainder representation and techniques for limiting long-time interval blow-up. citeturn25view0turn17search2

**Wittig et al., automatic domain splitting.** “Propagation of large uncertainty sets in orbital dynamics by automatic domain splitting,” *Celestial Mechanics and Dynamical Astronomy* 122 (2015), is probably the single most directly useful astrodynamics paper for your atlas design: arbitrary-order maps, a truncation indicator, automatic subdivision, and a concrete example where simply increasing global Taylor order fails. citeturn18view3turn24view2

**Acciarini et al., state plus parameter Taylor maps.** Their high-order uncertainty work gives a modern, explicit formulation of Taylor maps in joint initial-state and parameter perturbations, convergence-radius diagnostics and concrete high-order coefficient counts. citeturn23view2

**Cabré, Fontich and de la Llave, parameterization method.** Their series of papers formulates invariant manifolds through an invariance equation \(F\circ K=K\circ R\), including regularity with respect to parameters. This is the theoretical foundation closest to a joint \(K(w,c)\) or \(L(w,c)\) construction. citeturn4search4turn7search7

**Jain and Li, parameter-dependent invariant manifolds.** The 2026 computational paper is unusually relevant because it explicitly advocates separate expansion orders in state/manifold coordinates and parameter perturbations and gives nested recursions for bivariate Taylor coefficients. citeturn18view0turn18view1turn18view2

**Mañé, Sad and Sullivan.** Their analytic-family/J-stability work supplies the classical holomorphic-motion background: away from nonhyperbolic degeneracy, periodic dynamics can be continued analytically with parameter, and stable Julia dynamics moves holomorphically. citeturn19view1turn20view0

**FLINT/Arb.** This is probably the most directly useful low-level rigorous arithmetic library for your particular problem: arbitrary-precision complex balls (`acb`), complex polynomials (`acb_poly`), truncated products, composition, reversion and rigorous coefficient/range bounds. citeturn21search0turn21search2turn21search8

**AuDi/pyaudi.** Open-source C++/Python truncated Taylor polynomial algebra with complex support, sparse high-order representations, vectorized evaluation and a Taylor-model implementation using Bernstein bounds. A JOSS software paper was published in 2026. citeturn23view3

**TaylorModels.jl / TaylorSeries.jl.** TaylorSeries.jl supplies multivariate truncated polynomial algebra; TaylorModels.jl combines such polynomials with interval remainders for rigorous enclosures. citeturn21search7turn23view4

**DACE.** The Differential Algebra Computational Toolbox is another established C++ implementation aimed at high-order automatic differentiation, sensitivity and uncertainty propagation and is widely used in astrodynamics. citeturn12search0

**Reduced-basis methods.** Grepl and Patera's work is less directly relevant algebraically, but its offline/online decomposition, adaptive parameter sampling and rigorous online error estimator are exactly the right systems-level philosophy for a large rendering atlas. citeturn23view1