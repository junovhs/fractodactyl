# Which stages of the mid-band pixel can run in binary32?

Owner-run deep-research answer, pasted 2026-10-09, condensed here (bounds and numbers
kept, derivations shortened). Question: for the seven-stage pixel (perturbation, cycle
shift, offset, Newton inverse, jump, landing, finish), bound the rounding error of each
stage in IEEE binary32 as a pixel displacement and give a precision assignment for 1e-3
and 1e-2 px.

## Check against v0 (added 2026-10-09, `tools/research/misiurewicz/float32_stages.py`)

250 pixels per width at 1e-9, 1e-15, 1e-24, each stage forced to float32/complex64 in
turn, scored in px against 70-digit iteration. The answer's verdicts hold stage by stage.

| Arm | max px (worst width) | median px | over 1e-3 | over 1e-2 |
|---|---|---|---|---|
| all double | 6.6e-11 | 1e-11 | 0 | 0 |
| stages 1-4 float | 6.9e-4 | 8.5e-5 | 0 | 0 |
| jump: float exp(j·log λ) | 3.8e-1 | 2e-3 to 8e-3 | 181-233 of 250 | 3-93 |
| jump: float table λ0^j | 3.9e-4 | 2.5e-5 | 0 | 0 |
| landing: float Horner, double add | 3.9e-4 | 2.3e-5 | 0 | 0 |
| landing: float Horner + float add | 5.8e-2 | 4.8e-4 | 44-59 | 1-2 |
| finish in float | 9.3e-2 | 1.2e-3 | 135-152 | 0-4 |
| mix: float 1-6 with table, double add + finish | 4.2e-3 | 8.5e-5 | 0-2 | 0 |
| all float (table jump) | 9.3e-2 | 1.2e-3 | 134-153 | 0-4 |

- Predicted and measured agree: the naive float phase fails (predicted up to 0.09-0.12 px),
  the float landing add costs about 0.002 px (measured median 5e-4, 44-59 pixels over
  1e-3), and the float finish is the main loss (predicted about 0.05 px from the weak
  expansion near the cycle; measured median 1.2e-3, max 9.3e-2).
- The precomputed table of λ0^j (the answer's option C) removes the phase problem.
- The finish is about 40 of the 64 remaining steps, so under the 1e-3 px bar most of the
  per-pixel work stays in double.

## 1. Error budget

Adjacent pixels differ by h = 5e-4 relative; landing separation is about h·|w1| = 1.3e-5
to 1.5e-5. Budget: 5e-7 relative in w1 (1.3e-8 absolute at landing) for 1e-3 px; ten
times that for 1e-2 px. One binary32 unit roundoff (5.96e-8 relative) is 1.19e-4 px, so
about 4 of them fit in the 1e-3 budget.

## 2. Stages 1-4

- Stage 1: E_{n+1} ≤ A_n E_n + 2|δ̂_n|E_Z + E_dc + ρ_n with A_n = 2|Z_n| + |δ_n| + |δ̂_n|;
  computable from the actual orbit, not from |Z_n| ≤ 2 alone. Float plausible, not proven.
- Stage 2: |s0| ≥ 0.391, |4dc/s0²| ≤ 2.6e-8, |Δp| ≤ 2.56e-9. Good float candidate.
- Stage 3: cancellation risk κ3 = (|U0| + |δ24| + |Δp|)/|u0|; κ3 = 100 would cost 0.024 px.
- Stage 4: certify with the Newton residual, |ŵ − w*| ≤ (|r| + E_K + E_u0)/(1 − η).
- dc and u0 fit the binary32 exponent range; never form dc as (C + dc) − C.

## 3. Stage 5, up to 375 laps

With log λ0 held in binary32: 2.79e-6 relative in |w1| (0.0056 px) and 4.47e-5 rad in
phase (0.089 px); the float product j·θ adds up to 6.1e-5 rad (0.122 px). Fixes: (A) a
two-float log λ0 with compensated multiply and reduction; (B) fixed-point phase
Q = round(2^40·θ/2π), (j·Q) mod 2^40 in 64-bit integers, error 1.07e-9 rad; (C) a table
T_j = λ0^j for j ≤ 375 built at high precision, about 3 KB per zone, error one unit
roundoff (1.2e-4 px). The correction log1p(4dc/λ0) may be replaced by 4dc/λ0 (error
2.3e-15); dropping it costs up to 0.0026 px.

## 4. Stage 6

Float add of K(w1) ≈ 0.03 to p ≈ 0.75: up to 2.98e-8 absolute, 0.00199 px. Float Horner:
generic bound (γ72 + ε)·S_K, about 0.0087 px worst case, much less with the actual
coefficients. Recommended: Horner in float if its bound fits, then a double add.

## 5. Stage 7

Errors referred back to the landing point: e_N/P_N ≈ e_0 + Σ (δc + η_t)/P_{t+1}, with
P_{t+1} = 2z_t P_t. If each step doubled, all finish roundoff equals about one roundoff
at landing. Near the weak 2-cycle (|λ0| = 1.1532 per two steps) it is about 12.75
roundoffs: 0.05 px for absolute errors of 5.96e-8. Binary64 is the default for 1e-3 px.

## 6. Assignment as received

| Stage | 1e-3 px | 1e-2 px |
|---|---|---|
| 1 perturbation | double by default; float with a certified bound | float plausible |
| 2 cycle shift | float | float |
| 3 form u0 | double or compensated if cancellation | float if κ3 small |
| 4 Newton inverse | double by default; float with residual certificate | float |
| 5 jump | high-precision phase and log-magnitude, or table | high-precision phase mandatory |
| 6 Horner | double, or certified float | float |
| 6 add p(c) | double | float acceptable |
| 7 finish | double | double by default; float only per orbit |

Derivative dz/dc to 1e-4 relative: binary32 is likely adequate on well-conditioned pixels
(about 1e-5 to 2e-5 accumulated), not provably everywhere (cancellation when 2zD ≈ −1).
