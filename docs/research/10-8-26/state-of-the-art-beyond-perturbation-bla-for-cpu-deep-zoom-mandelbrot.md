# State of the Art Beyond Perturbation + BLA for CPU Deep-Zoom Mandelbrot

## What I would change first

Your measurements imply a very specific bottleneck. At 960×540 there are 518,400 pixels; if 32% fall through BLA and execute about 511 ordinary perturbation iterations, that is roughly **84.8 million scalar perturbation iterations per frame**. At this point, making the ordinary iteration a little cheaper is not the highest-leverage move. The state of the art is to make that fallback path almost disappear.

My priority order for a CPU Rust renderer would be:

**First, fix the `1e-300` BLA cliff with an exponent-carrying representation throughout the approximation machinery.** Depth by itself should not suddenly make first-order BLA useless: as the pixel offset \(c\) gets smaller, the \(B\,c\) contribution normally becomes *easier* to bound, not harder. A sharp loss of skips around the edge of binary64's exponent range is therefore a numerical-range red flag. I would treat it as a representation bug until disproved. This is exactly the range where modern renderers stop trusting ordinary floating point: Imagina's public code switches its reference/LA machinery to `FloatExp` below a half-height of \(2^{-896}\), roughly \(2\times10^{-270}\), while its ordinary short-range type is `double`. fileciteturn11file0 fileciteturn12file0 A recent independent second-order BLA implementation gives an especially vivid example: over a 4096-iteration skip its \(A\) coefficient can reach roughly \(10^{700}\); storing the coefficient and derived radius as ordinary doubles turns them into infinities, zeros, or NaNs even though the *combined approximation remains usable*. citeturn29search0turn29search6

**Second, copy Imagina's multi-stage LA + approximation-transformation architecture, rather than adding another layer of ordinary dyadic BLA.** Imagina and the derivative FractalShark implementation do more than look up a large linear skip. They recursively turn lower-level approximation entries into higher-level macro-iterations, detect periodic structure during that construction, and, when a suitable return map exists, build an `AT` that changes the problem so one transformed iteration corresponds to `StepLength` original iterations. The code literally reduces the effective maximum iteration count to approximately `MaxIt / StepLength`. fileciteturn8file0 fileciteturn11file1 This is the most directly relevant answer to your “511 ordinary iterations remain” problem.

**Third, move interior detection from “wait for a pixel orbit to look periodic” to “discover the component/period on the reference, refine the nucleus, then propagate contraction information through the skips.”** Claude Heiland-Allen's current deep-zoom treatment tracks the derivative of the pixel orbit with respect to its initial perturbation and terminates when its magnitude contracts sufficiently, using about \(10^{-3}\) as the practical threshold—but the derivation relies on the reference being the periodic critical orbit, i.e. essentially the minibrot nucleus. citeturn28search3 Atom-domain period candidates plus Newton refinement give you the missing mechanism for getting that reference into the right place. This specifically addresses your undetected interiors inside very deep high-period minibrots.

**Fourth, experiment with a quadratic, or “second-order BLA,” jet.** A 2026 WebGPU implementation retains

\[
w_{\text{out}}
 = A w + B\delta
 + Cw^2+D w\delta+E\delta^2 ,
\]

so the first omitted terms are cubic rather than quadratic. Its author reports a 21× render-time improvement at \(2.8\times10^{40}\) magnification, with 96.9% of iterations skipped and bit-identical results relative to its no-approximation path. That result is on a GPU and from a new independent implementation, so the **21× number should not be projected onto a CPU renderer**, but the algorithm attacks exactly the population for which a first-order BLA validity radius is too small. citeturn29search0turn29search6

Only after those would I add traditional high-order series approximation. Series approximation is still useful, especially as a cheap one-shot prefix skip, but BLA/LA is more general because it can fire at arbitrary points in the orbit. NanoMB-style bivariate series is more interesting than ordinary series for your high-period minibrots because one polynomial evaluation can replace an entire period. Claude's own newer BLA work likewise treats ordinary series as less general than arbitrary-position approximation. citeturn28search3turn29search0

## How the techniques compare

The nomenclature is confusing because “BLA,” “LA,” “series approximation,” “super-series approximation,” and “AT” overlap mathematically. The useful distinction is what unit of work each one removes.

