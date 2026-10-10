# Misiurewicz ladders: minibrots converging on a spiral centre

Owner-run deep-research answer, pasted 2026-10-09, condensed here (formulas, numbers and
references kept; derivations shortened). Question: do minibrot centres c_n accumulate on a
Misiurewicz point m with a closed-form law, how common are such points, and does the
per-pixel cost stay flat down the ladder?

## Check against v0 (added 2026-10-09, `tools/research/misiurewicz/ladder.py`, 27 s)

- **The answer's ladder is real and its constants are right.** With its A and B, the
  period-(25 + 2n) nuclei found by Newton satisfy (c_n − m)ρⁿ/A = 1 to 6e-13 at n = 200
  and 1e-18 at n = 366, and size/(Bρ⁻²ⁿ) = 1 to 2e-12 and 5e-17. The two-term predictor
  lands within 5e-5 and 2e-4 minibrot scales at n = 100 and 200, where the one-term
  formula misses by 57 and 97 scales. At n = 366 the printed 20 digits of A are too few
  (error 4e6 scales): the constants must be computed at working precision.
- **v0 is rung 0 of a second ladder** (period 764 + 2k; the answer's has odd periods).
  Newton from the one-term guess m + (c_0 − m)ρ⁻ᵏ converges in 2-5 steps to nuclei with
  (c_k − m)ρᵏ equal across k = 1, 2, 409, 7676 to 22 digits:

  | k | period | distance to m | size | Newton |
  |---|---|---|---|---|
  | 0 (v0) | 764 | 1.706e-25 | 4.12e-50 | - |
  | 1 | 766 | 1.48e-25 | 3.10e-50 | 0.4 s |
  | 2 | 768 | 1.28e-25 | 2.33e-50 | 0.4 s |
  | 409 | 1,582 | 8.06e-51 | 9.20e-101 | 0.7 s |
  | 7,676 | 16,116 | 8.45e-501 | 1.01e-1000 | 12 s |

  Sizes follow |ρ|⁻²ᵏ to six digits. Coordinates: `tools/research/misiurewicz/ladder_rungs.txt`.
- **Returns per pixel, crude count** (12 random pixels per frame, a return = |z| below
  size^(1/4) at a multiple of the period): frames 5,000 / 50 / 5 minibrot sizes wide give
  2-3 / 4 / 5-40 at v0 and 3-4 / 5-6 / 6-11 at the 1e-100 rung, while direct steps grow
  from about 3,000-4,400 to 7,700-11,700. Consistent with a bounded return count; the
  threshold is scale-dependent, so the one-return difference is not evidence either way.
- **Not done:** building a zone for a new rung and rendering it with the fast path. That
  is the real test of flat cost.

## 1. The ladder

References: Eckmann-Epstein, Comm. Math. Phys. 101 (1985) 283-289, Thm 1; Douady-Hubbard,
Étude dynamique des polynômes complexes; Tan Lei, Comm. Math. Phys. 134 (1990) 587-617,
§6.1; McMullen, "The Mandelbrot set is universal", Thm 4.1.

A Misiurewicz point has many ladders, one per return branch. Fix L ≥ 1, a preimage x(c)
with f_c^L(x) = 0, and an inverse branch of g_c = f_c^r whose iterates of x converge to
the cycle point a = z_q(m). With h_c the normalised Koenigs coordinate at a(c) continued
to x(c), let W(c) = h_c(z_q(c)), V(c) = h_c(x(c)), D = W′(m) = z_q′(m) − a′(m).

    exact matching equation:   W(c_n) = ρ(c_n)⁻ⁿ · V(c_n)                        (1)
    period:                    P_n = q + r·n + L                                 (2)
    centre:                    c_n = m + Aρ⁻ⁿ + O(n|ρ|⁻²ⁿ),   A = V(m)/D         (3)
    size:  β = ½(f_m^q)″(0) = Π_{j=1..q−1} 2z_j(m),  T = (f_m^L)′(x)/h_m′(x)
           B = 1/(β·D·T²);   M_n ≈ c_n + Bρ⁻²ⁿ·M;   orientation arg B − 2n·arg ρ   (4-6)
    two-term:  c_n = m + Aρ⁻ⁿ + (C − n·α·A²)ρ⁻²ⁿ + O(n²|ρ|⁻³ⁿ)                    (8)
               α = ρ′(m)/ρ,   C = (V_1·A − ½W_2·A²)/D,   V_1 = V′(m),   W_2 = W″(m)
               W_2 = z_q″(m) − a″(m) + 2h_2D²,   h_2 = −g_2/(ρ(ρ − 1))

V(m), h_m′(x) and V_1 come from limits along the inverse branch x_j → a:
V = lim ρʲ(x_j − a); h′(x) = lim ρʲ/Π g′(x_l); V_1 = lim ρʲ[x_j′ − a′ + jα(x_j − a)].
The one-term formula misses by about |C − nαA²|/|B| minibrot scales, growing with n.
Refine with (1) or with Newton on f^P(0) = 0.

## 2. How many spiral centres

Parameters landing on a cycle of exact period r are dense in ∂M for every r (McMullen
Prop. 2.1). They are the roots of Φ_r(z_q(c), c) with Φ_1 = z² − z + c, Φ_2 = z² + z + c + 1,
Φ_3 = (f³(z) − z)/(f(z) − z). Counts with preperiod ≤ Q: about 2^Q (r = 1), 2^Q (r = 2),
3·2^Q (r = 3). They equidistribute to harmonic measure on ∂M (Dujardin-Favre). No uniform
nearest-anchor distance follows: harmonic measure is very uneven, and the covering radius
satisfies R_r(Q) ≥ C_t·2^(−Q/t) for every t < 2.

## 3. Which ladders are cheap

Period 2: ρ = 4(c + 1); slow spirals (|ρ| near 1, many laps) just outside |c + 1| = 1/4;
|ρ| = 5 near c = 1/4, 4 near c = −2. Laps to depth d ≈ ln(1/d)/ln|ρ|. One lap turns by
arg ρ and shrinks by 1/|ρ|; per revolution the radius shrinks by exp(−2π ln|ρ|/|arg ρ|).
For M(24,2): 27.08° and 0.8671 per lap, 0.1502 per revolution, 13.3 laps per turn. The
number of arms is not fixed by q, r, arg ρ: it needs the external-ray portrait.
Period 1: ρ± = 1 ± √(1 − 4c).

## 4. Returns per pixel

With b_n = ½(f^{P_n})″(0) ≈ βTρⁿ and c = c_n + Bρ⁻²ⁿ·s, the renormalised return map tends
to ζ² + s, so the number of critical returns tends to the escape count of 0 under ζ² + s:
bounded in n for fixed s, unbounded as s approaches M (about π/√δ near the cusp s = ¼ + δ).
Arithmetic cost O(k(s)); precision still grows, about 2n·log10|ρ| − log10|B| digits.

## 5. Nesting

Inside M_n the tuned copy of a Misiurewicz point κ has a cycle of period P_n·r′ and its
own ladder with multiplier ρ_inner → ρ_κ as n grows. The Koenigs chart of the renormalised
map scales with the dynamical size |ρ|⁻ⁿ, not the parameter size |ρ|⁻²ⁿ.

## 6. Numbers for M(24,2), L = 1, x = −√(−m)

    m = −0.7432918908524302029316243259725107571763… + 0.1312405523087976047708459065814781430771…i
    ρ = 1.02683243659027918827350269610995697… + 0.524962209235190419083383626325913…i
    D ≈ −12.26364270270713 − 51.27646981363481i,   β ≈ 6.15531651373431 − 7.43613557256319i
    A ≈ 0.00146873010307155719 − 0.00176557355983146933i      |A| ≈ 2.2966e-3
    B ≈ 0.00002535504256552955 + 0.00002789044719997487i      |B| ≈ 3.7693e-5
    C ≈ 0.00099628274546597131 − 0.00115707322224533577i,   α = 4/ρ
    P_n = 25 + 2n

| target diameter | n | period | S_n = \|B\|\|ρ\|⁻²ⁿ | one-term error / S_n |
|---|---|---|---|---|
| 4e-50 | 366 | 757 | 1.779e-50 | 174 |
| 1e-100 | 775 | 1,575 | 3.970e-101 | 370 |
| 1e-1000 | 8,042 | 16,109 | 4.362e-1001 | 3,895 |
