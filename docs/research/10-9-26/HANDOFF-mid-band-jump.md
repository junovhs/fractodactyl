# Handoff: the mid-band fast path (PROB-14), 2026-10-09 evening

Written by a cloud session without Ishoo. Nothing here is filed as an issue yet: the
list under "File these" is for the next agent with Ishoo. Every number is in the
METHOD.md results log (rows dated 2026-10-09); the four owner-run research answers are in
this folder, each with a "check against v0" section on top.

## The result in four lines

- The shared-tail failure in the mid bands (PROB-12 seed tests) was caused by measuring
  each pixel's offset from the cycle point of C. Measuring it from the pixel's own cycle
  point p(c), a closed-form shift, fixes it.
- A mid-band pixel is then: 24 perturbation steps, one Koenigs jump, about 40 plain steps.
  That is about 64 steps at every width from 1e-6 to 1e-24, against 172-747 direct.
- It works in IEEE doubles from one set of zone constants, for class, ν, de and normal.
- **Sampled only:** 250 random pixels per width in Python. No whole frames, no Rust, no
  timing. Two configurations in this project passed sparse tests and failed whole frames
  (DEC-17), so treat this as a strong lead, not a kept result.

## The per-pixel recipe (what the probes implement)

Zone constants, built once at high precision and stored as doubles: C's orbit Z_0..Z_24;
U0 = Z_24 − p(C); s0 = √(−3 − 4C); λ0 = 4(C + 1); p(C); K coefficients k_n(C) and
dk_n/dc for n ≤ 18; a table T_j = λ0^j for j up to about 400.

Per pixel, from dc = c − C (never form c and subtract):

1. δ_{n+1} = 2Z_nδ_n + δ_n² + dc and D_{n+1} = 2(Z_n + δ_n)D_n + 1, for 24 steps.
2. Δs = −4dc/(2s0 + Δs) (three passes); Δp = ±Δs/2; p(c) = p(C) + Δp; p′ = −1/(2p(c) + 1).
3. u0 = U0 + δ_24 − Δp.
4. k_n = k_n(C) + dc·k_n′(C). Solve K(w0) = u0 by 3 Newton steps from w0 = u0.
5. j = ⌊ln(0.03/|w0|)/ln|λ|⌋; q = w0·T_j·(1 + j·4dc/λ0).
6. z = p(c) + K(q).
   dz/dc = p′ + (T_j·K′(q)/K′(w0))·[D_24 − p′ − ∂_cK(w0) + (4j/λ)·w0·K′(w0)] + ∂_cK(q),
   with ∂_cK(w) = Σ k_n′ wⁿ.
7. Plain z ← z² + c with the derivative, from step 24 + 2j, to escape.

Which pieces are needed where (measured):

| Piece | Needed |
|---|---|
| Offset from the pixel's own p(c) | always; without it 243-249 of 250 pixels fail |
| Exact λ(c) and first-order k_n | only above about 1e-9 |
| p′ term in dz/dc | always; dropping it is a constant 2.7% error in de and normal |
| Full derivative formula | above about 1e-9; below, dz/dc ≈ p′ + T_j·K′(q)·(D_24 − p′) is enough |
| Finish at c (not C) | above about 1e-9; a shared fixed-C finish passes from 1e-9 down |

## Scripts (tools/research/misiurewicz/)

| Script | Probe | Run time |
|---|---|---|
| `own_c_koenigs.py 250 18 0.03` | A/B: jump at the pixel's own c, 70 digits | 2 min |
| `shared_consts_double.py 250 18 0.03` | C: doubles, one set of zone constants | 19 s |
| `jump_derivative_double.py 250` | D: dz/dc through the jump | 19 s |
| `general_cycle_jump.py 30000 197,655` | E: long cycles on "Eye of the Universe" | 8 s |
| `float32_stages.py 250` | F: each stage forced to float32 | 11 s |
| `f32_frame.py 15` then `f32_compare.py nu_15.npy out.png` | float vs double picture | 5 min |

They import each other; run them from that folder.

## Do next, in order

1. **Whole frames in Rust.** Add a mid-band mode to `koenigs_bench` implementing the
   recipe; score every pixel of 1920x1080 frames at 1e-9, 1e-12, 1e-15, 1e-18, 1e-24
   against `fd control --bla per-frame` (ν, de, normal), and time it. This is the gate.
   Use the recursion for k_n′ in `parameter-taylor-truncation-of-the-koenigs-coefficients.md`
   (the probes used a central difference).