| Technique | What it eliminates | Validity / error criterion | Scaling with depth and period | Main failure mode |
|---|---|---|---|---|
| **Classic series approximation** | Usually one long **prefix** of per-pixel perturbation iterations | A truncated polynomial \(z_n(c)=\sum a_k c^k\). `rust-fractal` validates it empirically: it evolves probe pixels with perturbation, compares the series result, divides squared discrepancy by a derivative sensitivity term, and requires that to stay within one squared pixel spacing. fileciteturn15file0 | For order \(K\), coefficient construction in the straightforward recurrence is about \(O(NK^2)\); pixel evaluation is \(O(K)\). Deeper zoom generally extends validity because \(|dc|\) shrinks. Period gives no direct multiplicative benefit. | Only accelerates from the initial state; heuristic probes can miss localized bad regions; nonanalytic folds terminate useful series quickly; it can jump across an interval in which a glitch detector would otherwise notice trouble. |
| **Hierarchical first-order BLA** | Arbitrary contiguous ranges anywhere along perturbation | Approximate \(z_{n+l}=A z_n+B c\). Claude's criterion is that the discarded nonlinear contribution be smaller than the acceptable low-precision error; composed entries propagate a shrinking admissible radius. Entries are composed hierarchically so large valid segments can be selected greedily. citeturn28search3 | \(O(N)\)-scale table/storage for the dyadic hierarchy. Per accepted skip is \(O(1)\); lookup is logarithmic or a few level tests. Depth should not inherently kill it if exponents are preserved. | Near critical returns the nonlinear term becomes important; plain-double coefficients/radii overflow or underflow; first-order radius can become tiny despite a useful second-order map. |
| **Imagina-style multi-stage LA** | Builds **macro-iterations out of earlier macro-iterations**, not merely longer entries in one dyadic table | Imagina stores `Ref`, `ZCoeff`, `CCoeff`, `LAThreshold`, `LAThresholdC`, and `MinMag`. Its prepare step computes \(dz(2\,Ref+dz)\) and rejects the entry when its Chebyshev norm reaches the stored LA threshold. Thresholds are tightened during composition using coefficient magnitudes and a conservative \(2^{-24}\) scale. fileciteturn8file0 | Successive stages have fewer macro-iterations if composition is effective, so practical work resembles a geometric reduction. Periodic structure can turn an enormous raw period into a short macro sequence. | Poor reference placement, no useful contraction/period, or thresholds collapsing numerically. More complicated state machine than ordinary BLA. |
| **Approximation transformation, `AT`** | Replaces repeated blocks of \(L\) original iterations by **one transformed iteration** | Imagina derives an AT from an LA entry, checks a parameter-space threshold and transformed escape-radius condition, maps the pixel's \(dc\) into the transformed system, then uses roughly \(\lceil N/L\rceil\) transformed iterations. fileciteturn8file0 fileciteturn11file1 | This is the attractive high-period case: per-pixel iteration count can fall approximately in proportion to the accepted `StepLength`. | Only applies inside the AT's parameter/radius domain; smooth iteration counts and derivatives have to be mapped consistently back to the original iteration scale. |
| **Second-order BLA / quadratic jet** | The same arbitrary ranges as BLA, but for pixels that fail first-order validity | Keep all monomials through total degree two in \((dz,dc)\); the first dropped error is cubic. The recent WebGPU implementation uses explicit-exponent coefficients/radii and greedily tries doubling levels. citeturn29search0turn29search6 | Per entry is still constant-size, but composition is substantially more arithmetic than \(A,B\). It should become increasingly attractive when pixel iteration is expensive or first-order acceptance is low. | More build cost, bandwidth, and arithmetic; a loose remainder bound can give up most of the radius advantage. The strongest benchmark currently available is GPU-specific rather than CPU-specific. |
| **NanoMB1 bivariate super-series** | One evaluation replaces a complete **period-\(p\)** return around a minibrot; the map can be applied repeatedly | KF's implementation builds a polynomial in both orbit perturbation and parameter perturbation. The admissible domain is derived from polynomial coefficients; the code computes a radius using higher-order terms and normally scales it conservatively before use. fileciteturn1file0 | Setup is linear in period times the cost of bivariate polynomial convolution; the straightforward implementation grows roughly as \(p\) times a high polynomial in the chosen degrees. Per-pixel evaluation is polynomial-degree dependent but **not proportional to \(p\)**. | Needs a correct period and a good nucleus/reference. High orders are costly. The approximation can be brittle outside its component/domain. |
| **NanoMB2 chained super-series** | Period jumps across a hierarchy of nested minibrots; when the deepest return map stops applying it can fall outward to another SSA | Tries the deepest shifted SSA whose parameter/running delta is inside its stored escape radius, adds that SSA's period to the iteration counter in one operation, and repeats while valid. fileciteturn18file1turn18file2 | Excellent theoretical leverage in a nested high-period zoom: each polynomial evaluation can replace tens, thousands, or far more raw iterations. | Experimental. KF's source explicitly has missing/TODO glitch handling in this path, and its interior Newton convergence test contains an `epsilon2 = 0; // FIXME`, making the test depend on exact numerical stagnation. fileciteturn18file0 |
| **Interior / attractor detection** | Once a point is proven or confidently classified interior, it skips **all remaining max iterations** | Atom-domain candidates occur when \(|z_p|\) reaches a new record minimum; Newton can solve \(f_c^p(w)-w=0\), and an attracting solution has multiplier magnitude \(<1\). A cheaper nucleus-based deep-zoom test watches derivative contraction. citeturn23search0turn28search3 | A raw Newton test per pixel costs \(O(p)\) per Newton step and is therefore wrong for huge \(p\). Find/refine \(p\) once on the reference or region, then carry derivative/contraction information through macro steps. | Near parabolic boundaries the multiplier approaches one; off-nucleus references invalidate the simple contraction shortcut; nonhyperbolic boundary points should not be classified early. |
| **FloatExp / HDR / rescaled perturbation** | No iterations directly; it prevents legitimate **BLA/LA skips from disappearing numerically** | Mantissa plus explicit exponent, with normalization and exponent-aware magnitude comparisons. Imagina, rust-fractal and FractalShark all have such types. fileciteturn12file0 fileciteturn16file0turn16file1 fileciteturn20file1 | Fixed-mantissa per-pixel cost is effectively independent of zoom depth until the exponent integer itself becomes limiting. High-precision *reference* cost still grows with requested precision. | Software exponent alignment is slower than raw doubles; incorrect comparisons between normalized and non-normalized values are a subtle source of catastrophic rebasing/validity bugs. citeturn29search0 |

