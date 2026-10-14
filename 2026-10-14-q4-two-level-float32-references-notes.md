# 2026-10-14: Q4, two-level references for float32 pixels (research notes for filing)

**Author:** Claude, a cloud session with a compute sandbox (4 CPU cores, no GPU).
**Question:** frontier question 4 in `docs/research/10-11-26/frontier-questions.md`.
**Status:** **killed under the stated kill criterion.** The tiling itself works, and a hybrid
with binary64 fallback still projects 3–9x on RDNA1 (from operation counts, not measured).

**Results so far.** The 1e-10 and 1e-30 results are final. The 1e-100 and 1e-300 runs were
still going when this was written; see §10. All 1e-100 numbers measured before 14:00 are
**invalid** because of a simulator bug, fixed in §6.4.

**Process note for the filing agent:**

- Ishoo (`ishoo_*` MCP tools) was **not available** in this session.
- The owner explicitly told me to commit to `main` and to put all notes in this single root
  file so that another agent can file them. This overrides CLAUDE.md's rule against touching
  `main` without Ishoo.
- An earlier WIP commit was also pushed to `claude/trusting-volta-5t6ong`, before the owner's
  instruction. That branch is redundant and can be deleted.

---

## 0. Filing checklist (for the agent)

1. Create a research answer file `docs/research/10-11-26/q4-two-level-references-float32.md`
   from §1–§9. Then set row 4 of the table in `docs/research/10-11-26/frontier-questions.md`
   to: "**killed** (fallback 1.3–6% > 1%; kept error ≤ 3e-4 px); hybrid projects 3–9x, see
   lead 1".
2. Add a METHOD.md "Results log" entry. Suggested text is in §11.
3. File Ishoo issues for the ranked leads in §8 (one per lead, with the kill criterion
   already stated there). Also record a decision on the question in §8.0, which is the
   owner's call.
4. Record the two reusable engineering facts as findings or decisions (§6.4 and §6.3):
   - **Tile and reference orbits used by any binary32 kernel must carry their own exponent.**
     Near a nucleus the orbit leaves binary32 range and dz/dc collapses to 0. Plain float
     storage of W is a bug.
   - **Store reused orbits as float32 hi+lo.** Rounding errors in a re-read orbit add
     coherently, so error grows ∝ N instead of √N. hi+lo halves it at +4 ops/step.
5. Fold in the 1e-100 and 1e-300 numbers when the rerun lands (§10), or rerun them.

---

## 1. Verdict

The kill criterion, as stated in the question: kill if more than 1% of pixels need the FP64
fallback, or if the worst error exceeds 1e-3 px.

| Criterion | Result |
|---|---|
| Worst error of the pixels the guard keeps in float32 | **Pass.** Kept max ≤ 3.1e-4 px with T ≤ 16. 0 kept pixels over 1e-3 px and 0 kept class mismatches in every valid view. |
| FP64 fallback ≤ 1%, nu-only 1e-3 px contract | **Fail.** 1e-10: 0.3–0.5% (eye), 1.1–1.5% (mis), 3.7–5.1% (nuc). 1e-30: 1.6–2.1% (eye), 2.5–3.0% (mis), nuc rerun pending. |
| FP64 fallback, full project contract (nu, plus DE ≤ 0.2% and normal ≤ 0.2° where DE > 1e-3 px) | **Fail badly.** 2.4–23% |

**Interpretation.**

- **The structure works.** Tile references remove the view-size factor exactly as the lead
  predicted: the error is ∝ tile radius in pixels. At T = 8 px the median error is about 150x
  below single-level float32.
- **The fallback comes from a floor that no reference layout changes.** Float32 leaves a
  position error σ ≈ u·(T/2)·N^(0.5…1) px (u = 2^-24, N = executed steps).
  - A first-order error estimate is only valid where σ ≪ the pixel's DE, its distance to the
    set. Deep views are dense in boundary: in minibrot-centred views 1% of pixels lie within
    **about 8e-10 px** of the set (§5).
  - Holding fallback to ≤ 1% there needs σ smaller by 2^15–2^16. That is about 40-bit
    arithmetic (float-float or binary64), so **precision is the lever, not structure.**
- **The full contract is much stricter than the nu bar.** The DE gate (0.2% where
  DE > 1e-3 px) is a position-error requirement of ≤ 2e-3·DE: **2e-6 px at the DE floor, 500x
  stricter than the nu bar.** Every float32 "DE failure" inspected was this conditioning
  (position error about 1e-5 px against DE about 1e-3 px), not inaccuracy in dz/dc. GPU-02's
  guarded f32 finish failed the same way on the RTX 3050 Ti (DE/normal counts of 216–526 per
  depth).
