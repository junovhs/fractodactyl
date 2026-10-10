# Taylor-truncating the Koenigs coefficients in c: error bound, recursions, v0 numbers

Owner-run deep-research answer, pasted 2026-10-09, condensed here (formulas and numbers
kept, prose shortened). Question: replace each k_n(c) of the inverse Koenigs linearizer
at the 2-cycle by its degree-M Taylor polynomial in h = c − c0, keeping p(c) and
λ(c) = 4(c+1) exact. What is the error, how are the derivatives computed, and how does
it combine with the error in λ(c)^j?

## Check against v0 (added 2026-10-09, `tools/research/misiurewicz/shared_consts_double.py`)

Numerical consistency check, not a proof check.

- The answer's series-tail bounds at N = 18, ρ = 0.03 (5.06e-21 for p₊, 7.03e-18 for p₋)
  sit just above the tails computed earlier (5.0e-21, 5.7e-18).
- Its first-order Taylor error at r = 1e-9 (≤ 1.92e-19, worse branch) matches the
  measured 1.9e-19; at r = 1e-12 the measured value is 1.9e-25 against ≤ 1.92e-25.
- With frozen coefficients the measured error is 3.2e-11 at r = 1e-9 and 3.2e-14 at
  r = 1e-12, so the first-order term is what buys the last eight digits.
- The whole pixel in doubles with one set of zone constants passes at every width from
  1e-6 to 1e-24 (METHOD.md results log, 2026-10-09).

## 1. Setup and bound

    F_c(p + x) − p = λx + a x² + b x³ + x⁴,   a = 4p² − 2p − 2,   b = 4p,   q = −1 − p
    K_c(λw) = λK_c + aK_c² + bK_c³ + K_c⁴
    k_n = ( a[wⁿ]K² + b[wⁿ]K³ + [wⁿ]K⁴ ) / (λⁿ − λ),   n ≥ 2            (1)

The two cycle points share the multiplier but have different coefficients; bound each
branch separately. The error splits into a w-series tail (n > N) and a parameter Taylor
remainder (n ≤ N).

Majorant: pick R > r with 4R < min{|λ0| − 1, |−3 − 4c0|}. For 0 ≤ t ≤ R let

    L_t = |λ0| − 4t,   d_t = t / √(|−3 − 4c0| − 4t)
    A_t = |a0| + |8p0 − 2|·d_t + 4d_t²,   B_t = |b0| + 4d_t
    H_1 = 1,   H_n(t) = ( A_t ΣH_iH_j + B_t ΣH_iH_jH_k + ΣH_iH_jH_kH_l ) / (L_tⁿ − L_t)   (2)

Then |k_n(c)| ≤ H_n(t) for |c − c0| ≤ t, and

    sup |K_c − K̂_{N,M}| ≤ Σ_{n>N} H_n(r)ρⁿ + (r/R)^(M+1)/(1 − r/R) · Σ_{n=2..N} H_n(R)ρⁿ   (3)

Raising M shrinks only the second term; the series tail is fixed by N.

## 2. Derivative recursions

With D_n = λⁿ − λ and Q_n = aS_{2,n} + bS_{3,n} + S_{4,n}, where S_{l,n} = [wⁿ]K^l:

    p' = −1/(2p + 1),   p'' = −2/(2p + 1)³
    a' = (8p − 2)p',   a'' = 8p'² + (8p − 2)p'',   b' = 4p',   b'' = 4p'',   λ' = 4
    D_n' = 4(nλ^(n−1) − 1),   D_n'' = 16n(n − 1)λ^(n−2)
    k_n'  = (Q_n' − D_n'k_n) / D_n                                         (4)
    k_n'' = (Q_n'' − 2D_n'k_n' − D_n''k_n) / D_n                           (5)

k_1 = 1, k_1' = k_1'' = 0; the system is triangular, so k, k', k'' come from one forward
pass. With Taylor coefficients k_{n,m} = k_n^(m)(c0)/m! and
D_{n,m} = 4^m·C(n,m)·λ0^(n−m) − 4·[m = 1]:

    k_{n,m} = ( Q_{n,m} − Σ_{t=1..m} D_{n,t} k_{n,m−t} ) / D_{n,0}          (6)

## 3. Jump of length j

For w_j = λ(c)^j w0 with |w_j| ≤ ρ and an absolute error δ in the c used for λ:

    |w̃_j − w_j| ≤ ρ · (4jδ/|λ|) · (1 + 4δ/|λ|)^(j−1)                       (7)
    E_jump ≤ E_{N,M} + L_K · ρ · (4jδ/|λ|) · (1 + 4δ/|λ|)^(j−1)             (8)

L_K bounds |K_c'|. If λ(c) is formed from the exact c the second term vanishes.

## 4. Numbers for c0 = −0.74329189 + 0.13124055i, N = 18, M = 1, ρ = 0.03

    λ0 = 1.02683244 + 0.52496220i,   |λ0| = 1.153243327
    p₊ = −0.2502964114 − 0.2627926790i,   p₋ = −0.7497035886 + 0.2627926790i
    E_param(+) ≤ 0.1333 r² + 1.059e4 r³/(1 − r/0.016)
    E_param(−) ≤ 0.1911 r² + 3.290e4 r³/(1 − r/0.013)

| r | total, p₊ | total, p₋ | Taylor part, p₊ | Taylor part, p₋ |
|---|---|---|---|---|
| 1e-9 | 1.39e-19 | 7.23e-18 | 1.34e-19 | 1.92e-19 |
| 1e-12 | 5.07e-21 | 7.04e-18 | 1.34e-25 | 1.92e-25 |
| 1e-15 | 5.07e-21 | 7.04e-18 | 1.34e-31 | 1.92e-31 |

At j = 350: |λ0|^350 ≈ 4.70e21, so |w0| ≤ 6.38e-24; |K'| ≤ 1.214 (p₊), 1.319 (p₋) on
|w| ≤ 0.03; the jump error from an error δ in c is about 44.2δ (p₊) and 48.1δ (p₋).

Conclusions as received: the recursion is cheap; for r ≤ 1e-12 the w-series tail
dominates, so raising N helps more than raising M; over 350 laps the multiplier error
can dominate, so form c − c0 and λ^j carefully.