A useful way to see the hierarchy is:

\[
\text{perturbation}
\;\longrightarrow\;
\text{first-order BLA}
\;\longrightarrow\;
\text{multi-stage LA}
\;\longrightarrow\;
\text{periodic return map / AT},
\]

while high-order series and second-order BLA attack the same problem from different directions: **retain more Taylor information so a larger domain can be jumped safely**.

The crucial difference between ordinary dyadic BLA and multi-stage LA is therefore not just “more BLA levels.” Fraktaler 3's “BLA skip levels” setting is chiefly a way to omit levels and reduce BLA memory usage; Imagina's stages instead use one level of macro-iterations to construct the next representation. Fraktaler 3 also added periodicity checking during reference construction, which is a complementary route to discovering useful return structure. citeturn29search3

## What the named renderers actually do

**Kalles Fraktaler 2+.** KF2+'s established fast path is perturbation plus **series approximation**, not the later BLA architecture now documented for Fraktaler 3. The program's own status displays reference work, series-approximation percentage, pixel completion, and glitch/reference state, and its documentation describes SA as the mechanism for skipping large numbers of pixel iterations. citeturn28search1turn28search6

Its most interesting “beyond ordinary SA” experiments are **NanoMB1 and NanoMB2**, largely based on Knighty's public-domain fractalforums work. `nanomb1.inc` describes the algorithm as approximating \(n\) Mandelbrot iterations around a period-\(n\) minibrot with a bivariate polynomial. It uses a low-range `floatexp` representation for the polynomial/per-pixel path and high precision for reference construction. It also contains Newton code to locate the nearby polynomial root/nucleus—explicitly tied in the comments to the “atom domain” idea. fileciteturn1file0

NanoMB2 is the more ambitious version. It builds **multiple shifted super-series approximations corresponding to hyperbolic components**, ignores very low periods that would cost more than they save, and at render time searches from the deepest approximation outward. When accepted, one polynomial evaluation increments the original iteration index by that component's whole period. fileciteturn18file0 fileciteturn18file1turn18file3 This is exactly the property you want in a high-period minibrot: the cost of a macro step no longer scales with the minibrot's period.

The downside is maturity. KF's own manual labels NanoMB as experimental and says performance depends strongly on location, with the main benefit close to minibrots. NanoMB1 can still be followed by ordinary rendering/glitch correction; NanoMB2 disables normal glitch reporting in the current source. citeturn27search3 fileciteturn18file2 The interior code is tantalizing—it Newton-solves a periodic orbit and checks whether its multiplier has norm below one—but the `epsilon2 = 0 // FIXME` convergence condition is a concrete reason not to transplant that code verbatim. fileciteturn18file0

