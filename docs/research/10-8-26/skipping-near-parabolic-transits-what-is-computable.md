# Skipping Near-Parabolic Transits in \(z^2+c\): What Is Actually Computable

## The main conclusion

There **are** analytically exact ways to replace a long block of near-parabolic iterations by one coordinate change, a translation, and an inverse coordinate change. For the multiplier-\(-1\) situation near \(c=-3/4\), the cleanest theorem is not the usual simple-parabolic \(q=1\) Fatou-coordinate theory applied blindly to \(f^2\), but the **root-of-unity \(p/q=1/2\) near-parabolic theory**. Kapiamba's recent formulation gives an exact identity of the form

\[
g^{\,2n+k}
=
\chi_\pm\circ T_{\,n-1/\alpha}\circ \rho
\]

on appropriate compact incoming sections. This is an actual finite-iterate identity for a fixed nearby map \(g\), not merely a limit theorem. citeturn16view1turn21view2

For the particular parameter in the question, however, there is an important practical wrinkle. Direct calculation gives

\[
\alpha_f\approx -0.498806114254+0.065678412521i,
\qquad
\lambda=f'(\alpha_f)
\approx-0.997612228508+0.131356825041i,
\]

with

\[
|\lambda|\approx1.006223024.
\]

Thus the fixed point is only **0.62% repelling per iterate in modulus**. A radius increase by a factor \(100\), if governed approximately by the linearized multiplier, takes

\[
\frac{\log 100}{\log|\lambda|}\approx742
\]

iterations; a factor \(1000\) takes about \(1113\). Consequently, an orbit spending hundreds of iterations near this parameter can be experiencing a long **weak-repelling dwell near the fixed point**, not merely the canonical Écalle “gate-crossing time.”

That distinction matters because, for this particular \(c\), the most economical accelerator is likely a **Koenigs/Schröder linearization around the repelling \(\alpha\) fixed point**, with near-parabolic Fatou coordinates used only for the part of the orbit that genuinely traverses the fixed-point/period-two gate. Koenigs' theorem locally conjugates a non-neutral fixed point to multiplication by its multiplier, so a block of \(N\) iterations becomes one multiplication by \(\lambda^N\). citeturn22search4turn22search3

The practical hierarchy is therefore:

| Construction | Finite nearby \(c\) | Can replace hundreds of iterates exactly as an analytic identity? | Numerical burden | Best role |
|---|---|---:|---|---|
| Koenigs/Schröder coordinate at \(\alpha\) | Yes | **Yes**, while orbit stays in its local linearization domain | Low–moderate | Long weak-repelling dwell |
| Near-parabolic Fatou coordinates | Yes | **Yes** | Moderate–high | True gate transit |
| Kapiamba relative Lavaurs/transition map | Yes | **Yes** under the near-parabolic flower hypotheses | High precomputation, cheap per orbit | Whole incoming-to-outgoing passage |
| Classical Lavaurs map | Limit parabolic map / tuned perturbation sequences | **No**, not by itself for one fixed nonzero perturbation | Moderate | Asymptotic model |
| Écalle–Voronin / horn map | Parabolic, or relative near-parabolic version | Exact invariant/transition data | High | Compress and reuse transition |
| Shishikura near-parabolic renormalization | Yes | Exact analytic re-encoding | Very high | Repeated/multiscale parabolic passages |
| Buff rectifying coordinate | Yes | Not exactly; it is a controlled near-translation | Low–moderate | Excellent numerical preconditioner |
| Braverman long-iterate Taylor method | Exact parabolic rational map | To arbitrary prescribed numerical accuracy | Moderate | Rigorous rendering acceleration |

The striking literature result for rendering is Braverman's: he built a **provably polynomial-time Julia-set algorithm specifically by replacing long parabolic runs with directly computed long iterates**, with arbitrary \(2^{-s}\) accuracy. But his acceleration is based on Taylor coefficients of \(f^\ell\), not Lavaurs/horn/Shishikura machinery. citeturn15view3turn15view5

## Why \(c=-3/4\) is a \(q=2\) problem, not an ordinary simple parabolic

Let \(a\) denote the fixed point near \(-1/2\), and put \(x=z-a\). Since \(a=a^2+c\),

\[
f(a+x)-a=\lambda x+x^2,\qquad \lambda=2a.
\]

Squaring gives

\[
f^2(a+x)-a
=
\lambda^2x+\lambda(1+\lambda)x^2+2\lambda x^3+x^4.
\]

Exactly at \(c=-3/4\),

\[
a=-\frac12,\qquad\lambda=-1,
\]

and hence

\[
f^2(a+x)-a=x-2x^3+x^4.
\]

