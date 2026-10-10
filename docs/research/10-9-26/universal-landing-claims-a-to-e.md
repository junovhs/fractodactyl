# The "universal landing" conjecture: what holds and what does not

Owner-run deep-research answer, pasted 2026-10-09, condensed (formulas and verdicts kept).
The question proposed that a deep pixel near a ladder rung is a few steps of ζ² + s
followed by one lookup of a fixed function on a torus.

## Check against the ladder (added 2026-10-09, `tools/research/misiurewicz/ladder_bridge.py`)

The answer's main correction is a "bridge": after the quadratic regime a pixel makes about
log2(n) + O(1) further returns before it reaches the linearised regime (eq. B3), so the
return count is not flat down a ladder. Measured as minibrot periods before escape, six
pixels at the same relative positions in a frame 5,000 sizes wide:

| rung | period | n | size | periods before escape | change vs v0 | log2(n/n0) |
|---|---|---|---|---|---|---|
| 0 (v0) | 764 | 369 | 4e-50 | 3.88-4.17 | 0 | 0 |
| 409 | 1,582 | 778 | 9e-101 | 4.88-5.18 | +0.96 | +1.08 |
| 7,676 | 16,116 | 8,045 | 1e-1000 | 8.23-8.56 | +4.32 | +4.45 |

The law holds: one more return each time the depth exponent doubles.

## Verdicts as received

| Claim | Verdict |
|---|---|
| A torus | Correct. g(w) = G_m(a + K(w)) satisfies g(ρw) = 2^r g(w); E = −log2 g satisfies E(ρw) = E(w) − r. The periodic representative is T(t) = E(e^t) + (r/ln\|ρ\|)·Re t on ℂ/(2πiℤ + log ρ·ℤ). g is harmonic off its zero set; E is real-analytic and subharmonic there (ΔE = \|∇g\|²/(g² ln 2)), +∞ on the zero set. g = \|w\|^β·h with β = r·ln2/ln\|ρ\| = 9.723 for M(24,2). |
| B landing | Wrong as stated. The Koenigs argument has a non-zero translation, and the orbit does not pass from "first \|ζ\| > R" straight to the outer regime. |
| C finite n | The return map is exactly even in the critical coordinate: no ζ³ term. First corrections are s², sζ², ζ⁴, all O(ρ⁻ⁿ). |
| D nesting | Valid in renormalised coordinates with scale separation \|ρ\|⁻ⁿ ≪ \|σ\|⁻²ˡ; the inner linearizer's domain is O(1) in ζ and shrinks like \|ρ\|⁻ⁿ only in the physical z. Periods multiply; errors propagate through the Lipschitz constants of each level. |
| E derivative | Chain rule on the corrected landing coordinate; see (E1). |

## The corrected framework

Normalise with D_n = (f_{c_n}^{P_n−1})′(c_n), T_n = ∂_c f_c^{P_n}(0) at c_n, b_n = 1/(D_nT_n),
c = c_n + b_n·s, z = ζ/D_n, F_{n,s}(ζ) = D_n·f_c^{P_n}(ζ/D_n) = ζ² + s + …

With W_c(z) = K_c⁻¹(f_c^q(z) − a(c)) = t_c + u_c z² + O(z⁴) and Z_j = F_{n,s}^j(0):

    exact:     ν∞(c) = j·P_n + q + r·h + E_c(ρ(c)^h · W_c(Z_j/D_n)) + C0,   C0 = 1 + log2(ln 2)   (B1)
    matched:   ν∞(c) = j·P_n + q + n·r + E_m(w_{n,s} + κ_{n,s}·Z_j²) + C0 + O(|ρ|⁻ⁿ)             (B2)
               w_{n,s} = ρ(c)ⁿ t_c,   κ_{n,s} = ρ(c)ⁿ u_c / D_n² ~ ρ⁻ⁿ
    bridge:    further returns after the quadratic regime  ℓ ≈ log2( n·ln|ρ| / (2·G_s(ζ_k)) )    (B3)

(B1) has no error and is the decomposition fd's deep path already computes (loops, approach,
jump, finish). The quadratic approximation holds while ε|ζ|² ≪ 1 with ε = |ρ|⁻ⁿ, but the
outer coordinate only becomes order one at |ζ| ≈ |ρ|^(n/2) (2.85e11 at v0), so there is no
overlap: the last returns need the true return map.

Return map to second order (C1):

    F = ζ² + s + a20·s² + a11·s·ζ² + a02·ζ⁴ + a30·s³ + a21·s²ζ² + a12·s·ζ⁴ + a03·ζ⁶ + …
    a20 = U_n/(2D_nT_n²),  a11 = D_n′/(D_n²T_n),  a02 = W_n/(2D_n³)          (all O(ρ⁻ⁿ))

At n = 370: relative error 1e-9 up to |ζ| ≈ 8.7e6 for degree 2 (and degree 3), about
1.5e9 for degree 4 with all three first-order terms (order-of-magnitude estimates).

Derivative, on a region with a stable exit itinerary:

    ∂_cν ≈ (E_{m,w}(w)/b_n)·[ dw_{n,s}/ds + (dκ_{n,s}/ds)·Z_j² + 2κ_{n,s}·Z_j·dZ_j/ds ]            (E1)

## What this changes for fd

- No new shortcut: the exact identity is the pipeline fd has.
- Per-pixel cost down a ladder is logarithmic in the depth exponent, not flat: about one
  extra minibrot return per doubling of n (measured above).
- The loop map can be carried in y = ζ² (it is exactly even), halving its degree.
- The torus function is a finite, periodic object; under a film-quality bar it could be
  precomputed once per spiral and sampled (this is PROB-05 in another coordinate).