**Fraktaler 3.** Fraktaler 3 is Claude Heiland-Allen's cleaner modern architecture: reference perturbation, Zhuoran-style rebasing, BLA tables, derivative/distance handling, and interior detection are described from one unified formulation. citeturn28search3turn28search7 Current releases include configurable BLA skip levels to reduce table memory and a periodicity check in reference calculation. Version 3.1 also expanded the numeric options for situations where ordinary floating-point exponent range is inadequate. citeturn29search3turn19search3

The most important F3 lesson for your renderer is that **BLA should be an exponent-safe algorithm**. The mathematical BLA does not have a natural \(10^{-300}\) cutoff. Claude's BLA composition propagates coefficients and valid radii; if those quantities are represented robustly, a view becoming deeper does not in itself invalidate the method. citeturn28search3 In other words, before inventing a new skip algorithm, instrument whether your `R`, `A`, `B`, `|dc|`, `|dz|`, products such as `|B|·|dc|`, or reciprocal/norm calculations become zero, subnormal, infinite, or NaN.

F3 also illustrates a second useful optimization: **period-lock the reference when the period is known**. Its documentation recommends limiting the reference calculation to the known period after a successful Newton zoom, particularly in high-iteration areas; it warns that locking to an incorrect period produces bad/pixelated output. citeturn19search0 That's cheap reference-side work, but it also makes downstream LA and interior reasoning much cleaner.

**Imagina.** For a CPU-only Mandelbrot renderer, this is the project I would study most closely. Its public `HInfLAEvaluator` is SIMD-oriented and has a materially richer approximation state than a conventional BLA: `Ref`, `ZCoeff`, `CCoeff`, separate thresholds for orbit and parameter perturbations, minimum magnitude for period detection, stage metadata, and an optional approximation transformation. fileciteturn8file0

For one LA entry, the code forms

\[
q = dz\,(2\,Ref+dz)
\]

and requires its Chebyshev magnitude to remain below `LAThreshold`. During construction, a threshold is repeatedly restricted roughly by

\[
R_z \leftarrow
\min\!\left(
R_z,\,
\frac{\|z\|_\infty}{\|ZCoeff\|_\infty}\,2^{-24}
\right),
\]

with an analogous threshold for the \(c\)-coefficient. Composition tightens these bounds again before multiplying/composing the coefficients. fileciteturn8file0 FractalShark's descendant implementation makes the same operations particularly easy to inspect in `LAInfoDeep.h`. fileciteturn6file0

That differs subtly but importantly from the simplest \(Az+Bc\) BLA: the current perturbation's nonlinear \(dz^2\) contribution is included in the prepared quantity before the longer linearized macro-map is applied. The implementation also records decreases in `MinMag` or equivalent thresholds to identify candidate periods while constructing approximation data. fileciteturn8file0

The killer feature is **AT**. After building LA stages, Imagina can derive an `ATInfo` from a suitable approximation. For pixels within its `ThresholdC`, it transforms \(dc\), reduces the iteration budget to approximately

\[
N_\text{AT}
=
\left\lceil\frac{N}{L}\right\rceil
\]

where \(L=\text{StepLength}\), and runs an iteration in the transformed coordinate system. fileciteturn11file1 That is qualitatively different from shaving 20 or 100 iterations with a larger BLA: if your no-skip pixels are trapped for hundreds of iterations near a period-\(p\) return map, this can remove the dependence on \(p\) from the inner loop.

Imagina also gives you a useful numerical design point. Its normal scalar type is `double`, its high-range type is a `FloatExp`, and the code switches to the latter around \(2^{-896}\) view half-height. fileciteturn12file0 fileciteturn11file0 Your reported collapse at about \(10^{-300}\) is close enough that I would investigate this before anything else.

**FractalShark.** FractalShark has independently evolved a large, inspectable version of the same LA lineage. Its current `LAReference` has up to 1024 LA stages, `ATInfo`, `GenerateApproximationData`, per-stage macro-iteration counts, HDR number types, and CUDA mirrors of the LA data. fileciteturn4file0 `LAInfoDeep.h` shows the period-detection and threshold restrictions explicitly and supports HDR wrappers around float, double, and double-float-like mantissas. fileciteturn6file0

I would use FractalShark primarily as **reference code, not as a CPU performance target**. Its author says the CPU renderer is largely a debugging aid and explicitly points CPU-only users toward faster specialized implementations such as Imagina. fileciteturn20file0 Its value for you is that it cleanly separates numeric representations, compressed reference storage, CPU LA construction, CUDA LA representation, and AT/period infrastructure.