So \(f^2\) has multiplier \(1\), but its first nonlinear term is **cubic**, not quadratic. It is a two-petal, degenerate multiplier-one germ. This is why importing formulas for a simple parabolic germ

\[
z\mapsto z+z^2+\cdots
\]

directly into \(f^2\) is awkward. Lanford and Yampolsky explicitly distinguish the general root-of-unity case from the simple \(q=1\) setting before specializing their asymptotic expansion to \(z+z^2+\alpha z^3+\cdots\). citeturn17view2turn17view3

The more natural object is \(f\) itself as a nondegenerate parabolic germ with multiplier

\[
e^{2\pi i(1/2)}=-1.
\]

Kapiamba's theory was developed precisely to handle perturbations of arbitrary roots of unity and supplies \(q\) cyclic petals with coordinates satisfying

\[
\phi_j\circ g^q=T_1\circ\phi_j.
\]

For \(q=2\),

\[
\boxed{\phi_j(g^2(z))=\phi_j(z)+1}
\]

wherever the two sides are defined. The coordinate images are genuine strips, and the petals have both the parabolic fixed point and nearby nonzero fixed points of \(g^2\) on their boundaries. citeturn21view0turn21view1

For your numerical parameter, the two genuine period-two points follow from

\[
z^2+z+c+1=0,
\]

giving approximately

\[
p_1=-0.250329253510-0.262746040223i,
\]

\[
p_2=-0.749670746490+0.262746040223i.
\]

Their \(f^2\)-multiplier is

\[
4(c+1)=1.0268+0.5248i,
\qquad |4(c+1)|\approx1.15314,
\]

so the two-cycle is appreciably more repelling than the fixed point. That again points toward the weakly repelling \(\alpha\) fixed point as the likely source of the very longest dwell times.

There is another useful quantitative check. Write

\[
\lambda=e^{2\pi i\mu},
\]

choosing \(\mu\) close to \(1/2\). For your multiplier,

\[
\mu\approx0.479163736762-0.000987356031i.
\]

Kapiamba parameterizes a perturbation of \(p/q=1/2\) by

\[
\mu^\pm_{1/2}(\alpha)
=
\frac12\pm\frac{\alpha}{2(2+\alpha)},
\]

