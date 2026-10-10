# Fractodactyl: Can a compiled mathematical film beat independent rendering?

Verdict: Yes, there are fundamentally different forms of cross-frame reuse that could produce a decisive advantage. But Fractodactyl has not yet demonstrated that advantage against an independent renderer using the same mathematical accelerations.

The strongest direction is a continuous, parameter-dependent mathematical atlas: compile expensive analytic functions, dynamical operators, and validity proofs over regions of the complex plane, then evaluate those functions at the exact coordinates requested by thousands of different frames.

This differs fundamentally from caching reference orbits, storing previously rendered pixels, or interpolating images. It can preserve genuine fractal computation while allowing changing camera positions, rotations, scales, and sample grids.

The critical distinction is whether constructing those reusable functions costs enough that sharing them actually matters.

## 1. What the repository establishes

I examined the connected [Fractodactyl repository](https://github.com/junovhs/fractodactyl), including its benchmarks, production architecture, and October 8–10 research.

Original atlas vs independent BLA

# 0.996×

No demonstrated gain

Original atlas compilation

# 59.8 s

Additional upfront work

Rendering time spent per pixel

# 99.3%

Original bottleneck

The original 750-frame benchmark took 478.1 seconds for independent BLA rendering and 480.0 seconds for atlas playback, excluding the atlas's compilation time. The compiler stored 0.607 GiB, but saved only about 18.3 milliseconds of reference and BLA preparation per frame.

Those saved calculations represented just 2.9% of independent-frame rendering time. Even eliminating them with zero overhead would yield only about 1.03×.

Source: [BENCH.md — Gate C measurements](https://github.com/junovhs/fractodactyl/blob/main/docs/spec/BENCH.md).

### The newer results are more promising, but answer another question

Fractodactyl's Koenigs and return-map pipeline demonstrates roughly 20–54× acceleration on tested deep frames compared with its per-frame BLA renderer. The production zone kernel is described as producing approximately 2.5× acceleration for the complete original film.

These results show that better mathematical representations can eliminate expensive pixel iterations.

They do not establish a compiled-film advantage. An independent renderer supplied with the same valid Koenigs operator could execute essentially the same pixel calculations.

The newer automatic compiler also has a relevant result: repeated cold-frame tests at two Misiurewicz locations measured approximately 1.17–1.43× end-to-end acceleration over production BLA, including operator discovery. However, those experiments did not compare a shared film-wide operator against independently constructed copies of that same operator over thousands of frames.

Sources: [Project handoff](https://github.com/junovhs/fractodactyl/blob/main/PHILO-HANDOFF.md) and [Automatic compiler measurements](https://github.com/junovhs/fractodactyl/blob/main/docs/research/10-9-26/agent-investigations-10-10/01-automatic-compiler-native-prototype-results.md).

## 2. Five candidates for genuinely expensive reuse

| Representation                        | What persists between frames                                                | Mathematical status                                                         | Likely film-wide value                                                |
| ------------------------------------- | --------------------------------------------------------------------------- | --------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| Parameter-dependent operator families | Taylor coefficients in both state and parameter, with validity domains      | Exact recurrence; truncation requires bounded error                         | High where operator construction is costly                            |
| Local analytic exterior fields        | Continuous functions returning potential, gradients, and derived quantities | Analytic in exterior domains; can be certified                              | Very high in smooth exterior regions                                  |
| Certified parameter regions           | Proofs of escape, interior membership, or bounded output error              | Rigorous with interval arithmetic                                           | High for expensive proofs reused across many frames                   |
| Hierarchical return maps              | Reusable Koenigs/Fatou charts, cycle maps, and compositional relationships  | Exact identities on valid domains; numerical evaluation requires safeguards | High around recurrent structures                                      |
| Persistent GPU-resident mathematics   | Operators, coefficients, proofs, and dispatch structures kept in VRAM       | Inherits mathematical validity of stored objects                            | Useful enabling architecture, not an independent mathematical speedup |

### A. Parameter-dependent operator families

Instead of storing a function specialized to one center \\(C\\), store an approximation to the actual map as a function of both the incoming dynamical state and parameter displacement:

\\[ T(w,\delta)=\sum\_{i=0}^{p}\sum\_{j=0}^{q} a\_{ij}w^i\delta^j,\qquad \delta=c-C. \\]

For the quadratic family, the coefficient recurrence can be computed directly from \\(z\mapsto z^2+c\\). The complete polynomial iteration is exact before truncation; a finite Taylor map needs a remainder bound.

This is closely related to differential-algebra jet transport, used in accelerator physics and astrodynamics. Fractodactyl's [parameter-dependent transfer-map research](https://github.com/junovhs/fractodactyl/blob/main/docs/research/10-8-26/parameter-dependant-transfer-maps-rendering.md) already lays out the mathematics.

The significant advantage is that a single map can serve many different values of \\(c\\), not merely pixels on one grid.

The weakness is nonlinear stretching. A Taylor map valid across one viewport may fail just outside it. Increasing degree indefinitely is usually inferior to splitting the domain and recentering.

Assessment: Plausible genuine cross-frame reuse. It becomes decisive only when the independent renderer must repeatedly perform substantial high-precision map construction or validation that the film compiler performs once.

### B. Reusable local analytic fields

This is the most concrete new lead in the repository.

Outside the Mandelbrot set, the Green potential

\\[ G(c)=\log|\Phi_M(c)| \\]

is harmonic, where \\(\Phi_M\\) is the exterior uniformization. Locally, \\(G\\) is the real part of an analytic function. One Taylor representation can therefore supply both a potential and its gradient.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://abel.math.harvard.edu\&sz=32)

abel.math.harvard.edu



Instead of iterating every pixel, a renderer evaluates a low-degree analytic polynomial.

The October 10 field prototype already measures:

| Region               | Field coverage | Acceleration over direct orbit/derivative evaluation |
| -------------------- | -------------- | ---------------------------------------------------- |
| Far exterior         | 99.58%         | 2.065×                                               |
| Exterior mixture     | 37.17%         | 1.109×                                               |
| Wide Mandelbrot view | 11.17%         | 1.007×                                               |
| Right-tip region     | 2.17%          | 0.998×                                               |

These are single-thread CPU prototype measurements. They are not yet comparisons against optimized GPU, SIMD, or perturbation/BLA baselines. The faster field method is empirically accurate, not rigorously certified.

Source: [Compiled analytic exterior fields, October 10](https://github.com/junovhs/fractodactyl/blob/main/docs/research/10-10-26/mandelbrot_exterior_field_research.md).

The key opportunity is to construct a field once over a physical parameter-space region, rather than reconstruct it in every viewport that intersects the region.

For example, a camera might visit the same exterior neighborhood in 500 frames, but at 500 different orientations and scales. The polynomial still represents the same mathematical field. Every frame evaluates it at new, exact parameter coordinates.

That is not raster interpolation.

There is an important correctness qualification: the limiting Green potential is not automatically identical to the finite-bailout smooth-iteration statistic that `fd` currently outputs. Either the rendering contract must explicitly use the limiting potential, or the conversion and its error must be certified against the existing output contract.

### C. Certified sample regions

A certification atlas could store the claim that every parameter inside a disk has escaped by a particular iteration, or that a whole region belongs to a proved interior component.

For a disk of parameters centered at \\(C\\) with radius \\(R\\), the elementary orbit-radius enclosure is

\\[ e\_{n+1}=2|Z_n|e_n+e_n^2+R. \\]

Combined with a sufficient escape-radius test and outward-rounded arithmetic, this can certify escape for an entire disk.

A future frame does not need to repeat that proof. It only checks whether its pixel coordinate belongs to the certified region.

For continuous shading, however, an exterior certificate alone is insufficient. It must be combined with certified fields or derivatives. Likewise, failure to escape within a finite iteration budget does not prove interior membership.

Rigorous numerical foundations already exist in FLINT/Arb, which supports interval-enclosing real and complex arithmetic and polynomial calculations.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://flintlib.org\&sz=32)

FLINT 3.7.0-dev documentation

+1



Assessment: A compelling type of reusable expensive work. The challenge is coverage near the fractal boundary, where interval domains may need to become extraordinarily small.

### D. Hierarchical return maps and exact conjugacies

Near a repelling cycle, Koenigs coordinates can provide an identity of the form

\\[ g_c^{\\,n}(z) = \Psi_c\left(\lambda(c)^n\Phi_c(z)\right), \\]

where \\(g_c\\) is the cycle return map and the coordinate transformations are defined on appropriate domains.

This can represent thousands of repetitions without explicitly evaluating them. It is an exact mathematical identity, not a visual approximation.

A useful compiled film stores parameter-dependent coordinate changes and powers, plus the domains where the identity may be applied safely. A further factorization can express a long-period minibrot return through a short fundamental cycle, avoiding an enormous stored polynomial.

Fractodactyl's [factored-ladder investigation](https://github.com/junovhs/fractodactyl/blob/main/docs/research/10-9-26/agent-investigations-10-10/03-factored-ladder-return-maps-agent-report.md) identifies this opportunity for its period-16,116 deep ladder.

Near-parabolic Fatou coordinates offer a related route for lengthy bottlenecks, but general complex-domain, parameter-dependent, validated transit maps remain research work.

Assessment: Potentially the highest-value representation for difficult deep zooms. But its iteration savings belong equally to an independent renderer. Only reusable construction, certification, and perhaps expensive operator composition constitute the distinct film-wide benefit.

### E. Persistent GPU-resident mathematics

Keeping these structures in VRAM avoids repeated uploads and allows many frames to evaluate a shared operator library. This can be important after pixel iteration has become very cheap.

But GPU persistence is not sufficient on its own. If a map takes milliseconds to rebuild, holding it in VRAM cannot create a large advantage over a well-optimized independent renderer.

VRAM also imposes a hard constraint: excessive tables cause bandwidth pressure, cache misses, register pressure, and possibly spills. A small, well-used operator is preferable to a gigantic low-hit-rate atlas.

## 3. Changing positions, scales, rotations, and precision

The correct mathematical key is the complex parameter \\(c\\), not the frame number or raster coordinate.

For a camera transform, write

\\[ c(t,u,v)=C(t)+s(t)e^{i\theta(t)}(u+iv), \\]

with the appropriate aspect-ratio adjustment.

A parameter-space field or operator remains usable as long as the resulting \\(c\\) lies in its certified domain and the evaluation meets the precision requirement.

Translation changes which domains are accessed; reuse decreases when the path visits new territory. Rotation changes neither the mathematical field nor its validity domain. Zooming changes the density and scale of queries, so nested views can reuse coarse exterior patches while requiring additional fine patches near new boundary detail.

Precision is harder. At width \\(10^{-1000}\\), distinguishing coordinates requires approximately 3,322 significant bits before guard precision. Double precision cannot directly store those absolute coordinates.

A practical atlas should therefore store high-precision anchors, scale/exponent metadata, and locally normalized coefficients. Low-precision polynomial evaluation is allowed only when the associated error enclosure remains valid. Coefficients should be promoted or rebuilt at higher precision rather than assuming a low-precision representation remains accurate at arbitrary depth.

### Memory and precomputation

The storage economics differ dramatically by representation.

A hypothetical 100,000-patch atlas with 16 complex-double coefficients per patch uses just 25.6 MB for coefficients, before domain metadata and certificates. A bivariate degree-12-by-6 map has 91 complex coefficients, so 100,000 patches require approximately 146 MB in double precision.

But storing all 91 coefficients at roughly 3,322-bit precision for every patch would require about 7.6 GB, before metadata. That already exceeds Fractodactyl's 3 GiB original atlas cap.

These are illustrative size calculations, not observed atlas requirements. They show why normalized, mixed-precision, adaptive-degree representations matter.

The total footprint must depend on the union of covered mathematical domains, not on the number of frames. Otherwise the compiler is merely saving per-frame data under a different name.

## 4. The decisive economic test

Let \\(B_f\\) denote the work an independent accelerated renderer performs to construct and validate operators for frame \\(f\\). Let \\(E_f\\) denote its optimized pixel evaluation cost.

Then:

\\[ T\_{\mathrm{independent}} = \sum\_{f=1}^{F}(B_f+E_f). \\]

For a compiled film with global construction cost \\(B\_{\mathrm{film}}\\), shared evaluation cost \\(E_f'\\), and lookup/loading overhead \\(L_f\\),

\\[ T\_{\mathrm{compiled}} = B\_{\mathrm{film}} +\sum\_{f=1}^{F}(E_f'+L_f). \\]

The comparison must use the same mathematical acceleration, output requirements, and hardware.

If \\(E_f'\approx E_f\\), the film wins almost entirely by eliminating repeated construction.

For a 2× end-to-end speedup, the saved construction work must exceed the remaining evaluation work plus twice the global compilation cost and associated overhead.

Consider a hypothetical 1,000-frame workload in which a good independent renderer needs 120 ms to construct and validate an operator and 50 ms to evaluate its frame. A film compiler needs 10 seconds to build the shared operators, after which each frame still costs 50 ms.

Independent, same math

# 170 s

1,000 × (120 + 50 ms)

Compiled film, same math

# 60 s

10 s + 1,000 × 50 ms

Result: 2.83× genuine cross-frame acceleration.

No pixels were reused and no lesser approximation was permitted. Only mathematically identical construction work was amortized.

This is a feasibility example, not a measured Fractodactyl result.

Conversely, if the same operator takes just 6 ms to construct and 50 ms to evaluate, the maximum warm advantage from eliminating construction is only 1.12×, even with a free cache.

That is exactly the kind of limitation Gate C exposed.

## 5. What I would implement and test next

The best concrete experiment is a certified analytic-field cache with a continuous parameter-space key, combined with Fractodactyl's existing zone operators.

Build the fields once for a chosen sequence of overlapping, differently rotated and scaled views. Store their analytic coefficients, domains, error bounds, and precision metadata. At rendering time, test each exact sample coordinate against the cached domains, evaluate the field where valid, and use the same accelerated independent kernel for every rejected sample.

Run three matched configurations: an optimized independent renderer with per-frame analytic field construction; a shared atlas using identical field mathematics and validation; and an independent renderer that constructs only profitable fields, not an artificially weakened opponent.

All variants must receive the same output requirements for class, smooth potential or `nu`, distance estimate, normal, supersampling, and iteration budget. Fractodactyl's established \\(10^{-3}\\)-pixel displacement target is a useful starting gate, but field-gradient accuracy and class correctness require their own checks. Disagreements near the bailout budget need high-precision adjudication; the original BLA benchmark itself contained a rare incorrect classification.

Measure cold end-to-end time including precomputation, warm per-frame time, operator hit rate, saved construction milliseconds, actual fallback arithmetic, peak RAM, and GPU traffic where applicable. Run multiple repeats on the same hardware. Include a wide exterior scene, a boundary-heavy scene, a coherent rotated zoom, and a panning path leaving cached regions.

A meaningful promotion threshold would be at least 2× cold end-to-end acceleration over the same-math independent renderer, with zero incorrect accepted regions and bounded memory. The test should still report successful narrower wins, but not describe them as decisive.

## Final assessment

There are two separate achievements here.

The first—making fractal pixels intrinsically cheaper—is already real. Fractodactyl's newer dynamical operators demonstrate major gains, and the exterior-field prototype demonstrates that continuous analytic representations can amortize orbit work across many sample positions.

The second—making thousands of frames substantially cheaper because they share precomputed mathematics—remains unproved.

A compiled-film renderer can plausibly win when it shares expensive validated parameter-dependent maps, analytic fields, and structural discoveries that an independent accelerated renderer must reconstruct for every frame. The advantage is inherently dependent on path overlap, operator construction cost, coverage, memory, and precision.

The most promising untested hypothesis is not that all Mandelbrot computation can be cached. It is that a relatively small library of certified continuous mathematical functions can answer billions of distinct pixel queries while requiring far fewer expensive constructions than independent frames.

That is the experiment most likely to establish a real successor to Atlas v0.