The other useful FractalShark lesson is that at fantastically high precision the **reference orbit itself** eventually belongs on the GPU. The current author-reported v0.54 result on an RTX 5090 is about 70× the speed of a single-threaded AVX2 MPIR baseline at 16,384 32-bit limbs. The README explicitly warns that its multithreaded CPU comparison was broken, so this is not a fair 70× “GPU versus optimized CPU renderer” number; it measures an extreme-precision reference calculation. fileciteturn19file0 FractalShark also reports reference compression saving multiple gigabytes in a period-600,000,000 test, using an idea originally implemented by Imagina. fileciteturn19file0

**rust-fractal.** `rust-fractal-core` is useful because its series validity strategy is unusually transparent and already Rust. It supports perturbation/glitch correction, series approximation, a tiled probe system, Rayon parallelism, and exponent-carrying complex numbers. fileciteturn14file0

The SA builder stores polynomial coefficients at configurable iteration intervals. To decide how far a region can skip, it samples a grid of probes across the image. At a stored checkpoint it compares an actually perturbed probe with the polynomial prediction, forms squared error, divides by squared derivative sensitivity—clamping that sensitivity to at least one—and rejects the skip when the result exceeds the squared pixel spacing. It first tests corner probes, then the remaining grid, and assigns each tile the minimum safe iteration among its four surrounding samples. fileciteturn15file0 This is a pragmatic screen-space error test rather than an analytic proof, but it has a very nice property for a renderer: the tolerance is tied directly to **whether the approximation can move the result by more than a pixel**.

Its range types are also simple enough to borrow: `FloatExtended` is an `f64` mantissa plus `i32` exponent, and `ComplexExtended` stores a complex-double mantissa plus an explicit exponent. fileciteturn16file0turn16file1 That is close to the representation I would use for your CPU BLA data.

**The fractalforums/mathr lineage.** Two community developments matter most. First, Zhuoran's 2021 “reference reset to zero” observation replaced much of the old multi-reference glitch machinery with what is now normally called rebasing: after forming the full pixel state \(Z+dz\), when

\[
|Z+dz|<|dz|,
\]

use \(Z+dz\) as the new delta and restart against the reference from iteration zero. Claude's current deep-zoom document uses this exact criterion. citeturn28search3turn11view0 This prevents perturbation glitches far more cleanly than repeatedly finding new high-precision references.

Second, Claude's mathr posts trace the transition from ordinary series approximation to bivariate “super-series,” then to rebasing and BLA. Ordinary SA writes the perturbation as a polynomial in \(dc\); Knighty's bivariate construction adds dependence on the incoming orbit delta, making a whole period reusable as a return map. NanoMB2 then attempts to stack those maps over a minibrot hierarchy. citeturn28search4turn13view0turn13view1 Modern LA/AT is, in effect, the lower-order, cheaper, more compositional descendant of the same idea.

## Interior detection, atom domains, and glitches

Your second symptom—**deep high-period minibrot interiors run all the way to the iteration limit**—is not best solved by generic cycle detection on every pixel.

For the quadratic Mandelbrot family, the reference critical orbit itself gives a cheap sequence of period candidates. Whenever

\[
|Z_p| < \min_{0<k<p}|Z_k|,
\]

\(p\) is an atom-domain “partial”: it is a plausible period associated with a nearby hyperbolic component. Claude's practical interior algorithm uses precisely these record minima to decide when it is worth attempting the more expensive attractor/Newton calculation rather than testing every possible period. citeturn23search0

Given a candidate \(p\), solve the periodic-point equation

\[
F_c^p(w)-w=0
\]

by Newton iteration, while simultaneously propagating the derivative

\[
D_{k+1}=2z_kD_k
\]

for the derivative with respect to the starting value. A converged periodic orbit is attracting when

\[
|(F_c^p)'(w)| < 1.
\]

That is a real dynamical interior test rather than “the orbit has not escaped yet.” citeturn23search0

Doing this naively for every pixel at period \(p=100{,}000\) would be disastrous: each Newton step itself needs approximately \(p\) map evaluations. The production architecture should instead be:

**Detect the period on the reference**, preferably while you are already building LA stages. **Refine the minibrot nucleus once at high precision.** Recompute or align the reference to that exact critical periodic orbit. Then **carry the appropriate contraction derivative through LA/macro steps** for each pixel. Once it contracts below your chosen safe threshold, terminate that pixel as interior.

Claude's current F3-oriented derivation uses a practical magnitude threshold around \(10^{-3}\) for this derivative contraction. Its important caveat is that the neat test assumes the reference parameter is periodic with the critical orbit returning to zero—the nucleus situation. Applying it indiscriminately to an arbitrary nearby reference is not justified. citeturn28search3

