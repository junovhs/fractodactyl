# The parameter derivative through the Koenigs jump (for de and normal)

Owner-run deep-research answer, pasted 2026-10-09, condensed here (formulas and numbers
kept, prose shortened). Question: with u0 = z_{n0} − p(c), w0 = H_c(u0) and the jumped
point Z(c) = p(c) + K_c(λ(c)^j·w0) = z_{n0+2j}(c), give dZ/dc exactly and say which terms
matter.

## Check against v0 (added 2026-10-09, `tools/research/misiurewicz/jump_derivative_double.py`)

Measured in doubles on 250 pixels per width, scored on g = z/(dz/dc) at escape
(de = 2|g|ln|z|, normal = arg g), against 70-digit direct iteration.

- **Dropping the cycle-point term gives a constant 2.7% error at every depth**, as the
  answer's table predicts (T_p/T_D = 2.68e-2). ν is unaffected, so only the shading shows it.
- **Formula (2) is exact to the double noise floor:** at most 5e-8 at 1e-6 and 1e-9
  deeper, on pixels at least 0.01 px from the set.
- **The short form (8) is enough from 1e-9 down** (3.6e-7 at 1e-9, 1e-9 deeper) and fails
  at 1e-6 (3e-4), because H′(u0) ≠ 1 there, as the answer warns (|H′ − 1| ≈ 8.46|u0|).
- Pixels closer than about 1e-7 px to the set show errors up to 2e-2 with every formula:
  that is the position noise of doubles divided by a vanishing de, not the jump.

## 1. Exact derivative

With D0 = dz_{n0}/dc, q = λ^j·w0, λ′ = 4:

    dZ/dc = p′ + ∂_cK_c(q) + λ^j·K_c′(q)·[ H_c′(u0)(D0 − p′) + ∂_cH_c(u0) + (4j/λ)·w0 ]    (1)

| Term | Expression | Origin |
|---|---|---|
| T_D | λ^j K′(q) H′(u0) D0 | propagated critical-orbit derivative |
| T_p | p′[1 − λ^j K′(q) H′(u0)] | motion of the cycle point |
| T_H | λ^j K′(q) ∂_cH(u0) | parameter dependence of the inverse coordinate |
| T_λ | (4j/λ) q K′(q) | parameter dependence of λ^j |
| T_K | ∂_cK(q) | parameter dependence of the forward coordinate |

    p′ = −1/(2p + 1)
    H′(u) = 1/K′(H(u)),   ∂_cH(u) = −∂_cK(H(u)) / K′(H(u))

so, using only the forward linearizer:

    dZ/dc = p′ + (λ^j K′(q)/K′(w0))·[ D0 − p′ − ∂_cK(w0) + (4j/λ)·w0·K′(w0) ] + ∂_cK(q)    (2)

## 2. Size of the λ^j term

    |T_λ|/|T_D| = 4j|w0| / (|λ|·|H′(u0)|·|D0|) ≈ (4j/|λ|)·|u0|/|D0|                       (3)

The factor j comes with the small entry offset, so it is not large by itself; but
|u0| ≤ 1e-6 and j ≤ 350 alone do not guarantee 1e-6 (the bound is 1.214e-3/|D0|, about
2.4e-5 at |D0| = 51.5). T_p is about |p′/D0| and cannot be dropped.

## 3. Coefficient derivatives

    ∂_cK(w) = Σ k_n′ wⁿ
    k_n′ = S_n′/(λⁿ − λ) − 4(nλ^(n−1) − 1)·S_n/(λⁿ − λ)²,   S_n = [wⁿ](a2K² + a3K³ + K⁴)   (5)
    a2 = 6p² + 2c (= 4p² − 2p − 2),   a3 = 4p,   a2′ = 12pp′ + 2,   a3′ = 4p′
    H′(u) − 1 = −2k_2·u + O(u²),   ∂_cH(u) = −k_2′·u² + O(u³)

At c0 = −0.74329189 + 0.13124055i, p = −0.749703588569 + 0.262792679016i:
k_2 = −4.156673508 − 0.796185293i, k_2′ = 19.276717454 − 16.733742884i, so for
|u| ≤ 1e-6: |H′ − 1| ≲ 8.465e-6 and |∂_cH| ≲ 2.553e-11.

## 4. Benchmark numbers (n0 = 24, q = 0.03 real, N = 18)

|D0| = 51.53377920, |p′| = 1.379280828, |K′(0.03)| = 0.794688453,
|∂_cK(0.03)| = 0.017365295.

| Term (relative to the largest) | j = 100 | j = 350 |
|---|---|---|
| T_D | 1 | 1 |
| T_p | 2.67646e-2 | 2.67646e-2 |
| T_H | 1.841e-16 | 2.016e-47 |
| T_λ | 1.2974e-7 | 1.503e-22 |
| T_K | 2.725e-10 | 9.017e-26 |

Short form, keeping only T_D and T_p with H′ = 1:

    dZ/dc ≈ p′ + λ^j·K′(q)·(D0 − p′)                                                      (8)

Relative error of (8) in the benchmark: 2.449e-7 (j = 100), 1.763e-22 (j = 350). The
answer notes these are for the stipulated inputs, not uniform guarantees, and that at the
rounded c0 the real entry offset is 1.3e-7 (the 100-digit v0 centre gives 9.0e-24).
