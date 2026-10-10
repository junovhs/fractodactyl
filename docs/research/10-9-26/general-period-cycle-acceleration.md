# Koenigs jump at a repelling cycle of any period r

Owner-run deep-research answer, pasted 2026-10-09, condensed here (recurrences and
numbers kept, prose and derivations shortened). Question: extend the closed-form r = 2
machinery (cycle point, multiplier, local map, linearizer, radius, derivative) to a
repelling r-cycle known only numerically.

## Check on a real location (added 2026-10-09, `tools/research/misiurewicz/general_cycle_jump.py`)

Centre orbit of Maths Town's "Eye of the Universe" (first 30,000 steps, 1,191 digits),
cycles refined by the Newton step below, linearizer built with the recurrences of
sections 2 and 3 at 90 digits, jumped point compared with the true orbit lap by lap.

- **The recurrences are correct.** Inside the usable radius the jumped point matches plain
  iteration to 2e-55 (r = 197) and 3e-71 (r = 655). Build time 0.1-3 s for N = 8-32.
- **r = 655** (dwell 9249..12059, 4.3 laps, |λ| = 9.51, entry offset 6.6e-12): one jump
  covers 3 laps with N = 8, 4 with N = 16, 5 with N = 32 (3,275 steps, out to |u| = 4.5e-5).
  The whole dwell is skippable.
- **r = 197** (dwell 15786..16902, 5.7 laps, |λ| = 5.78, entry offset 5.6e-8): only 2 of
  5.7 laps at any N up to 32. The orbit enters already at the edge of the usable radius.
- **Why the radius is small:** |a_m|^(1/(m−1)) reaches 2.2e7 (r = 197) and 3.6e9
  (r = 655), and |k_n|^(1/(n−1)) reaches 8.1e5 and 3.7e7, so the linear regime is about
  1e-6 and 3e-8 wide. 197 and 655 are record-return periods of this orbit: these cycles sit
  beside nested minibrots, each lap passes close to the critical point, and the lap map is
  quadratic-like there. This is section 4's warning (B enormous for long cycles) in practice.
- **Reading:** on this location the long "repelling-cycle dwells" counted by
  `orbit_laps.py` are nested-minibrot loops. The Koenigs jump handles their inner laps
  exactly; the outer laps need the return-map (biseries) tool fd already uses in the v0
  deep band. Centre orbit only, high precision only: no pixels, no doubles, no timing.

## 1. Cycle, its motion, the multiplier

With x_0 = z and partial derivatives A = ∂_z, B = ∂_c, C = ∂_zz, E = ∂_zc, H = ∂_cc of
x_i = f_c^i(z), started at (1, 0, 0, 0, 0):

    x' = x² + c,  A' = 2xA,  B' = 2xB + 1,  C' = 2A² + 2xC,  E' = 2AB + 2xE,  H' = 2B² + 2xH

    Newton:  z ← z − (x_r − z)/(A_r − 1)                 (2r multiplications per pass)
    λ = A_r,   ṗ_0 = B_r/(1 − λ),   p̈_0 = (C_r ṗ_0² + 2E_r ṗ_0 + H_r)/(1 − λ)
    ṗ_i = A_i ṗ_0 + B_i,   p̈_i = A_i p̈_0 + C_i ṗ_0² + 2E_i ṗ_0 + H_i

Division-free multiplier derivatives, with T = ∂_zzz, U = ∂_zzc, V = ∂_zcc:

    T' = 6AC + 2xT,  U' = 4AE + 2BC + 2xU,  V' = 2AH + 4BE + 2xV
    λ̇ = C_r ṗ_0 + E_r,   λ̈ = T_r ṗ_0² + 2U_r ṗ_0 + V_r + C_r p̈_0

Cost per pass: 2r (Newton), 9r (second-order jets), 17r (through third order).

## 2. Local return map in O(r·N²)

With U_i(u) = f_c^i(p_0 + u) − p_i truncated at degree N, coefficients α_{i,m}:

    U_0 = u,   U_{i+1} = 2p_i U_i + U_i²
    α_{i+1,m} = 2p_i α_{i,m} + Σ_{l=1..m−1} α_{i,l} α_{i,m−l},        a_m = α_{r,m}
    β_{i+1,m} = 2ṗ_i α_{i,m} + 2p_i β_{i,m} + 2 Σ_l α_{i,l} β_{i,m−l},   ȧ_m = β_{r,m}

Check: α_{r,1} = λ, β_{r,1} = λ̇. Cost about r(N² + 2N) multiplications, O(N) memory.

## 3. Linearizer

    k_1 = 1,   k_n = P_n/(λⁿ − λ),   P_n = Σ_{m=2..n} a_m [wⁿ]K^m
    k̇_n = ( Ṗ_n − (nλ^(n−1) − 1) λ̇ k_n ) / (λⁿ − λ)

Period-independent; only a_m, ȧ_m, λ, λ̇ change. One linearizer per cycle phase is
cheaper per pixel than aligning to p_0 (average (r − 1)/2 plain steps), and the others
follow from K_0 by transport, with d_i = 2p_i:

    K_{i+1}(w) = d_i K_i(w/d_i) + K_i(w/d_i)²
    k_{i+1,n} = d_i^(1−n) k_{i,n} + d_i^(−n) Σ_{l=1..n−1} k_{i,l} k_{i,n−l}

## 4. Radius and error

K is entire (K(w) = G^j(K(w/λ^j))); the limit is truncation accuracy and the inverse.
Given a scale S and a bound |a_m| S^(m−1) ≤ B for all 2 ≤ m ≤ 2^r, with L = |λ|,
d = L² − L:

    s = min(1/4, d/(16B)),   R = s/2,   θ = min{ 1/8, (ε/(2Ss))^(1/(N+1)) }
    ρ_w = S·R·θ   ⇒   |K − K_N| ≤ ε on |w| ≤ ρ_w, K injective there, inverse defined on |u| < ρ_w/2

B without forming F_c: h_0 = T ≥ S, h_{i+1} = 2|p_i| h_i + h_i², B = (S/T²)(h_r − L·T).
The bound grows with L at fixed B; for long cycles B can be enormous. Rescaling u helps
conditioning and overflow but does not enlarge the domain in z. Growth order of K is
r·ln2/ln|λ| (9.74, 114.4, 201.7 for the three cases below). Suggested practice: certify
conservatively, then find the working radius from the residual K_N(λw) − G(K_N(w)).

## 5. Break-even

Jump pays when r·j·C_f > 2·C_poly(N) + C_inv(N); with t Newton steps for the inverse,
roughly r·j > (2 + 2t)·N, i.e. r·j > 10N at t = 4.

| Case | Laps | Steps skipped | Largest N that still pays |
|---|---|---|---|
| r = 2, \|λ\| = 1.153, \|u0\| = 1e-15, ρ = 1e-2 | 210.3 | 421 | 42 |
| r = 197, \|λ\| = 3.3 | 18.6 | 3,664 | 366 |
| r = 655, \|λ\| = 9.5 | 4.3 | 2,817 | 281 |

## 6. Derivative

The r = 2 formula holds unchanged for any r, with j counting whole laps of F_c = f_c^r
and λ̇ from section 1:

    D_j = ṗ + ∂_cK(q) + (λ^j K′(q)/K′(w0))·[ D_0 − ṗ − ∂_cK(w0) + j(λ̇/λ)·w0·K′(w0) ]

For entry near phase p_i use p_i and K_i with the common λ. A change of the adaptive lap
count j is a branch change, not an extra term; K_N′ and ∂_cK_N need their own error control.