because \(q=2\) and \(q'_\pm=1\). citeturn21view0

The branch with \(\Re\alpha>0\) gives

\[
\alpha_{\rm NP}
\approx0.0869604214595+0.00430035246970i,
\]

and therefore

\[
\frac1{\alpha_{\rm NP}}
\approx11.4714310834-0.5672833246i.
\]

Kapiamba's normalized incoming and outgoing coordinates differ by exactly

\[
\boxed{\phi_{\pm1}=T_{-1/\alpha_{\rm NP}}\circ\phi_0.}
\]

citeturn21view2

Thus the natural near-parabolic phase width for this parameter is of order \(11\) units of the \(f^2\)-translation coordinate, or roughly \(22\) raw \(f\)-iterations for a canonical gate passage. It is **not** intrinsically hundreds. The observed hundreds therefore almost certainly include a long approach to or departure from the gate at extremely small distance from the weak repeller.

That observation is practically important: do not build a full Shishikura renormalization engine before testing a much cheaper local linearizer.

## The constructions that really can skip the transit

### Koenigs linearization for the long weak-repelling dwell

For a fixed parameter with \(|\lambda|>1\), there is a local analytic coordinate \(K\), normalized by

\[
K(0)=0,\qquad K'(0)=1,
\]

such that for

\[
F(x)=\lambda x+x^2
\]

one has

\[
K(F(x))=\lambda K(x).
\]

Koenigs' linearization theorem gives precisely this local conformal conjugacy for a non-neutral holomorphic fixed point. citeturn22search4turn22search3

Consequently,

\[
\boxed{F^N(x)=K^{-1}\!\left(\lambda^N K(x)\right)}
\]

for every \(N\) for which the orbit remains in the chosen linearization domain.

That is an **exact analytic identity**. Numerically, one evaluation of \(K\), one complex exponential or power for \(\lambda^N\), and one inversion of \(K\) can replace 500 or 1000 quadratic iterations.

A Taylor representation is particularly simple:

\[
K(x)=x+k_2x^2+k_3x^3+\cdots.
\]

Substituting in

\[
K(\lambda x+x^2)=\lambda K(x)
\]

determines the coefficients successively. For example,

\[
k_2=\frac{1}{\lambda-\lambda^2}
=\frac1{\lambda(1-\lambda)}.
\]

Higher coefficients are obtained by ordinary truncated polynomial composition. The triangular equations contain near-resonant denominators involving powers of \(\lambda\); as \(\lambda\to-1\), odd-order resonances eventually make the coefficients badly conditioned. That deterioration is not a programming accident—it is exactly why parabolic/Fatou coordinates become the uniform construction at the limiting parameter.

For the stated \(c\), however, the map is not yet neutral, and a modest-order Koenigs series may cover a useful disk around \(\alpha\). A renderer can therefore use:

\[
z\longmapsto x=z-\alpha
\longmapsto K(x)
\longmapsto \lambda^N K(x)
\longmapsto K^{-1}
\longmapsto z.
\]

The safe \(N\) should be chosen so that the predicted endpoint remains comfortably inside a prevalidated subset of \(K\)'s univalent image. This is substantially simpler than computing an Écalle cylinder.

### Exact near-parabolic Fatou-coordinate skip

On a Kapiamba near-parabolic petal for \(q=2\),

\[
\phi\circ g^2=\phi+1.
\]

Therefore,

\[
\boxed{g^{2N}(z)=\phi^{-1}\!\bigl(\phi(z)+N\bigr)}
\]

whenever the entire branch is represented by the same petal coordinate. This is the most direct answer to “can Fatou coordinates exactly skip the transit?”: **yes, analytically**. citeturn21view1

The stronger construction handles the change from the incoming petal to an outgoing one. Kapiamba extends the attracting coordinate to a map \(\rho\) and the inverse outgoing coordinate to \(\chi_\pm\). The relative horn map is

\[
H_\pm=\rho\circ\chi_\pm,
\]

and \(H_\pm\) commutes with unit translation. It extends to a translation-invariant domain containing sufficiently high upper and lower half-planes. citeturn21view2

Most importantly, if \(X\) is a compact set in an incoming section and \(n\) differs from \(\Re(1/\alpha_{\rm NP})\) by a bounded amount, then for sufficiently small perturbations

\[
\boxed{
\chi_\pm\circ
T_{\,n-1/\alpha_{\rm NP}}\circ
\rho
=
g^{\,2n+k}
}
\]

on \(X\), with \(k\in\{0,1\}\) determined by the section. citeturn16view1

That formula is about as close as the theory gets to an abstract “skip the gate” instruction:

1. map the orbit into Écalle/Fatou coordinates with \(\rho\);
2. make one complex translation by \(n-1/\alpha_{\rm NP}\);
3. map back with \(\chi_\pm\).

No hundreds of pointwise polynomial iterations appear in the formula.

### Classical Lavaurs maps are a limit, not the exact finite-\(c\) shortcut

At the limiting parabolic map,

\[
L_\delta^\pm
=
\chi_\pm\circ T_\delta\circ\rho
\]

is a Lavaurs map. Lavaurs maps analytically encode passage through the parabolic point and are intertwined with the horn map and parabolic renormalization. citeturn21view0

But this distinction is essential:

**a classical Lavaurs map for the limiting parabolic polynomial is generally not equal to \(f_c^N\) for one fixed nearby \(c\).**

Rather, high iterates of specially tuned perturbation sequences converge to Lavaurs maps. Using a limiting Lavaurs map as a replacement for a finite nearby orbit therefore gives an **asymptotic approximation**, unless quantitative perturbation estimates are also carried along. Kapiamba's near-parabolic \(\rho,\chi_\pm\) identity above is the version that restores an exact finite-iterate identity for the perturbed map. citeturn16view1

So, for a renderer that requires bitwise-correct escape classification near a boundary, “just use the Lavaurs map of \(c=-3/4\)” is not sufficient.

### Horn maps and Écalle–Voronin data

A horn map records the mismatch between attracting and repelling Fatou coordinates. In a normalization where

\[
H(w+1)=H(w)+1,
\]

the difference

\[
H(w)-w
\]

is \(1\)-periodic. Near the ends of the Écalle cylinder it can therefore be represented in Fourier variables \(e^{\pm2\pi i w}\). Écalle–Voronin invariants occur precisely as the Fourier coefficients of these lifted horn maps; Dudko and Sauzin derive this from the resurgent Fatou-coordinate construction. citeturn21view3turn16view3

Schematically, at the upper end,

\[
H^{\rm up}(w)
=
w+\sum_{m\ge1} A_m e^{2\pi i m w},
\]

and at the lower end the negative Fourier modes occur instead. Because

\[
|e^{2\pi i m w}|
=e^{-2\pi m\,\Im w},
\]

this is numerically excellent sufficiently far up or down the cylinder: Fourier terms die exponentially with the Écalle height.

This can be a very good **compressed transition representation**. Compute the horn map once for a fixed Julia set, tabulate perhaps a few dozen Fourier coefficients, and later evaluate it very cheaply per orbit.

The drawback is that a horn map is not a magical way to avoid computing Fatou coordinates in the first place. One must still determine the incoming coordinate, evaluate or analytically continue the horn map, and invert an outgoing coordinate.

### Shishikura near-parabolic renormalization

Kapiamba writes the near-parabolic renormalization as

\[
\boxed{
\mathcal R_f^\pm g
=
\operatorname{Exp}_\pm
\circ H_\pm
\circ T_{-1/\alpha_{\rm NP}}
\circ
\operatorname{Exp}_\pm^{-1}
}
\]

and obtains

\[
(\mathcal R_f^\pm g)'(0)
=
\operatorname{Exp}_\pm(-1/\alpha_{\rm NP}).
\]

citeturn16view1

This is extraordinarily powerful conceptually: an entire near-parabolic transit is compressed and exponentiated into a new dynamical germ. It is the right framework for repeated near-parabolic behavior, parabolic implosion, continued-fraction towers, and small-scale universality. Kapiamba's treatment also corrects a subtle phase formula in the older general-\(q\) Shishikura formulation: for \(q>1\), the phase is not simply the naive \(-1/\beta\) parameter used in some older statements. citeturn21view2

For **one escape-time orbit at one fixed parameter**, though, full renormalization is usually overkill. It does not eliminate the need to compute the constituent Fatou/horn coordinate maps; it reorganizes them. I would use it as a renderer accelerator only if the application repeatedly encounters nested near-parabolic passages and can amortize an expensive precomputation.

## How to compute these objects in double precision

There are three practically useful numerical routes, and their convergence properties are quite different.

### Direct near-parabolic lift

Kapiamba's appendix is more constructive than the abstract terminology suggests.

For a \(q\)-root-of-unity perturbation, \(g^q\) has \(q\) nearby nonzero fixed points \(\sigma_{2j}\). Kapiamba constructs a polynomial

\[
\psi_g(z)=z^q(s_0+s_1z+\cdots+s_{q-1}z^{q-1})
\]

of degree at most \(2q-1\) that maps all those nonzero fixed points to one common value \(\sigma\). The coefficients require solving only \(q\) linear equations. citeturn16view2

For the present case \(q=2\), this means:

- locate the two period-two points, which are explicitly quadratic;
- center them at \(\alpha_f\);
- solve a **\(2\times2\)** complex linear system;
- obtain a polynomial of degree at most \(3\).

That part is trivial in binary64.

Next define

\[
\alpha'
=
\frac{2\alpha_{\rm NP}}{2+\alpha_{\rm NP}}.
\]

For the stated parameter,

\[
\alpha'
\approx0.083345052952+0.003949424124i,
\]

\[
\frac1{\alpha'}
\approx11.9714310834-0.5672833246i.
\]

The universal covering used in the construction is

\[
\boxed{
\tau_g(w)
=
\frac{\sigma}
{1-\exp(-2\pi i\alpha'w)}
}
\]

in the convention \(\operatorname{Exp}(z)=e^{2\pi iz}\). Kapiamba then lifts the dynamics through \(\psi_g\) and \(\tau_g\) and obtains maps that approach translation; their \(q\)-step version has the form

\[
G_j^q(w)=w+1+O(w^{-2})
\]

at the appropriate ends. citeturn16view2

The actual lifted step can be evaluated through logarithms of rational combinations of

\[
\psi_g(z),\quad
\psi_g(g^n(z)),\quad
\sigma,
\]

with a consistently chosen logarithm branch. The construction then produces an analytic coordinate \(\Phi_j\) satisfying

\[
\Phi_j\circ G_j^2=\Phi_j+1,
\]

and the physical Fatou coordinate is obtained by composing \(\Phi_j\) with the covering and polynomial coordinate changes. citeturn16view2

Numerically this is attractive because most operations are ordinary complex arithmetic, a cubic polynomial, exponentials, logs and one Abel-coordinate correction. The difficult object is not \(\tau_g\) or \(\psi_g\); it is the final solution of the Abel equation.

### Asymptotic Fatou series

For a simple parabolic germ normalized as

\[
f(z)=z+z^2+\gamma z^3+O(z^4),
\]

set

\[
w=-\frac1z.
\]

Then

\[
F(w)=w+1+\frac{A}{w}+O(w^{-2}),
\qquad A=1-\gamma,
\]

and a formal Fatou coordinate has the form

\[
\boxed{
\Phi(w)
=
w-A\log w+
\sum_{j\ge1} b_jw^{-j}.
}
\]

The coefficients \(b_j\) are obtained **recursively** simply by substituting into

\[
\Phi(F(w))=\Phi(w)+1
\]

and matching powers of \(1/w\). At truncation order \(N\),

\[
\Phi_N(F(w))-\Phi_N(w)-1
=
O(w^{-N-2}),
\]

while the actual Fatou coordinate differs from the truncated expansion by

\[
O(|w|^{-N-1})
\]

uniformly inside any proper sector avoiding the logarithmic cut. citeturn17view2turn17view3

For the \(q=2\) situation one should not apply this formula directly to the cubic \(f^2\) germ. Instead, Kapiamba's covering/lift reduces the root-of-unity problem to a near-translation \(G_j^2(w)=w+1+O(w^{-2})\), after which the same style of Abel-equation expansion is numerically applicable. citeturn16view2

There is a crucial answer to the question about **convergence radii**:

> The Fatou series is an **asymptotic Gevrey-1 series**, not an ordinary convergent Taylor series with a useful disk radius.

Lanford and Yampolsky explicitly describe it as a slowly divergent Gevrey-1 expansion and estimate that roughly the first \(N\) terms are useful when \(|w|\) is larger than a constant times \(N\). Its Borel transform is the object that converges near the origin and can be analytically continued in the Borel plane, avoiding the lattice \(2\pi i\mathbb Z^*\). citeturn16view3

So the right numerical rule is **optimal truncation**, not “evaluate until terms stop because the series converges.”

Lanford and Yampolsky give a useful real implementation benchmark. In their computation of parabolic renormalization they choose

\[
M\approx100,
\]

evaluate the attracting asymptotic coordinate for \(\Re w\ge M\), the repelling one for \(\Re w\le-M\), and bridge the gap with about

\[
N\approx2M
\]

ordinary iterates. They then sample the resulting analytic map on a circle and recover Taylor coefficients by a discrete Fourier transform. citeturn17view0

That is a very sensible pattern for a renderer as well:

\[
\text{series far away}
\;\to\;
\text{a few exact iterates}
\;\to\;
\text{series/inverse far away}.
\]

It is safer and simpler than demanding that a single series span the whole gate.

Their published double-precision computation of the **parabolic-renormalization fixed point** was empirically reliable to about

\[
10^{-14}
\]

in a disk of radius \(5\); they estimated a radius of convergence near \(41\) for the Taylor series of that particular renormalization fixed point. Those numbers should **not** be confused with a universal Fatou-coordinate convergence radius: the \(10^{-14}\) result concerns their specific computed renormalization map, while the Fatou expansion itself is asymptotic. citeturn17view1

### Buff's rectifying coordinate is the easiest near-parabolic approximation

Petersen and Zakeri's 2024 Buff-form construction is particularly attractive for practical code because it gives an explicit coordinate built directly from \(f\) and \(f'\):

\[
\boxed{
\omega_f(z)
=
\frac{f'(z)-1}
{(f(z)-z)\,\Log f'(z)}
\,dz.
}
\]

citeturn18view4

Its primitive

\[
\phi_f(z)=\int\omega_f
\]

rectifies the map almost to translation. In this coordinate the lifted dynamics is

\[
Z\longmapsto Z+1+u_f(z),
\]

where

\[
\boxed{
u_f(z)
=
-1+
\int_{[z,f(z)]}\omega_f(s).
}
\]

The defect \(u_f\) extends holomorphically through fixed points and vanishes there. At a parabolic fixed point of multiplicity \(q+1\), the defect has a zero of order \(2q\). citeturn19view0

For perturbations of a primitive \(q\)-th root of unity, applying this to

\[
F=g^q
\]

produces a remarkably concrete decomposition:

\[
\boxed{
\phi_F(z)
=
\Lambda\log z
+
M\sum_{j=1}^{q}\log(z-p_j)
+
E(z),
}
\]

where

\[
\Lambda=\frac1{\Log \lambda^q},
\qquad
M=\frac1{\Log\mu},
\]

\(\mu\) is the multiplier of the bifurcated \(q\)-cycle, and \(E\) is analytic in a fixed disk. citeturn20view0turn20view1

For your \(q=2\) example, direct substitution gives approximately

\[
\Lambda
=
\frac1{\Log\lambda^2}
\approx
0.1805718905+3.8106248656i,
\]

and with period-two multiplier \(\mu=4(c+1)\),

\[
M
\approx
0.5850509900-1.9400123157i.
\]

Thus most of the coordinate is just **three complex logarithms**: one at the fixed point and one at each point of the nearby two-cycle.

The analytic correction \(E\) is also computationally friendly. Petersen and Zakeri express its derivative as the analytic remainder after subtracting the explicit pole terms, and give a Cauchy-integral representation on an enclosing circle. citeturn20view0

So a concrete implementation can precompute Taylor coefficients of \(E\) once, by sampling that analytic remainder on a circle, and thereafter evaluate

\[
\Lambda\log z
+
M\log(z-p_1)
+
M\log(z-p_2)
+
E(z)
\]

very cheaply.

The catch is that this Buff coordinate is **not itself an exact Fatou coordinate**. Petersen and Zakeri prove instead that, for every prescribed \(0<\varepsilon<1\), one may choose a sufficiently small near-parabolic disk so that

\[
|F_{\rm lift}(Z)-(Z+1)|<\varepsilon
\]

per step; their stronger path estimate is

\[
|Z_t-(Z_0+t)|<\varepsilon t.
\]

citeturn18view3turn19view1

So Buff coordinates are excellent for:

- predicting the exit coordinate;
- constructing initial guesses for an exact Fatou coordinate;
- cheaply determining which side of the gate an orbit will follow;
- preconditioning Newton inversion.

They do **not**, without a correction, justify replacing \(N\) steps by exactly \(Z\mapsto Z+N\). A crude accumulated error bound would be of order \(N\varepsilon\); a much sharper method is to compute the Abel correction from the known defect \(u_f\).

## Turning floating-point formulas into rigorous error bounds

The analytic identities above are exact, but an IEEE-754 double evaluation of a Fatou coordinate is obviously not exact. Fortunately the functional equation itself gives a convenient a posteriori validation mechanism.

Suppose a truncated or numerically fitted coordinate \(\widetilde\Phi\) has residual

\[
R(w)
=
\widetilde\Phi(F(w))
-\widetilde\Phi(w)-1.
\]

For the asymptotic expansion of order \(N\),

\[
R(w)=O(w^{-N-2}).
\]

citeturn17view3

On an attracting sector, define formally

\[
h(w)
=
\sum_{j=0}^{\infty}R(F^j(w)).
\]

Then

\[
h(F(w))-h(w)=-R(w),
\]

so

\[
\Phi=\widetilde\Phi+h
\]

satisfies the Abel equation exactly.

This is not merely conceptual. Lanford and Yampolsky's sector estimates let one choose a region in which

\[
|F(w)-(w+1)|<\frac12,
\]

and hence

\[
\Re F(w)\ge\Re w+\frac12.
\]

citeturn17view3

If, on such a region,

\[
|R(w)|\le C|w|^{-N-2},
\]

and \(X=\Re w>0\), a directly computable majorant is

\[
|h(w)|
\le
C\sum_{j\ge0}
\left(X+\frac j2\right)^{-N-2}.
\]

For example,

\[
|h(w)|
\le
C\left[
X^{-N-2}
+
\frac{2}{N+1}X^{-N-1}
\right].
\]

This converts a measured or interval-bounded functional-equation residual into an explicit rigorous coordinate-error bound.

The same philosophy works especially well for the Buff coordinate, because its one-step defect is explicit:

\[
u_f(z)
=
-1+\int_{[z,f(z)]}\omega_f.
\]

Petersen and Zakeri prove that this defect extends holomorphically and can be made uniformly small near the parabolic point; their theorem gives arbitrary \(\varepsilon\) near-translation control. citeturn19view0turn18view3

For a production implementation, I would validate four things:

**Coordinate error.** Bound the Abel residual on the switching contour and use the convergent correction sum above.

**Series tail.** If the analytic correction \(E\) in the Buff representation is represented by an ordinary Taylor series inside a certified disk, use Cauchy bounds from a larger contour to bound its omitted coefficients. Petersen and Zakeri explicitly establish analyticity of \(E\) on a fixed disk and give the relevant Cauchy representation. citeturn20view0

**Inverse-coordinate error.** Do one or two Newton iterations for

\[
\Phi(z)=W
\]

and certify the solution with a complex interval/ball Newton step. This is preferable to relying on a forward-error estimate when \((\Phi^{-1})'\) becomes large near a petal boundary.

**Roundoff.** Plain binary64 can deliver a very accurate result but is not, by itself, a proof of an error bound. A rigorous renderer should perform the certification stage with outward-rounded interval/ball arithmetic. The expensive coordinate evaluation can still usually be done first in ordinary double and then cheaply certified.

There is also a particularly attractive validation strategy for the local Koenigs shortcut. Compute a truncated

\[
K_m(x)
\]

and evaluate the Schröder residual

\[
R_K(x)
=
K_m(\lambda x+x^2)-\lambda K_m(x)
\]

on the boundary of the proposed working disk. As the disk is shrunk, this residual falls rapidly. Once a univalence margin and residual bound are certified, the fast step

\[
x_{\rm out}
=
K^{-1}(\lambda^N K(x_{\rm in}))
\]

has a straightforward validated enclosure.

The central numerical rule is therefore:

> **Use double precision for speed, but validate the switching domains and truncation/inversion residuals rather than trusting a nominal “series radius.”**

For Fatou coordinates there often is no ordinary radius to trust in the first place.

## What the literature actually demonstrates for fractal rendering

There is a surprisingly large gap between the analytic theory and mainstream rendering practice.

The strongest rigorous rendering result I found is Mark Braverman's *Parabolic Julia Sets are Polynomial Time Computable*. He explicitly identifies the rendering problem caused by parabolic slowdown. For \(r(z)=z^2+1/4\), a point \(z=1/2+2^{-n}\) can require \(O(2^n)\) ordinary iterations to leave a fixed neighborhood, so naïve iteration destroys polynomial-time complexity. His solution is to compute a **long iterate directly**. citeturn16view0turn15view3

If locally

\[
f(z)
=
z+a_r z^{r+1}+a_{r+1}z^{r+2}+\cdots
\]

has positive convergence radius and

\[
|z|<\frac1m,
\]

Braverman proves that for an easily computable constant \(C\), one can take roughly

\[
\ell=\left\lfloor\frac{m^r}{C}\right\rfloor
\]

and compute both

\[
f^\ell(z)
\]

and its derivative to accuracy

\[
2^{-s}
\]

in time polynomial in \(s\) and \(\log m\). The value computation loses only a constant number of input bits; the derivative incurs \(O(-\log|z|)\) bits of conditioning loss. citeturn15view5

His proof is highly implementable. The coefficients of \(f^\ell\) are polynomial functions of \(\ell\); to achieve \(2^{-s}\) value accuracy, only about the first

\[
s+2
\]

terms are required under his normalization, and about \(2s+2\) terms suffice for the derivative. citeturn15view0turn15view5

The resulting Julia-set algorithm repeatedly invokes these long steps near parabolic points and terminates in polynomial time rather than following every raw iteration. citeturn15view3

So the answer to the broad question

> “Has anyone rigorously accelerated fractal rendering by skipping parabolic iteration?”

is unambiguously **yes**.

But the answer to the narrower question

> “Has anyone published an escape-time renderer that uses Lavaurs maps, Écalle–Voronin horn maps, or Shishikura near-parabolic renormalization as the per-pixel accelerator?”

is different.

In the primary papers and implementation searches I found, **I did not find such a published production renderer or a quantitative benchmark showing, for example, ‘horn-map acceleration gives \(10^{-13}\) orbit accuracy and an \(X\times\) Mandelbrot speedup.’** The numerical literature I found uses Fatou coordinates and parabolic renormalization to study the renormalization operator itself, while rigorous Julia rendering literature uses direct long-iterate algorithms.

Lanford and Yampolsky are the closest numerical demonstration on the Écalle/Fatou side: their implementation genuinely computes Fatou coordinates, composes attracting and repelling coordinates, and evaluates a parabolic renormalization numerically. They report empirical reliability at roughly \(10^{-14}\) in double precision for their computed fixed point on \(|z|<5\). citeturn17view0turn17view1

That is strong evidence that these coordinate constructions are perfectly feasible in ordinary double arithmetic. It is **not** an escape-time rendering benchmark.

Petersen and Zakeri give a newer construction that is even more renderer-friendly—the Buff coordinate—with an arbitrary prescribed near-translation error on a sufficiently small disk, but their application is geometric control of near-parabolic invariant curves, not acceleration benchmarks. citeturn18view3

Kapiamba provides the strongest exact finite-perturbation identity for the operation a renderer would want, but the paper is analytic rather than an IEEE-double implementation study. citeturn16view1

Thus there is a genuine opportunity here: the theory contains an exact “gate skip,” but the rendering literature appears not to have turned it into a standard optimized primitive.

## What I would implement for this specific quadratic family

For \(c=-0.7433+0.1312i\), I would **not** start with a full Lavaurs/horn/Shishikura implementation. I would use a layered accelerator.

### Use Koenigs coordinates first

Because

\[
|\lambda|\approx1.006223,
\]

the very long part of the orbit is likely the weak repulsion around \(\alpha_f\). Build once per fixed \(c\)

\[
K(x)=x+k_2x^2+\cdots+k_mx^m,
\qquad
K(\lambda x+x^2)=\lambda K(x),
\]

and its inverse or a Newton solver.

For an incoming \(x=z-\alpha_f\), choose the largest \(N\) such that

\[
\lambda^N K(x)
\]

lies inside a safely validated subset of \(K(D_r)\), and return

\[
z_{\rm new}
=
\alpha_f+
K^{-1}(\lambda^N K(x)).
\]

That directly attacks the 500–1000-iteration dwell and uses the mathematically natural exact coordinate for a **repelling, non-neutral** fixed point. Koenigs/Poincaré linearizers are precisely the standard linearization of repelling holomorphic fixed points. citeturn22search4

Nearer and nearer to \(c=-3/4\), the usable Koenigs disk will deteriorate because the multiplier approaches a root of unity. That is the point at which one switches to the near-parabolic machinery.

### Use the \(q=2\) near-parabolic coordinate for the actual gate

Once the orbit is in a chosen incoming gate section, compute

\[
w=\rho(z).
\]

Choose the appropriate integer \(n\) near

\[
\Re(1/\alpha_{\rm NP})
\]

and jump to

\[
w_{\rm out}
=
w+n-\frac1{\alpha_{\rm NP}}.
\]

Then

\[
z_{\rm out}
=
\chi_\pm(w_{\rm out}).
\]

Within Kapiamba's hypotheses this is not a heuristic:

\[
z_{\rm out}=f^{2n+k}(z).
\]

citeturn16view1

For the example parameter, the natural \(n\) is only around \(11\), which reinforces the point that this part alone does not account for hundreds of raw iterations.

### Build \(\rho\) and \(\chi\) from the explicit lift

For \(q=2\), precomputation is unusually manageable:

\[
\text{period-two points}
\rightarrow
\psi_g\ {\rm(degree\le3)}
\rightarrow
\tau_g
\rightarrow
G_j
\rightarrow
\Phi_j.
\]

The first two stages are elementary algebra; the only serious numerical stage is solving the Abel equation for \(\Phi_j\). Kapiamba supplies the polynomial and covering construction explicitly. citeturn16view2

I would solve the Abel equation with a hybrid:

\[
\text{asymptotic series at large }|w|
+
\text{short ordinary iteration}
+
\text{Newton inversion}.
\]

The Lanford–Yampolsky implementation shows that a threshold of order \(M\sim100\) is entirely realistic in binary64 for this style of calculation. citeturn17view0

### Use the Buff coordinate as the cheap initial model

For \(F=f^2\), precompute

\[
\phi_B(x)
=
\Lambda\log x
+
M\log(x-p_1')
+
M\log(x-p_2')
+
E(x),
\]

where \(p_j'=p_j-\alpha_f\).

This already captures the singular geometry of both the fixed point and the two-cycle. Petersen and Zakeri's decomposition is tailor-made for exactly this fixed-point-plus-bifurcating-\(q\)-cycle geometry. citeturn20view0

Use \(\phi_B\) to:

- choose branches;
- estimate the exit point;
- select the horn/gate;
- initialize the inverse exact Fatou coordinate.

Then correct it to an exact Abel coordinate using its measured functional-equation defect. This is likely substantially more robust than initializing a high-order Fatou asymptotic series directly in the physical \(z\)-plane.

### Precompute horn-map Fourier coefficients only if many orbits reuse the same \(c\)

For one Julia set with millions of pixels, precomputing

\[
H(w)-w
\]

as a Fourier series can make sense. Kapiamba guarantees a translation-invariant domain containing sufficiently high upper/lower half-planes, and the periodicity means the end behavior naturally becomes an exponentially scaled Fourier problem. citeturn21view2

For a Mandelbrot renderer in which \(c\) changes per pixel, it is much less appealing: \(\alpha_{\rm NP}\), the fixed/cycle geometry, coordinate normalization and relative horn map all change with \(c\). The per-parameter precomputation can easily cost more than the iterations it saves.

### Keep Braverman's long-iterate method as the rigorous fallback

Very close to an exact parabolic parameter, where the Koenigs coordinate becomes ill-conditioned and one does not want to implement the full Écalle apparatus, Braverman's method is compelling. It directly computes a block of

\[
\ell\asymp |z|^{-r}
\]

iterations from a truncated series with a prescribed \(2^{-s}\) error, and its proof explicitly controls coefficient tails and precision loss. citeturn15view5

It is less geometrically elegant than a Fatou translation, but from a renderer-engineering standpoint it has an enormous advantage: the error estimate is already part of the algorithm.

The resulting practical architecture would therefore be

\[
\boxed{
\text{ordinary iteration}
\;\to\;
\text{Koenigs fast-forward}
\;\to\;
\text{near-parabolic/Buff gate map}
\;\to\;
\text{Koenigs or ordinary exit}
}
\]

with Braverman-style Taylor long steps as a rigorously controlled fallback extremely close to the exact parabolic locus.

The most important conclusion is that **there is no need to accept hundreds of raw iterations**. Exact analytic shortcuts exist. But for the numerical parameter in the question, the full Écalle–Shishikura apparatus is probably not the first shortcut to implement: the computed phase \(1/\alpha_{\rm NP}\approx11.47-0.57i\) says the canonical \(q=2\) gate is only a few dozen raw iterates wide, whereas \(|\lambda|=1.006223\) naturally creates several-hundred-iteration weak-repelling dwell times. A validated Koenigs fast-forward plus a much smaller near-parabolic transition map is therefore the most practical division of labor.