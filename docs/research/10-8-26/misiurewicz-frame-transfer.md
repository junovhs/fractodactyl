# Misiurewicz frame transfer: genuine deep frames as exact transforms of shallower ones

Probe run 2026-10-08. Scripts and data are in the session scratchpad; the commands are
reproduced below.

## Idea (complex dynamics × video codecs)

Tan Lei (1990, later Kawahira) proved that the Mandelbrot set is asymptotically
self-similar about every Misiurewicz point c0. For a preperiod-l, period-p point with
cycle multiplier ρ = (f^p)'(z_l), zooming in by |ρ| and rotating by −arg ρ reproduces
the picture, and the error vanishes with depth. In video-codec terms, the deeper frame is
the shallower frame under a known "motion vector" (an exact similarity) with a residual
that tends to zero. Artists use this for approximate "quantum zoom loops" ("never actually
continuous"). We found no renderer that uses it to produce genuine frames under a
measured error contract.

## Measurements (480x270, our kernel, fd render; error = oracle-style px displacement, |Δν|·ln2·de)

**c = i** (M(1,2), ρ = 4(1+i), |ρ| = 4√2, arg ρ = 45°), 1e-300 against 1e-300/|ρ|
rotated −45°:
- 0/129,600 class mismatches.
- ν shift exactly +2.000000000 (the period); max deviation 3.6e-9.
- de and normal bitwise identical.
- Rotating +45° instead fails, so the sign matters.

**v0 path target** (period-764 minibrot C, 1.7e-25 from M(24,2), whose
ρ = 1.02683 + 0.52496i, |ρ| = 1.15324, arg ρ = 27.08°, so the picture repeats every
**0.0619 decades**; the v0 path moves 0.0657 decades/frame). The frame centred at C with
width w/|ρ|, rotated −arg ρ, is compared against the frame centred at
**C' = c0 + ρ(C − c0)** (the exact shift) with width w, and ν − 2:

| shallow width | class mismatches | displacement p50 | p99 | p99.99 | max |
|---|---|---|---|---|---|
| 1e-6 | 0 | 6.6e-3 | 6.4e-2 | 3.3 | 8.3 px |
| 1e-8 | 0 | 8.3e-5 | 4.5e-4 | 2.7e-2 | 0.10 px |
| 1e-10 | 0 | 1.0e-6 | 5.3e-6 | 3.3e-4 | **9.8e-4 px** |
| 1e-12 | 0 | 1.2e-8 | 6.4e-8 | 1.2e-6 | 1.2e-5 px |
| 1e-16 | 0 | 1.8e-12 | 9.7e-12 | 1.0e-7 | 3.5e-7 px |
| 1e-20 to 1e-23 | 0 | 0 | 8e-12 | 8e-8 | ≤3.5e-7 px |

The error falls about 100x per 2 decades, as the O(w) linearisation error predicts. From
1e-10 down it is inside the 1e-3 px `fd compare` contract, and from 1e-16 it is at the
kernel's noise floor. Without the C' shift, the median ν offset drifts (2.00007 at 1e-20,
1.43 at 1e-24): the shift is required.

## What it buys

- **Any zoom into a Misiurewicz zone is one period of work.** Render one log-period
  (0.062 decades for M(24,2)) as an exponential-map ring around c0. Every deeper frame,
  at any depth, speed or rotation, is a similarity transform of that ring, with
  ν += p·k, de unchanged and the normal rotated with the view. The ring is expensive to
  build once and inherently cross-frame: an atlas payload an independent renderer cannot
  match without caching it.
- **v0 path:** widths from about 1e-10 to about 1e-22 (frames ~161-343, 182 of 750,
  roughly 90 s of BENC-01's 480 s) are in the zone. The expensive tail (1e-29 to 2e-49)
  is not: see below.
- **Path design** (NORTH-STAR: choose the cheaper equally beautiful genuine path): paths
  that linger in Misiurewicz zones become almost free. c=i-style zooms to 1e-1000 cost
  one ring.

## What it does not buy (tested)

**Minibrot renormalisation as a similarity.** The deep frame near the period-764 nucleus
(|λ| = 4.12e-50, arg λ = −0.589) was compared with whole-M frames scaled by λ, at
2e-49 through 1e-44. The minibrot body matches: its interior pixel count agrees within
0.4% (deep pixels show Unresolved because of FIX-04). The exterior does not match (de log
ratio about 6.6, normals about 80° apart), because a deep minibrot is wrapped in
decoration hairs that the shallow set lacks. So the final approach is not an affine copy
of shallow frames.

**Analytic exterior patches** (Böttcher-coordinate polynomials in M-free disks): on the
hard views (valley 1e-28, v0 at 1e-40 and 2e-49), 0% of pixels are ≥32 px from the set.
Only Misiurewicz/dendrite views (c=i: 83%) have large empty regions. Killed as a general
accelerator.

## Open next hypotheses

1. **The "Julia morphing" band between the zones** (1e-25 to 1e-45 on v0). In the
   minibrot's renormalised coordinate u = (c − C)/λ, the decoration levels sit at scales
   related by u → u² (each level is a log-scale halving). Hypothesis: level j+1 is the
   conformal z² warp of level j, plus a ν offset. That is a nonlinear motion vector, not
   a similarity. Probe it the same way.
2. **Error contract** (DEC-10): use a rigorous O(w) linearisation bound, or spot-check K
   true samples per frame against the transform, falling back to a direct render on any
   miss.
3. **Choosing c0 automatically:** find the Misiurewicz point governing a path segment
   (atom-domain and preperiod search on the reference orbit) and the zone limits
   (shallow: error < 1e-3 px; deep: w ≳ 1000·|C − c0|).

## Reproduce

```text
# c = i
fd render --re 0 --im 1 --width 1e-300 --size 480x270 --iter 20000 --columns nu,de,normal -o A.fds
fd render --re 0 --im 1 --width 1.767766952966368811e-301 --rotation -0.7853981633974483 ... -o B.fds
# v0: c0 = M(24,2) refined by Newton on z26 - z24; rho = 4 z24 z25; C' = c0 + rho (C - c0)
fd render --re C'.re --im C'.im --width 1e-12 ... -o s12.fds
fd render --re C.re --im C.im --width 1e-12/|rho| --rotation -arg(rho) ... -o v12d.fds
# compare: class equality; displacement = |nu_B - nu_A - p| * ln2 * de_A
```

## Follow-up (same day): the approach band IS structured. Returns plus one shared exit table

The hypothesis "level j+1 is the z² warp of level j" was tested pointwise with mpmath
(oracle-grade, 110 digits), using 120-160 random pixels per depth from 1e-30 to 2e-48 on
the v0 target. Error is measured as px displacement against direct iteration of the
deep pixel.

1. **Naive z² map** (ν(C+d) = ν(C+d+B²d²) + p): fails, with errors of hundreds of px at
   every depth. Kicking the orbit's position once is not the same as shifting the
   parameter.
2. **Tan Lei correspondence after one return** (c' = c0 + (z_{p+1} − c0)·D/Δ′(c0), with
   D = (f^{l−1})′(c0), Δ′ = d/dc(z_l − α(c)), K = 0.110+0.146i): **exact at 1e-30**
   (ν offset 764.000000, max 7.8e-7 px), near-exact at 1e-35, and broken deeper,
   because one return does not get the orbit far enough from the minibrot.
3. **Repeated returns** (keep adding periods while |z − C| < 1e10·|C − c0|): ν offset is
   exactly k·764. The error is then purely a function of the exit distance |ζ − c0|:
   ≤1e-7 gives ≤1e-4 px; 1e-6 gives 6e-3; 1e-5 gives 3e-2; ≥1e-3 gives large errors
   (the parameter-plane linearisation range).
4. **Exit tail in the dynamical plane with the fixed parameter c0** (ν(c) = k·p + 1 +
   E_{c0}(ζ), where E_{c0} is the escape of the point ζ under z² + c0): **exact at every
   depth tested**. 0 class mismatches; max displacement 1.2e-8 px (1e-40), 2.5e-12
   (1e-43), 4.8e-10 (1e-46), 4.2e-12 (1e-47), 2.3e-9 (2e-48). Returns per pixel:
   k = 2 to 7.

**Meaning:** every pixel of the minibrot approach band decomposes into (a) k returns
around the minibrot (period p = 764 each), then (b) a tail that depends only on the exit
point ζ, through **one fixed function E_{c0}** on the dynamical plane. E_{c0} does not
depend on the pixel, the frame or the depth. Near c0 it is exactly self-similar (the
repelling 2-cycle), so it is a finite object: a self-similar ring plus a shallow region.

**Not yet shown:** (i) cost. The returns were computed by brute iteration here; the win
needs a cheap per-period return map (NanoMB/Imagina-AT style) accurate to this
tolerance. (ii) A sampled E table with an interpolation error contract and its size.
(iii) Nested minibrot chains (deeper zooms), where each level adds returns.

**If (i) and (ii) hold:** a deep pixel costs about k ≤ 7 return-map evaluations plus one
table lookup, instead of 1,600-4,200 iterations (about 536 with BLA). That is roughly
depth-independent, and E is shared by every frame of the approach: a genuine atlas
payload.

## PROB-07: tight exits recover the decomposition (2026-10-07)

The old 1.7e-15 exit radius required the biseries to operate far outside its local
domain. The new `returns_exit_tail.py --tight-probe` stops before an input exceeds
1e-26, using the candidate's own orbit and return count. At that exit it evaluates
**1+k*764+E_C(zeta)** with parameter fixed at the minibrot nucleus C. C differs from
c0 by about 1.7e-25, which matters in this transition band. The pixel parameter is
used only by the biseries and the auxiliary exact-prefix diagnostic, never E_C.

On the unchanged PROB-03 frozen 48-point pack, degree 4 passes all six depths:
maximum smooth error 2.172e-4 px, maximum local state displacement 2.237e-4 px,
zero class mismatches. Degrees 6 and 8 also pass. Degree 2 at 1e-26 fails at
1e-38 (1.550 px), 1e-43 (0.02235 px local), and 1e-46 (0.06015 px smooth).
With a narrower 1e-30 guard it passes all six (worst 7.652e-5 px).
These are sampled results, not certified domains or full-frame oracle promotion.
METHOD.md holds all per-depth/degree errors, operation models and BLA comparison.
The common-guard probe including six separate 8x4 per-frame-BLA controls took 2.49 s,
exit 0; no shared table was built. BLA uses a different point cohort, so no measured
speedup is claimed.

For degree 4, the candidate uses 1–5 returns per pixel (240–1080 mean arithmetic
ops/pixel across depths), then one projected table lookup. Direct E_C evaluation
still takes 161–849 mean raw steps/pixel. The fixed-C control from an exact prefix
has max smooth displacement 1.45e-11 px, so the observed error is dominated by
map truncation, not switching the tail parameter from C+d to C.

### Exit-table handoff for PROB-04

The **input guard does not bound the output radius**. Degree-4 exits span
1.249e-26 to 7.648e-5; common-guard bin counts are identical for degrees 2/4/6/8.
The smaller quadratic guard spans 1.441e-30 to 7.648e-13 and costs more direct-tail
work. Bins below are floor(log10(|zeta-C|)), exponent: sample count, eight per row.

| Width | Common guard 1e-26 | Quadratic guard 1e-30 |
|---|---|---|
| 1e-35 | -22: 3, -21: 5 | -22: 3, -21: 5 |
| 1e-38 | -7: 1, -6: 3, -5: 4 | -28: 3, -27: 5 |
| 1e-40 | -15: 1, -14: 3, -13: 4 | -15: 1, -14: 3, -13: 4 |
| 1e-43 | -26: 3, -25: 4, -6: 1 | -27: 1, -26: 3, -25: 4 |
| 1e-46 | -26: 1, -24: 4, -10: 1, -6: 1, -5: 1 | -30: 1, -28: 1, -27: 1, -26: 1, -24: 4 |
| 2e-48 | -26: 4, -25: 1, -18: 1, -16: 1, -12: 1 | -30: 1, -26: 4, -25: 1, -18: 1, -16: 1 |

Use E_C, not E_c0, in the transition region. The sampled exit points range well
beyond 1e-22 and, for the narrower quadratic variant, begin below 1e-26. Any
connection to the self-similar Misiurewicz ring requires a measured transfer/error
contract. Histograms give radial coverage, not a table-size estimate: angular
coverage, resampling and unresolved/interior regions still need measurement in
PROB-04. A per-frame renderer can also build the biseries; the expensive shared
E_C table is the candidate cross-frame atlas payload (DEC-15).

## PROB-04: uniform log-polar exit tables fail interpolation (2026-10-07)

`returns_exit_tail.py --table-probe` builds E_C tables over log10 |zeta-C|
from -26 through -4, with a periodic angular seam. It stores smooth escape nu
and escape step count, uses bilinear log-polar interpolation, and applies the
degree-4 prefix at guard 1e-26. No c0 substitution or unvalidated self-similar
wrap is used. The same table is queried by every one of the six depth cohorts.

The 8 radial intervals/decade x 64-angle configuration has 11,328 nodes and
135,936 payload bytes; the 32 x 256 configuration has 180,480 nodes and
2,165,760 bytes (2.07 MiB). Both fail on the unchanged 48-point frozen pack:
max displacement 154.713 / 27.004 px, versus the required 0.001 px. Each has
zero class mismatches and zero query-domain fallbacks. Payload size fits the
atlas budget, but accuracy does not. This is a rejection of these uniform nu
tables, not a lower bound on the size of every possible representation.

The failure persists when every used interpolation corner is evaluated directly
at 110 dps (150 / 152 distinct corners). At the finer resolution, per-depth
maximum errors remain 0.963, 1.323, 1.095, 3.788, 8.048 and 27.004 px.
The same degree-4 exits with direct double perturbation tails instead of table
lookups pass within 2.172e-4 px; PROB-07's 110-dps local-prefix diagnostic stays
within 2.237e-4 px. Thus interpolation, rather than the fixed-C decomposition,
is the decisive failure. METHOD.md includes the metric, all controls and timings.

The final run takes 4.840 s (exit 0), including table builds, exact-node checks
and matched-point BLA controls. Warm predecoded Python map+lookup takes
18.776 / 28.123 us/pixel; Rust per-frame BLA on the identical points as 1x1
views measures 283.213 us/pixel of render time. Different column work and
one-pixel reference/overhead effects prevent a frame-speed claim. Both
correctness-gated scores are 0. One table is shared by six diagnostic depth
cohorts (6:1); accepted coverage across the actual 750-frame v0 path is not
measured.

**Outcome:** recorded kill under PROB-04's alternative proof of done. DEC-14
stops these candidates before >=1,000 points/depth promotion; no widened
correctness or atlas-win claim is made. Rejected table values require direct
fallback. Adaptive or resume-state representations remain untested and would
need a new cheap probe. Continue the plan with PROB-05.

## Koenigs exit tail and real timing: the deep band is 8.5-25x faster (2026-10-07)

PROB-04 showed the exit function cannot be interpolated from a table. Instead the exit
tail is now *computed*, cheaply and exactly:

1. **Loops** (PROB-07): k returns of the degree-4 biseries in u = (z − C)/1e-25, while
   |z − C| ≤ 1e-28 (degree 3 also passes; degree 2 and guards of 1e-26 or ≤ 1e-29 fail on
   whole frames). The v powers are folded in once per pixel, so a return is a degree-4
   Horner in u.
2. **Approach:** 23 double perturbation steps against C's critical orbit, from the exit
   offset δ = ζ − C to the Misiurewicz neighbourhood of the 2-cycle point α_C.
3. **Koenigs jump:** φ (12-term Koenigs series of f_C² at α_C, multiplier
   ρ = 1.02683+0.52496i) linearises the spiral-out. Jump j = ⌊log(R0/|φ(h0)|)/log|ρ|⌋
   cycles at once (R0 = 0.03), then invert with Newton on φ.
4. **Finish:** 56-342 plain double steps of z² + C to escape.

Everything per pixel is IEEE double. The per-zone constants (biseries, orbit, α, ρ, φ) are
computed once at high precision (`koenigs_bench_consts.py`, about 0.1 s).

**Result (PROB-08):** Rust bench `tools/research/misiurewicz/koenigs_bench`, GitHub
Actions workflow `koenigs-bench.yml`. Whole 480x270 frames centred at C, at 1e-35, 1e-38,
1e-40, 1e-43, 1e-46 and 2e-48. Every pixel was compared with `fd control --bla per-frame`:
0 wrong pixels and 0 class mismatches out of 777,600, max 3.2e-4 px.
- Speed-up against fd's per-frame BLA: **8.1-25x**, across three runs (4 threads twice,
  1 thread once).
- Speed-up against a lean double-perturbation loop written in the same file: 11-37x.
- The pipeline's frame time is flat with depth; both opponents slow down as depth grows.

This is a per-frame technique (DEC-15). Any renderer that knows the zone can use it.

**What's left per pixel:** the 56-342 finish steps. The diagnostic
(`koenigs_leftover.py`) shows the long tails wander 0.1-0.5 from the weakly repelling α
fixed point (|λ| = 1.00622) near the parabolic c = −3/4: the "seahorse gate". The
deep-research report `skipping-near-parabolic-transits-what-is-computable.md` gives exact
ways to skip it: Koenigs at α, Kapiamba's q = 2 near-parabolic identity, Buff
coordinates, and Braverman long iterates. PROB-09 tests them.

**Lessons:**
- 48 frozen points were not enough. Two configurations passed them and failed whole
  frames (DEC-17, proposed).
- Script bugs that looked like results: ζ rounded straight to double, and ν off by one.

**Next:**
- PROB-09: skip the gate.
- PROB-10: every v0-path frame in the band, at full resolution.
- PROB-11: off-centre and shallower frames (a c ≠ C correction).
- KERN-01: put it in fd so the films get faster.
- PROB-06: other zones, with automatic detection.