2. **The finish.** About 40 of the 64 steps. Try the existing tail patch atlas on it from
   1e-9 down, where a fixed-C finish passes.
3. **Handover near 1e-27,** where minibrot returns begin and the deep path (loops, then
   the same jump) takes over. Not probed.
4. **Shallow end (1e-6 to 1e-9).** Passes with the full recipe; the finish needs the
   pixel's own c there, so no shared patch atlas.
5. **GPU float kernel.** Only if the owner accepts a looser bar for films (below).

## Open questions for the owner

- **A film-quality bar for GPU renders?** An all-float32 pixel misses 1e-3 px (median
  1.2e-3, worst 0.09) but the rendered frame matches the double one everywhere except
  inside unresolved speckle, where both are pixel noise. The finish must stay double to
  hold 1e-3 px. Untested: slope lighting, motion, full resolution with `--ss 2`.
- DEC-16 and DEC-17 are still proposed.

## Things learned that change earlier entries

- **Eye of the Universe (2026-10-07 row):** its long "repelling-cycle dwells" (r = 197,
  655) are record-return periods, i.e. loops beside nested minibrots. The Koenigs jump is
  exact on the inner laps (whole r = 655 dwell at 32 terms; 2 of 5.7 laps at r = 197) but
  its usable radius there is 1e-6 to 3e-8, so the outer laps need the return map.
- **Certified radii are far too cautious to set the jump radius** (about 1e-4 against the
  working 0.03). Keep them as the PROB-13 fallback proof.
- **Killed:** hex or jittered sample lattices (no gain over the square grid at equal
  samples), and every result in OpenAI's 2026-10-06 math release as a speed lever.

## File these (needs Ishoo)

- PROB-14: record probes A-F as its evidence; next step is item 1 above.
- New: whole-frame mid-band bench in `koenigs_bench` (item 1).
- New: finish reduction in the mid band with the tail patch atlas (item 2).
- New: owner decision on a film-quality error bar for float32 GPU renders.
- PROB-12: note the Eye of the Universe correction and that a census of well-known zoom
  locations (share of each orbit in spiral laps, minibrot loops, near-parabolic gates) is
  the next measurement; it needs a list of coordinates.
- PROB-13: the four research answers as input; working radius from an a-posteriori bound.

## Added later the same evening: the ladder (probe G)

v0 is rung 0 of a ladder of minibrots converging on M(24,2): periods 764 + 2k, distance
to m shrinking by 1/|ρ| and size by 1/|ρ|² per rung. Rungs at sizes 9.2e-101 (k = 409,
period 1,582) and 1.0e-1000 (k = 7676, period 16,116) were found by Newton in 0.7 s and
12 s; coordinates are in `tools/research/misiurewicz/ladder_rungs.txt`, the theory and
the check in `misiurewicz-ladders.md`, the script is `ladder.py`.

Why it matters: every rung is a v0-style destination (same 2-cycle, same λ(c) = 4(c + 1),
same K series), so the fast path should apply at any depth with only the reference orbit
and the minibrot loop map rebuilt. That is untested.

Next, after the whole-frame bench:

6. **Render a new rung.** Generalise `make_zone.sh` / `koenigs_bench_consts.py` to take a
   centre and period, build the zone for k = 409, and render its landing with `--zone`,
   every pixel scored. Then k = 7676. This is the test of flat cost with depth.
7. **Other spiral centres.** The same construction at any Misiurewicz point; period-2
   landing points are dense in the boundary (about 2^Q with preperiod ≤ Q). This is the
   "destination catalogue" for FEATURE-MAP.md / `fd trip`.

File with Ishoo: a new issue for item 6 (PROB-12 family), and a note on FEATURE-MAP.md
that ladders give exact, formula-addressed minibrot destinations at any depth.

## Added last: the landing formula and the bridge (probe H)

- A proposed shortcut (a few steps of ζ² + s, then one lookup of a fixed torus function)
  is **wrong as stated**; see `universal-landing-claims-a-to-e.md`. The exact identity in
  that file (B1) is the decomposition fd's deep path already uses.
- **Cost down a ladder is logarithmic, not flat:** about one more minibrot return per pixel
  each time the depth exponent doubles (measured +0.96 at 1e-100 and +4.32 at 1e-1000
  against v0; `ladder_bridge.py`). Budget for it when rendering a new rung.
- Small, usable: the return map is exactly even in the critical coordinate, so the loop
  map can be a polynomial in ζ² of half the degree.
