# Explicit error radius for the parameter-dependent Koenigs chart at the 2-cycle

Owner-run deep-research answer, pasted 2026-10-09. Question asked: for z² + c near a
repelling 2-cycle, what is the explicit radius in c and w where the N-term
parameter-dependent Koenigs series and its inverse stay under a given error?

## Check against v0 (added 2026-10-09, `tools/research/misiurewicz/own_c_koenigs.py`)

This is a numerical consistency check, not a proof check.

- The formulas reproduce λ(C) = 1.02683+0.52496i, and the expansion coefficients
  a = 4p² + 2q, b = 4p match F_c = f_c∘f_c expanded at the cycle point.
- ρ_c = 0.0192: one chart in c covers every v0 frame narrower than about 1e-2.
- The majorant claim Σ|k_n|Rⁿ ≤ R/4 holds with about 80x slack (3.4e-6 against 2.6e-4 at
  p₊; 1.8e-6 against 1.6e-4 at p₋).
- **The certified radius is far smaller than the working one.** R = 1.05e-3 (p₊) and
  6.5e-4 (p₋), so ρ_w ≤ 1.05e-4 and 6.5e-5, against fd's jump radius 0.03. The computed
  coefficients grow like 2.9ⁿ and 4.2ⁿ (series radius about 0.35 and 0.24), and the
  18-term truncation error at |w| = 0.03 is 5e-21 and 6e-18.
- Jumping only to the certified radius would leave about 40 more laps (80 plain steps) per
  pixel, more than the whole remaining tail. Use the theorem as the fallback proof; get
  the working radius from an a-posteriori bound on the computed coefficients (PROB-13).

## The answer as received

Work with F_c = f_c∘f_c, so the repelling 2-cycle becomes a repelling fixed point.

### 1. The 2-cycle and parameter radius

The cycle points are p±(c) = (−1 ± √(−3 − 4c))/2, with multiplier under F_c

    λ(c) = 4(c + 1).

Fix c0 with L := |4(c0 + 1)| > 1, choose one cycle point p0 and call the other q0. An
admissible parameter radius is

    ρ_c = (L − 1)/8.

For |c − c0| ≤ ρ_c the cycle points continue analytically and |λ(c)| ≥ (L + 1)/2 > 1.

### 2. Constants for the spatial radius

    d0 = −3 − 4c0,   m = ρ_c·√(2/|d0|),   q = (L + 1)/2
    A = 4(|p0| + m)² + 2(|q0| + m)
    B = 4(|p0| + m)
    D = q(q − 1)
    R = min{ 1/4, D / (16(A + B + 1)) }

### 3. The error-radius theorem

With u = z − p(c) and G_c(u) = F_c(p(c) + u) − p(c), the normalised Koenigs map H_c and
its inverse K_c satisfy

    H_c(G_c(u)) = λ(c)·H_c(u),   K_c(λ(c)·w) = G_c(K_c(w)),
    H_c(0) = K_c(0) = 0,   H_c'(0) = K_c'(0) = 1.

Let H_{c,N} and K_{c,N} be the degree-N Taylor truncations. For N ≥ 1 and ε > 0 take

    ρ_w = min{ R/10, (R/5)·(20ε/R)^(1/(N+1)) }.

Then for |c − c0| ≤ ρ_c and |w| ≤ ρ_w:

    |H_c(w) − H_{c,N}(w)| ≤ ε
    |K_c(w) − K_{c,N}(w)| ≤ ε
    |K_c(w) − (H_{c,N})⁻¹(w)| ≤ ε

The third line is about the actual local inverse of the truncated polynomial. The common
stronger estimate, for |w| ≤ R/10, is

    max{ |H_c − H_{c,N}|, |K_c − K_{c,N}|, |K_c − H_{c,N}⁻¹| }(w) ≤ (R/20)·(5|w|/R)^(N+1).

### 4. Why the bound works

    G_c(u) = λ(c)·u + a(c)·u² + b(c)·u³ + u⁴,   a = 4p² + 2q,   b = 4p.

On the parameter disk |a| ≤ A, |b| ≤ B, |λ| ≥ q. The inverse linearizer's coefficients
satisfy

    (λⁿ − λ)·k_n = [wⁿ](a·K² + b·K³ + K⁴),   n ≥ 2,

and |λⁿ − λ| ≥ q(q − 1) = D, so a positive-coefficient majorant gives
Σ_{n≥2} |k_n|Rⁿ ≤ R/4. Hence |K_c(w) − w| ≤ |w|²/(4R) and |K_c'(w) − 1| ≤ 1/4 on
|w| ≤ R/2, which gives local univalence. Inverse-function and Cauchy estimates then give
|h_n| ≤ (R/49)(4/R)ⁿ, and summing the geometric tails gives the estimate above.

These are conservative certified radii, not maximal convergence radii. They cover
truncation in w with the coefficient functions evaluated exactly in c. If the
coefficients are themselves Taylor-truncated in c − c0, a parameter-truncation error
must be added.