That caveat explains a lot of “interior works at normal depths, fails in deep minis” behavior: if the center/reference is merely visually inside the minibrot rather than numerically placed at its nucleus, the critical-orbit simplification is missing. **Atom-domain candidate detection + Newton nucleus refinement is therefore not just a navigation feature; it is an acceleration prerequisite.**

You can make this nearly free once AT exists. If an AT compresses \(L\) original iterations into one macro-iteration, propagate its derivative in macro coordinates and test contraction after macro steps. For a period-\(p\) component, the cost of demonstrating attraction then tracks the number of return-map iterations rather than the raw \(p\)-step orbit. Imagina's LA state already carries the coefficient information needed for derivative propagation, and its feature-finding code uses LA-aware evaluation for periodic points. fileciteturn7file2 fileciteturn8file0

Glitch handling should remain conceptually separate. **Rebasing fixes numerical loss of significance; interior detection stops mathematically bounded orbits; approximation reduces the number of steps.** Rebasing by itself does not lower that 511-iteration tail. It can even leave you doing more ordinary work around a reference's near-zero returns. Its value is that it lets you use one reference robustly and makes the approximation machinery deterministic.

The performance history makes that distinction clear. In the original reference-reset-to-zero discussion, a Dinkydau Flake example reportedly rendered with one reference in about **18.5 s**, versus **29.8 s** using 22 references while tolerating isolated glitches, while a much more thorough 97-reference correction took several minutes. citeturn11view0 Older threshold-based glitch correction in KF showed exactly why: increasing the glitch sensitivity could push one test from about 9 references and 1m30s to hundreds of references and tens of minutes. citeturn22search2 Rebasing largely removes that *reference-generation* explosion, but you still need LA/AT or interior detection to remove the pixel iterations themselves.

## Reported speedups and what they actually prove

Published numbers in this niche are frustratingly heterogeneous. There is no standard suite analogous to a CPU benchmark corpus; authors typically benchmark a single difficult location, different precision, image sizes, processors, or GPU generations. The following are useful only when their exact scope is preserved.

| Result | Settings / technique | What it demonstrates |
|---|---|---|
| **~6×** in an older KF/forum test | Roughly **512×512**, about **90 s with series approximation** versus **~9 min with SA disabled** at the reported location. citeturn22search2 | Classic SA can eliminate an enormous initial perturbation prefix. It does **not** establish that SA beats arbitrary-position BLA/LA. |
| **18.5 s vs 29.8 s vs several minutes** | Zhuoran reset-to-zero/rebasing thread: one reference versus 22-reference partially corrected output and a 97-reference high-quality path. citeturn11view0 | Modern rebasing can almost eliminate the expensive multi-reference glitch-repair loop. |
| **21×, 96.9% iterations skipped** | 2026 second-order BLA WebGPU implementation at **2.8×10^40** zoom; author reports bit-identical output against its approximation-disabled path. citeturn29search0turn29search6 | Keeping quadratic terms can radically enlarge the domain that accepts long skips. The number is **GPU- and implementation-specific**, but the acceptance result is highly relevant to CPU algorithms. |
| **~70×** reference-orbit result | FractalShark v0.54, RTX 5090, **16,384 32-bit limbs**, versus single-threaded AVX2 MPIR. Author notes the MT CPU baseline was broken. fileciteturn19file0 | Massive GPU wins become possible for the *high-precision reference* at extreme depth. It says almost nothing about your current 84.8M low-precision pixel iterations. |
| **No controlled numeric figure published in the code/manual I found** | Imagina LA/AT, KF NanoMB1/2, rust-fractal tiled SA, current F3 BLA. Their code documents the mechanisms, but public performance claims are mostly location-dependent or qualitative. citeturn27search3 fileciteturn14file0 | Benchmark these against **your exact 960×540 workload** rather than choosing by anecdotal ratios. |

The second-order result deserves special attention because it also validates the extended-exponent diagnosis. That implementation found that coefficients over long skips could grow to around \(10^{700}\), so it stores both coefficients and validity radii with explicit exponents. citeturn29search6 That is precisely the sort of failure you should instrument for: the *pixel delta* may be \(10^{-300}\), but the internal BLA coefficient may simultaneously be \(10^{+500}\); the physically meaningful product can be perfectly ordinary while either factor is impossible in binary64.