- **Not killed: cost.** Weighting each fallback pixel by its real iteration count, the hybrid
  (float32 two-level plus a compacted binary64 rerun queue) projects **5.9–9.1x (nu) and
  2.9–6.0x (full)** over all-binary64 perturbation on the RX 5700 XT, at T = 8–16, at 1e-10
  and 1e-30. The 1% criterion was a proxy for "fallback must not eat the win". Measured,
  fallback is 5–40% of the cost, which still leaves a several-fold margin. Whether to reopen
  on that basis is the owner's call (§8.0).

---

## 2. Method

### 2.1 Code

All code is in `tools/research/twolevel/`; see the file index in §12.

- **`ref.py`** (gmpy2) builds the exact centres and the high-precision global reference
  orbit, rounded to binary64. The pixel spacing is h = 2^K, with K = floor(log2(W/1920)), so
  that rescaling is exact.
- **`tile32.c`** (C with OpenMP) has two modes:
  - **truth:** binary64 perturbation off the global reference, with Zhuoran rebasing (when
    |z| < |δ|, or when the reference ends). Every value is a double mantissa times 2^k: the
    fd "scaled" tier.
  - **f32:** the candidate scheme, described next.

### 2.2 The candidate scheme

1. **Tile references.** For each tile of T×T px, a binary64 secondary reference sits at the
   tile centre pixel (option: the deepest pixel by truth, "maxit"). Its orbit is computed as a
   rebasing perturbation off the global reference, and the full orbit W_n is stored, as
   binary64 and as a binary32 mantissa+exponent hi+lo copy (§6.3, §6.4).
2. **Pixel recurrence.** Each pixel iterates its offset from the tile orbit:

   e_{n+1} = 2 W_n e_n + e_n² + ε,  where ε = c − c_tile is an exact small integer times h.

   - Every quantity (e, z = W + e, dz/dc, W) is a **binary32 mantissa times an int exponent**:
     real C `float` operations with FMA contraction off, round to nearest, rescaling only by
     exact powers of two, and exponent-aligned adds. This mirrors a GPU float32 kernel using
     `ldexp`, which is the same idea as FractalShark's HDRFloat and Imagina's FloatExp.
   - Derivative: dz/dc' = 2 z dz/dc + 1, also float-exp.
3. **Rebasing on the tile orbit.** When |z| < |e| or the tile orbit ends, set e ← z and
   restart at W_0 = 0. z is formed in binary64 (W64 + e) only at rebases and at escape: one
   binary64 add each.
4. **BLA (optional).** A per-tile BLA is built on W (levels 2^l, aligned blocks; A and B in
   binary32 mantissa+exponent; validity radius ε_bla = 2^-24 times |W|, merged in the
   standard way, with ε_max = 0.75·T px). It is applied to e as e' = A e + B ε and
   dz/dc' = A dz/dc + B.
5. **Guard state.** Carried per pixel:
   - a variance model of the error on e,
     s' = 4|z|²·s + 4u²(|2We|² + |e²|² + |ε|²) per step, and s' = |A|²s + … per BLA step,
     reported as σ_px = √s / (|dz/dc|·h);
   - the minimum over steps of |z_n|²/s_n (the nonlinearity margin);
   - optionally, a rigorous first-order linear bound (abandoned, §6.1).

### 2.3 Scoring

- **Frames.** Real 1920×1080 pixel spacing. 1/8 of the 64×64 blocks across the whole view are
  selected by hash: **276,992 pixels per view.** The same pixels are used for every T. NMAX is
  1e5 and the bailout radius 1e4.
