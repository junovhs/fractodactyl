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