For GPUs, there are two distinct opportunities. Fraktaler 3 can run its OpenCL pixel work on CPU/GPU devices and benchmark/select execution modes, while FractalShark pushes both LA evaluation and very-high-precision reference machinery into CUDA. citeturn19search0 fileciteturn3file8turn3file12 For your CPU renderer, however, I would not move to GPU just to mask the 511-iteration tail. **Eliminate that tail algorithmically first.** A CPU executing 10 macro-iterations is preferable to a GPU executing 500 raw ones, and the same LA/AT design can later be ported to SIMD or GPU kernels.

## A concrete Rust architecture for your renderer

I would evolve your existing perturbation+BLA engine rather than replace it.

Start by making a small **extended-range scalar that does not add significand precision**:

```rust
#[derive(Clone, Copy, Debug, Default)]
struct FExp {
    mant: f64,  // normalized, e.g. |mant| in [0.5, 1) except zero
    exp2: i32,
}

#[derive(Clone, Copy, Debug, Default)]
struct CExp {
    re: f64,
    im: f64,
    exp2: i32, // shared complex exponent
}
```

This is the basic design used in spirit by Imagina's `FloatExp` and rust-fractal's `FloatExtended`/`ComplexExtended`: keep a normal machine-float significand but carry range separately. fileciteturn12file0 fileciteturn16file0turn16file1 A shared exponent for a complex mantissa is particularly convenient for SIMD and norm comparisons.

Do **not** restrict this representation to `dc`. It needs to cover every BLA quantity whose magnitude participates in validity:

\[
dc,\quad dz,\quad A,\quad B,\quad R_z,\quad R_c,
\quad |B|\,|dc|,
\quad\frac{R}{|A|},
\]

and any second-order coefficients you later add. If a radius calculation converts through `f64` even once, an otherwise exponent-safe renderer can still hit exactly the cliff you are seeing.

For comparisons, avoid reconstructing the floating-point value. Compare an exponent plus normalized mantissa. For multiplication, exponents add. For addition, align the smaller exponent; when the exponent difference is beyond the significand's meaningful range, dropping the smaller operand is mathematically the same thing normal floating point would do. That keeps the fast path far cheaper than arbitrary precision.

Then add an **Imagina-like LA entry** alongside your current BLA object. Conceptually:

```rust
struct LaEntry {
    len: u64,

    // Reference state at entry.
    z_ref: CExp,

    // Linear response over this macro-step.
    z_coeff: CExp,
    c_coeff: CExp,

    // Safe domains.
    z_threshold: FExp,
    c_threshold: FExp,

    // Useful for period detection.
    min_ref_mag: FExp,
}
```

Your existing BLA composition already gives you much of `z_coeff` and `c_coeff`. The important additions are a separately propagated \(c\)-domain threshold, record-minimum/period information, and the ability to construct **a new stage from the previous stage**, rather than merely constructing the dyadic level \(2L\) from two adjacent length-\(L\) entries. Imagina and FractalShark both expose precisely this stage abstraction. fileciteturn4file0 fileciteturn8file0

I would implement the Imagina acceptance form before inventing a new bound. Compute the nonlinear first-step quantity

\[
q=dz(2Z+dz)
\]

and require its infinity norm to fit the entry's threshold; maintain conservative thresholds during construction based on reference magnitude divided by the appropriate coefficient magnitude. The source's \(2^{-24}\) safety scale is extremely conservative for a double-mantissa CPU implementation; I would initially copy its philosophy, then tune only after differential testing. fileciteturn8file0

Next, make **period discovery a side effect of LA generation**. Maintain the running minimum of reference magnitude. When an LA step or macro-step produces a sufficiently strong new minimum, record its cumulative raw iteration length as a period candidate. Imagina does this with `MinMag`/period-detection thresholds, while Fraktaler 3 now performs a reference periodicity check as well. fileciteturn6file0 citeturn29search3

Once a strong candidate \(p\) exists, run high-precision Newton on the **reference parameter only** to refine a nucleus satisfying

\[
F_c^p(0)=0.
\]

Do not do this for every pixel. Recompute the reference from the refined center if Newton succeeds. Atom-domain minima are the right way to avoid trying every period. citeturn23search0

With a nucleus-aligned reference, add two accelerators from the same data.

The first is **interior contraction**. Propagate the derivative needed for the nucleus-based interior test through plain iterations and LA entries. If one LA entry represents a map with local \(z\)-coefficient \(A\), then the corresponding derivative update is a macro multiplication by that map's derivative rather than \(L\) individual multiplications. Stop interior pixels once the contraction criterion is met. This turns high-period interior from “execute `max_iter`” into “execute enough macro-return iterations to establish attraction.” citeturn28search3