- **Metric** (the project's DEC-10 nu metric): displacement = |Δν|·ln2·DE_truth in px, with
  smooth ν = n + 1 − log2 ln|z|.
  - Class mismatches (escaped against not escaped) are counted separately.
  - DE is scored relative to truth where DE_truth > 1e-3 px; the tolerance is 0.2%.
- **Adjudication.** `check.py` iterates directly at 3.33·digits + 400 bits. It agreed with
  binary64 truth on **every disputed pixel checked (5/5)**:
  - mis-10 (302,599): truth n = 611, float32 n = 1049;
  - nuc-10 (906,500): escapes at 95,609; (954,504): escapes at 46,138;
  - nuc-10 (966,561) and (962,564): no escape by 1e5, where float32 escaped at 41,400 and
    87,167.

### 2.4 Locations

Generated by `ref.py`; metadata is in `tools/research/twolevel/results/refs/*.ref.txt`.

| name | description | 1e-10 | 1e-30 | 1e-100 | 1e-300 |
|---|---|---|---|---|---|
| `eye` | eye-of-universe centre, 1,121 digits (census) | N̄ 446 | N̄ 5,334 | N̄ 28,659 | N̄ 57,198 |
| `mis` | Misiurewicz M(24,2) (seahorse valley, Newton at full precision); view centre at M + (0.2317 − 0.1381i)·W | N̄ 317, ref escapes at 293 | N̄ 983 | N̄ 3,263 | pending |
| `nuc` | nucleus near M(24,2): lowest-period nucleus in a ball of radius r around M + (0.3+0.17i)r; r bisected so that size/W is about 1/30; Newton steps clamped to r/4 | p 145, size 0.014 W, N̄ 593, 0.54% interior | p 465, size 0.021 W, N̄ 2,492 | p 1,589, size 0.054 W, N̄ 13,626, 5.3% interior | p 4,827, size 0.017 W |

N̄ = mean escape iteration of escaping pixels.

---

## 3. Measured error distribution

`summarize.py` generates these tables.

**Guard** (λ = 4). A pixel falls back to binary64 if either:
- it escapes and 4σ > min(1e-3 px, DE_f32), or
- it does not escape and 4σ_n > |z_n| at some step.

**Full contract** adds: escaping, DE_f32 > 1e-3 px and 4σ > 2e-3·DE_f32.

**Columns:**
- "max px (no guard)" is over all pixels;
- "kept" means the pixels that stay in float32;
- "cost share" is the fallback's share of projected float32-hybrid issue slots;
- projection details are in §7.

**Run variants:** the 1e-10 rows use the plain float32 tile orbit. The 1e-30 rows use the
fixed float-exp, hi+lo build.

| view | T | p50 px | p99.99 px | max px (no guard) | nu fallback | nu cost share | kept max px | kept bad (px/cls) | full fallback | proj f32 ms | proj f64 ms | speed-up nu / full |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| eye-10 | 4 | 6.8e-07 | 7.8e-06 | 6.0e-05 | 0.29% | 3% | 1.0e-05 | 0/0 | 2.42% | 21.3 | 94.0 | 4.4x / 4.0x |
| eye-10 | 8 | 1.3e-06 | 1.4e-05 | 8.9e-05 | 0.39% | 5% | 1.9e-05 | 0/0 | 3.74% | 12.4 | 94.0 | 7.6x / 5.8x |
| eye-10 | 16 | 2.6e-06 | 3.2e-05 | 4.3e-04 | 0.51% | 7% | 3.7e-05 | 0/0 | 5.45% | 10.3 | 94.0 | 9.1x / 6.0x |
| eye-10 | 64 | 1.1e-05 | 1.9e-04 | 1.2e-03 | 0.89% | 11% | 1.5e-04 | 0/0 | 10.58% | 10.0 | 94.0 | 9.4x / 4.6x |
| eye-10 | 2048 (single level) | 1.6e-04 | 4.5e-03 | 3.9e-02 | 96.67% | 91% | 2.6e-04 | 0/0 | 97.83% | 98.9 | 94.0 | 1.0x / 0.9x |
| mis-10 | 4 | 5.9e-07 | 8.5e-05 | 6.7e-04 | 1.11% | 13% | 8.2e-06 | 0/0 | 2.58% | 16.4 | 66.8 | 4.1x / 3.7x |
| mis-10 | 8 | 1.2e-06 | 1.9e-04 | 2.0e-03 | 1.32% | 20% | 1.6e-05 | 0/0 | 3.44% | 10.2 | 66.8 | 6.5x / 5.4x |
| mis-10 | 16 | 2.4e-06 | 3.5e-04 | 1.2e-03 | 1.52% | 24% | 2.9e-05 | 0/0 | 4.39% | 8.9 | 66.8 | 7.5x / 5.8x |
| mis-10 | 64 | 9.6e-06 | 1.9e-03 | 1.2e-02 | 2.04% | 29% | 9.8e-05 | 0/0 | 6.98% | 8.9 | 66.8 | 7.5x / 5.0x |
| mis-10 | 2048 (single level) | 2.1e-04 | 6.5e-02 | 2.0e-01 | 96.98% | 91% | 2.2e-04 | 0/0 | 96.98% | 71.2 | 66.8 | 0.9x / 0.9x |
| nuc-10 | 4 | 7.7e-07 | 2.2e-04 | 7.7e-04 | 3.72% | 19% | 1.3e-05 | 0/0 | 9.00% | 61.4 | 236.7 | 3.9x / 3.4x |
| nuc-10 | 8 | 1.5e-06 | 5.0e-04 | 2.0e-03 | 4.39% | 28% | 2.3e-05 | 0/0 | 11.94% | 40.0 | 236.7 | 5.9x / 4.6x |
| nuc-10 | 16 | 3.1e-06 | 1.0e-03 | 2.8e-03 | 5.10% | 33% | 4.1e-05 | 0/0 | 15.22% | 35.4 | 236.7 | 6.7x / 4.7x |
| nuc-10 | 64 | 1.2e-05 | 4.1e-03 | 1.4e-02 | 6.83% | 39% | 1.6e-04 | 0/0 | 23.37% | 36.5 | 236.7 | 6.5x / 4.0x |
| nuc-10 | 2048 (single level) | 2.1e-04 | 1.0e-01 | 8.1e-01 | 95.75% | 84% | 3.5e-04 | 0/0 | 97.42% | 139.7 | 236.7 | 1.7x / 1.7x |
| eye-30 | 8 | 5.2e-06 | 9.9e-05 | 1.7e-03 | 1.60% | 13% | 5.8e-05 | 0/0 | 17.03% | 160.6 | 1122.5 | 7.0x / 3.3x |
| eye-30 | 16 | 1.2e-05 | 2.5e-04 | 8.4e-04 | 2.11% | 18% | 1.1e-04 | 0/0 | 23.51% | 139.6 | 1122.5 | 8.0x / 2.9x |
| eye-30 | 64 | 5.1e-05 | 1.9e-03 | 1.2e-02 | 37.16% | 80% | 3.1e-04 | 0/0 | 59.55% | 523.8 | 1122.5 | 2.1x / 1.4x |
| mis-30 | 8 | 4.4e-06 | 6.3e-04 | 1.6e-03 | 2.51% | 23% | 2.6e-05 | 0/0 | 7.29% | 33.0 | 207.0 | 6.3x / 4.7x |
| mis-30 | 16 | 8.6e-06 | 1.9e-03 | 7.6e-03 | 2.95% | 28% | 5.1e-05 | 0/0 | 9.24% | 29.1 | 207.0 | 7.1x / 4.7x |
| mis-30 | 64 | 3.7e-05 | 7.0e-03 | 2.6e-02 | 4.01% | 35% | 2.3e-04 | 0/0 | 14.27% | 30.1 | 207.0 | 6.9x / 3.9x |
| nuc-30, all of 1e-100 and 1e-300 | | pending rerun (§10) | | | | | | | | | | |

**Earlier 1e-30 nuc numbers** (plain float32 W build, so a pre-fix variant; no underflow was
seen at that depth, so they are indicative only):

| T | p50 px | max px (no guard) | nu fallback | full fallback | projected speed-up, nu / full |
|---|---|---|---|---|---|
| 8 | 2.1e-5 | 1.5e-2 | 5.46% | 16.2% | 5.4x / 3.6x |
| 16 | 4.2e-5 | 3.5e-2 | 6.36% | 20.2% | 5.9x / 3.4x |

hi+lo on the same view gives p50 8.8e-6 and p99.99 2.3e-3 (§6.3).

---

## 4. How error grows (derivation checked against the measurements)

Rounding at step n injects about u·|terms| into e. Propagated to escape and expressed in px,
it contributes about u·|ε|/h·κ_n, where κ_n ≈ 1 except at cancellation. Summing:

  **σ_px ≈ u · (|ε|/h) · f(N) · κ̄**, with |ε|/h ≤ T/√2.

- **Tile size.** The error is linear in T. Measured p50 at eye-10 for T = 4/8/16/64/2048:
  6.8e-7 / 1.3e-6 / 2.6e-6 / 1.1e-5 / 1.6e-4. That is exactly ∝ T up to 64. Single-level
  float32 (T = frame) is about 100x worse and fails outright (max 0.04–0.8 px), as the
  project already found.
- **Iterations.** f(N) is between √N and N. eye p50 at T = 8 goes 1.3e-6 (N̄ 446) →
  1.5e-5 (N̄ 5,334), which is ∝ N^0.95 with the plain W.
  - **Cause:** coherent rounding. A pixel rebases onto W_0 about once per period (measured
    0.9–5.5 rebases/px) and re-reads the same rounded W_0..W_p on every lap, so W-rounding
    errors add in phase.
  - **hi+lo W removes about half:** eye-30 p50 becomes 5.2e-6, mis-30 1.0e-5 → 4.4e-6,
    nuc-30 2.1e-5 → 8.8e-6.
- **Skip tables (BLA).** The error tracks *executed* steps (each BLA step is about one
  rounding plus 2^-24 truncation), not iterations. Measured at 1e-10, T = 8:

  | view | steps/px | p50 px | max px |
  |---|---|---|---|
  | eye | 447 → 267 | 1.3e-6 → 1.1e-6 | 8.9e-5 → 2.1e-4 |
  | nuc | 1,125 → 392 | 1.5e-6 → 1.2e-6 | 2.0e-3 → 1.4e-3 |

  Speculation, not measured: tile-local BLA should also find *longer* valid blocks, because
  the validity test is on |e| ≲ T px rather than |δ| ≲ 1,000 px. I did not isolate this
  effect.
- **Rebasing** onto the tile orbit is free (W_0 = 0) and stable. End-of-tile-orbit rebases
  were 0.000–0.015 per pixel, so choosing the "deepest pixel" reference (KF style) was not
  needed.
- **Optimum tile size: T = 8–16.**
  - At T = 4 the binary64 tile references cost about 456/16 = 29 issue slots per
    pixel-iteration, comparable to the 43-slot float32 pixel. Projections drop to about 4x.
  - From T = 64 error and fallback grow, sharply at 1e-30 (eye-30 fallback 37%).
  - Between 8 and 16 the projected speed-ups agree within about 15%: T = 16 is faster when the
    boundary is sparse, T = 8 when it is dense.

---

## 5. The fallback floor (why tiling cannot reach ≤ 1%)

`floor.py` computes this. A first-order guard needs λσ ≤ DE, so

  fallback ≥ P(DE_truth < λσ).

The σ needed for ≤ 1% fallback is q_1%(DE)/λ. Interior pixels count as DE = 0, so they are
always guarded.

| view | escaped share with DE < 1e-4 px | DE 1% quantile (px) | median σ at T = 8 (px) | σ needed / σ achieved | extra mantissa bits needed |
|---|---|---|---|---|---|
| eye-10 | 0.66% | 2.9e-04 | 7.6e-06 | 9.7 | 0 |
| mis-10 | 1.75% | 6.5e-06 | 6.3e-06 | 0.26 | 2 |
| nuc-10 | 5.60% | 8.1e-10 | 8.6e-06 | 2.3e-05 | 15 |
| eye-30 | 1.59% | 3.3e-05 | 2.7e-05 | 0.31 | 2 |
| mis-30 | 3.04% | 6.1e-07 | 1.2e-05 | 1.3e-02 | 6 |
| nuc-30 (pre-fix) | 5.99% | 8.1e-10 | 1.8e-05 | 1.1e-05 | 16 |

eye-10 needing 0 extra bits but still showing 0.39% fallback is consistent: that view is
under 1%. The fraction P(DE < x) falls only slowly as x shrinks (a fractal boundary), so
halving σ by halving T moves fallback by only 10–25% (§3).

---

## 6. The guard: what was tried, what died, and the bugs found

### 6.1 Guards tried

| guard | outcome |
|---|---|
| Rigorous first-order linear bound, ρ' = 2\|z\|ρ + 6u·(L1 magnitudes) | **Dead.** About 500x pessimistic (geometric-mean bound/actual 440–660), so 63–98% fallback at T = 16–64. It is also **not a bound** in the nonlinear regime: bound/actual = 0.38 at mis-10 (302,599), DE 6.7e-6 px, where float32 escaped at 1049 against a true 611. An L1 norm inside the propagation would compound √2 per step, so the propagation must use the Euclidean \|z\|. |
| Variance model plus λσ ≤ 0.1·DE | Works: no kept bad pixels. 1–7% fallback at 1e-10. Too conservative. |
| Variance model plus a per-step rule λσ_n ≤ 0.1\|z_n\| for every pixel | Worse: 2.9 / 3.7 / 12.5% at 1e-10. At critical returns \|z_n\| is legitimately tiny next to σ_n. |
| **Kept design:** escaping pixels need λσ ≤ min(tol, DE); non-escaping pixels need λσ_n ≤ \|z_n\| | The only test that catches float32 "interior" pixels that truly escape near NMAX (nuc-10, 2 such pixels). Results in §3. |
| Threshold sweep (`analyze.py`): F ∈ {0.1, 0.3, 1, 3} on the DE and \|z\| rules | All keep 0 bad pixels at 1e-10; DE-rule F = 1 is the cheapest. nuc-10 still 4.4%. |
| Rounding-diversity cross-check: a second float32 run off a reference offset by T/2 + 1 px, accept if they agree | **Dead.** Compared in px with the float32 DE, it is unsafe: wrong runs have DE ≈ 1e-17 px, so huge Δν looks tiny. Requiring Δν ≤ 0.01–0.1 makes it safe, but then it falls back on 5.1–13.7% of nuc-10, worse than the kept guard. Near-boundary pixels legitimately show large Δν at tiny px displacement. |
| D-accuracy guard (variance of the relative error in dz/dc) | Unnecessary. DE failures are position conditioning (§1), not dz/dc rounding. |

### 6.2 Calibration of the variance model

The calibration figure is max(actual/σ) over pixels with DE_truth > 1e-2 px.

| depth | plain W | hi+lo W |
|---|---|---|
| 1e-10 | 1.5–1.7 | — |
| 1e-30 | 2.0–3.1 (degrades with N; correlated rounding) | 1.5–1.9 |
| 1e-100 (eye, pre-fix, plain W) | 12.8–28 | **not valid evidence** (the float-exp bug may contribute); rerun pending |

With λ = 4 the margin is fine to 1e-30 with hi+lo. It is a **calibrated model, not a
certificate**: rounding is correlated, which is the documented failure mode of
Higham–Mary-style probabilistic analysis.

### 6.3 hi+lo tile orbit (a measured, reusable win)

Store W as a binary32 hi plus a binary32 lo at the same exponent, and add 2·W_lo·e inside the
step (+4 ops/step).

| view | p50 px | p99.99 px |
|---|---|---|
| mis-30 | 1.0e-5 → 4.4e-6 | 1.5e-3 → 6.3e-4 |
| nuc-30 | 2.1e-5 → 8.8e-6 | 5.3e-3 → 2.3e-3 |

Calibration improves from 3.1 to 1.9. **Fallback is unchanged** (2.51 → 2.52%,
5.46 → 5.46%): the guard's σ is set by the rounding of the products, which hi+lo does not
touch. Any float32 kernel that re-reads a stored orbit (including the zone finish) should
adopt it.

### 6.4 Bug: the tile orbit must carry an exponent (important for any GPU float32 kernel)

**First version.** The orbit W was stored as plain float32, and z = W + e was formed in plain
float32.

**Failure.** In nucleus-centred deep views the orbit passes far below 1e-38 near the critical
point. W and z underflowed to 0. The derivative's "+1" (2^-kd in mantissa units) also
underflowed, so dz/dc = 0.

**Symptoms at nuc-100:**
- every escaping pixel non-finite, or (in a slightly different build) p50 1.2 px with 510
  class mismatches;
- eye-100's calibration ratio of 13–28 may also be partly this.

**Fix.** W is stored as binary32 mantissa + int exponent (struct `Wfe`, plus the lo part).
z, e, D and the variance are all float-exp with exponent-aligned adds. The fixed build
reproduces 1e-10 exactly (eye-10 fallback 0.391%, nuc-10 4.393%, same medians).

**Consequence.** All 1e-100 results from `results/logs/matrix-1e-30-1e-100-pre-fix-INVALID-deep.log`
are **invalid**. The 1e-30 eye/mis rows in §3 come from the fixed build.

**Caveat for the truth arm.** The binary64 truth formed z = Z + δ in plain double. That is
fine to about 1e-308, but a 1e-300 view near a nucleus may need the same treatment; spot-check
it with `check.py` (§10).

---

## 7. Projected RDNA1 throughput (operation counts; NOT a GPU measurement)

The model is in `project.py`. Hardware: RX 5700 XT, 2,560 lanes, 1.755 GHz game clock,
binary64 at 1/16 rate, 448 GB/s.

| item | issue slots per lane per step |
|---|---|
| Binary64 perturbation step with exponent, one level (the opponent) | 28 FP64 ops × 16 + 8 int = **456** |
| Float32 two-level step | **43**. Breakdown: z = W + e (4); dz/dc (6); 2We + e² + ε (13); escape and rebase tests (7); renormalisation (3); variance guard (7); loop (3). |
| Tile reference | One binary64 step per tile per raw iteration in a separate lockstep kernel (one lane per tile, chunks of about 1,024 steps): 456/T² slots per pixel-iteration. Plus 24 B per tile per iteration written, then read once per workgroup into LDS. |
| Fallback | A compacted queue; each fallback pixel reruns in binary64 from n = 0, weighted by its true iteration count. Fallback pixels are near-boundary and iterate longer than average, which is why "cost share" ≫ "fallback %". |

- Per-step ratio: 456/43 ≈ **10.6x**. The whole-frame projections in §3 include the tile
  references, their memory traffic and the weighted fallback.
- **Opponent:** binary64 perturbation on the same GPU, same iteration count, no BLA (DEC-14 §2
  fair opponent). With BLA both sides shrink; §4 shows that the error tracks executed steps,
  so the ratio should hold. That last point is speculation.
- **Ignored:** occupancy, register pressure, divergence before compaction, and queue
  overhead. GPU-02's guarded f32 finish "tied f64" largely because of wave divergence, so
  treat these numbers as an **upper bound**. A real kernel is needed (lead 1).
- **Tile-reference cost caveat.** If any pixel in a tile is interior, the tile orbit must run
  to NMAX in binary64. That gives nuc-10 17.6 tile-reference iterations per pixel at T = 8,
  which is about 16% of the hybrid's cost.

---

## 8. Ranked leads

### 8.0 Owner decision needed

The 1% criterion is failed, but the cost projection survives. Options:
- (a) accept "killed" as the stated criterion demands, and stop; or
- (b) reopen under a cost criterion: kill if the measured whole-frame hybrid is < 2x the
  binary64 perturbation kernel.

### 8.1 Hybrid float32 two-level plus binary64 near-boundary queue

- **Mechanism:** §2.2 with the kept guard (§6.1), hi+lo float-exp W, T = 8–16.
- **Evidence:** 0 kept errors over 1e-3 px and 0 kept class errors in all valid views; kept
  max ≤ 3.1e-4 px.
- **Projection:** 5.9–9.1x (nu) or 2.9–6.0x (full) against binary64 perturbation on RDNA1, at
  1e-10 and 1e-30.
- **Cheapest next experiment:** port `run32` to the GPU-13 hand-written WGSL route as two
  kernels, then time on the RX 5700 XT at 480×270, 1e-30, eye and nuc, against the same WGSL
  binary64 perturbation kernel:
  - kernel A: tile orbits in binary64, one lane per tile, chunked;
  - kernel B: float32 pixels plus the guard, appending to a compaction queue.
- **Kill if:** whole frame including the queue is < 2x faster.

### 8.2 float-float (2×binary32), only for the queue

- **Mechanism:** the floor table (§5) says queued pixels need about 40 mantissa bits, not 53.
- **Projection:** on RDNA1 float-float costs about the same as binary64 (1/16 rate), so the
  win would only appear on the RTX 3050 Ti (1/64 rate).
- **Next experiment:** the 10-10 report's float-float microbenchmark, restricted to queue
  pixels.
- **Kill if:** float-float is not ≥ 2x faster per step than binary64 on the 3050 Ti.

### 8.3 Checkpoint-and-continue instead of a full rerun (speculation)

- **Mechanism:** keep periodic float32 checkpoints together with σ. At the end, resume
  binary64 from the latest checkpoint whose λσ_k ≤ 0.5·the final allowance.
- **Projection:** saves the prefix of each fallback pixel, and fallback is 5–40% of the
  projected cost.
- **Next experiment:** log the σ trajectory per fallback pixel in `tile32.c`, then recompute
  the projection.
- **Kill if:** the median resumable prefix is < 30% of the steps.

### 8.4 hi+lo stored orbits everywhere (adopt now)

Measured: it halves the coherent error and the tail at +4 ops/step (§6.3). It applies to any
float32 kernel that re-reads an orbit. No fallback benefit.

### 8.5 DE contract question for the owner

The full contract's DE gate is effectively a position-error bar of 2e-3·DE, 500x stricter
than the nu bar near DE = 1e-3 px. If shading only needs DE to about 0.2% for DE ≳ 1 px, then
fallback under the full contract drops to roughly the nu numbers. That is a contract decision,
not research.

### Dead ends (do not retry)

- single-level float32 with exponent (max 0.04–0.8 px at 1e-10);
- the rigorous linear guard (63–98% fallback, and not a bound near the boundary);
- the rounding-diversity cross-check (unsafe, or 5–14% fallback);
- the per-step |z| rule applied to escaping pixels (worse than the DE rule);
- choosing the "deepest pixel" as tile reference (not needed: rebasing handles short tile
  orbits).

---

## 9. Prior art raided

- **Kalles Fraktaler glitch correction.**
  - Pauldelbrot's test, |z| < 1e-3·|Z|, finds glitched blobs, which are then re-rendered from
    a new full-precision reference.
  - That is a two-level reference scheme, but it exists for *correctness*, not for arithmetic
    width. Zhuoran's 2021 rebasing made it unnecessary, and rebasing is reused here at the
    tile level.
  - KF's reference placement inspired the "maxit" option, which turned out not to be needed.
- **FractalShark HDRFloat and Imagina FloatExp.** A mantissa plus an int exponent. This sim
  uses that representation, with exponent-aligned adds, and the §6.4 bug shows why the
  *reference* needs it too. It provides range, not precision. FractalShark docs were not
  reachable from this sandbox, so these details are from memory.
- **Mixed-precision iterative refinement** (Carson–Higham). The tile orbit plays the role of
  the high-precision residual, but only one level deep.
- **Multigrid and hierarchies.** A sub-tile reference level (64 → 8 → px) does not help: the
  error is already ∝ T, and the floor comes from u.
- **Probabilistic rounding error analysis** (Higham–Mary 2019; Hallman–Ipsen 2021;
  Bhola–Duraisamy 2024). This is the basis of the λσ guard. The coherent ∝ N growth measured
  here is that analysis's independence assumption failing.

Sources:
- Higham & Mary, probabilistic analysis: https://eprints.maths.manchester.ac.uk/2673/1/paper.pdf
- Sharper probabilistic bounds: https://www-pequan.lip6.fr/~tmary/doc/SharperProbAna.pdf
- Refined probabilistic bound for sums: https://arxiv.org/pdf/2104.06531
- Bias- and variance-aware analysis: https://arxiv.org/pdf/2404.12556
- Kalles Fraktaler (Mu-ency): https://mrob.com/pub/muency/kallesfraktaler.html
- KF glitch-detection thread: https://www.fractalforums.com/kalles-fraktaler/kalles-fraktaler-2-7/15

---

## 10. Pending at time of writing

- **Matrix rerun with the fixed float-exp, hi+lo build:**
  - nuc-30 (T = 8/16/64);
  - eye/mis/nuc at 1e-100 (T = 8/16/64);
  - eye/mis/nuc at 1e-300 (T = 8/16).

  Log: `tools/research/twolevel/results/logs/matrix-fixed-floatexp-hilo.log`. Regenerate the
  tables with `python3 summarize.py $S` and `python3 floor.py $S`. Expected from trend, not
  measured: fallback grows with N (eye N̄ 29k at 1e-100, 57k at 1e-300), so the verdict cannot
  improve.
- **1e-300 truth spot-check:** run `check.py LOC -300 x,y` on 3–5 worst pixels per view, to
  confirm that the binary64 truth with plain-double z is right near the nucleus (§6.4 caveat).
- Truth files already built: eye-300 (276,992 px, N̄ 57,198, 521 s); mis-300 and nuc-300 not
  yet.

---

## 11. Suggested METHOD.md results-log entry

> ### Q4: two-level float32 references (2026-10-14, Claude; killed under the stated criterion)
>
> **Setup.** CPU simulation of GPU float32-with-exponent pixels off per-tile binary64
> references (`tools/research/twolevel/`). eye, mis (M(24,2)) and nuc (size ≈ W/30 nucleus)
> at 1e-10 and 1e-30, with 1e-100 and 1e-300 pending. 1920×1080 spacing, 276,992 px per view,
> NMAX 1e5. Truth is binary64 scaled perturbation, verified 5/5 against direct iteration on
> disputed pixels.
>
> **Error.** ∝ tile size (T = 4…64) and ∝ N^0.95 because of coherent W rounding. hi+lo W
> halves it. With a λ = 4 variance guard (escaping pixels: 4σ ≤ min(1e-3 px, DE);
> non-escaping: 4σ_n ≤ |z_n|), kept pixels have max 3.1e-4 px and 0 class errors.
>
> **Kill.** Fallback is 1.3–6% (nu contract) and 2.4–23% (full contract). The floor is
> P(DE < 4σ): minibrot views need 15–16 more mantissa bits for ≤ 1%.
>
> **Projection** (not measured). The hybrid with a binary64 queue gives 5.9–9.1x (nu) or
> 2.9–6x (full) against binary64 perturbation on RDNA1. Owner decision pending on reopening
> it under a cost criterion (next: a WGSL two-kernel A/B, kill if < 2x).
>
> **Engineering findings.**
> - Tile and reference orbits used by float32 kernels must carry an exponent: they underflow
>   near a nucleus, and dz/dc collapses to 0.
> - Reused orbits should be stored hi+lo.
>
> **Dead ends.** Rigorous linear guard (500x pessimistic, and not a bound near the boundary);
> rounding-diversity cross-check.
>
> Full notes: `2026-10-14-q4-two-level-float32-references-notes.md`.

---

## 12. File index (everything this session produced)

| path | what |
|---|---|
| `2026-10-14-q4-two-level-float32-references-notes.md` | this file: the single source for filing |
| `tools/research/twolevel/README.md` | how to build and run the tools |
| `tools/research/twolevel/ref.py` | exact centres (eye, mis via Misiurewicz Newton, nuc via box-period plus clamped Newton plus size bisection) and global reference orbits; writes `.ref` (int64 N, int32 K, N complex doubles) and `.ref.txt` metadata |
| `tools/research/twolevel/tile32.c` | the simulator, in two modes: `truth REF NMAX S OUT` and `f32 REF NMAX S TILE TRUTH [centre\|maxit] [tol] [bla]`. Env vars: `REC=` per-pixel records (10 doubles, or 13 with `ALT=1`); `WLO=1` hi+lo; `ALT=1` cross-check run; `DUMP=1` and `DUMPDE=1` bad-pixel printouts; `DBG=1` non-finite diagnostics |
| `tools/research/twolevel/check.py` | direct high-precision adjudication of pixels |
| `tools/research/twolevel/analyze.py` | record loader, guard-threshold sweep, cross-check scorer |
| `tools/research/twolevel/project.py` | RDNA1 op-count projection and the kept-guard rules |
| `tools/research/twolevel/summarize.py` | §3 table from records and logs |
| `tools/research/twolevel/floor.py` | §5 fallback-floor table |
| `tools/research/twolevel/run_matrix.sh` | width × location × tile matrix (env `WIDTHS`, `TILES`) |
| `tools/research/twolevel/results/summary-current.md` | §3 table as generated |
| `tools/research/twolevel/results/floor-current.md` | §5 table as generated |
| `tools/research/twolevel/results/refs/*.ref.txt` | metadata for all 12 views: K, Nref, bits, nucleus period and size |
| `tools/research/twolevel/results/logs/matrix-1e-10-pre-fix.log` | 1e-10 runs (T = 4/8/16/64/2048) plus the start of 1e-30; plain W; valid at 1e-10 (reproduced exactly by the fixed build) |
| `tools/research/twolevel/results/logs/matrix-1e-30-1e-100-pre-fix-INVALID-deep.log` | pre-fix 1e-30/1e-100 runs; the 1e-100 rows are **invalid** (§6.4) |
| `tools/research/twolevel/results/logs/matrix-fixed-floatexp-hilo.log` | fixed-build rerun (in progress when committed) |

The `.ref` orbit files and per-pixel record files (about 22 MB each) were not committed; they
are regenerated by `ref.py` and `run_matrix.sh`. Compiled binaries are git-ignored.