The second is **AT/return-map transformation**. When your LA stages discover a repeating macro-step of length \(L\), derive a transformed parameter and map whose one iteration represents \(L\) raw iterations, subject to a \(dc\) threshold. Set

\[
N'=\left\lceil N/L\right\rceil.
\]

That is the mechanism I expect to have the largest effect on your current 511-iteration residual. Imagina's public code is the reference implementation I would port from; FractalShark is useful as a more heavily templated second implementation for checking your interpretation. fileciteturn11file1 fileciteturn4file0

After this is stable, add **quadratic LA entries**:

```text
out =
    A*z
  + B*c
  + C*z*z
  + D*z*c
  + E*c*c
```

Do the composition with a tiny truncated-polynomial type rather than writing the coefficient equations by hand. In Rust, represent a two-variable polynomial jet indexed by \((i,j)\) with \(i+j\le2\), and make composition automatically discard terms above total degree two. This dramatically reduces the chance of a coefficient algebra bug.

The hard part is not the five coefficients; it is the **error radius**. For production correctness, maintain an upper bound on the discarded degree-\(\ge3\) remainder and accept the macro-step only when that bound is below your numerical/pixel error budget. The 2026 WebGPU implementation demonstrates the payoff of pushing the neglected term from quadratic to cubic, but its empirical performance numbers should be treated as motivation rather than as a substitute for your own CPU error proof and benchmark. citeturn29search0turn29search6

I would keep classic series approximation as an optional **prefix accelerator**, particularly because `rust-fractal` provides a straightforward Rust design. A tiled probe grid can cheaply answer, “How many initial iterations can every pixel in this cell replace by one polynomial evaluation?” The validity test

\[
\frac{\|z_\text{perturb}-z_\text{series}\|^2}
     {\max(1,\|D_\text{series}\|^2)}
\le
(\text{pixel spacing})^2
\]

is an appealing screen-space guard. fileciteturn15file0 But I would not make SA your primary answer to arbitrary 511-iteration holes: it primarily removes a prefix, while LA/AT and quadratic BLA can attack later recurrences as well.

NanoMB is worth a later specialized path if high-period minis are central to your workload. Rather than porting KF's implementation literally, I would reuse its idea: once you know a nucleus and period \(p\), construct a bivariate return polynomial \(P(dz,dc)\) for \(F^p\), and use it only when its rigorously/conservatively estimated domain beats your LA/AT path. One evaluation then costs according to polynomial degree rather than \(p\). KF's source is valuable for the polynomial machinery and shifted minibrot hierarchy, but the explicit FIXME in its interior convergence and missing NanoMB2 glitch detection make it a research reference rather than a correctness template. fileciteturn18file0

Finally, add counters before tuning anything. For each pixel/frame I would collect:

| Counter | Why it matters |
|---|---|
| Largest accepted BLA/LA level | Distinguishes “table has no useful macro-entry” from lookup problems. |
| Failed acceptance reason: `dz`, `dc`, radius, exponent overflow/underflow, rebase | This should expose your \(10^{-300}\) cliff immediately. |
| Raw iterations remaining after every accelerator | The key number to drive toward zero. |
| Number and cumulative length of accepted macro-steps | Shows whether AT/LA is actually collapsing long periods. |
| Rebase count | Separates reference quality problems from approximation validity. |
| Candidate period and nucleus-refinement success | Diagnoses deep-minibrot behavior. |
| Interior classification stage / raw-equivalent iteration | Quantifies how much max-iteration work disappears. |
| FloatExp versus plain-double operation count | Lets you demote stages back to native `f64` when their range permits, as Imagina does. fileciteturn11file0 |
| First- versus second-order acceptance | Tells you whether quadratic BLA earns its extra arithmetic. |

For your exact case, the success criterion is not “BLA skip percentage went up.” It is much sharper: **make the 32% no-skip cohort disappear, or make its median remaining raw iteration count collapse from ~511 to a few macro-iterations.** Extended-exponent state should remove the depth cliff; multi-stage LA/AT should attack the exterior and near-periodic residual; nucleus/atom-domain plus contraction should remove the high-period interior residual. Second-order BLA is the next lever if a substantial set of boundary pixels still fails first-order validity.

That combination—**exponent-safe LA state + hierarchical macro stages + periodic AT + nucleus-aware interior contraction**—is the strongest architecture in the current public CPU deep-zoom lineage. Traditional SA and NanoMB remain useful specialized accelerators, while GPU reference computation becomes important much later, when high-precision reference arithmetic rather than those ~85 million low-precision pixel iterations is actually the dominant cost. fileciteturn8file0 fileciteturn4file0 citeturn28search3