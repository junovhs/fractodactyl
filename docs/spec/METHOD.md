# How We Work (research method)

NORTH-STAR.md says **what** Fractodactyl is for. This document says **how** we go after
it. It binds every contributor, human or agent. DEC-14 records its core rule.

The reason it exists: the first decisive experiment (BENC-01) took a day to build and
about half an hour per arm to run, and it answered no. That is a fine result, but at
that pace we get one answer a day. Rendering fractals is slow, so the research loop must
not be rendering fractals.

## 1. Three gates

No hypothesis gets an expensive run until it has won a cheap one.

| Gate | Budget | Input | Purpose |
|---|---|---|---|
| **Probe** | under 30 s | hundreds to a few thousand points or samples, one tile, a few tiny frames | kill or keep. Run constantly |
| **Promotion** | under 5 min | 20-50 frames at 160x90 to 320x180 | check cross-frame behaviour and an honest wall-clock time |
| **Full benchmark** | as long as it takes | the real path at real size | confirm a win the first two gates already showed |

A full benchmark that is not confirming a probe win is a mistake. If a probe cannot be
built for an idea, building the probe is the first task.

## 2. Every claim has a fair opponent

A speedup counts only against the best competitor that could adopt the same trick
without our machinery. For atlas claims, that is the independent renderer that does the
same thing per frame (BENC-01 arm B). For kernel claims, it is the current best kernel
path.

Every experiment states which question it answers:

- **Cheaper frames:** does this make a genuine frame cheaper for anyone? This serves the
  north star directly.
- **Atlas wins:** does this make compiled, reused work beat recomputing per frame? It
  wins only if what it caches is expensive to rebuild or inherently cross-frame.

Both questions are worth asking, but they must not be confused. BENC-01 confused them
until arm B separated them.

## 3. Measure cost, not just counts

Iteration counts are the fast first signal, but not the verdict. Operations differ in
cost: BENC-01 measured BLA doing fewer iterations than plain perturbation yet running
0.78-0.97x as fast between 1e-14 and 1e-29. A probe reports cost-weighted operations, or
real time on a fixed batch, next to the counts. A step reduction under about 2x is
treated as noise until real time agrees.

## 4. Frozen truth

Correctness is checked against a frozen truth pack, not recomputed each time. The pack is
a few nasty locations times about a thousand chosen points (boundary, filament, deep
minibrot, near-parabolic), computed once by the oracle at high precision and committed.
A candidate is compared in seconds. The independent oracle (tools/oracle.py) runs only
when a candidate is promoted. Errors use the oracle's measure, equivalent pixel
displacement (DEC-10).

Frozen points rank and kill; they do not promote. A per-pixel shortcut is **kept** only
after it matches fd (or the oracle) on every pixel of at least one whole frame per tested
depth (≤1e-3 px, 0 class mismatches), with the worst pixels re-checked at high precision
to see which side is wrong (DEC-17, proposed). On 2026-10-07 two configurations passed 48
frozen points and failed whole frames. Whole-frame checks run on GitHub Actions
(`gh workflow run koenigs-bench.yml`).

## 5. Generate and test

Ideas are cheap and scoring decides. Each probe ends in **one number per candidate**,
with correctness as a gate, so that many variants can be generated and ranked
automatically, including overnight by agents. The work is made searchable before it is
made clever.

## 6. Where ideas come from

- **Measured constraints.** Every idea starts from a measured bottleneck and a numeric
  target, for example: "remove the 511 fallback steps/pixel on the 32% of pixels no BLA
  block covers, using under 1 KB per tile". Vague goals produce textbook answers.
- **Forced analogies.** Map the measured problem through other fields on purpose: maps,
  video codecs, JIT and trace compilers, ray-tracing acceleration structures, multigrid,
  virtual texturing, compression, interval arithmetic. Most die. That is the point.
- **Human intuition as a constraint.** The owner's hunches ("Google Maps for fractals",
  "1-3 px features can be smooshed") are hypotheses to put through a probe, not
  decorations and not dogma.
- **Prior art first.** Before building, spend a few minutes on who did this already
  (Kalles Fraktaler, Fraktaler 3, nanomb, zoomasm, the deep-zoom forums and papers).
  Build on it. Prior art is the floor, not the destination.
- **Deep research before a new investigation.** Before opening a new line of
  investigation, the agent drafts 2-3 focused deep-research questions. The owner runs
  them as external deep-research jobs across many sources. A good question names the
  measured bottleneck, the constraints (genuine depth, error contract, bytes, CPU and
  GPU), what we already tried and measured, and the exact form the answer should take:
  methods, their cost scaling, sources, known failures. The answers are saved under
  `docs/research/<date>/` and read before the probe is designed.

## 7. Kill fast, write it down

A dead idea is a result. Record negative and positive results in the log below, with
the command and the numbers, so nobody pays for the same answer twice.

## Results log

### PROB-20: v0 film frames decided before rendering (2026-10-10)

**Measured locally** (Ryzen 9 3900X, 24 threads, 1280x720, 1 run per frame, max_iter 1e5). Command: `THREADS=24 RUNS=1 SIZE=1280x720 MAXIT=100000 bash tools/research/misiurewicz/koenigs_bench/run.sh target/release/fd out/prob20 bench/path-atlas-v0.txt`. Both films are single `fd control` executions over all 750 frames. In the mixed film, `fd control --zone` decides each frame before rendering it.

| Film | Wall s | Seconds in frames |
|---|---:|---:|
| fd per-frame BLA only | 629.4 | 614.7 |
| Mixed: zone on the 315 admitted frames, fd elsewhere | 267.0 | 252.3 |
| **Measured speed-up** | **2.36x** | 2.44x |

- **Admitted frames: 435-749 (315).** On those frames fd takes 383.2 s and the zone 15.8 s.
- **Correctness on every admitted frame:** 0 nu over 1e-3 px (max 7.7e-5), 0 non-finite, 0 de over 0.2% (max 0.13%), 0 normal over 0.2 deg (max 0.12).
- **Class disputes:** after the 367,389 FIX-04 pairs, 4 samples remain, and `diagnose_pixels.py` (180-digit mpmath) assigns all 4 to fd at the max_iter boundary.
  - 742 (625,291) and (603,297), 747 (730,358): fd says Unresolved, while mpmath escapes at n = 99981, 99976 and 99901. The zone has the right class and nu (1e-13 px), but its de is off by 7.4x, 33x and 3.4x and the normal by up to 153 deg. These samples are not de-scored (class differs), so this was filed as a zone defect (FIX-42). **FIX-42 found it is not one:** mpmath puts these three samples 3e-14, 5e-15 and 3e-14 px from the boundary (de), where de and normal are ill-conditioned for any f64 renderer (the zone keeps nu within 1e-13 px). fd compare never scores de/normal below 1e-3 px (BENC-04). diagnose_pixels.py now judges a zone escape against mpmath whatever fd's class, with fd compare's tolerances and that floor. The 2-frame rerun (742, 747) passes: all three are fd faults.
  - 748 (457,177): fd escaped; mpmath and the zone are unresolved at 1e5.

**The band-top cutoff, from zone data and frame geometry.** At frames 431/432, the de errors are not from fixing the parameter at C (mpmath: 1e-16), from the Koenigs, psi or tail-patch series (more terms change nothing), or from f64 C (1e-6). They come from truncating the first return at degree 4. Its relative state error (6e-9 at |v| = 0.94e-3) moves a pixel by about 2e-6 px. Near the scored de floor (1e-3 px) that is a ~0.2% de change.
- **Bound:** the zone now carries the first return's diagonal series M(v,v) = sum S_n v^n to degree 6 (`diag` lines, mantissa + exponent). `zone_covers` bounds the shift at the frame's outer radius r as (|S5| r^5 + |S6| r^6) / (lower bound of |dM/dv| at r) / pixel spacing.
- **Rule:** admit when the bound is <= 5e-7 px, i.e. 0.2% x 1e-3 px / 4, where the 4 covers the measured excess of up to 2.2x.
- **Check against frames 431-434:** bounds 2.6e-6, 1.7e-6, 1.1e-6, 6.7e-7 px, so all are refused. Their measured de maxima were 0.50%, 0.36%, 0.09% and 0.11%. Frame 435 bounds at 4.2e-7 px and is admitted.
- **Scope of the rule:** it depends on resolution (at 320x180, frame 433 is admitted). It covers only the first return, since later returns' inputs do not depend on the frame. Not certified (PROB-13).

**max_iter margin rule: none.** mpmath shows the zone's class at the max_iter boundary is right where fd's is wrong (here and in PROB-18). A frame-level exclusion would only hand those pixels back to the wrong side, so admitted frames are scored with class disputes adjudicated. Compared with PROB-18's post-render 2.46x (projected), the frames are now chosen before rendering, and 2.36x is one measured execution.

### PROB-19: the k = 7676 (~1e-1000) rung renders; three-rung table (2026-10-10)

**Measured locally** (Ryzen 9 3900X, 24 threads, 480x270, best of 3 runs per frame, max_iter 40P, nu/de/normal). Command: `THREADS=24 RUNS=3 SIZE=480x270 RUNGS="0 409 7676" FRAMES="5000 500 50 5" bash tools/research/misiurewicz/run_rungs.sh target/release/fd out/prob19`. Same exact centres and widths as PROB-17; every one of 129,600 pixels per frame scored with `fd compare`. The rival is `fd control --bla per-frame`. At v0 and k=409 that is BLA (`bla.use=used`). At k=7676 the scaled tier has no BLA (`bla.use=none`, FIX-03), so it is plain scaled perturbation.

| Rung | Width / minibrot size | Zone build s | Zone frame s | fd frame s | Speed-up | Map returns/px | nu >1e-3 px | Max nu px | Real class errors | FIX-04 pairs |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v0, P=764 (~4e-50) | 5000 | 11.9 | 0.00768 | 0.162 | 21x | 3.033 | 0 | 2.0e-8 | 0 | 0 |
| v0 | 500 | | 0.00788 | 0.186 | 24x | 4.009 | 0 | 1.4e-9 | 0 | 1 |
| v0 | 50 | | 0.00877 | 0.176 | 20x | 4.789 | 0 | 1.2e-7 | 0 | 117 |
| v0 | 5 | | 0.01065 | 0.177 | 17x | 9.633 | 0 | 6.2e-8 | 0 | 11904 |
| k=409, P=1582 (~9e-101) | 5000 | 13.7 | 0.00900 | 0.168 | 19x | 4.103 | 0 | 4.0e-6 | 0 | 0 |
| k=409 | 500 | | 0.00843 | 0.182 | 22x | 5.016 | 0 | 1.5e-8 | 0 | 1 |
| k=409 | 50 | | 0.00854 | 0.189 | 22x | 6.071 | 0 | 1.4e-6 | 0 | 120 |
| k=409 | 5 | | 0.01103 | 0.189 | 17x | 10.611 | 0 | 1.7e-7 | 0 | 11902 |
| **k=7676, P=16116 (~1e-1000)** | 5000 | 158.5 | 0.0232 | 22.9 | 990x | 8.002 | 0 | 1.4e-8 | 0 | 0 |
| k=7676 | 500 | | 0.0239 | 25.0 | 1050x | 8.171 | 0 | 1.2e-8 | 0 (1 fd fault) | 0 |
| k=7676 | 50 | | 0.0250 | 27.5 | 1100x | 9.243 | 0 | 9.4e-9 | 0 (1 fd fault) | 118 |
| k=7676 | 5 | | 0.0295 | 42.2 | 1430x | 13.620 | 0 | 1.8e-8 | 0 (1 fd fault) | 11903 |

- **Correctness (DEC-17, nu/class): passes at all three rungs.** Every frame has 0 nu samples over 1e-3 px, 0 non-finite, 0 de over 0.2% and 0 normal over 0.2 deg. The maxima are de 2.8e-4 and normal 0.016 deg, both at k=409.
- **The three k=7676 class disagreements are fd's errors.** They are pixels (184,201), (306,79) and (112,119) in frames 500/50/5: fd says Unresolved, the zone says escaped. 1200-digit mpmath escapes them at n = 145079, 161229 and 209590, and the zone matches within 1e-14 px on nu, 3.3e-8 on de and 0 deg on the normal. Filed as FIX-41 (scaled kernel close-return early-out). **Fixed by FIX-41:** the scaled kernel now ends a sample on a close return only if the orbit contracts over it (the f64 kernel's FIX-02 rule, at most 4 checks). These orbits shadow the weakly repelling M(24,2) 2-cycle and expand, so they keep iterating. Rerun locally (16 threads, 1 run, frames 500/50/5): `fd compare` against the zone gives 0 real class errors in every frame, with FIX-04 pairs unchanged at 0/118/11903. The old and new `fd control` differ on exactly these 3 pixels (Unresolved to escaped) and on no nu value. On the 5-size frame, `fd control` takes about 2% longer (46.9/46.9 s before, 47.5/47.9 s after).
- **FIX-04 pairs** (fd Unresolved, zone Interior) remain and are listed, not waived.
- **Independent check before the run:** 12 spread k=7676 pixels against 1200-digit mpmath. The zone agrees on 10 escapes within 2e-13 px on nu, 5.5e-8 on de and 0 deg on the normal; the other 2 are minibrot interior (mpmath does not escape by max_iter).

**Cost criterion: not met as stated.** The issue keeps the result if zone time per frame grows no faster than the return count. From v0 to k=7676, zone time grows 3.02/3.03/2.85/2.77x at widths 5000/500/50/5, while returns per pixel grow 2.64/2.04/1.93/1.41x. Each return at k=7676 costs 1.15-2.0x a v0 return.

The reason is arithmetic, not more returns. At k=7676 the return-map coefficients reach 2^1666 and the states and entry offset reach 2^-1661, so the deep path computes stage 1 and the Koenigs entry in mantissa + binary exponent. Two changes in this issue cut that path's frame time by about 30%:
- term-wise polynomial sums that align and normalise once;
- lazy-exponent approach and tail steps.

k=409 still runs on the f64 path; its time grows 0.97-1.17x against returns 1.10-1.35x, within the criterion.

**What it means:** a full 1e-1000 minibrot frame costs about 3x a 1e-50 one with the zone, against roughly 130-240x for fd without BLA (0.16 s at v0 vs 23-42 s at k=7676). The 1e-1000 rival has no BLA (FIX-03), so the ~1000x is against the best fd renderer available at that depth, not against a BLA-equipped one.

**Zone file:** `make_zone.sh` writes `biseries_x` and `z24_minus_alpha_x` lines (mantissa pair + binary exponent) next to the f64 lines. `zone.rs` uses the exact lines only when the f64 constants under- or overflow, and refuses a zone with neither (fail closed, DEC-21). Old zone files load unchanged, and on v0 the deep path matches the f64 path within 1e-6 px (unit test).

**Method note (FIX-40):** two separately built fd binaries (shared incremental target vs a fresh `-p fd-cli` build) of the *same source* differed by ~10% in speed. Compare only binaries built the same way, timed interleaved.

### PROB-18: noncontiguous passing frames (2026-10-10)

**Local full-path result (manager, Ryzen 9 3900X, 24 threads, 1280x720, max_iter 1e5):** 314 frames accepted, 5 rejected (431, 432, 742, 747, 748), 431 frames uncovered by the guard. On the accepted frames fd takes 379.4 s and Koenigs 15.9 s (23.8x). Whole film: 611.7 s with fd alone vs 248.3 s with Koenigs on the accepted frames, **2.46x end-to-end measured**. Every accepted frame has 0 wrong pixels, 0 class mismatches beyond FIX-04, and 0 de/normal over tolerance. Max nu displacement 5.1e-4 px.

**mpmath adjudication (diagnose_pixels.py, 180 digits):**
- **431 and 432 (band top, about 1.5e-28): Koenigs fault.** The 82 and 4 disputed samples all escape (mpmath agrees with fd's class). The Koenigs de is off by 0.20–0.36% relative (tolerance 0.2%) and the normal by up to 0.19°; fd's de matches mpmath within 3e-8. The error is shading accuracy near the top of the guard band. A guard starting just below 1.4e-28 would exclude both frames.
- **441:** no disputed samples in the single-frame repro (it passed this run too).
- **742 and 747: fd fault.** fd reports class 2 while Koenigs says escaped. mpmath escapes at n = 99,976–99,981 and 99,901, just under max_iter = 100,000, so Koenigs has the right class. Its de/normal for these near-max-iteration escapes is poor (de rel up to 33, normal up to 153°).
- **748: fd fault.** fd says escaped, while Koenigs and mpmath both say unresolved at 100,000.
- **Conclusion:** the remaining class mismatches are fd errors at the max_iter boundary, not Koenigs errors. The only Koenigs defect is de accuracy at the band top, which a tighter guard removes. Still open: a prospective pre-render guard (DEC-19); acceptance here is decided after rendering.

**Retrospective full-path replay, not a new render.** Recomputed independent
per-frame selection on the complete 750-frame, 1280x720, max_iter=100000,
4-thread [PROB-10 Actions run 38045874904](https://github.com/junovhs/fractodactyl/actions/runs/38045874904)
(artifact: outside.jsonl, times.jsonl, scores.jsonl). All 319 guard-covered
frames were previously scored at every one of 921,600 pixels. The independent
gate accepts **314 frames**, rejecting 431, 432, 742, 747 and 748; the other
431 frames are outside the pre-render guard. Across *every accepted frame*:
**0 wrong nu pixels, 0 other class mismatches beyond FIX-04, 0 nonfinite,
0 de over 0.2%, 0 normal over 0.2°**. The separate fd-Unresolved/zone-Interior
FIX-04 gap is 232,543 samples on accepted frames.

| Full-path replay (same Actions 4-thread runner) | Seconds |
|---|---:|
| fd on all 750 frames | 3302.122 |
| fd on the 314 accepted frames | 2060.809 |
| Koenigs on the 314 accepted frames | 84.286 |
| Estimated fd + accepted Koenigs film | **1325.600** |

End-to-end **2.491x projected** from measured per-frame times, not an
independent execution of the mixed film. On the owner's 24-thread Ryzen,
PROB-10 measured 621.0 s fd-only and 313/319 passing candidates; the new
mixed film timing is still to be measured locally. The gate scores complete
frames *after* rendering; it is not a prospective production guard (DEC-19).

**Six reported failures, all independently gated.** Disposition is from the
180-digit mpmath adjudication above (who is wrong, not who failed):

| Frame | Failure mode beyond FIX-04 | Disposition |
|---:|---|---|
| 431 | 82 de and 1 normal over tolerance (local and Actions) | Koenigs (de 0.20-0.36% at band top; fd matches mpmath) |
| 432 | 4 de over tolerance (local and Actions) | Koenigs (de at band top) |
| 441 | 1 de over locally once; passed Actions and the single-frame repro | none reproduced |
| 742 | 2 fd-Unresolved/zone-Escaped (Actions) | fd (mpmath escapes just under max_iter) |
| 747 | 1 fd-Unresolved/zone-Escaped (Actions) | fd (mpmath escapes just under max_iter) |
| 748 | 1 fd-Escaped/zone-Unresolved (Actions) | fd (mpmath unresolved at max_iter) |

The historical artifact has scores, but not the paired FDS columns identifying
those pixels, so the frames were re-rendered for the adjudication. To repeat it, use the exact per-pixel 180-dps mpmath tool
tools/research/misiurewicz/koenigs_bench/diagnose_pixels.py on paired FDS
outputs. Reproduce each of the six frames separately:

```bash
cargo build --release
mkdir -p out/prob18-repro
python3 - <<'PY'
from pathlib import Path
import sys
sys.path.insert(0, "tools/research/misiurewicz/koenigs_bench")
from path_mode import read_path, lines
frames = read_path("bench/path-atlas-v0.txt")
for i in (431, 432, 441, 742, 747, 748):
    Path(f"out/prob18-repro/{i}.txt").write_text(lines([frames[i]]))
PY
bash tools/research/misiurewicz/make_zone.sh out/prob18-repro/v0.zone
for i in 431 432 441 742 747 748; do
  mkdir -p "out/prob18-repro/fd-$i" "out/prob18-repro/zone-$i"
  target/release/fd control "out/prob18-repro/$i.txt" --size 1280x720 \
    --iter 100000 --columns nu,de,normal --threads 24 --bla per-frame \
    --runs 1 -o "out/prob18-repro/fd-$i" > "out/prob18-repro/fd-$i.jsonl"
  target/release/fd control "out/prob18-repro/$i.txt" --size 1280x720 \
    --iter 100000 --columns nu,de,normal --threads 24 --bla per-frame \
    --runs 1 --zone out/prob18-repro/v0.zone \
    -o "out/prob18-repro/zone-$i" > "out/prob18-repro/zone-$i.jsonl"
  python3 tools/research/misiurewicz/koenigs_bench/diagnose_pixels.py \
    "out/prob18-repro/fd-$i/frame-00000.fds" \
    "out/prob18-repro/zone-$i/frame-00000.fds" \
    > "out/prob18-repro/oracle-$i.txt"
done
```

Run the full path locally (Ryzen 9 3900X, 24 threads; Python needs mpmath,
gmpy2, numpy) and record its new report.md here:

```bash
THREADS=24 RUNS=1 SIZE=1280x720 MAXIT=100000 \
  bash tools/research/misiurewicz/koenigs_bench/run.sh \
  target/release/fd out/prob18-full bench/path-atlas-v0.txt
```

### PROB-17: conventional zones down the v0 ladder (2026-10-10)

**Measured on GitHub Actions:** [whole-frame run](https://github.com/junovhs/fractodactyl/actions/runs/38052204667), Ubuntu, 4 threads, 480x270, one run per frame, max iteration budget 40P, columns nu/de/normal. For each rung the same exact centre and width were rendered with `fd control --zone` and with fresh `fd control --bla per-frame` (`bla.use=used` at v0 and k=409). Every one of 129,600 pixels/frame was scored using `fd compare`; all frames used the zone. The v0 and k=409 zones have degree-4 biseries, guard 1e-3 of the zone state scale, 12 phi terms, 18 psi terms, and depth-6 tail patches. Zone builds include the whole patch atlas. Times below are one-run frame seconds, **not hardware-independent benchmarks**.

| Rung | Frame width / minibrot size | Build s | Zone frame s | BLA frame s | Map returns/px | nu >1e-3 px | Max nu px | Real class errors | FIX-04 Unresolved/Interior |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v0, P=764 | 5000 | 10.337 | 0.033418 | 0.717314 | 3.033 | 0 | 1.96e-8 | 0 | 0 |
| v0 | 500 | — | 0.033022 | 0.814273 | 4.009 | 0 | 1.43e-9 | 0 | 1 |
| v0 | 50 | — | 0.037331 | 0.769993 | 4.789 | 0 | 1.17e-7 | 0 | 117 |
| v0 | 5 | — | 0.046035 | 0.766539 | 9.633 | 0 | 6.23e-8 | 0 | 11904 |
| k=409, P=1582 (size ~9.2e-101) | 5000 | 11.137 | 0.035667 | 0.685252 | 4.103 | 0 | 4.05e-6 | 0 | 0 |
| k=409 | 500 | — | 0.034337 | 0.769040 | 5.016 | 0 | 1.48e-8 | 0 | 1 |
| k=409 | 50 | — | 0.036591 | 0.772772 | 6.071 | 0 | 1.43e-6 | 0 | 120 |
| k=409 | 5 | — | 0.047289 | 0.766662 | 10.611 | 0 | 1.71e-7 | 0 | 11902 |

Across all eight 480x270 frames: **0 nu samples over 1e-3 px, 0 nonfinite,
0 de tolerance failures, 0 normal tolerance failures, 0 real class errors**;
FIX-04 Unresolved-vs-Interior pairs remain (24045 total, strictly failing
`fd compare` on six frames). Maximum de relative difference is 2.751e-4
(0.02751%, below the 0.2% tolerance) and maximum normal-angle difference is
0.01648 degrees (below 0.2 degrees). These values score escaped pixels in
both frames, with the usual near-boundary exception for de/normal. FIX-04
pairs are listed separately, not silently waived; the strict whole-frame
class gate remains **unpassed** at widths 500/50/5.

**Cost gate at 1e-100:** at 5000 sizes wide, the k=409/v0 counted-map-return
ratio is 4.103/3.033 = 1.35, and zone-time ratio is 0.035667/0.033418 =
1.07, below the cost-growth limit. At widths 500/50/5, zone-time ratios
are 1.04/0.98/1.03 versus return-count ratios 1.25/1.27/1.10.
Map-return counts exclude the initial orbit period; add one for comparison
with Probe H's steps/P on escaped pixels. At 5000 sizes wide this gives
~4.03 and ~5.10 periods/px, broadly consistent with Probe H's six-point
4.03/4.99. BLA time did **not** grow across these two rungs; the claimed
growth with depth exponent was not established here. This is a positive
speed/correctness probe at 1e-100, **not a three-rung keep result**.

The initial [48x27 diagnostic](https://github.com/junovhs/fractodactyl/actions/runs/38051482894)
failed numerically at k=409 (1230/1210/1026 outliers at widths 5000/50/5):
the generator incorrectly scaled the map by `size * 1e25` rather than
`sqrt(size)`. The corrected state scale plus high-precision map construction
passed the subsequent [48x27 check](https://github.com/junovhs/fractodactyl/actions/runs/38051966768)
with zero nu/de/normal failures; the 480x270 rows above use the correction.
The failed diagnostic is not counted as a result of the final implementation.

**k=7676, P=16116, size ~1e-1000:** the high-precision zone **built in
43.708 seconds** on the same runner (14.6 MB zone file, including 16190
patch leaves; 33.75 s spent generating constants and 8.1 s on patches).
It preserves the nucleus as exact decimal text and the size as mantissa
times binary exponent (~2^-1661). However, `z24_minus_alpha` is ~1e-500
and rounds to zero in the existing Rust zone data structure. The parser
explicitly declines this case instead of producing an invalid jump; a
scaled Koenigs entry/jump is necessary before `--zone` can render any
k=7676 landing. Also, on the scaled tier `fd control --bla per-frame`
has no usable BLA (`bla.use=none`; FIX-03). **No k=7676 frame seconds,
return counts or class/nu/de/normal score can be claimed.** The
logarithmic-growth hypothesis at k=7676 is still open. **Superseded by PROB-19 above:** the k=7676 rung now renders and is scored.

Reproduce on Ryzen 9 3900X (24 threads) from the repo root (Python 3,
mpmath and NumPy installed):

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release
THREADS=24 RUNS=3 SIZE=480x270 RUNGS="0 409" FRAMES="5000 500 50 5" \
  bash tools/research/misiurewicz/run_rungs.sh target/release/fd out/prob17
BUILD_ONLY=1 RUNGS="7676" FRAMES="5000" \
  bash tools/research/misiurewicz/run_rungs.sh target/release/fd out/prob17
```

The comparison command exits nonzero on the known FIX-04 class mismatches;
the per-frame JSONL and exact class-pair scores are still written.
After a scaled Koenigs jump and scaled-tier BLA become available, rerun
the k=7676 frames with
`THREADS=24 RUNS=3 SIZE=480x270 RUNGS=7676 FRAMES="5000 500 50 5" bash tools/research/misiurewicz/run_rungs.sh target/release/fd out/prob17`.
Before those fixes this fails by design. GitHub Actions passed
`cargo test --workspace`, Clippy with `-D warnings`, shell syntax
and Python compilation. The Actions run reports failure because strict
`fd compare` rejects the documented FIX-04 class pairs.

FACT-01 measurement contract: normalized parameter scale `s_c = 4.1205e-50 * abs(rho)^(-2k)`, state scale `s_z = sqrt(s_c)`; errors are `abs(delta state)/s_z` and `abs(delta (dz/dc))*s_c/s_z`. All deep coordinates and widths stay in mpmath. Cold build includes centre parsing and one 48-term parameter-dependent chart; chart rebuild is timed separately for each of eight pixel parameters; evaluation/direct times are totals for 16 one-return calls. Bytes count decimal-serialized complex coefficients and parameter jets, not Python object heap. Chart entry/output radius <=1e-3, inverse residual and amplified last-eight-term estimate guard evaluations; **this is not a certified bound**. On rejection return a reason and use direct jet iteration as fallback. Only 16 sampled inputs, not whole frames (DEC-17); timing is Python-on-sandbox, not Rust benchmark.

| Date | Question | Probe | Answer |
|---|---|---|---|
| 2026-10-10 | PROB-10: how much of the actual 750-frame v0 film is covered by the full-frame 1e-28 guard, and what is the end-to-end gain over per-frame BLA at 1280x720? | `gh workflow run koenigs-bench.yml --ref gpt/PROB-10 -f path=true -f path_file=bench/path-atlas-v0.txt -f path_size=1280x720 -f path_runs=1 -f path_maxit=100000 -f threads=4`; branch-push Actions run [38045874904](https://github.com/junovhs/fractodactyl/actions/runs/38045874904) began the full run. Reproduce on Ryzen 9 3900X: `THREADS=24 RUNS=1 SIZE=1280x720 MAXIT=100000 bash tools/research/misiurewicz/koenigs_bench/run.sh target/release/fd out/prob10-local bench/path-atlas-v0.txt` after `cargo build --release` and Python `mpmath gmpy2 numpy` setup. | **Measured locally (Ryzen 9 3900X, 24 threads, 1280x720, max_iter 100000); band not yet promoted.** Guard 1e-28 covers 319 frames (431–749), 63.3% of whole-film fd time (393.2 of 621.0 s). Across those 319 frames Koenigs takes 16.7 s vs fd 393.2 s (23.6x; per-frame 19–30x). 313 frames pass every-pixel: 0 wrong pixels and 0 non-finite values. 6 fail: frames 431, 432, 441 (band top, ~1.5e-28 width) on de over tolerance (82, 4, 1 samples; one normal), and frames 742, 747, 748 (deepest) with 2, 1, 1 class mismatches beyond FIX-04. Max nu displacement 5.1e-4 px. FIX-04 Unresolved→Interior: 367,389 samples, reported separately. The harness's contiguous-deep-suffix rule then accepts only frame 749 (0.25% of the film), so end-to-end it is 1.00x as built. If the 6 failures are fixed (or a per-frame guard abstains on them), the band would cut the film to about 244 s, about 2.5x overall. Follow-ups: the de fault at the band top and the class mismatches at the deepest frames. |
| 2026-10-10 | FACT-01, P=764: can a parameter-dependent period-764 return and total dz/dc be built without 764 raw steps? | `python tools/research/misiurewicz/factored_return.py --period 764`; mpmath 135 dps; 48 Koenigs coefficients and their parameter jets; q=24, jump 189 two-cycles, direct exit 360 steps. Eight genuine pixel parameters (4 offsets at 5 and 5,000 minibrot sizes), first two biseries-loop inputs each; 16 direct high-precision state/derivative jets. | **KEEP as research probe (not DEC-17 production promotion).** Cold build 0.080 s; 8 pixel-parameter chart rebuilds 0.643 s; 16 evaluations 0.260 s vs 0.182 s direct; serialized operator 26,829 bytes; chart coverage 16/16; failures 0; max normalized state error 7.156e-106 and dz/dc error 5.116e-109. Coefficient conditioning: max abs(K_j) 1.182e21, max abs(dK_j/dc) 7.047e23, min abs(rho^j-rho) 0.6062. |
| 2026-10-10 | FACT-01, P=1,582 (k=409): does the same chart work at the ~1e-100 rung? | `python tools/research/misiurewicz/factored_return.py --period 1582`; mpmath 185 dps, same eight parameter positions and two consecutive direct-jet return inputs (16 comparisons); q=24, chart jump, 502 direct exit steps. | **KEEP as research probe.** Cold build 0.082 s; 8 chart rebuilds 0.670 s; 16 evaluations 0.327 s vs 0.376 s direct; serialized operator 36,771 bytes; coverage 16/16; failures 0; max normalized state 1.504e-130 and dz/dc 1.075e-133. Coefficient maxima 1.182e21 / 7.047e23, min denominator 0.6062. |
| 2026-10-10 | FACT-01, P=16,116 (k=7,676): is build genuinely sublinear and the return jet accurate near ~1e-1000? | `python tools/research/misiurewicz/factored_return.py --period 16116`; mpmath 1,080 dps, same 16 real consecutive-loop inputs; q=24, multiplier power, 502 direct exit steps; no P-length work during construction. | **KEEP FACT-01 for FACT-02 only, not as a kept per-pixel production shortcut.** Cold build 0.645 s (8.1x P=764 vs 21.1x raw period); 8 chart rebuilds 4.719 s; 16 evaluations 1.864 s vs 19.090 s direct; serialized operator 212,182 bytes; coverage 16/16; failures 0; max normalized state 1.588e-575 and dz/dc 1.136e-578. Coefficient maxima 1.182e21 / 7.047e23, min denominator 0.6062. The 1e-3 normalized research budget passes; **whole-frame every-pixel/class/nu/DE/normal verification remains for FACT-02**. |
| 2026-10-10 | FIX-35: can sub-1e-300 samples near weak repelling and near-parabolic cycles be miscalled Interior? | Pre-fix `scaled.rs` loop reproduced in Python, independently checked by mpmath direct iteration (1200-1550 bits), offset `2^-1100`, escape `r²=1e20`; 20,000 steps (6,000 for interior). 5 points at `c=i`, `0.250001`, `0.25001`, `0.25000001`, `-0.75+0.01i`; 5 offsets at ladder rung k=7676 near M(24,2); 3 offsets inside the k=409, ~1e-100 minibrot (`tools/research/misiurewicz/ladder_rungs.txt`). | **Pre-fix: 0 falsely Interior / 9 oracle escapes.** All nine escaped at the oracle's count (887, 3145, 996, 320, and 10647-10662); the remaining four were Unresolved in both implementations within budget. No triggering false positive found in this small probe, so it is not an exhaustive proof. FIX-35 now declines uncertified near returns as Unresolved rather than claiming Interior. Python probe ran locally; `cargo test --workspace --release` and Clippy `-D warnings` pass locally (checked 2026-10-10 on main ead9352). |
| 2026-10-07 | Does the v0 atlas beat independent frames? (BENC-01) | full benchmark, 750 frames | **No.** It ties per-frame BLA (0.996x). It cached reference and tables worth 18 ms of a 640 ms frame. BENCH.md |
| 2026-10-07 | At extreme depth, does reference cost grow enough that caching it wins? | `fd control` with one frame, 960x540, iter 1e5, BLA none and per-frame: 1e-48, 1e-90, c=i at 1e-300 and 1e-1000 | **No.** The reference took 14-111 ms against renders of 0.8-2.8 s (at most 4%). Side findings: (a) BLA builds no usable table on the scaled tier (1e-300 and beyond run 800-2700 plain steps/pixel); (b) inside the period-764 minibrot at 1e-90, every pixel is Unresolved (interior not detected): 48 s plain, 1.75 s with BLA |
| 2026-10-08 | Prior art: cross-frame reuse in zoom videos (deep research, `docs/research/10-8-26/zoom-computation-reuse.md`) | report, plus an analytic check | **Keep: exponential-map strips** (Kalles Fraktaler / Fraktaler 3 export, zoomasm assembly). Each zoom octave is computed once as a log-polar strip of palette-independent samples, and frames are reprojected from it. Rotation and zoom timing are free at assembly time; panning off the fixed centre breaks it. The report's "95-99.9%" compares undersampled strips. At matched sample density the saving is frames-per-octave dependent: **1.8x** on the v0 path (1.97 dec/s, 30 fps), **5.7x** at 0.5 dec/s and 24 fps, **11.3x** at 0.25 dec/s. Its claim that the reference dominates at 1e500 contradicts our measurement (111 ms against a 2.8 s render at 1e-1000). Next: PROB-02 |
| 2026-10-08 | Prior art: return maps near minibrots (deep research, `docs/research/10-8-26/minibrot-renormalization-local-maps.md`) | report | **Keep, needs primary sources.** The idea exists as NanoMB1/NanoMB2 (Heiland-Allen, in Kalles Fraktaler): a per-period bivariate series iterated until escape. BLA's linear validity collapses as the reference passes near 0, about once per period near a minibrot, which is a likely cause of the 32% of pixels with no BLA skip. A map with a quadratic term survives that pass (Douady-Hubbard: the local return map is quadratic-like). The report has no measured speedups and several garbled formulas; read the NanoMB source and blog before designing the probe |
| 2026-10-08 | Prior art: what removes the per-pixel tail beyond perturbation + BLA (deep research, `docs/research/10-8-26/state-of-the-art-beyond-perturbation-bla-for-cpu-deep-zoom-mandelbrot.md`) | report | **Keep, three leads.** (1) Our "no BLA at 1e-300+" is most likely a range bug: BLA coefficients and radii must carry an explicit exponent (Imagina switches to FloatExp below about 2^-896 half-height, roughly where our scaled tier starts); see FIX-03. (2) Deep-minibrot interiors: atom-domain period detection on the reference, Newton nucleus refinement, then per-pixel derivative contraction (about 1e-3, valid only with a nucleus reference; our v0 centre already is one); see FIX-04. (3) The 511-step tail: Imagina's multi-stage LA plus "approximation transformation" (AT; one transformed step equals StepLength raw steps, N' = N/L) is the most direct CPU prior art, then second-order BLA (a 2026 WebGPU implementation reports 21x and 96.9% skipped at 2.8e40, GPU only). All three are per-frame techniques an independent renderer can adopt (the "cheaper frames" question, not "atlas wins"). The report's citations are placeholders, so verify against the Imagina and FractalShark source before porting |
| 2026-10-08 | Can analytic (Böttcher-coordinate) patches replace iteration in M-free regions? | de histogram on 10 views, 480x270 | **Kill as a general accelerator.** On the hard views (valley 1e-28, v0 at 1e-40 and 2e-49), 0% of pixels are ≥32 px from the set; only dendrite views (c=i) have large empty areas. `docs/research/10-8-26/misiurewicz-frame-transfer.md` |
| 2026-10-08 | Are deep frames near a Misiurewicz point exact transforms of shallower frames (Tan Lei similarity)? | paired renders, 480x270, c=i at 1e-300 and the v0 target near M(24,2) at 1e-6 to 1e-23 | **Keep: strongest lead so far.** Using the multiplier ρ (zoom by \|ρ\|, rotate by −arg ρ, shift the centre to c0+ρ(C−c0), ν += period): 0 class mismatches at every depth; max displacement 9.8e-4 px at 1e-10, 1.2e-5 at 1e-12, ≤3.5e-7 px (kernel noise) from 1e-16 to 1e-23. One 0.062-decade ring covers the whole zone at any depth, speed or rotation, and it is inherently cross-frame. The near-minibrot analogue (scale by λ) is **not** a similarity: the body matches but the surrounding decoration hairs do not. `docs/research/10-8-26/misiurewicz-frame-transfer.md` |
| 2026-10-08 | Is the minibrot approach band (1e-25 to 1e-49 on v0) structured like the Misiurewicz zone? | pointwise mpmath, 120-160 random pixels per depth, 1e-30 to 2e-48 | **Keep: decomposition proven exact.** Naive z² map: fails. Per pixel: k returns around the minibrot (ν += 764 each; k = 2-7), then a tail depending only on the exit point ζ through **one fixed dynamical-plane function E_{c0}(ζ)**. 0 class mismatches, max 2.3e-9 px down to 2e-48. Open: a cheap return map (NanoMB/AT) and a sampled E table with an error contract. Potential: about 7 cheap evals plus 1 lookup per deep pixel, shared across frames. `docs/research/10-8-26/misiurewicz-frame-transfer.md` |

| 2026-10-07 | Can one fixed cheap biseries replace every original period-764 return? (PROB-03) | `.venv/Scripts/python.exe tools/research/misiurewicz/returns_exit_tail.py --probe --jobs 18`; 48 committed 110-dps truth points, six depths; 7.7 s | **Kill this all-return replacement, not NanoMB in general.** Degrees 1, 2, 4, 6, 8, 10, 12 all score 0 across depths. Degree 4 passes the complete 1e-38 cohort (max 2.17e-4 px, 0/8 class mismatches); later return inputs exceed the local domain elsewhere. Keep raw fallback. See the contract and table below. |

| 2026-10-07 | Tight biseries exit plus a shared fixed-C tail (PROB-07) | `returns_exit_tail.py --tight-probe --jobs 18`; unchanged 48-point truth, 2.49 s incl. BLA control | **Keep on sample:** degree 4/6/8 pass every depth at input guard 1e-26, zero class mismatches; degree 4 max error 2.24e-4 px. Degree 2 fails three depths at that guard but passes all at 1e-30. Exit histogram and projected costs below; table not built. |

| 2026-10-07 | Can a shared fixed-C log-polar exit table replace the tail? (PROB-04) | `returns_exit_tail.py --table-probe --jobs 18 --report <external>/table.json --bla-exe <existing>/fd.exe`; 48 frozen points, 4.840 s, exit 0 | **Kill the two uniform bilinear nu tables.** 135,936 / 2,165,760 bytes; max 154.713 / 27.004 px versus 0.001 px required, 0/48 class mismatches. Exact 110-dps corner values leave the interpolation failure intact. Correctness-gated scores 0; no wider promotion or atlas-win claim. |

### PROB-03: fixed period-764 biseries (negative result)

The question is **cheaper frames**. A per-frame renderer can build the same map;
this is not an atlas win. Read RESE-02's scope first: its primary-source distillation
is still backlog, so this experiment does not claim to implement Imagina AT or the
full KF NanoMB algorithm. Primary formulation: [Heiland-Allen's deep-zoom write-up](https://mathr.co.uk/blog/2021-05-14_deep_zoom_theory_and_practice.html)
and [KF's NanoMB manual](https://mathr.co.uk/kf/manual.html). Both describe a biseries
in orbit and parameter offsets, repeated for one period at a time, then regular
iteration outside its escape radius. This implementation derives its own coefficients
from the quadratic recurrence; no upstream code was copied.

For reference orbit Z beginning at C, write z=Z+s*u, c=C+s*v, s=1e-25.
Compose `u_next=2*Z*u+s*u*u+v` for 764 steps, truncating total degree after
each step and retaining all mixed terms. Reference and nucleus residual use 110-dps
mpmath; coefficients use NumPy complex128. Start the pixel at z1=C+d and apply the
map once for every original return. The frozen truth fixes the original return
count for this diagnostic: this is not a runnable production exit scheduler.
After those returns, directly iterate the candidate at the actual pixel parameter
(rather than c0) to isolate map error from the separate exit-table hypothesis.

**Validity/error/fallback contract:** original exit radius is 1.7e-15 around C;
the candidate's conservative empirical input guard is 1e-26 for degree >=4.
This guard is sampled, not a certified disk. Error limit is 1e-3 px, both for
smooth-iteration displacement (truth DE divided by w/480) and the local equivalent
parameter displacement `|z_map-z_truth|/|dz_truth/dc|/(w/480)`. Outside-guard
polynomial evaluations are diagnostic only, never accepted. Nonfinite/huge output,
escape inside a raw period, an input outside the guard, class mismatch, or error
above the limit prevents an all-return pass. The unchanged legacy raw-iteration
path remains the fallback. No points are silently removed from the eight-per-depth
denominator. Class checks are Escaped versus Unresolved at 20,000 iterations;
Unresolved is not a proof of interior membership. The frozen fixture is generated
by `--freeze --jobs 18`; normal probes never recompute truth. This is a small
issue-local pack, not completion of TRUT-01 or an independent-oracle promotion.

Highest tested degree (12), original radius; errors below are **unguarded
diagnostics** on finite candidates, including candidates needing fallback:

| Width | Max tail px error | Max local state px error | Class mismatches / evaluated | Fallback / 8 | Map return ops/px | Raw return ops/px |
|---|---:|---:|---:|---:|---:|---:|
| 1e-35 | no finite candidate | no finite candidate | 0 / 0 | 8 | 2800 | 9168 |
| 1e-38 | 1.35e-12 | 1.12e-12 | 0 / 8 | 0 | 2800 | 9168 |
| 1e-40 | 2.79e-12 | 1.12e-12 | 0 / 7 | 1 | 2975 | 9741 |
| 1e-43 | 13.53 | 4.29e6 | 0 / 6 | 7 | 4200 | 13752 |
| 1e-46 | 3.50e-12 | 1.48e-12 | 0 / 4 | 5 | 5600 | 18336 |
| 2e-48 | 8.42e-11 | 7.63e-11 | 0 / 5 | 7 | 7525 | 24639 |

Build times for degrees 1/2/4/6/8/10/12 were
0.009/0.011/0.032/0.092/0.213/0.477/0.897 s. Map costs per return are
36/90/240/446/708/1026/1400 real arithmetic operations against 4584 for
764 raw squares/adds (complex multiply=6, square=4, add=2).
Per-pixel columns multiply this by the truth's mean number of requested returns;
they are theoretical attempted-map costs, not successful end-to-end costs.
Tail, derivatives, fallback, memory traffic, and build amortization are excluded.
Degree 1 is a local linear-return diagnostic, **not** our hierarchical BLA renderer.
Because every candidate fails correctness/domain coverage, no speedup over BLA
or promotion is claimed; a fair BLA timing is required before any later keep result.

**Handoff:** this finite sweep kills one fixed, low-degree map over the original
return schedule. It does not rule out higher degrees, a smaller exit radius with
raw transition steps, adaptive charts, or multi-stage LA/AT. RESE-02 remains the
prior-art prerequisite for that next experiment. PROB-04 must not assume that all
returns are now cheap. The exit table, Rust kernel and nested chains were not changed.

### PROB-07: tight exits and one fixed-parameter E_C tail

2026-10-07. **Keep the decomposition on the frozen sample.** With the common
1e-26 input guard, degree 4 is the lowest tested degree passing all six depths;
degree 2 fails at 1e-38, 1e-43 and 1e-46, while degrees 6 and 8 also pass everywhere.
All 192 candidate evaluations have zero class mismatches and zero numerical/return-limit
failures. This corrects the overly wide 1.7e-15 exit schedule tested in PROB-03.

Command (Python environment needs mpmath and NumPy):

```text
.venv/Scripts/python.exe tools/research/misiurewicz/returns_exit_tail.py --tight-probe --jobs 18 --report C:/Users/SpencerNunamakerTrav/fractodactyl/target/PROB-07.json --bla-exe C:/Users/SpencerNunamakerTrav/fractodactyl/target/release/fd.exe
```

Exit 0, 2.494 s including the BLA control. The original 48-point,
110-dps fixture is unchanged. Candidate scheduling starts at z1=C+d, applies a
biseries only while its **input** satisfies |z-C| <= guard, and stops immediately
after an output leaves that domain. It never uses truth's old return count.
At exit zeta, the candidate smooth value is **1+k*764+E_C(zeta)**, where E_C
is directly evaluated at fixed C with a fixed 20,000-step tail budget. No c0 or
pixel parameter is used in that tail. Total escape steps are then checked against
the fixture's 20,000-step horizon. Direct tail evaluation stands in for the future
shared table; there is no table, interpolation, Rust change or promotion here.

**DEC-10 contract:** keep requires both smooth displacement and local equivalent
parameter displacement <=1e-3 px, zero class mismatches and zero failures. The local
exit-state diagnostic runs only an auxiliary exact prefix at the pixel parameter,
at the candidate's exit count, and divides state difference by its d/dc derivative
and w/480. Frozen smooth/class truth is not recomputed. A second control runs E_C
from that exact prefix state: its worst smooth error is 1.45e-11 px with the common
guard, separating map truncation from fixed-C substitution. These are sampled
guards, not certified disks; unsupported inputs and observed errors require raw
fallback in a later implementation. All eight samples per depth stay in the score.

Common input guard 1e-26; every row has class mismatches 0/8:

| Width | Degree | Max smooth px | Max local px | Returns range | Mean direct-tail steps | Projected map ops/px | Verdict |
|---|---:|---:|---:|---:|---:|---:|---|
| 1e-35 | 2 | 1.169e-8 | 1.127e-8 | 1–1 | 722.3 | 90.0 + lookup | KEEP |
| 1e-38 | 2 | 1.550e+0 | 7.798e-1 | 2–2 | 155.5 | 180.0 + lookup | KILL |
| 1e-40 | 2 | 6.284e-5 | 7.652e-5 | 2–2 | 426.1 | 180.0 + lookup | KEEP |
| 1e-43 | 2 | 1.024e-2 | 2.235e-2 | 2–3 | 753.6 | 191.3 + lookup | KILL |
| 1e-46 | 2 | 6.015e-2 | 3.132e-2 | 3–4 | 550.8 | 303.8 + lookup | KILL |
| 2e-48 | 2 | 3.079e-5 | 1.832e-5 | 4–5 | 701.0 | 405.0 + lookup | KEEP |
| 1e-35 | 4 | 1.450e-11 | 6.472e-13 | 1–1 | 722.3 | 240.0 + lookup | KEEP |
| 1e-38 | 4 | 2.172e-4 | 2.237e-4 | 2–2 | 161.1 | 480.0 + lookup | KEEP |
| 1e-40 | 4 | 2.787e-12 | 1.117e-12 | 2–2 | 426.1 | 480.0 + lookup | KEEP |
| 1e-43 | 4 | 6.506e-7 | 5.636e-7 | 2–3 | 753.6 | 510.0 + lookup | KEEP |
| 1e-46 | 4 | 1.553e-6 | 1.495e-6 | 3–4 | 550.8 | 810.0 + lookup | KEEP |
| 2e-48 | 4 | 2.776e-12 | 1.471e-12 | 4–5 | 701.0 | 1080.0 + lookup | KEEP |
| 1e-35 | 6 | 1.450e-11 | 6.472e-13 | 1–1 | 722.3 | 446.0 + lookup | KEEP |
| 1e-38 | 6 | 7.974e-8 | 3.990e-8 | 2–2 | 161.1 | 892.0 + lookup | KEEP |
| 1e-40 | 6 | 2.787e-12 | 1.116e-12 | 2–2 | 426.1 | 892.0 + lookup | KEEP |
| 1e-43 | 6 | 1.430e-11 | 8.498e-12 | 2–3 | 753.6 | 947.8 + lookup | KEEP |
| 1e-46 | 6 | 8.389e-11 | 4.382e-11 | 3–4 | 550.8 | 1505.3 + lookup | KEEP |
| 2e-48 | 6 | 2.776e-12 | 1.471e-12 | 4–5 | 701.0 | 2007.0 + lookup | KEEP |
| 1e-35 | 8 | 1.450e-11 | 6.472e-13 | 1–1 | 722.3 | 708.0 + lookup | KEEP |
| 1e-38 | 8 | 2.217e-11 | 9.802e-12 | 2–2 | 161.1 | 1416.0 + lookup | KEEP |
| 1e-40 | 8 | 2.787e-12 | 1.116e-12 | 2–2 | 426.1 | 1416.0 + lookup | KEEP |
| 1e-43 | 8 | 1.329e-12 | 1.116e-12 | 2–3 | 753.6 | 1504.5 + lookup | KEEP |
| 1e-46 | 8 | 1.166e-12 | 1.351e-12 | 3–4 | 550.8 | 2389.5 + lookup | KEEP |
| 2e-48 | 8 | 2.776e-12 | 1.471e-12 | 4–5 | 701.0 | 3186.0 + lookup | KEEP |

The remaining direct-tail work is real: 161–849 steps/pixel for the passing
degree-4 cohort. With an actual E_C table it would become one lookup; lookup cost
and interpolation error remain unmeasured. Arithmetic uses PROB-03's model
(complex multiply=6, square=4, add=2), excluding derivatives, guards, memory and
build amortization. Maps cost 90/240/446/708 ops per return at degrees 2/4/6/8;
measured build seconds were 0.012/0.025/0.077/0.192. Worst-depth scores against frozen raw arithmetic,
with lookup unpriced, are 0/21.127/11.369/7.162. They are projections, not measured
speedups.

**Fair opponent (DEC-14):** the existing fd binary runs per-frame BLA on separate
8x4 grids at the same six widths, via `fd control <six-line-path> --size 8x4
--iter 20000 --columns nu,de --threads 18 --bla per-frame --runs 1`.
All 192 grid samples escaped. BLA work is measured, but its grid differs from the
frozen eight points, so this is contextual cost comparison, not a matched-point
speedup. BLA arithmetic is modeled at 14 ops per perturbation fallback or linear
block (2Z*dz+dz^2+dc or A*dz+B*dc), excluding derivative and guard work.
A per-frame rival can build the same biseries. The shared E_C function is the
cross-frame candidate; atlas benefit is still unproven until PROB-04 builds and
prices the table.

| Width | Frozen direct ops/px | BLA raw fallback/px | BLA blocks/px | Modeled BLA ops/px | Degree-4 map ops/px |
|---|---:|---:|---:|---:|---:|
| 1e-35 | 8923.5 | 670.19 | 37.00 | 9900.6 | 240.0 + lookup |
| 1e-38 | 10140.8 | 658.28 | 44.13 | 9833.7 | 480.0 + lookup |
| 1e-40 | 11730.8 | 761.19 | 47.63 | 11323.4 | 480.0 + lookup |
| 1e-43 | 14268.8 | 616.25 | 51.50 | 9348.5 | 510.0 + lookup |
| 1e-46 | 18781.5 | 675.66 | 67.97 | 10410.8 | 810.0 + lookup |
| 2e-48 | 24840.0 | 714.00 | 81.78 | 11140.9 | 1080.0 + lookup |

**Exit-radius histogram:** bins are floor(log10(|zeta-C|)); each cell sums to eight.
All four degrees at the common guard have these same bin counts. Their outputs
can overshoot far beyond the input guard: degree-4 exit radii range from
1.249e-26 to 7.648e-5. PROB-04 must cover the actual exit distribution, not assume
it ends at 1e-22. The annuli alone do not determine table size or interpolation error.

| Width | Common 1e-26 guard, degrees 2/4/6/8 (exponent: count) | Smaller degree-2 guard 1e-30 (exponent: count) |
|---|---|---|
| 1e-35 | -22: 3, -21: 5 | -22: 3, -21: 5 |
| 1e-38 | -7: 1, -6: 3, -5: 4 | -28: 3, -27: 5 |
| 1e-40 | -15: 1, -14: 3, -13: 4 | -15: 1, -14: 3, -13: 4 |
| 1e-43 | -26: 3, -25: 4, -6: 1 | -27: 1, -26: 3, -25: 4 |
| 1e-46 | -26: 1, -24: 4, -10: 1, -6: 1, -5: 1 | -30: 1, -28: 1, -27: 1, -26: 1, -24: 4 |
| 2e-48 | -26: 4, -25: 1, -18: 1, -16: 1, -12: 1 | -30: 1, -26: 4, -25: 1, -18: 1, -16: 1 |

**Separate smaller-domain quadratic variant:** `--quadratic-guard 1e-30` exits
earlier and passes all six depths (exit 0, 2.519 s). It uses a narrower sampled
guard rather than claiming degree 2 is valid throughout 1e-26. Its worst error is
7.65e-5 px; max fixed-C-only control error is 1.78e-9 px. It trades more tail work
for fewer map operations, and has exit radii 1.441e-30 to 7.648e-13:

| Width | Max smooth px | Max local px | Returns range | Mean direct-tail steps | Projected map ops/px | Verdict |
|---|---:|---:|---:|---:|---:|---|
| 1e-35 | 1.169e-8 | 1.127e-8 | 1–1 | 722.3 | 90.0 + lookup | KEEP |
| 1e-38 | 1.783e-9 | 1.189e-11 | 1–1 | 925.1 | 90.0 + lookup | KEEP |
| 1e-40 | 6.284e-5 | 7.652e-5 | 2–2 | 426.1 | 180.0 + lookup | KEEP |
| 1e-43 | 1.388e-10 | 7.744e-11 | 2–2 | 849.1 | 180.0 + lookup | KEEP |
| 1e-46 | 9.226e-11 | 7.188e-11 | 3–3 | 837.3 | 270.0 + lookup | KEEP |
| 2e-48 | 1.064e-7 | 5.349e-8 | 4–5 | 796.5 | 393.8 + lookup | KEEP |

**Handoff:** degree 4 at 1e-26 is the common-domain baseline for PROB-04; the
narrower quadratic variant is a separate viable option. E_C must use **C**, not c0,
through the transition region. The exit samples suggest a wide transition domain
and motivate testing table coverage/interpolation before claiming atlas speed.

### PROB-04: shared exit table, uniform interpolation rejected

Question: **atlas wins**, conditional on a correct shared E_C payload. The probe
builds two actual tables at fixed parameter C, then reuses each across the six
depth cohorts. It stores float64 smooth escape nu and int32 escape step count
per node (12 bytes/node), rather than resume states, de or normal. This measures
only the smooth/class requirement; it cannot establish complete shading accuracy.
The 764-step degree-4 prefix retains the PROB-07 input guard 1e-26.

Representation: explicit log-polar rings centred at C, log10 radius -26 through
-4, periodic angular seam, bilinear interpolation in log radius and angle.
This covers the measured 1.249e-26 to 7.648e-5 exits. No M(24,2) self-similar
wrap is used: transferring E_C across C-c0 without a measured error contract
would add another unsupported approximation. Table nodes use fixed-C
perturbation around a 110-dps reference starting at C; C-relative deltas preserve
the small offsets instead of rounding C+delta to a double.

Validity/fallback (DEC-10): lookups outside log10 radius [-26,-4), non-finite
inputs, unresolved corners, an ambiguous 20,000-step horizon, or a 60-return
prefix limit require direct iteration. No interpolation bound is certified.
The frozen-pack correctness gate rejects an entire configuration if any point
exceeds 1e-3 px, changes escape class or needs fallback. Both tested
configurations are rejected, so their interpolated values must not be used in
production. Every query remains in the denominator; none of the 48 queries
needed domain/horizon fallback and all escaped in both truth and tables.

Reproduce from the PROB-04 worktree using its .venv (mpmath 1.4.1, numpy 2.5.3):

```powershell
.venv/Scripts/python.exe tools/research/misiurewicz/returns_exit_tail.py --table-probe --jobs 18 --report C:/Users/SpencerNunamakerTrav/fractodactyl/target/prob04/table.json --bla-exe C:/Users/SpencerNunamakerTrav/fractodactyl/target/release/fd.exe
```

The unchanged committed `return_map_truth.json` has SHA-256
`aa93abe304f55d3c28cde6d58c9e4814bd5dfcac0b639cf5b27c560693e5fa89`.
The report records payload sizes, timings, controls, per-depth errors and sharing.
Reports and BLA output stay outside the worktree. The final run took 4.840 s,
exit 0; the biseries build took about 0.03 s. Timing is host-dependent.
Regression: `--tight-probe --jobs 18 --report <external>/tight-regression.json`
took 2.254 s, exit 0, preserving the degree-4/6/8 passes and degree-2 failures.
`git diff --check` also passed (exit 0).

| Radial intervals/decade | Angles | Nodes | Payload bytes | Build s | Warm map+lookup us/px | Max px | Class mismatches | Score |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 8 | 64 | 11,328 | 135,936 | 0.0381 | 18.776 | 154.713 | 0/48 | 0 |
| 32 | 256 | 180,480 | 2,165,760 | 1.8657 | 28.123 | 27.004 | 0/48 | 0 |

These are array payload bytes, excluding metadata, the temporary reference and
construction scratch. Both payloads are small against DEC-03's 2.5 GiB target /
3 GiB cap, but neither meets accuracy. No estimate of sufficient table size is
established by these failures; errors do not decrease uniformly with resolution.

| Width | Max px, 8x64 | Max px, 32x256 | 32x256 with exact corner values | Direct double-tail control px |
|---|---:|---:|---:|---:|
| 1e-35 | 39.6736 | 0.962934 | 0.962934 | 1.450e-11 |
| 1e-38 | 2.03553 | 1.32289 | 1.32289 | 2.172e-4 |
| 1e-40 | 33.5045 | 1.09474 | 1.09474 | 2.787e-12 |
| 1e-43 | 154.713 | 3.78794 | 3.78794 | 6.506e-7 |
| 1e-46 | 53.0709 | 8.04783 | 8.04783 | 1.553e-6 |
| 2e-48 | 16.0497 | 27.0038 | 27.0038 | 2.883e-12 |

Displacement is |nu_predicted-nu_truth| ln(2) de_truth / (width/480).
The deep truth derivative encoded in de accounts for sensitivity through the
returns; interpolated tail derivatives are not substituted into the error metric.
The exact-prefix fixed-C control stays within 1.450e-11 px; the map's local state
error stays within 2.237e-4 px. Direct double-tail controls also pass, with no class
mismatches. Independent 110-dps evaluations replace **all** used corner values:
150 coarse / 152 fine nodes. Maximum node smooth discrepancies are 0.008478 /
4.021e-5, with zero node class or escape-step differences. Re-interpolation from
those exact corners still fails by the values above. This isolates interpolation
as the decisive failure, rather than mistaking a node-evaluation defect for it.

**Measured opponent and timing limits:** `fd control` receives 48 1x1 views,
each centred exactly at the corresponding frozen C+d, same width, 20,000-step
budget, nu/de columns, one thread and per-frame BLA. Pixel-centre sampling makes
these matched coordinates, unlike PROB-07's separate 8x4 grid. Final measured
BLA render cost averages 283.213 us/pixel. Candidate timing is 20 repetitions of
the same predecoded scalar Python degree-4 map + lookup batch, excluding table
build, truth checks and fallback. BLA includes its render/column work; the
candidate computes nu only. Tiny one-pixel views, distinct per-point references
and Python/Rust differences prevent a full-frame speed claim. Score is matched
BLA render time / warm candidate time **only after correctness passes**; both
scores are 0, regardless of the apparent lookup-time advantage. A fair rival can
adopt the same biseries; the table, rather than that map, is the reuse hypothesis.

**Sharing and handoff:** one table serves six depth cohorts, a measured 6:1
frames-per-table ratio on these diagnostic views (48 lookups). Full-v0 frame
coverage and effective accepted sharing are unmeasured; no frame is accepted by
these failed configurations. DEC-14 therefore stops before the >=1,000 points
per depth validation. This satisfies PROB-04's recorded-kill alternative, not its
success alternative. It rejects uniform bilinear nu interpolation at these two
resolutions, not adaptive/derivative-aware tables, resume-state tables, the
return decomposition, or all tables fitting the budget. Next planned work is
PROB-05; another exit-table representation needs its own winning cheap probe.

| 2026-10-08 | Can the exit tail skip its spiral-out along the repelling 2-cycle analytically (Koenigs jump), with no table? | `python tools/research/misiurewicz/koenigs_tail.py tools/research/misiurewicz/return_map_truth.json TERMS R0`; frozen 48-point truth, exact prefix to the PROB-07 1e-26 exit, then a fixed-C tail; about 1 s per configuration | **Keep.** The tail goes 23 direct steps to the 2-cycle point α_C, then φ (Koenigs series of f_C² at α_C, ρ_C) jumps j = ⌊log(R0/\|φ\|)/log\|ρ\|⌋ cycle-steps, then ψ = φ⁻¹ (Newton) and finishes directly. 40 terms with R0 = 1e-3, 1e-2 or 3e-2 is exact against truth (max 6.5e-12 px, 0 class mismatches, all six depths). **12 terms with R0 = 0.03: max 2.6e-8 px; 8 terms: max 2.3e-5 px.** R0 ≥ 0.1 fails (outside the series' useful radius). Tail steps fall from 113-996 to 56-342 (most 56-150); no orbit re-approached α. Replaces the rejected PROB-04 interpolated table for the spiral-out part. It is a per-frame technique ("cheaper frames", DEC-15), not an atlas payload. Open: why some tails still need about 300 steps after the jump; ψ by series reversion instead of Newton; real timing against BLA |
| 2026-10-07 | After the Koenigs jump, why do some tails still need about 300 steps? Is there a second spiral to skip? | `python tools/research/misiurewicz/koenigs_leftover.py tools/research/misiurewicz/return_map_truth.json` (from that directory); same 48 points, 12 terms, R0 = 0.03; under 1 s | **No simple second spiral.** f_C's α fixed point has multiplier −0.9976+0.1314i (\|·\| = 1.0062, very slowly repelling), but the long tails (279 and 319 steps; most others 33-151 after the 23 approach steps) do not spiral out from it monotonically: they wander 0.1-0.5 away from it, through the region shared by the fixed point and the 2-cycle. That is the near-parabolic "seahorse gate" around c = −3/4. A Koenigs series cannot linearise it. Skipping it would need near-parabolic Fatou coordinates (Lavaurs/Douady), a harder construction. It is not attempted here. |
| 2026-10-07 | Can the whole exit tail (approach, Koenigs jump, finish) run in plain IEEE double? | `python tools/research/misiurewicz/koenigs_double.py tools/research/misiurewicz/return_map_truth.json TERMS R0` (from that directory); exact mpmath loop prefix, then the exit point is passed as the double offset δ = ζ − C; 23 approach steps are double perturbation against C's critical orbit, h0 = (Z_24 − α) + δ_24; φ is Horner in double, ψ is Newton in double, the finish is plain double; about 1 s | **Keep.** Matches the 80-digit tail: 40 terms, R0 = 0.03: max 1.2e-10 px. 12 terms, R0 = 0.03: max 2.65e-8 px. 12 terms, R0 = 0.01: 1.3e-10 px. 8 terms, R0 = 0.03: 2.3e-5 px. 6 terms, R0 = 0.01: 3.3e-6 px. 0 wrong pixels on all six depths; tail steps 56-342, as before. Rounding ζ itself to double fails (ζ − C is 1e-26 to 1e-5), so the offset form is required. Two script bugs were caught before logging: ζ was rounded directly (fixed by the offset form), and an off-by-one in ν (exactly 1.0 ν, 3-64 px). The tail is therefore cheap hardware arithmetic (one complex square-add per step plus about 100 flops for the jump). Still open: the loop prefix in double/FloatExp, and real timing against per-frame BLA. |
| 2026-10-07 | Can the whole deep pixel (loops + tail) run in IEEE double with no per-pixel multiprecision? Which loop degree/guard is cheapest? | `python tools/research/misiurewicz/koenigs_double_full.py tools/research/misiurewicz/return_map_truth.json DEGREE 12 0.03 [GUARD]` (from that directory); per pixel only v = (c−C)/1e-25 is rounded from high precision; biseries coefficients come from `returns_exit_tail.build_map` (double); tail from `koenigs_double`; under 1 s | **Keep.** Degree 4 with guard 1e-26: max 2.17e-4 px, 0 wrong pixels, identical to the 80-digit PROB-07 result. Guard sweep (max px / wrong pixels > 1e-3): 1e-26 → degree 2 1.55 / 11, degree 3 2.9e-2 / 4, degree 4 2.2e-4 / 0. 1e-28 → degree 2 9.2e-5 / 0, degree 3 1.75e-6 / 0, degree 4 1.75e-6 / 0. 1e-30 → 6.1e-3 / 1 for every degree. 1e-32 → 7.9 / 7 for every degree. Guards ≤ 1e-30 fail independently of degree: the fixed-C tail ignores c − C, which matters when the exit offset is too small (the expected PROB-07 lower bound, not a precision bug). Cheapest passing loop: degree 2 (5 terms) at guard 1e-28; degree 3 at 1e-28 has more margin. 8 points per depth only. Next: real timing in Rust against per-frame BLA. |
| 2026-10-07 | PROB-08: real timing. Is the all-double deep-pixel pipeline (biseries loops + Koenigs tail) faster than per-frame BLA on whole frames, with every pixel correct? | `bash tools/research/misiurewicz/koenigs_bench/run.sh target/release/fd OUT` (Rust bench, 480x270, 4 threads, best of 2-3 runs, six frames centred at C from 1e-35 to 2e-48, max_iter 20,000); manual CI: `gh workflow run koenigs-bench.yml` | **Keep (local, Windows, 20-core host).** Degree 4, guard 1e-28, 12 φ terms, R0 0.03, scored against `fd control --bla per-frame` on all 777,600 pixels: 0 wrong pixels (> 1e-3 px), 0 class mismatches, max 3.2e-4 px. The worst pixels were checked against 90-digit direct iteration: fd is exact (≤ 1e-11 px), so all residual error is the candidate's. Frame seconds, fd BLA / lean perturbation / Koenigs: 1e-35 0.327/0.505/0.033 (9.9x vs fd); 1e-38 0.551/0.577/0.048 (11.4x); 1e-40 0.923/0.673/0.036 (25.9x); 1e-43 0.482/0.802/0.044 (11.0x); 1e-46 0.497/1.080/0.044 (11.2x); 2e-48 0.569/1.457/0.040 (14.1x). Against the lean perturbation baseline (same Rust file, so the comparison isn't distorted by code quality): 12-36x. That baseline is slower than fd, so fd is not a bloated opponent. Candidate time is flat with depth; both opponents grow. **Full-frame kills:** degree 4 at guard 1e-26 (32-178 wrong pixels per frame, up to 0.14 px, which the 48-point probe missed); degree 2 at 1e-28 (up to 4,720 wrong, 0.5 px, though it passed 48 points); 1e-29 (up to 4 wrong). Degree 3 at 1e-28 also passes. Scope: frames inside the minibrot band centred at the zone nucleus; shallower frames (pixels farther than the guard from C) need another method. Constants are per zone (built once in Python in about 0.1 s) and shared by every frame. This is per-frame speed ("cheaper frames", DEC-15), adoptable by any renderer that knows the zone. |
| 2026-10-07 | PROB-08 on GitHub Actions: does the local result hold on a clean runner? | `gh workflow run koenigs-bench.yml -f threads=4 -f runs=5` (run 37672629317, ubuntu-latest, AMD EPYC 7763, 480x270, 4 threads, best of 5) | **Keep, confirmed.** CORRECTNESS PASS: 0 wrong pixels, 0 class mismatches over 777,600 pixels, max 3.2e-4 px. Frame seconds, fd per-frame BLA / lean perturbation / Koenigs: 1e-35 0.511/0.776/0.060 (8.5x vs fd); 1e-38 0.840/0.905/0.082 (10.3x); 1e-40 1.367/1.040/0.059 (23.4x); 1e-43 0.678/1.263/0.071 (9.6x); 1e-46 0.726/1.685/0.071 (10.2x); 2e-48 0.782/2.258/0.070 (11.2x). Against lean perturbation: 11.1-32.2x. The deep band is 8.5-23x faster than our BLA renderer, timed and exact, on frames centred at the zone nucleus. |
| 2026-10-07 | PROB-08 repeat runs on GitHub Actions: does the speed-up repeat, and does it hold on one thread? | `gh workflow run koenigs-bench.yml -f threads=4 -f runs=5` (run 37673459489) and `-f threads=1 -f runs=3` (run 37673455061); ubuntu-latest, 480x270 | **Keep, repeatable.** Both runs: CORRECTNESS PASS, 0 wrong pixels, 0 class mismatches over 777,600 pixels, max 1.1e-4 to 3.2e-4 px. Speed-up vs fd per-frame BLA at 1e-35/1e-38/1e-40/1e-43/1e-46/2e-48: 4 threads 8.7/10.9/25.1/10.1/10.6/11.7x (first run: 8.5/10.3/23.4/9.6/10.2/11.2x); 1 thread 8.1/10.5/24.9/9.6/10.2/11.2x. Single-thread frame seconds, fd → Koenigs: 1.05→0.13, 1.90→0.18, 3.18→0.13, 1.48→0.15, 1.60→0.16, 1.71→0.15. The gain is per-core, not a threading artefact. Against lean perturbation: 11-37x. |
| 2026-10-07 | Deep-research report: can the near-parabolic "seahorse gate" leftovers be skipped? (`docs/research/10-8-26/skipping-near-parabolic-transits-what-is-computable.md`) | Literature (Kapiamba, Lanford–Yampolsky, Petersen–Zakeri, Braverman) | **Lead, untested.** Exact skips exist: Kapiamba's q = 2 near-parabolic identity g^(2n+k) = χ∘T_(n−1/α_NP)∘ρ, and Koenigs at the weak α fixed point (\|λ\| = 1.00622). The report computes the canonical gate width as only about 22 raw iterations (1/α_NP ≈ 11.47 − 0.57i). It argues the hundreds of leftover steps are weak-repelling dwell near α, best skipped by a second Koenigs jump at α, with Buff coordinates (three logs plus an analytic correction) as a cheap gate model and Braverman long-iterates as a rigorous fallback. No published renderer uses Fatou/Lavaurs/horn maps per pixel. Caveat from our koenigs_leftover diagnostic: the long tails stay 0.1-0.5 from α without monotone spiralling, so they may be repeated gate passes rather than one dwell. Next cheap probe: count gate passes per long tail, and fit a Koenigs series at α (residual vs radius). |
| 2026-10-07 | PROB-08 lunch sweep on GitHub Actions: does the win hold at real resolutions, and which loop setting is cheapest? | `gh workflow run koenigs-bench.yml` with size/degree/guard inputs; runs 37675282913 (1280x720), 37675287111 (1920x1080), 37675291422 (deg 3, 1e-28), 37675295432 (deg 4, 1e-27), 37675299540 (deg 3, 1e-27), 37675303742 (960x540); 4 threads | **Speed holds at every resolution:** 7.4-24x vs fd per-frame BLA (1920x1080 per frame, fd → Koenigs: 8.1→0.94, 13.3→1.30, 21.7→0.92, 10.7→1.12, 11.5→1.13, 12.4→1.11 s). **Strict correctness slips at high resolution.** Degree 4 / 1e-28: 960x540 has 1-4 wrong pixels per frame (max 1.8e-3 px); 1280x720 has 15 at 1e-38 and 1 at 1e-46 (max 3.4e-3); 1920x1080 has 105 / 1 / 7 at 1e-38 / 1e-46 / 2e-48 out of 2.07M (max 4.4e-3 px). That is ≤0.005% of pixels, all under 0.005 px, invisible but over the 1e-3 bar; the worst depth is 1e-38. At 480x270: degree 3 / 1e-28 PASS (max 3.2e-4); degree 4 / 1e-27 PASS with smaller errors at most depths (max 7.4e-4); degree 3 / 1e-27 FAIL (up to 399 wrong, 0.098 px). More pixels expose rarer errors, as DEC-17 predicts. Next: find which stage causes the rare 1e-3 to 5e-3 px pixels (loop truncation, fixed-C tail, or Koenigs terms), re-run 1920x1080 with the fix, and give run.sh a TERMS input. |
| 2026-10-07 | FIX-20 on GitHub Actions: which setting causes the rare 1080p misses (105 of 2.07M pixels, max 4.4e-3 px)? | `gh workflow run koenigs-bench.yml -f size=1920x1080 -f runs=2` with `-f guard=1e-27` (run 37681537407), `-f terms=40` (37681541796), both (37681546005) | **Cause found: the 12-term Koenigs inverse.** 40 φ terms alone: CORRECTNESS PASS, 0 wrong over 12.4M pixels, max 2.8e-5 px. Guard 1e-27 alone: 20 wrong at 1e-38 (max 2.2e-3). Both: 6 wrong at 1e-38 (max 2.6e-3). Guard 1e-27 is rejected at 1080p; guard 1e-28 stays. 40 Newton terms are slow (1.7-2.1 s/frame), which the series ψ below fixes. |
| 2026-10-07 | PROB-09 step 1: are the post-jump tails one long dwell near α, or repeated gate passes? | `python tools/research/misiurewicz/gate_diag.py 12 0.03 300` (90,000 samples of the fundamental ring R0/\|ρ\| ≤ \|w\| < R0; the tail depends only on w) | **Neither, mostly.** Tail steps p50 42, p95 243, max 1409, mean 75. The top 10% of tails (mean 280 steps) spend only 4 steps within 0.05 of α and 16 within 0.1; they circulate 0.2-0.4 from α between α and the 2-cycle, and fall back within 0.03 of the 2-cycle point A about 3 times. A Koenigs jump at α could save at most about 5% of tail steps, so it was not built. **Profile (new `koenigs-loops`/`koenigs-jump` stage-cutoff modes):** the loops take 3-9 ms per 480x270 frame, the approach + jump about 80 ms (60% of the pixel), and the tail 30-80 ms. The jump's Newton ψ averages 8.6 iterations, and 12% of pixels hit the 30-iteration cap because the 1e-16 stop test is at double roundoff. |
| 2026-10-07 | PROB-09: ψ = φ⁻¹ as a series (series reversion of the 40-term φ) instead of Newton | `python tools/research/misiurewicz/psi_series.py 0.03`; bench `PSI=18`; CI `-f psi=18` (480x270 run 37682509050; 1920x1080 run 37682500239) | **Keep.** 18 terms reach a relative error of 2e-16 at \|w\| = 0.03; one Horner replaces about 17 φ evaluations. 480x270, 4 threads: CORRECTNESS PASS, max 1.7e-6 px (was 3.2e-4); fd → pipeline s: 0.509→0.032 (15.9x), 0.839→0.055 (15.3x), 1.367→0.031 (44.6x), 0.678→0.043 (15.9x), 0.725→0.044 (16.6x), 0.781→0.042 (18.6x); 1.5-1.9x faster than the Newton pipeline. **1920x1080: CORRECTNESS PASS, 0 wrong over 12.4M pixels, max 2.8e-5 px** (fixes FIX-20); 16.0-47.7x vs fd (0.49-0.87 s/frame, was 0.92-1.30). With guard 1e-27 it still fails (run 37682504736: 6 wrong at 1e-38). A larger jump radius is not worth it: ψ needs 24 terms at 0.05 and 40 at 0.1, costing about what it saves. |
| 2026-10-07 | PROB-09: tail patch atlas. F_n(w) = f_C^n(A + ψ(w)) is entire in w, unlike ν (PROB-04), so store it exactly: a quadtree over s = log w on the fundamental ring, with a degree-16 Taylor patch per leaf for the largest n that fits | `python tools/research/misiurewicz/tail_patches.py 16 1e-11 6 6 rel CONSTS` (patch error bounded as an s-shift: \|δz\| ≤ 1e-11·\|dF/ds\|; measured \|ds/dpixel\| ≥ 5e-6 at 480x270, so at most 1e-5 px at 1080p); bench reads the tree from the constants; CI `-f psi=18 -f patch=6` | **Keep, modest.** Absolute tolerance 1e-14 stalls at a mean tail of 37 steps whatever the depth; the s-shift tolerance keeps improving. Mean tail steps to \|z\| > 2 (from 69.4; p50 37, p95 225): 596 leaves → 29.5; 16,190 leaves (depth 6, 14 s build, about 4 MB) → 23.6 (p50 2, p95 137); 130,652 leaves (depth 8, 172 s) → 22.0. No invalid leaves. Local 1 thread, 480x270, s/frame, series ψ → + depth-6 patches: 0.067→0.050, 0.122→0.075, 0.064→0.049, 0.092→0.065, 0.094→0.066, 0.087→0.067; every pixel passes. CI 480x270 (run 37683668679, a faster runner): CORRECTNESS PASS, max 2.7e-6 px; 16.3/20.5/48.1/18.7/19.4/20.3x vs fd (series ψ alone: 15.9/15.3/44.6/15.9/16.6/18.6x). The patch atlas is per zone and shared by every frame, the first true atlas win in the DEC-15 sense, though small. Not tried: folding the 23 approach steps and φ into one series in δ (needs 24 terms; saves about 11 operations per pixel). |
| 2026-10-07 | PROB-09 at 1080p on GitHub Actions: does the tail patch atlas (with series ψ) hold on every pixel of full-HD frames, and what does it buy? | `gh workflow run koenigs-bench.yml --ref ishoo/PROB-09 -f size=1920x1080 -f runs=2 -f psi=18 -f patch=6` (run 37683672292) and the same with `-f terms=20` (run 37683675710); 4 threads | **Keep.** CORRECTNESS PASS: 0 wrong pixels, 0 class mismatches over 12.4M pixels, max 7.4e-5 px. Frame s, fd per-frame BLA → pipeline: 8.07→0.41 (19.8x), 13.32→0.56 (23.7x), 21.73→0.40 (54.2x), 10.73→0.48 (22.2x), 11.50→0.51 (22.7x), 12.38→0.52 (23.9x). Against series ψ alone at 1080p (run 37682500239: 0.51/0.87/0.49/0.68/0.70/0.67 s): 1.2-1.6x. Against this morning's Newton pipeline (0.94/1.30/0.92/1.12/1.13/1.11 s): 2.1-2.5x. 20 φ terms change nothing, so 12 stays. |
| 2026-10-07 | BENC-04: head-to-head against the community's BLA renderer. How does our deep pipeline compare with fraktaler-3 3.1 (mathr's successor to Kalles Fraktaler: perturbation + BLA, official Windows build, benchmarked wisdom) on the same machine? | `.github/workflows/f3-bench.yml` → `tools/research/misiurewicz/koenigs_bench/f3_bench.sh` (windows-latest, 4 cores, all three renderers at 4 threads/all cores; fraktaler-3 `--batch` with 20,000 iterations, escape radius 1e10, max perturb/reference/BLA steps 20,000); run 37688451277, 480x270, best of 3 | **Ours is 23-29x faster than fraktaler-3.** Seconds per frame, fraktaler-3 / fd per-frame BLA / ours: 1e-35 0.718/0.598/0.026; 1e-38 0.824/0.939/0.035; 1e-40 0.749/1.492/0.025; 1e-43 0.764/0.764/0.031; 1e-46 0.794/0.821/0.031; 2e-48 0.826/0.879/0.031. fraktaler-3's fixed per-process cost (16x9 frame) is 0.063 s, so the ratio excluding it is 21-27x. **fd's per-frame BLA is as fast as fraktaler-3** (within ±25%, faster at 1e-35, slower at 1e-40), which confirms fd as a fair opponent for all earlier results. **Agreement:** our bench sampled fraktaler-3's own jittered points (its hybrid.h jitter reproduced in Rust; matching variant: frame 0, rows bottom-up). 0 class mismatches on all frames; median disagreement 2-8e-6 px, p99 ≤ 3.3e-5 px. 0-58 points per frame differ by up to 5.4e-3 px as scored with fraktaler-3's DE, but 90-400-digit checks (`truth_f3.py`) show those points have a true distance to the boundary of 1e-9 to 1e-16 px: a last-bit change in the sample position moves ν by hundreds of iterations there, and fraktaler-3's DE is off by 10³-10⁸ at them. They are ill-conditioned measurement points, not errors in either renderer. Our pipeline vs fd on the same runner: CORRECTNESS PASS. |
| 2026-10-07 | BENC-04 at 1920x1080: does the lead over fraktaler-3 hold at full HD, where its 0.07 s process start-up is negligible? | `f3_bench.sh` via f3-bench.yml, run 37689189967 (windows-latest, 4 cores, best of 2) | **Yes: ours is 22-28x faster than fraktaler-3.** Seconds per frame, fraktaler-3 / fd per-frame BLA / ours: 1e-35 12.04/10.63/0.455 (26.5x); 1e-38 14.03/16.83/0.626 (22.4x); 1e-40 12.70/26.62/0.447 (28.4x); 1e-43 12.95/13.78/0.532 (24.3x); 1e-46 13.25/15.26/0.550 (24.1x); 2e-48 13.84/15.86/0.556 (24.9x). Our pipeline vs fd on the same runner: CORRECTNESS PASS, 0 wrong of 12.4M pixels, max 1.7e-4 px. Against fraktaler-3 at its jittered points: 0 class mismatches; 555-5,324 points per frame (0.03-0.26%) exceed 1e-3 px when scored with fraktaler-3's DE (max 3.4e-2). 120-digit checks of the 18 worst points, at the exact double-rounded sample position, show they all lie within 5e-9 px of the boundary (true DE 5e-18 to 5e-9 px). With the true DE, both renderers' displacement errors there are negligible (fraktaler-3 ≤ 4.2e-6 px, ours ≤ 1.6e-7 px); the large values come only from fraktaler-3's DE, which is wrong by orders of magnitude at such points. Follow-up: score with a reliable DE (export ours) rather than fraktaler-3's. |
| 2026-10-07 | PROB-12 seed test: is the v0 zone special? How much of a minibrot's nucleus orbit circles one repelling cycle (the part a Koenigs jump skips)? | `python tools/research/misiurewicz/zone_seed.py 20 1` (about 20 s: the v0 centre, the classic valley centre, 20 random near-boundary points; atom-domain period → nucleus Newton at 120 digits → longest run where the orbit repeats after r steps while repelling, for every r < p/2) | **v0 is the strong case; random minibrots are not.** v0: p 764, 683 steps (89%) within 1e-2 of the 2-cycle from step 24 = **341 laps** (\|λ\| 1.153); D-ranking also recovers q 24, r 2, distance 1.71e-25. Other minibrots (p 37-1601): the longest repeat runs are 0-503 steps, but only 0-4.7 laps of the cycle (best rnd13: 503 steps of a 108-cycle; valley 503 of 429; most < 2 laps). With r ≤ 8 only, every random case had 0 dwell. Reading: laps ≈ ln(minibrot size / distance to the Misiurewicz point)/ln\|λ\|, so the fast path pays off for minibrots parked near a spiral centre (as v0 was chosen, and as spiral-following art zooms do), not for arbitrary minibrots. The D-ranking picks spurious (q ≈ p/2, \|λ\| < 1) relations at random minibrots; the dwell/lap measure is the robust one. Next (PROB-12): measure laps band by band along real zoom paths. |
| 2026-10-07 | PROB-12 seed test: along whole zoom paths, which depth bands have the v0 structure? | `python tools/research/misiurewicz/path_laps.py NAME RE IM -48` (2.1 s for both paths: per 3-decade band, the lowest-period minibrot in view, i.e. smallest p with \|z_p/u_p\| < width; 150-digit Newton; laps = longest repelling-cycle repeat run / r) | **The v0 path has it at every depth from 1e-6 down.** Width → period, laps around the 2-cycle, skippable share: 1e-6 → 143, 40, 55%; 1e-9 → 241, 89, 73%; 1e-12 → 337, 137, 81%; 1e-15 → 435, 186, 85%; 1e-18 → 531, 234, 88%; 1e-21 → 627, 282, 90%; 1e-24 → 731, 332, 91%; 1e-27 to 1e-48 → 764, 342, 89%. Every band's minibrot circles the same r = 2 cycle (M(24,2)), with period growing about 96 per band. So the Koenigs series and the tail patch atlas (per 2-cycle) could serve 45 of the film's 49 decades from one build, with only the minibrot loop map changing per band (PROB-08/09 tested 1e-35 to 2e-48 only). **The classic valley path** (`bench/path-valley.txt` centre, 36 digits): p 35, 78 (0 laps), then p 998 with 1.2 laps from 1e-9 to 1e-15; no minibrot below p 5,000 deeper. It is not aimed at a spiral centre and would gain little. |
| 2026-10-07 | PROB-12 seed test: can shallower v0 bands reuse one fixed-parameter tail (Koenigs + patch atlas built once)? | `python tools/research/misiurewicz/shared_tail.py N` (80-digit truth vs the same orbit with the last T steps run at a shared parameter; error \|Δν\|·ln2·DE in 480-px frames). 1st run: 40 pixels/band, own band nucleus vs C764, T 100/300/600 (86 s). 2nd run: 16 pixels, C764 with and without a first-order c-correction d' = 2zd + (c − C), T 300/600/1000 (53 s) | **The reach of a shared tail shrinks with shallower bands; a first-order c-correction extends it about 1000x where it applies.** Max px error, swap the last T steps: 1e-24: T 100 0, 300 1.6e-11, 600 5.6e-3 (own c_H) / 2.3e-3 (C764). 1e-15: T 100 1.7e-9, 300 2.4e-3 (own) / 1.4e-3 (C764), 600 fails. 1e-9: T 100 3.0e-3 (own) / 1.0e-3 (C764), 300+ fails. The band's own nucleus is no better than C764 (the frame's pixels span the width either way). With the correction: 1e-15 T 300 → 1.2e-6 px (0 of 16 over 1e-3); T ≥ 600 still fails (non-escaping or wrong orbits); 1e-9 fails at every T. Reading: from about 1e-15 down, one tail plus a dF/dc derivative patch per atlas leaf is plausible if the tails there are ≲300 steps (to check); shallower bands need per-band tails or plain BLA, and are cheap anyway. These probes are slow (mpmath); write the next ones in double. |
| 2026-10-07 | PROB-12 seed test (double, < 1 s per band): how long are the tails in shallower v0 bands, and do they fit the ~300-step reach of a shared corrected tail? | `python tools/research/misiurewicz/tail_length.py WIDTH PERIOD 4000` (double perturbation against the band's nucleus, Zhuoran rebasing; tail = steps from the last close return \|z\| < 1e-3 to escape) | **No: a single shared tail covers only the deep end.** Tail steps median / p90 / max, share ≤ 300: 1e-15 (p 435): 465 / 572 / 1281, 0%; 1e-24 (p 731): 750 / 814 / 2178, 0%; 1e-40 (p 764): 420 / 542 / 1269, 0% (but there pixels are within 1e-35 of C, so a fixed-C tail is exact). In the 1e-15 and 1e-24 bands most pixels make no close return at all: they escape within about one period. This corrects FIX-23's "one build could serve 45 of 49 decades". Mid bands need parameter-dependent tails beyond first order (a real research item), or BLA. |
| 2026-10-07 | PROB-12 seed test on a famous published zoom: how much of Maths Town's "Eye of the Universe" (plain Mandelbrot, zoom 3.4e1091, ~17M iterations; location from maths.town/videos/eye-of-the-universe-video) would the spiral-centre Koenigs jump skip? | `python tools/research/misiurewicz/orbit_laps.py N RMAX 4 1e-3 - RCAP` (centre orbit at 1,191 digits, then numpy scan for runs where the orbit repeats after r steps for ≥ 4 laps around a repelling cycle). 20k steps: 1.2 s; 2M steps: 103 s; one minibrot period (160k steps, r ≤ 400 plus record periods below 150k): 8 s | **Modest there: about 24% of steps, against 89% on v0.** Record returns (nested minibrot periods): 5, 64, 197, 655, 2217, 19205, 57190, 102716, 159413. Over 2M steps, 94% of the orbit is 11.5 laps of the centre's own period-159,413 minibrot: the minibrot-loop structure NanoMB already exploits (and our loop map does), not the new spiral jump. Within one minibrot period, repelling short-cycle dwells of ≥ 4 laps cover 24.3% (largest: r 197 for 18.6 laps at \|λ\| 3.3; r 655 for 4.3 laps at \|λ\| 9.5, recurring). Reading: on zoom-doubling dives into nested minibrots the main skip is minibrot loops (prior art), and the Koenigs jump adds at most about 1.3x; the large new win is on spiral-centre dives like v0 (89% of steps, 341 laps). |
| 2026-10-07 | BENC-08 seed test: can the deep pipeline run in single precision (GPU speed)? | `python tools/research/misiurewicz/single_precision.py CONSTS FD_REF` (numpy reimplementation of the koenigs_bench pixel pipeline with a precision switch per stage, scored on all pixels of the six 480x270 PROB-08 frames against fd per-frame BLA; about 0.3-1 s per frame and variant) | **Not as-is, but most of it can be float32, and what must stay float64 is small.** Wrong px (> 1e-3) per frame, 1e-35 / 1e-38 / 1e-40 / 1e-43 / 1e-46 / 2e-48: A all f64 (sanity): 0/0/0 (= Rust bench). B all f32: 20,088 / - / 826 / - / - / 122,893 (median 150 px at 2e-48): fails. F f32 front, f64 finish: same as B, so the front end is the problem. G f32 with f64 glue (h0 = Z24-α + d, φ, log/arg, t): 104 / 17,856 / 53 / 929 / 123,704 / 122,896: the pixel offset (~1e-28) added to a ~1e-23 constant wiped it out in f32. L f64 loops + approach + glue, f32 patch polynomial and finish: 24 / 17,908 / 4 / 885 / 3,161 / 5,202 (median 1e-7 to 2e-4 px): the minibrot loop map needs f64 (its ~1e24-scale terms cancel near the minibrot). P all f64 except the f32 finish: 22 / 16,980 / 4 / 824 / 2,960 / 4,915, about the same as L, so the patch polynomial is fine in f32 and the remaining errors come from the plain escape steps. R as L with the finish as f32 offsets from a per-patch f64 reference orbit: worse (9,860-35,556 wrong; patch centres are too far from their pixels). Reading: float32 is safe for the patch polynomial; the loops (2-5 per pixel), the glue (a few ops) and the escape steps (median 2, long tail) need f64. That is roughly 600 f64 flops per pixel (estimate), so even a 1/64-rate consumer GPU is plausible. Measure it, don't assume it (BENC-08). |
| 2026-10-07 | BENC-08 GPU test: the deep pipeline as an OpenCL kernel on the owner's laptop GPU (RTX 3050 Ti Laptop, 4 GB, consumer NVIDIA with 1/64-rate float64) | `python tools/research/misiurewicz/koenigs_bench/gpu_bench.py CONSTS OUT 1920x1080 5 WIDTHS` (pyopencl, same stages as the Rust bench; PATCH32=1 evaluates the tail patch in float32); scored with compare.py against local `fd control --bla per-frame` 1080p frames | **Correct, and modestly faster than the CPU here.** float64 kernel vs fd: CORRECTNESS PASS at 480x270 (max 2.5e-6 px) and 1920x1080 (0 wrong of 12.4M, max 1.6e-4 px). 1080p ms per frame, GPU float64 / our CPU bench at 20 threads: 1e-35 88/111; 1e-38 103/178; 1e-40 82/127; 1e-43 113/137; 1e-46 119/158; 2e-48 131/152 (1.2-1.7x). fd per-frame BLA on the same laptop: 22.9 s for the six 1080p frames (about 3.8 s each), so the GPU kernel is about 35x faster than fd here. float32 patch (PATCH32): barely faster (79-129 ms) and **fails at 1080p** (7 to 274,523 wrong px, max 3.5 px). This corrects FIX-27: the patch is float32-safe only at 480x270, so the kernel must be all float64. Consumer AMD GPUs usually have a much higher float64 rate relative to float32; the same OpenCL code runs there (DEVICE=AMD). |
| 2026-10-07 | Two quick seeds. (1) Free CPU speed from `-C target-cpu=native`? (2) Do higher-order parameter corrections let the 1e-15 band share the fixed-parameter tail? | (1) koenigs_bench built with and without `RUSTFLAGS="-C target-cpu=native"`, 1920x1080, 20 threads, 6 PROB-08 frames, best of 3, twice. (2) `python tools/research/misiurewicz/shared_tail.py 10 order`: tail at C764 plus Taylor corrections in e = c − C up to order K (d_k recurrences), T 465/700/1000 steps before escape, 10 pixels, 60-digit truth (23 s) | **(1) No gain:** 80-117 ms (default) vs 81-121 ms (native), since scalar complex code is not auto-vectorised. Explicit 4-pixel SIMD is a restructure, not a quick win. **(2) Kill:** at 1e-15 (median escape 481 steps), max px error with K = 1/2/3/4 is ∞ / 1.7e4 / 1.1e4 / 8.3e3, with 8-10 of 10 pixels wrong at every T. The shared parameter's orbit is periodic (a nucleus) while the pixels escape, and a parameter series cannot bridge that over the frame's spread. The fixed-parameter fast path is a deep-band technique; mid bands stay on BLA unless a different construction is found (owner's deep-research question on transfer maps from accelerator physics and astrodynamics). |
| 2026-10-08 | How much of a whole v0 film does the deep fast path speed up? And what does the parameter-transfer deep-research report change? | `bench/benc-01-results.json` depth bins (arm B, per-frame BLA, 960x540): seconds per frame × frames per bin; owner-run report `docs/research/10-8-26/parameter-dependant-transfer-maps-rendering.md` | **About 2.3x on the whole v0 film, today.** Frames 450-750 (width ≤ 1e-29, the fast-path band) take 284 of 478 s (59%); at 20x on the band the film takes 1/(0.41 + 0.59/20) ≈ 2.3x less time (2.4x at 50x). Bins 1e-9 to 1e-29 are another 176 s (37%). With the path_laps skippable shares (73-91%) as caps, covering them would give roughly 8x on the film (an estimate, not measured). **Report:** the failed c ≠ C corrections (K 1-4 at 1e-15) used one monolithic δ-series through the 2-cycle spiral, which amplifies δ geometrically, so the report's literature (domain splitting beats higher order) explains the kill. Its main idea is a parameter-dependent Poincaré chart: λ(c) = 4(c+1) exactly (it reproduces ρ = 1.02683+0.52496i at C), and F_c^m(L_c(w)) = L_c(λ(c)^m w). This is filed as PROB-14 together with the already-passing first-order corrected short tail (T 300). Its direct bivariate patches amount to higher-order BLA (ACC-03); its Taylor-model remainder r' ≤ 2Br + r² + τ goes to PROB-13. |
| 2026-10-08 | KERN-01: the PROB-09 pipeline inside fd (`fd control --zone`), on the real v0 film, on the owner's desktop (Ryzen 9 3900X, 24 threads) | `make_zone.sh data/zones/v0.zone` (14 s); `fd control bench/path-atlas-v0.txt --size 960x540 --iter 100000 --columns nu,de,normal --threads 24 --bla per-frame [--zone v0.zone]`; covered band (frames 431-749, width ≤ 1.71e-28) written both ways and scored with `fd compare`; 6 PROB-08 frames at 1920x1080 | **Keep.** Whole film: per-frame BLA 395.9 s → with --zone 158.6 s (**2.5x**); frames 0-430 unchanged (146 s / 148 s); the 319 band frames 249.8 s → 10.4 s (**24.1x**). 1080p, 6 PROB-08 frames: 18.1 s → 0.80 s (22.6x). Band scoring, 165.4M samples: 0 samples over 1e-3 px (max 1.3e-4 px); 0 escaped/interior disagreements; 1 sample fd escapes at n 99,862 (de 1e-12 px) that the zone leaves Unresolved (it will not start a return that could cross max_iter); 100,123 samples fd leaves Unresolved are classed Interior by the zone's return-map test (fd cannot detect this interior, FIX-04; the escaped sets are identical at 200k iterations). de/normal (1080p): on samples with de > 1e-3 px, de within 0.14% and normals within 0.11°; larger differences only within 1e-3 px of the boundary (ill-conditioned, as in BENC-04). Interior test after adding the q ≤ 4 attracting-cycle Newton check: unresolved at frame 749 fell from 34,002 to 6,129 of 518,400 (left: the minibrot's near-parabolic rim, unresolved in fd too). |
| 2026-10-09 | Does any result in OpenAI's 2026-10-06 math release (openai/math, 372 result families, 719 manuscripts) help fd? And the one testable lead: does a triangular (hex) sample lattice beat the square `--ss` grid at equal samples per pixel (their result 090, universal optimality of the triangular lattice)? | All 372 result statements read; full TeX source of every manuscript searched for about 100 technique terms (Koenigs, Fatou coordinate, parabolic, holomorphic motion, Böttcher, Julia, interval arithmetic, sampling, ...). Probe: `python tools/research/sampling/hex_sample_lattice_probe.py 0.5` (numpy only, about 3 min, not fd): plain double iteration, 320x180, three shallow views (seahorse 4e-3 and 2.5e-5, elephant 1e-3), banded shading at two band densities, Gaussian reconstruction (sigma 0.5 px), scored as RMS shaded-value error against a 100-samples-per-pixel truth, 3 lattice offsets each | **No, and the hex lattice is killed.** RMS error relative to square 4 spp (`--ss 2`): hex 4 spp 0.99-1.01x; jittered 4 spp 1.04-1.07x; hex 3.46 spp (13% fewer samples) 1.07-1.08x; hex 3 spp 1.16-1.17x; square 3.06 spp 1.15-1.17x. Sample arrangement is not a lever: error follows sample count, and the regular grid is already as good as hex and better than jitter. Rest of the release: no manuscript treats z²+c, linearisation, near-parabolic transit or holomorphic motions (the "Mandelbrot" and "Misiurewicz" hits are bibliography entries). Multiplication and Fourier results (109, 130, 107) are dead: the reference is at most 4% of a frame (row 2 above) and the savings are asymptotic only. Crouzeix (325) gives nothing for loops of at most 7 scalar returns; Brennan (072) adds nothing to the per-sample de column. One small keep: 090's verification layout (python-flint Arb checker bound to a manifest) as a template for PROB-13 zone certificates. Limits: shallow views in numpy, not fd and not deep frames. |
| 2026-10-09 | PROB-14 probe A/B: does a Koenigs jump at each pixel's own c (2-cycle in closed form, λ(c) = 4(c+1)) cover the mid bands, where the frozen-C tail failed? And does the owner-run explicit-radius theorem (`docs/research/10-9-26/explicit-radius-parameter-dependent-koenigs-2-cycle.md`) give a usable jump radius? | `python tools/research/misiurewicz/own_c_koenigs.py 250 18 0.03` (mpmath, 70 digits, 2 min): 250 random pixels per width in a 1920x1080 frame at the v0 centre; exact steps to the first close pass of the 2-cycle (step 24), K_c series (18 terms) built at the pixel's c, jump to \|w\| ≤ 0.03, plain steps to escape; error \|Δν\|·ln2·de in px against direct iteration. Three arms: own-c jump + own-c finish; own-c jump + finish at C; jump and finish at C (control) | **Keep (sampled, not whole frames, not double, not timed).** Own-c jump + own-c finish: 0 of 250 over 1e-3 px at every width, max 5e-13 px. Truth steps (median) → steps left: 1e-6 172 → 66; 1e-9 268 → 65; 1e-12 365 → 63; 1e-15 463 → 66; 1e-18 558 → 65; 1e-24 747 → 62 (24 approach + about 40 finish), so the remaining work is flat with depth. Finish at C instead of c: max px 2.7e-1 at 1e-6 (220 of 250 fail), 1.8e-4 at 1e-9, 7.9e-8 at 1e-12, 1.2e-10 at 1e-15, noise below: a shared fixed-C finish (the tail patch atlas) reaches up to about 1e-9. Control (everything at C): 243-249 of 250 fail at every width, which is the PROB-12 shared-tail failure. No minibrot loop map is needed in these bands (pixels escape within one period). Theorem: consistent with the v0 numbers but its certified radius is about 1e-4 against the working 0.03 (about 80 more plain steps per pixel if obeyed); fallback proof only. **Not yet shown:** double precision (u0 = z − p(c) must come from perturbation deltas), cost of building K_c per pixel or per block, whole frames (DEC-17), and timing against `fd --bla per-frame`. |
| 2026-10-09 | PROB-14 probe C: does the mid-band pixel work in IEEE doubles with ONE set of jump constants per zone (the per-pixel series rebuild of probe A was too slow)? And do the numbers in the owner-run answer `docs/research/10-9-26/parameter-taylor-truncation-of-the-koenigs-coefficients.md` hold on v0? | `python tools/research/misiurewicz/shared_consts_double.py 250 18 0.03` (19 s). Zone constants built once at 70 digits and stored as doubles: C's first 24 orbit points, U0 = Z_24 − p(C), √(−3−4C), k_n(C) and dk_n/dc (central difference; the answer's recursion (4) is the production form). Per pixel, doubles only, from dc = c − C: 24 perturbation steps, closed-form cycle shift p(c) − p(C), u0 = U0 + δ − Δp, 3 Newton steps for w0, jump by exp(j·(log λ0 + log1p(4dc/λ0))) to \|w\| ≤ 0.03, one 18-term Horner, plain steps at c. 250 random pixels per width, error \|Δν\|·ln2·de in px (1920-px frame) against 70-digit direct iteration | **Keep (sampled; not whole frames, not timed).** First-order k with exact λ(c): 0 of 250 over 1e-3 px at every width; max px 1.8e-9 (1e-6), 2.7e-11 (1e-9), 5.0e-11 (1e-12), 5.0e-11 (1e-15), 4.5e-11 (1e-18), 7.4e-11 (1e-24); about 64 steps left at every width (24 approach + about 40 finish). Frozen k with exact λ(c): 2 of 250 fail at 1e-6 (max 1.6e-3), passes from 1e-9 down (8.4e-7). Frozen k and frozen λ: 240 of 250 fail at 1e-6, passes from 1e-9 down (7.1e-4 at 1e-9, 5.6e-7 at 1e-12). So measuring the offset from the pixel's own cycle point p(c) is what fixed the PROB-12 shared-tail failure; exact λ(c) and the first-order coefficients are needed only above about 1e-9. Answer check: its first-order Taylor error bounds (≤ 1.92e-19 at r 1e-9, ≤ 1.92e-25 at r 1e-12) match the measured 1.9e-19 and 1.9e-25; frozen coefficients give 3.2e-11 and 3.2e-14. **Not yet shown:** whole frames (DEC-17), timing against `fd --bla per-frame`, the de and normal columns, and handover to the deep path where minibrot returns begin (about 1e-27). |
| 2026-10-09 | PROB-14 probe D: can dz/dc be carried through the Koenigs jump in doubles, so the mid-band fast path produces the de and normal columns? (owner-run answer: `docs/research/10-9-26/derivative-through-the-koenigs-jump.md`) | `python tools/research/misiurewicz/jump_derivative_double.py 250` (19 s). Same zone constants and per-pixel doubles as probe C, plus D0 from the derivative recurrence over the 24 approach steps, the jump derivative, and the recurrence over the finish. Scored on g = z/(dz/dc) at escape (de = 2\|g\|ln\|z\|, normal = arg g) as \|g/g_truth − 1\| against 70-digit direct iteration; 250 random pixels per width. Three arms: formula (2) of the answer; its short form (8) dZ = p′ + λ^j K′(q)(D0 − p′); derivative riding through (dZ = λ^j K′(q) D0) | **Keep formula (2); the ride-through is wrong by a constant 2.7%.** On pixels at least 0.01 px from the set (232-239 of 250 per width), max error: formula (2) 5.3e-8 (1e-6), 3.5e-10 (1e-9), 6.7e-10 (1e-12), 5.2e-10 (1e-15), 1.1e-9 (1e-18), 3.3e-10 (1e-24). Short form (8): 3.1e-4 at 1e-6 (fails, H′(u0) ≠ 1), 3.6e-7 at 1e-9, then equal to (2). Ride-through: 2.7e-2 at every width, the answer's predicted cycle-point term (T_p/T_D = 2.68e-2); ν does not see it, only de and normal do. Pixels closer than about 1e-7 px to the set reach 1e-3 to 2e-2 with every arm (worst 2.2e-2 at de 2.8e-10 px): double position noise over a vanishing de, not the jump. **Not yet shown:** whole frames, timing, fd's own de/normal on the same pixels as the fair comparison for the near-set outliers. |
| 2026-10-09 | Probe E: does the Koenigs jump work at long repelling cycles (r = 197, 655) on a real published zoom, the case where the spiral-centre fast path has nothing? (owner-run answer: `docs/research/10-9-26/general-period-cycle-acceleration.md`) | `python tools/research/misiurewicz/general_cycle_jump.py 30000 197,655` (8 s). "Eye of the Universe" centre orbit, 30,000 steps at 1,191 digits; longest dwell per period (\|z_{n+r} − z_n\| < 1e-3); cycle by Newton on f^r; a_m by truncated composition around the cycle; k_n by the Schröder recurrence; all at 90 digits. Jumped point p + K_N(λ^j w0) against the true orbit at each lap, pass = relative error under 1e-10 | **Mixed: the recurrences are exact, the usable radius is small.** r = 655 (dwell 9249..12059, 4.3 laps, \|λ\| 9.51, entry offset 6.6e-12): one jump covers 3 laps at N 8, 4 at N 16, 5 at N 32 (3,275 steps, out to \|u\| 4.5e-5), error 3e-71 on the inner laps; the whole dwell is skippable. r = 197 (dwell 15786..16902, 5.7 laps, \|λ\| 5.78, entry offset 5.6e-8): 2 of 5.7 laps at every N up to 32 (error 2e-29 at lap 2, 5e-5 at lap 3). Coefficient growth \|a_m\|^(1/(m−1)) up to 2.2e7 and 3.6e9, \|k_n\|^(1/(n−1)) up to 8.1e5 and 3.7e7, so the linear regime is about 1e-6 and 3e-8 wide. Both periods are record returns of this orbit: the cycles sit beside nested minibrots and each lap passes near the critical point. So the "24% in repelling-cycle dwells" of the 2026-10-07 Eye row is mostly nested-minibrot looping: inner laps are exact Koenigs, outer laps need the return-map (biseries) tool. Build 0.1-3 s per cycle. **Not shown:** pixels, doubles (coefficients need rescaling), the phase-transported linearizers, timing, any other location. |
| 2026-10-09 | PROB-14 probe F: which stages of the mid-band pixel survive 32-bit floats (the GPU-native type)? (owner-run answer: `docs/research/10-9-26/binary32-precision-analysis-of-the-mid-band-pixel.md`) | `python tools/research/misiurewicz/float32_stages.py 250` (11 s). The probe C pipeline with each stage's arithmetic forced to float32/complex64 in turn (numpy scalars); the jump either as exp(j·log λ) or from a table of λ0^j built at 70 digits; 250 random pixels at 1e-9, 1e-15, 1e-24; error \|Δν\|·ln2·de in px against 70-digit iteration | **An all-float pixel misses the 1e-3 px bar; a mixed one passes; the finish has to stay double.** Max px (worst width) / pixels over 1e-3: all double 6.6e-11 / 0; stages 1-4 float 6.9e-4 / 0; jump as float exp 3.8e-1 / 181-233; jump from float table 3.9e-4 / 0; float Horner with double add 3.9e-4 / 0; float Horner with float add 5.8e-2 / 44-59; finish in float 9.3e-2 / 135-152; mix (float stages 1-6 with table, double add and finish) 4.2e-3 / 0-2; all float 9.3e-2 / 134-153 (median 1.2e-3; over 1e-2: 0-4 of 250). The answer's stage verdicts match: naive float phase fails, the table fixes it, the float landing add costs about 2e-3 px, and the finish roundoff is amplified about 13x by the weak cycle. The finish is about 40 of the 64 remaining steps, so under the 1e-3 bar most work stays in double; an all-float kernel is a 1e-2 px-class renderer with rare pixels near 0.1 px. **Not shown:** GPU timing of float against double for this kernel, float-float arithmetic for the finish, whole frames. |
| 2026-10-09 | Does an all-float32 mid-band frame look worse than the double one? (follow-up to probe F) | `python tools/research/misiurewicz/f32_frame.py 15` (5 min, 2 processes) then `f32_compare.py nu_15.npy out.png`: the v0 centre at width 1e-15, 160x90, one sample per pixel, every pixel computed twice with the probe C pipeline (all double; all float32 with the λ0^j table), both shaded identically from ν (icy bands at density 0.05 and 0.25 with thin lines) | **No visible difference outside unresolved speckle.** ν difference by region (regions set by the double render's largest step to a 4-neighbour): smooth (step < 0.25, 43% of the frame) median 6.7e-6, max 7.3e-5; moderate (0.25-2, 20%) max 7.9e-4; busy (2-20, 4%) median 1.3e-4, max 0.52; unresolved (> 20, 30%) median 3.7e-3, max 620. Colours off by more than one 8-bit level: 1,404 of 14,400 pixels at density 0.05 and 1,982 at 0.25, nearly all in the speckle, where both renders are pixel noise. Not tested: de/normal shading (slope light), motion, full resolution with `--ss 2`. Owner decision pending: a looser bar for float32 GPU film renders. |
| 2026-10-09 | Probe G: is the v0 minibrot one rung of a ladder of minibrots converging on M(24,2), and can deeper rungs be found from a formula? (owner-run answer: `docs/research/10-9-26/misiurewicz-ladders.md`) | `python tools/research/misiurewicz/ladder.py` (27 s, up to 1,150 digits). m by Newton on z_26 − z_24; the answer's A, B, C checked against Newton nuclei of period 25 + 2n at n = 100, 200, 366; v0's own ladder (period 764 + 2k) from the guess m + (c_0 − m)ρ⁻ᵏ refined by Newton on f^P(0), k = 1, 2, 409, 7676; crude close-return count on 12 pixels per frame at v0 and at k = 409 | **Keep: the ladder is real and v0 is on one.** Answer's ladder: (c_n − m)ρⁿ/A − 1 is 6e-13 at n 200 and 1e-18 at n 366; size/(Bρ⁻²ⁿ) − 1 is 2e-12 and 5e-17; two-term predictor within 5e-5 and 2e-4 minibrot scales at n 100, 200 (one-term: 57, 97); at n 366 the printed 20-digit A is too coarse (4e6 scales). v0 ladder: (c_k − m)ρᵏ agrees across k = 1, 2, 409, 7676 to 22 digits; rungs k → period, distance to m, size: 1 → 766, 1.48e-25, 3.10e-50; 2 → 768, 1.28e-25, 2.33e-50; 409 → 1,582, 8.06e-51, 9.20e-101; 7676 → 16,116, 8.45e-501, 1.01e-1000 (Newton 2-5 steps, 0.4-12 s; sizes on the \|ρ\|⁻²ᵏ law to six digits). Coordinates in `tools/research/misiurewicz/ladder_rungs.txt`. Returns per pixel for frames 5,000 / 50 / 5 sizes wide: 2-3 / 4 / 5-40 at v0, 3-4 / 5-6 / 6-11 at k 409, with direct steps 3,000-4,400 against 7,700-11,700. **Not shown:** a zone built at a new rung and rendered with the fast path (the real flat-cost test); the fitted two-term guess for v0's ladder was poor (Newton still converged), so compute A, C from the answer's limits; other spiral centres. |
| 2026-10-09 | Probe H: is the per-pixel return count flat down a ladder, or does it grow like log2(n) as the owner-run answer `docs/research/10-9-26/universal-landing-claims-a-to-e.md` (eq. B3) says? | `python tools/research/misiurewicz/ladder_bridge.py` (105 s): direct iteration of 6 pixels at the same relative positions in a frame 5,000 minibrot sizes wide, at the v0 rung and at rungs k = 409 and 7676 of its ladder; periods before escape = steps / period | **It grows like log2(n), as predicted.** Periods before escape: v0 (period 764, n 369) 3.88-4.17, mean 4.03; k 409 (period 1,582, n 778, size 9e-101) 4.88-5.18, mean 4.99, change +0.96 against log2(n/n0) = +1.08; k 7676 (period 16,116, n 8,045, size 1e-1000) 8.23-8.56, mean 8.36, change +4.32 against +4.45. So a 1e-1000 destination costs about four more minibrot returns per pixel than v0: logarithmic in the depth exponent, not flat. The answer also rejects the one-lookup "universal landing" formula (the quadratic regime and the linearised regime do not overlap; the last returns need the true return map) and confirms fd's loops + approach + jump + finish as the exact decomposition. Only 6 pixels, one frame width, direct iteration; not the fast path. |
| 2026-10-10 | PROB-14 Rust mid-band whole-frame gate: six v0 widths, 1920x1080, nu/de/normal; exact-coordinate co-moving 2-cycle, 24-step perturbation, first-order Koenigs coefficients with analytic k_n' (Taylor recursion (4)), 3 Newton steps, T_j jump, p' and full jump derivative, plain finish at c. | GitHub Actions [whole-frame gate](https://github.com/junovhs/fractodactyl/actions/runs/38041722639), AMD EPYC 9V45 (4 vCPUs), THREADS=4, RUNS=3, MAXIT=20000, 100-digit mpmath constants built once. `koenigs-bench --mid` writes canonical .fds; `fd compare` scores against `fd control --bla per-frame --kernel fx` on every pixel, to avoid the inaccurate f64-tier shallow control. Timing is against **default** `fd control --bla per-frame` on the same runner; both timing values below are best of three whole-frame runs (fd regenerates reference and BLA per run). Validity/fallback: heuristic only, abs(dc) <= 1e-6, finite Newton and derivative, 1 <= j < 512, abs(q) <= 0.031, budget enough; otherwise full 764-point reference perturbation. At 1e-6, escaped samples whose shortcut de < 1 px also use full perturbation; this guard was added after an unguarded 1080p frame had 1,307 de and 219 normal tolerance failures despite 0 class or nu-px failures (a near-boundary numerical-sensitivity gate; the failing stage was not isolated, and no pointwise bound is claimed). Scored bar: nu 1e-3 px; de <= 0.2% and normal <= 0.2 degrees on reference de > 1e-3 px. Run locally with `cargo build --release && THREADS=24 RUNS=3 SIZE=1920x1080 MAXIT=20000 bash tools/research/misiurewicz/koenigs_bench/run_mid.sh target/release/fd out/prob-14`. | **Keep (whole frames, all six):** 12,441,600 samples, 0 class mismatches, 0 over 1e-3 px, 0 nonfinite, 0 de or normal failures. This is a numerical heuristic validated on these frames, not a certified bound or authorization for fd-kernel integration. |
| 2026-10-10 | PROB-14 mid-band **width 1e-6**, v0, 1920x1080 (2,073,600 samples) | Same-runner default fd BLA 1.7776 s vs mid 1.0616 s (1.67x); mid full-perturbation fallbacks 341,796. Every-pixel `fd compare` against forced-fx per-frame BLA: max nu displacement 4.81e-6 px; max de relative difference 1.0066e-5; max normal angle difference 0.00549 degrees (de/normal scored above 1e-3 px reference de). | **Keep:** 0 class mismatches, 0 nu samples > 1e-3 px, 0 de/normal samples over BENC-04 tolerances, 0 nonfinite values. |
| 2026-10-10 | PROB-14 mid-band **width 1e-9**, v0, 1920x1080 (2,073,600 samples) | Same-runner default fd BLA 2.1961 s vs mid 0.6568 s (3.34x); mid full-perturbation fallbacks 0. Every-pixel `fd compare` against forced-fx per-frame BLA: max nu displacement 3.45e-9 px; max de relative difference 1.1912e-7; max normal angle difference 0 degrees (de/normal scored above 1e-3 px reference de). | **Keep:** 0 class mismatches, 0 nu samples > 1e-3 px, 0 de/normal samples over BENC-04 tolerances, 0 nonfinite values. |
| 2026-10-10 | PROB-14 mid-band **width 1e-12**, v0, 1920x1080 (2,073,600 samples) | Same-runner default fd BLA 3.5579 s vs mid 0.6646 s (5.35x); mid full-perturbation fallbacks 0. Every-pixel `fd compare` against forced-fx per-frame BLA: max nu displacement 9.39e-10 px; max de relative difference 1.1908e-7; max normal angle difference 0.00549 degrees (de/normal scored above 1e-3 px reference de). | **Keep:** 0 class mismatches, 0 nu samples > 1e-3 px, 0 de/normal samples over BENC-04 tolerances, 0 nonfinite values. |
| 2026-10-10 | PROB-14 mid-band **width 1e-15**, v0, 1920x1080 (2,073,600 samples) | Same-runner default fd BLA 5.8930 s vs mid 0.6650 s (8.86x); mid full-perturbation fallbacks 0. Every-pixel `fd compare` against forced-fx per-frame BLA: max nu displacement 6.79e-10 px; max de relative difference 1.1917e-7; max normal angle difference 0.00549 degrees (de/normal scored above 1e-3 px reference de). | **Keep:** 0 class mismatches, 0 nu samples > 1e-3 px, 0 de/normal samples over BENC-04 tolerances, 0 nonfinite values. |
| 2026-10-10 | PROB-14 mid-band **width 1e-18**, v0, 1920x1080 (2,073,600 samples) | Same-runner default fd BLA 5.4356 s vs mid 0.6431 s (8.45x); mid full-perturbation fallbacks 0. Every-pixel `fd compare` against forced-fx per-frame BLA: max nu displacement 2.00e-9 px; max de relative difference 1.1867e-7; max normal angle difference 0 degrees (de/normal scored above 1e-3 px reference de). | **Keep:** 0 class mismatches, 0 nu samples > 1e-3 px, 0 de/normal samples over BENC-04 tolerances, 0 nonfinite values. |
| 2026-10-10 | PROB-14 mid-band **width 1e-24**, v0, 1920x1080 (2,073,600 samples) | Same-runner default fd BLA 6.0060 s vs mid 0.6028 s (9.96x); mid full-perturbation fallbacks 0. Every-pixel `fd compare` against forced-fx per-frame BLA: max nu displacement 9.07e-10 px; max de relative difference 1.1908e-7; max normal angle difference 0 degrees (de/normal scored above 1e-3 px reference de). | **Keep:** 0 class mismatches, 0 nu samples > 1e-3 px, 0 de/normal samples over BENC-04 tolerances, 0 nonfinite values. |

### AUTO-01: guarded automatic finite-cycle compiler (2026-10-10)

Scope: isolated research CLI, branched from main; not `fd --zone`. Base
is `research/auto-misiurewicz-discovery`'s discoverer and native executor.
The earlier 960x540 3-run medians (unguarded, original branch) were **1.4346x**
(c=i) and **1.1677x** (q=3,p=2), with zero class or 1e-3 px failures;
these are *not* evidence for the changed AUTO-01 operator.

**Coordinates (DEC-21):** exact decimal camera centre and width are retained
as strings. The compiler obtains a high-precision reference orbit and cycle
at that exact centre; the native model stores a normalized complex
`bias = z_q - s`, normalized pixel spacing `h_m * 2^h_e`, and the exact
centre metadata. Both prefix and tail iterate reference/phase plus offsets;
no `c0+dc` in f64. A Rust test checks a centre with 1e-400 magnitude and
nonzero sub-f64 offset; another checks a 2^3500 jump multiplier.

**Shortcut guards (DEC-10 and DEC-19):**

| Guard | Rule, fallback | Justification |
|---|---|---|
| State (inverse Koenigs) | `|u|<r/4`, convergent Newton residual, `j>=2`, `|w_out|<=r/2`, finite values and room before max_iter; else phase-reference perturbation | Empirical conservative chart disk; not a certified remainder bound. |
| Parameter | `|dc|<=r/[64(1+|dp|+r|dlam/rho|+sum(|dk_n|r^n))]`, `|dlam dc|<0.01|rho|`; else fallback | First-order parameter sensitivity estimate; empirical domain, not a rigorous error certificate. |
| Derivative consistency | Directly evaluate **one local period** from the chart exit predecessor; compare state with `K(lambda*w)` (relative 1e-7) and `dz/dc` with the differentiated Koenigs identity (relative 2e-4); else fallback | Differentiated conjugacy is an exact mathematical identity; polynomial truncation, phase arithmetic and thresholds are empirical checks, not a proven remainder. |
| Profitability | Before rendering, compare `T_discover+T_build+0.26 us*pixel_count` to `0.50 us*pixel_count` with 5% margin; if it loses, **decline**, no jump | Empirical cost model calibrated from an earlier 960x540 guarded run; machine-dependent, no BLA truth run consulted for a decision. |

The PROB-20 zone first-return truncation gate is *not* used or assumed.
`--force` is a research-only override of the profitability choice. The
BLA control is only a retrospective acceptance check, now scored with
`fd compare` (class, nu, de, normal). An unprofitable camera `c=i`,
width `1e-4`, must log `"pre_render_decision":"decline"` and no jumps.

The one added `.github/workflows/auto-misiurewicz.yml` runs 3 independent
cold 960x540 trials each at `c=i`, q=3,p=2, and a 0.1-width real offset
from each, saving `fd compare` records and reporting speedup median/range.
Local Ryzen 9 3900X commands (24 threads):

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --manifest-path tools/research/misiurewicz/auto_cycle_bench/Cargo.toml
cargo build --release -p fd-cli
cargo build --release --manifest-path tools/research/misiurewicz/auto_cycle_bench/Cargo.toml
python3 -m pip install mpmath numpy
python3 tools/research/misiurewicz/auto_discover.py --fd target/release/fd --native tools/research/misiurewicz/auto_cycle_bench/target/release/native-cycles --re 0 --im 1 --width 1e-95 --size 960x540 --iter 20000 --threads 24 --out out/auto01-i-run1
python3 tools/research/misiurewicz/auto_discover.py --fd target/release/fd --native tools/research/misiurewicz/auto_cycle_bench/target/release/native-cycles --re 0 --im 1 --width 1e-4 --size 960x540 --threads 24 --no-compare --out out/auto01-decline
```

Repeat each successful command independently three times with distinct `--out`
paths for median and min/max. For all four exact target coordinates, see
the workflow's `cameras.txt` generation; no performance measurements from
this branch are claimed until its remote workflow finishes.

**AUTO-01 guarded acceptance measurements** — GitHub Actions
[run 38077355119](https://github.com/junovhs/fractodactyl/actions/runs/38077355119),
Ubuntu hosted 4-vCPU runner, Python 3.12 with mpmath/numpy/gmpy2,
native Rust release, 4 threads, 960x540, width 1e-95, max_iter 20,000,
three independent cold invocations per camera. Each trial counted
discovery + operator build/serialization + native process/render against
same-runner `fd control --bla per-frame` cold seconds. `fd compare`
was only run *after* the pre-render choice and did not influence it.
Off-centre targets shift the real component by +1e-96 (0.1 frame width).
The q=3,p=2 exact centre is written above in the research note.

| Camera | Cold speedup median | 3-run min–max | Max equivalent nu displacement (px) | Largest scored de relative error | Largest scored normal error (deg) |
|---|---:|---:|---:|---:|---:|
| c=i | 1.4151x | 1.3237–1.4172x | 1.54e-11 | 6.06e-8 | 0 |
| c=i, off-centre | 1.2875x | 1.2781–1.2989x | 1.56e-11 | 0 | 0 |
| q=3,p=2 | 1.2745x | 1.2717–1.2775x | 3.73e-11 | 8.21e-8 | 0 |
| q=3,p=2, off-centre | 1.2650x | 1.2603–1.2685x | 9.52e-11 | 0 | 0 |

**Gate:** all 6,220,800 samples across 12 whole frames passed
`fd compare`: 0 classification mismatches, 0 nu samples over 1e-3 px,
0 de tolerance failures and 0 normal tolerance failures, 0 nonfinite.
Native fast-chart table was enabled in all trials, with 518,400 jump
samples each; these tested views had no fallback samples. For nonzero
fallback counts, test a broader corpus under AUTO-02 before production.
At c=i, width 1e-4, the compiler declined prior to any rendering
(no cycle operator build); the separate 64x36 profitability-decline
regression also passed: predicted 0.0767 s total auto vs 0.001152 s
cold BLA, so no rendering occurred and 0 jumps were issued. An earlier, non-optimized guarded
version correctly scored all pixels but lost to BLA (0.58–0.70x medians);
the fast phase-offset finishing path is essential to the win.

### PROB-16: centre-orbit census, first 100k-step run (2026-10-10)

**Measurement:** Owner's 24-thread Ryzen, Python probe **single-threaded**,
39.1 s wall (39.216 s shell `real`), exit 0; `test_census` passed.
Command: `python3 tools/research/misiurewicz/census.py` at
commit `936c9c7`. Centre recurrence computed with mpmath at each row's
declared decimal precision (Eye: 1,171 digits), cast to complex128
*only after* high-precision iteration for observational recurrence scans.
The band figures below are candidate covered shares, not validated
Koenigs or return-map skips. Bands are **iteration ranges**, not zoom
widths. Last band for escaping published centres ends at escape.

**Ten nontrivial zoom centres** (all six periodic/nucleus-centred rows,
plus the four geometric control points, are excluded). `P` gives the
periods observed in the 1001+ bands. A dash means that the orbit escaped
before the band, and `<` escape iteration where relevant.

| Nontrivial centre | 1-100 | 101-1k | 1k-10k | 10k-100k | P (observed) | Escape |
|---|---:|---:|---:|---:|---|---:|
| v0 | 100% | 100% | 100% | 99.9% | 2, 764 | no |
| Eye of the Universe | 0% | 0% | 8.3% | 26.1% | 1, 4, 5, 64, 197, 655 | no |
| famous-deep | 100% | 100% | 96.0% | 94.3% | 78, 998 | no |
| published-1 | 100% | 100% | 76.4% (to 2936) | — | 6, 569 | 2936 |
| published-2 | 100% | 100% | 78.6% (to 6520) | — | 6, 1229 | 6520 |
| published-3 | 100% | 100% | 77.4% (to 6149) | — | 1111 | 6149 |
| published-4 | 30.0% | 36.9% | 69.9% (to 7859) | — | 6, 249 | 7859 |
| published-5 | 100% | 100% | 100% | 98.9% | 9, 1183 | no |
| published-6 | 100% | 100% | 100% | 45.2% (to 13203) | 13, 2449 | 13203 |
| published-7 | 100% | 100% | 97.7% | 94.5% (to 15594) | 31, 594 | 15594 |

Published centres are literal decimals from
[Divetoxx/Mandelbrot](https://github.com/Divetoxx/Mandelbrot);
v0, Eye and famous-deep are taken from this repo's research probes.
Six excluded nucleus/component-centred rows (antenna-minibrot,
Feigenbaum at limited decimal precision, seahorse-p36, two-arm-p24,
elephant-p36, triple-spiral-p48) all measured approximately 99-100%
`minibrot` in the last band: **tautological periodicity, not
representative exterior skip capacity**. Four further excluded controls:
antenna end, cardioid cusp, seahorse root, period-three bulb. The source
for spiral-on-spiral is absent pending FILM-07.

**Dominance restricted to the ten nontrivial rows:** repeated
long-period structures (78, 197, 249, 569, 594, 655, 764, 998, 1111,
1183, 1229, 2449) occur in most zoom orbits, while v0 also has a
strong 2-cycle contribution. These are *candidate recurrence
detections*, and the `minibrot` label relies on proximity to a
record return rather than verification of a return-map operator.
Near-parabolic gates are not dominant in these ten centres.
**PROB-12 order:** (1) generic long-cycle discovery and cycle/return-map
operators, while retaining the working fast 2-cycle spiral path;
(2) nested minibrot return composition, then (3) near-parabolic
gates. Prioritize actual validated whole-frame wins, not coverage alone.

**Eye diagnosis (completed):** Owner's Ryzen diagnostic at commit
`082ce63`, 14.0 s probe (14.103 s shell real), exit 0;
`test_census` passed. In its 10,001-100,000 band, baseline
coverage is 1,088 spiral + 17,288 minibrot + 5,102 gate =
**23,478/90,000 (26.1%)**; the remaining 66,522 (73.9%) are
`other`. The same 100,000-step high-precision centre orbit was
rescanned under four variants:

| Recurrence scan | Classified of 90,000 | Change from baseline |
|---|---:|---:|
| baseline: rmax=256, tol=1e-3, min_laps=4 | 23,478 (26.1%) | — |
| rmax=2048, tol=1e-3, min_laps=4 | 23,478 (26.1%) | +0 |
| rmax=256, tol=1e-2, min_laps=4 | 28,665 (31.9%) | +5,187 |
| rmax=256, tol=1e-3, min_laps=2 | 71,536 (79.5%) | +48,058 |
| rmax=2048, tol=1e-2, min_laps=2 | 79,651 (88.5%) | +56,173 |

**Cause:** the four-lap minimum, not the 256-period scan cap, is
the principal reason for low baseline coverage. The large +48,058
gain from admitting two-lap dwells at unchanged tolerance shows
that much of Eye's apparently unstructured tail consists of short
recurrence dwells (typically two or three laps), rather than long
four-plus-lap stretches. Increasing the cycle period cap alone
adds nothing; relaxing tolerance alone adds only 5,187 steps.
The combined result does not separate interactions between
the changed thresholds. These counts establish recurrence
under the stated heuristics, not certified cycle shadowing.

**PROB-12 implication:** support general long-period cycles and
nested returns, but do not assume that one Koenigs jump yields
a large saving at every deep location. At Eye, many candidate
opportunities span only 2–3 laps: operator discovery,
construction, entry/exit and correctness-guard costs can outweigh
the few iterations saved. Before enabling a jump, require
per-dwell profitability and validity/error/fallback checks, and
otherwise iterate normally. Retain the four-lap baseline for
cross-location comparisons; no classifier output was changed.
The long-cycle-first research order above remains provisional
as a *performance* decision until measured against full frames,
not pending further census runs.

**Validity/fallback (DEC-10/17):** this is a centre-only observation.
Neither approximate recurrence nor a multiplier sign supplies a
Koenigs convergence domain, an error bound, or a general pixel jump.
A later production shortcut must decline outside its validated domain,
fall back to exact iteration, and pass whole-frame every-pixel checks.

### KERN-02: production mid-band handover (2026-10-10; Ryzen full-path gate)

The v0 `make_zone.sh` appends PROB-14's 100-digit `Z_0..Z_24`,
`U0`, `s0`, `lambda0`, `p(C)`, `k_n`, `k_n'` and
`lambda0^j` table as `mid_*` records. The existing `ref` records supply
full-period perturbation fallback. Old zones without `mid_*` are unchanged;
other nuclei do not gain mid constants (PROB-12). Frames use the conventional
return-map path when its frame guard accepts, else the mid operator when the
whole camera lies within 1e-6 of the v0 nucleus and escape radius is 1e10;
otherwise fd takes over. Each individual mid jump checks its own chart state,
parameter offset, multiplier, index and remaining iteration budget, falling back
to a direct perturbation of the pixel's own parameter when declined. The post-jump
finish at `c = C + dc` and the complete `dz/dc` transport follow PROB-14.
The earlier sampled PROB-14 results were not production validation.
The completed 750-frame control and 1080p60 film measurements are recorded
under the round-2 heading below; no numerical error certification is implied.

### KERN-02 review round 1: a guarded mid-band interval (2026-10-10)

Owner's 14-frame 960x540, 100000-iteration, 24-thread subset against fresh
`--bla per-frame`, with `nu,de,normal` and `fd compare`: 11 passed, 3 failed.
Every frame had **0 class mismatches** and **0 nonfinite samples**.
Failing frames before this guard: frame 125 (width 2.37e-8: 112 de over 0.2%,
max 0.31%); frame 150 (5.35e-10: 713 nu over 1e-3 px, max 0.016 px;
20,294 de over 0.2%, max 13.2%; 14,483 normal over 0.2 degrees, max
7.476 degrees); frame 430 (1.99e-28: 4 de over 0.2%, 1 normal over
0.2 degrees, maxima 0.315% / 0.236 degrees). Passing anchors:
frame 100 (1.05e-6, 1.5x), 175 (1.21e-11, 4.2x),
200..400 (5.9-9.6x), and 480 (1.02e-31, 24.2x).
These are **pre-fix owner measurements**. The round-2 owner run of
`scripts/kern02_subset.sh` passed all 14/14 after the guard change.

**DEC-10/17/19 prospective abstention, not a proof of numerical error bounds:**
the mid operator's empirically admitted parameter offsets are
`|dc| <= 7e-12` or `1e-7 <= |dc| <= 1e-6`. Mid jumps outside these
sets return the pixel to unaccelerated reference perturbation.
The whole-frame dispatcher only selects mid when the **outermost**
parameter radius falls in one of these regimes. It also refuses the
mid path when that radius is below `5e-27`, where minibrot returns
need the separate guarded deep operator; if deep declines, fd's
per-frame BLA/plain perturbation renders the frame instead.
This preserves the tested fast-path regime at 175..400 and the broad
outer regime while deliberately sacrificing coverage in the
unvalidated 125/150 annulus and near the deep handover.
The domain is attached to the operator, not hard-coded frame indices,
path depths, or CLI entry points. These cutoffs were chosen from the owner subset and subsequently
checked on the 750-frame v0 path (round 2); they remain **empirical**,
not a PROB-13 certificate, and must not be generalized to new nuclei/paths.

### KERN-02 review round 2: whole-path and 1080p60 measurements (2026-10-10)

**Owner-run hardware and contract:** Ryzen 9 3900X, 24 threads, fresh v0
zone from `make_zone.sh`, `scripts/kern02_subset.sh` **14/14 passed**,
then `THREADS=24 bash scripts/kern02_gate.sh`. Control rendered **all 750**
`bench/path-atlas-v0.txt` frames at **960x540**, 100,000 iterations,
`nu,de,normal`, single run, compared every pixel to independent
`--bla per-frame` (`fd compare` thresholds: 1e-3 px,
0.002 de relative error, 0.2 degrees normal). All 388,800,000
samples were checked. Seconds below are summed single-run control
frame wall timings on the **same machine**, not film end-to-end seconds.

| Path regime | Frames | BLA (s) | KERN-02 route (s) | Speed-up | Actionable failures |
|---|---:|---:|---:|---:|---:|
| Mid `zone-mid-koenigs` | 246 | 98.4996 | 12.6906 | **7.762x** | **0** |
| Deep `zone-koenigs` (existing KERN-01) | 315 | 218.1264 | 8.9516 | **24.367x** | **0** (38 strict-comparator class frames, FIX-04/main below) |
| Outside zone / BLA-perturbation fallback | 189 | 35.3195 | 35.7131 | **0.989x** | **0** |
| **Full 750-frame path** | **750** | **351.9455** | **57.3553** | **6.136x** | **0** under the separately stated existing-baseline exceptions |

**Raw correctness, with no suppressed evidence:** `fd compare`'s
unchanged strict criterion exited **1**, flagging 38 deep-path frames,
indices **712..749**, for **206,603** class differences:
**206,602 `unresolved_to_interior`** (FIX-04: reference BLA runs
out of its 100,000-iteration budget; the existing deep zone identifies
interior) and **1 `escaped_to_unresolved`** at frame **748**.
The owner rendered frame 748 with **main `d051685`** and that main
revision's own zone and reproduced exactly the same 39,900
differences (39,899 `unresolved_to_interior` and the one
`escaped_to_unresolved`) against the very same BLA .fds.
Therefore the one escaping mismatch is an explicitly documented,
**pre-existing main-baseline discrepancy**, not an effect introduced
by KERN-02. There were **0 other class changes**, **0 nonfinite
samples**, and **0 samples exceeding the ν/de/normal tolerances**
over the full path. Maximum reported ν displacement was
**7.0591e-5 px**; maximum scored de relative difference was
**8.5425e-4**. The full raw comparator results remain in
`out/kern02/compare.jsonl`. This distinction is **not** a claim that
strict `fd compare` passes or that FIX-04 is resolved.

**Gate policy:** `scripts/kern02_gate.sh` continues to retain the
unmodified raw comparison; it separately reports the FIX-04
`unresolved_to_interior` counts only on `zone-koenigs/` deep
frames, exempts **at most one** `escaped_to_unresolved`
sample **only at deep frame 748**, and fails on every other
classification change, missing/corrupt frame, nonfinite sample
or ν/de/normal tolerance breach. The production kernel and the
strict `fd compare` executable are unchanged. In particular,
the deep-only exemption does **not** silently admit these classes
in the KERN-02 mid operator.

**Empirical handover (DEC-17/19, not certified by PROB-13):**
the mid co-moving jump abstains per pixel for parameter offsets
`7e-12 < |dc| < 1e-7`, reverting to full reference perturbation.
It also declines frame routing when the camera's maximum offset
falls below `5e-27`, so the separately guarded deep return
operator takes over if eligible, else the normal BLA/plain
fallback. Other cameras outside the admitted zone also fall
back. These validity islands are empirical for v0 and should not
be interpreted as general error bounds or extrapolated to
different nuclei.

**1080p60 film (production `fd film --keyframes 2`):**
a **25 s / 1,500-frame** v0 zone film, 1920x1080, 60 fps,
100,000 iterations, completed in **655.096 s wall**, with
**209.966 s rendering** (122 zoned keyframes, 7 BLA, 36
perturbation), **436.572 s shading**, and **4.985 s encode wait**.
Shading is now the dominant stage; see **FX-05**. No complete
zone-free 1,500-frame film was run, so **6.136x** above applies
to the 960x540 control path, **not** 1080p60 end-to-end film.

**Matched deep keyframe clip** (`--frames 1200..1250`, 50 output
frames, 7 rendered keyframes, 1920x1080):
per-frame BLA needed **123.112 s render**, **140.364 s wall**;
the zone needed **3.113 s render**, **21.585 s wall**.
That is **39.5x render speed-up** and about **6.50x end-to-end
wall speed-up** for this clip (including shading), not a claim
about the complete film.


### AUTO-02: blind structural-compiler corpus (2026-10-10)

**Frozen operator:** AUTO-01 landed code commit `e42ca0baae4bd2434330f96ca267d2274cd255ef`; AUTO-02 branched at main `da3f9cb95df2c577119b15bbc3c4a763cd37223e`. The discoverer and native executor are unchanged, and the corpus runner checks their Git blob hashes before testing. This is not an `fd --zone` integration. Corpus: `tools/research/misiurewicz/auto_corpus.txt` (18 precommitted off-centre views); 14 from PROB-16 census / FEATURE-MAP, four genuinely held-out new widths/offsets at AUTO-01's Misiurewicz calibration centres. All real/imag/width fields are literal exact decimals, never converted through f64. No view is an exact seed centre. The spread includes near 1e-10 through 1e-360, sampled repelling periods and potential interior/escaped/unresolved mixes; actual cycle and class detections are recorded by the gate, not presumed.

**Protocol:** `.github/workflows/auto-corpus.yml` runs Rust workspace clippy/test, the frozen native tests, then one independent cold 960x540, 20,000-iteration trial per row on the same runner with four threads. Compiler makes its pre-render decision without BLA truth. A jump renders the native candidate and a cold `fd control --bla per-frame` reference and `fd compare`s every pixel in class/nu/de/normal (DEC-17). A decline spends its discovery/build cost and renders the untouched fd baseline; no correctness credit is assigned for an unattempted jump. The runner records discovery/build/native render/cold BLA seconds, jumped-pixel coverage, fallback fraction, baseline E/I/U classes, all comparison failures, and mpmath direct orbit/derivative checks at the worst class/nu/de/normal pixels. Scoring de/normal near tiny distances follows BENC-04 (reference de >= 1e-3 px). The first wrong jump halts the corpus (DEC-14); no retuning on these views is allowed.

**Measured blind result — gate FAIL (no demonstrated speedup).**
[GitHub Actions cold run 38094065953](https://github.com/junovhs/fractodactyl/actions/runs/38094065953),
Ubuntu hosted 4-vCPU, Python 3.12, 4 Rust threads, 960x540, max_iter 20,000.
The frozen AUTO-01 discoverer and native executor passed their Git blob guards,
workspace and native Rust tests/Clippy passed, and all 18 cold baseline runs completed.
Each view was a single cold trial, not a median. `fd compare` and the direct
mpmath spotcheck are **N/A**, not zero-error confirmations: no view jumped.

| View | Width | Source | Decision (reason) | q/p | Discover s | Build s | Render s (fd) | Cold BLA s | Net BLA/total | Jump/fallback | Baseline E/I/U | Correctness |
|---|---:|---|---|---|---:|---:|---:|---:|---:|---|---|---|
| v0-near | 1e-10 | PROB-16 | decline (no-cycle) | -/- | 0.078 | 0 | 0.834 | 0.834 | 0.91x | 0%/100% | 518400/0/0 | abstained; no candidate |
| v0-deep | 1e-85 | PROB-16 | decline (no-recurrence) | -/- | 0.079 | 0 | 3.042 | 3.042 | 0.97x | 0%/100% | 0/0/518400 | abstained; no candidate |
| eye-ultradeep | 1e-320 | PROB-16 | decline (no-recurrence) | -/- | 0.084 | 0 | 99.323 | 99.323 | 1.00x | 0%/100% | 0/0/518400 | abstained; no candidate |
| famous-deep | 1e-28 | PROB-16 | decline (no-recurrence) | -/- | 0.078 | 0 | 20.941 | 20.941 | 1.00x | 0%/100% | 0/0/518400 | abstained; no candidate |
| published-1 | 1e-12 | PROB-16 | decline (no-recurrence) | -/- | 0.083 | 0 | 0.364 | 0.364 | 0.81x | 0%/100% | 518400/0/0 | abstained; no candidate |
| published-3 | 1e-70 | PROB-16 | decline (no-recurrence) | -/- | 0.080 | 0 | 1.058 | 1.058 | 0.93x | 0%/100% | 518400/0/0 | abstained; no candidate |
| published-4 | 1e-125 | PROB-16 | decline (no-recurrence) | -/- | 0.081 | 0 | 0.453 | 0.453 | 0.85x | 0%/100% | 518400/0/0 | abstained; no candidate |
| published-6 | 1e-200 | PROB-16 | decline (no-recurrence) | -/- | 0.083 | 0 | 0.752 | 0.752 | 0.90x | 0%/100% | 518400/0/0 | abstained; no candidate |
| published-7 | 1e-340 | PROB-16 | decline (no-recurrence) | -/- | 0.088 | 0 | 69.086 | 69.086 | 1.00x | 0%/100% | 518400/0/0 | abstained; no candidate |
| seahorse-valley | 1e-10 | FEATURE-MAP | decline (no-recurrence) | -/- | 0.009 | 0 | 0.095 | 0.095 | 0.91x | 0%/100% | 518400/0/0 | abstained; no candidate |
| two-arm-spiral | 1e-14 | FEATURE-MAP | decline (no-recurrence) | -/- | 0.078 | 0 | 1.098 | 1.098 | 0.93x | 0%/100% | 518400/0/0 | abstained; no candidate |
| elephant-valley | 1e-11 | FEATURE-MAP | decline (no-recurrence) | -/- | 0.078 | 0 | 0.480 | 0.480 | 0.86x | 0%/100% | 518400/0/0 | abstained; no candidate |
| triple-spiral | 1e-10 | FEATURE-MAP | decline (no-recurrence) | -/- | 0.016 | 0 | 0.124 | 0.124 | 0.89x | 0%/100% | 518400/0/0 | abstained; no candidate |
| antenna-tip | 1e-315 | FEATURE-MAP | decline (no-cycle) | -/- | 0.087 | 0 | 2.033 | 2.033 | 0.96x | 0%/100% | 518400/0/0 | abstained; no candidate |
| i-medium | 1e-40 | AUTO-01 new width | decline (savings) | 2/2 | 0.059 | 0 | 0.335 | 0.335 | 0.85x | 0%/100% | 518400/0/0 | abstained; no candidate |
| i-ultradeep | 1e-360 | AUTO-01 new width | decline (cost-model) | 2/2 | 0.090 | 0.047 | 3.642 | 3.642 | 0.96x | 0%/100% | 518400/0/0 | abstained; no candidate |
| q32-deep | 1e-75 | AUTO-01 new width | decline (cost-model) | 3/2 | 0.081 | 0.040 | 0.356 | 0.356 | 0.75x | 0%/100% | 518400/0/0 | abstained; no candidate |
| q32-ultradeep | 1e-310 | AUTO-01 new width | decline (no-recurrence) | -/- | 0.084 | 0 | 1.360 | 1.360 | 0.94x | 0%/100% | 518400/0/0 | abstained; no candidate |

**Decision audit:** 18/18 honest abstentions, no accelerated views, no jumped
pixels, no per-pixel fallback from a jumped operator, no rejected whole-frame
`fd compare` results, and no oracle checks (there was nothing to spotcheck).
`no-recurrence` means no critical-orbit landing at the camera width;
`no-cycle` means no numerically usable primitive repelling cycle;
`savings` rejected the optimistic operator step-savings bound;
`cost-model` rejected pre-render discovery+build+native cost against estimated BLA.
Two compiler attempts built an operator (i-ultradeep: 0.047 s;
q32-deep: 0.040 s), then declined. The reported net ratio includes
abstention overhead; every view is slower than its standalone BLA baseline.
All 18 baseline frames were homogeneous (escaped or unresolved); none
contained a verified interior sample. Therefore this first blind corpus
does **not** satisfy the issue's requested mixed-class coverage or meaningful
share of net wins. It is **not** evidence to promote AUTO-01 (DEC-17).
Do not retune AUTO-01 against these observed cameras and call a rerun blind;
a subsequent test requires a fresh, precommitted unused corpus and/or a new
compiler revision. The original 18-view corpus and failed verdict are retained.

Machine-readable per-view decisions, class counts, costs and full logs:
[auto02-blind-results artifact, run 38094065953](https://github.com/junovhs/fractodactyl/actions/runs/38094065953).

Local replay from repo root (fresh `out/auto02-local` directory):

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --manifest-path tools/research/misiurewicz/auto_cycle_bench/Cargo.toml
cargo build --release -p fd-cli
cargo build --release --manifest-path tools/research/misiurewicz/auto_cycle_bench/Cargo.toml
python3 -m unittest discover -s tools/research/misiurewicz -p test_auto_corpus.py
python3 tools/research/misiurewicz/auto_corpus.py --fd target/release/fd --native tools/research/misiurewicz/auto_cycle_bench/target/release/native-cycles --size 960x540 --iter 20000 --threads 24 --out out/auto02-local
```

### AUTO-02 review round 2: rescue and blind admission (2026-10-10)

Run 1 above is preserved, including its original verdict. The 15 all-escaped
views are legitimate abstentions. The three 100%-Unresolved views are **invalid
baselines for a speed gate**, not evidence of compiler failure. The earlier
blanket gate FAIL interpretation was premature. Published-7 is an expensive
valid escaped-only decline (69.086 s at 960x540 on Actions).

**Iteration rescue rule, frozen before measurements:** for each of the three
originally blank cameras, rerender one cold fd per-frame BLA at max_iter
20,000, 40,000, 80,000, 160,000, 320,000, 640,000, 1,000,000 in order.
Stop at the first limit with strictly fewer than 5,184 unresolved pixels of
518,400 (<1%), or declare **capped unresolved** after the 1M limit. Record
every attempted budget and class counts. Use exactly the qualifying budget
for the subsequent compiler/BLA trial. This rule is identical for all three;
it does not consult or retune AUTO-01.

**Positive controls, excluded from the blind verdict:**
auto02_controls.txt contains AUTO-01's c=i, c=i+1e-96 and q=3/p=2
development views (width 1e-95, 20,000 iterations). Initial 960x540
[run 38094795618](https://github.com/junovhs/fractodactyl/actions/runs/38094795618)
proved one real c=i jump: 518,400 jumping pixels, zero every-pixel
class/nu/DE/normal fd compare errors; three worst pixels passed independent
mpmath direct iteration (worst candidate nu 7.4e-12 px, DE 1e-15 relative,
normal 3e-16 rad), and 0.247 s compiler total vs 0.361 s BLA (1.46x).
The other two controls **declined** at 960x540 on the frozen cost guard.
A 1920x1080 repeat in auto02-controls.yml checks whether longer frames
amortize without --force or compiler changes; see its separate run artifact.

**Completed positive-control gate:**
[run 38094927806](https://github.com/junovhs/fractodactyl/actions/runs/38094927806)
at 1920x1080, four threads, 20,000 iterations, three independent
cold control frames. All three pre-render decisions were accelerate,
and 2,073,600/2,073,600 samples jumped in each frame (0 fallback).
`fd compare` scored all 6,220,800 pixels: zero class mismatches and
zero nu/DE/normal tolerance failures. Three worst pixels per frame
were independently checked with direct high-precision orbit/derivative;
max candidate nu-equivalent displacement 5.925e-11 px, DE relative
1.036e-8, normal 2.014e-9 rad; no oracle class error.

| Calibration | Discover s | Build s | Native render s | Total s | Cold BLA s | Net win | Jump coverage |
|---|---:|---:|---:|---:|---:|---:|---:|
| c=i | 0.062 | 0.039 | 0.590 | 0.691 | 1.416 | 2.05x | 100% |
| c=i+1e-96 | 0.083 | 0.040 | 0.580 | 0.702 | 1.421 | 2.02x | 100% |
| q=3/p=2 | 0.080 | 0.040 | 0.586 | 0.706 | 1.397 | 1.98x | 100% |

Controls demonstrate the harness can reach the jump + whole-frame
comparator + direct oracle without --force; they do not contribute
to blind speed-win percentages.
**New corpus selection is baseline-only.** 24 new off-centre exact-decimal
views were precommitted in auto02_candidates.txt *before any compiler use*.
A single cold 960x540, 100,000-iteration fd per-frame BLA run tests
each. Admission requires escaped >0, interior >0, Unresolved <1%, and
cold BLA >=5 seconds. Select the first at most 10 in committed input
order, with at least 6 needed to constitute the requested fresh corpus.
auto02_precheck.py uses shared parsing helpers but invokes no compiler. The admitted
subset and the complete rejected precheck evidence must be committed
**before** running the compiler on these candidates. If too few pass,
report inadequate selection, not compiler failure; never silently
relax thresholds or call all-Unresolved frames expensive workloads.

**Baseline-only first screen result:**
[Actions run 38094793669](https://github.com/junovhs/fractodactyl/actions/runs/38094793669)
completed the 24 precommitted views in auto02_candidates.txt.
**0/24 admitted**, without ever calling the compiler: every reference
had zero Interior pixels (v0 minibrot neighbourhoods had significant
Unresolved fractions and some entirely Unresolved frames; the famous
deep views were entirely or almost entirely Escaped). Cold BLA costs
ranged 1.099..44.170 s. The screen exited 2 because its pre-registered
minimum of six was not met, not because of an acceleration failure.
Full class/cost table and JSON in the run's auto02-admission-only
artifact, retained as baseline-only negative evidence.

**Second preregistered screen, still before any new compiler trial:**
auto02_candidates_roots.txt adds 24 completely unused off-centre
attachment-root views at periods 1..8, widths 1e-5..1e-7.
The exact roots c=lambda/2-lambda^2/4 (lambda=e^(2*pi*i/p)) are from
the FEATURE-MAP notion of cardioid attachments. Selection is still
the **unchanged** E>0, I>0, U<1%, and BLA>=5s rule at 100,000
iterations. auto02-precheck.yml emits a separate complete report and
proposed admission list for this pool; it cannot run the compiler.
If six pass, commit the list and its baseline evidence before invoking
auto02-trial.yml. If fewer pass, preserve another selection failure.
**Gate after admission:** same committed camera and iteration budget for
frozen compiler and cold BLA; per-view decision, discover/build/render
seconds, BLA cold seconds, operator coverage/fallback, class distribution
and every-pixel fd compare (class, nu, DE, normal), with mpmath orbit
and derivative checks at worst pixels. Any wrong jumped pixel is a
DEC-14 stop. Net wins require total discovery+build+render below cold
BLA. Report wins/valid decisions, including the exact denominator of
expensive admitted views; exclude calibration controls, exclude capped
Unresolved frames. No run-2 acceleration verdict is claimed while
the baseline admission and capped rescue remain outstanding.

Owner Ryzen command for the costly three-view rescue (from repo root):

```bash
cargo build --release -p fd-cli
cargo build --release --manifest-path tools/research/misiurewicz/auto_cycle_bench/Cargo.toml
python3 tools/research/misiurewicz/auto02_precheck.py --mode rescue --views tools/research/misiurewicz/auto02_rescue.txt --fd target/release/fd --threads 24 --out out/auto02-rescue-owner
```

If the rescue writes a nonempty out/auto02-rescue-owner/admitted.txt,
the exact next scoring command is:

```bash
python3 tools/research/misiurewicz/auto_corpus.py --subset --corpus out/auto02-rescue-owner/admitted.txt --fd target/release/fd --native tools/research/misiurewicz/auto_cycle_bench/target/release/native-cycles --threads 24 --out out/auto02-rescue-scored
```

Archive the generated precheck.json, precheck.md, summary.md and report.json
rather than copying only aggregate counts; operator code is unchanged.

#### AUTO-02 run 2: fresh minibrot pool (local, 2026-10-10, owner took over from web mode)

**Why a new pool.** Both of ChatGPT's fresh pools (`auto02_candidates.txt`, `auto02_candidates_roots.txt`) admitted 0 views. The admission rule required Unresolved < 1%. Deep minibrot interior is exactly what per-frame BLA leaves Unresolved (FIX-04): 2-view probe mb-famous-deep-p998 gave E/I/U 452584/26941/38875 at 26.1 s BLA. So the rule excluded every expensive mixed view by construction. Root-pool BLA-only admission locally: 0/11 finished views admitted (all-interior or mostly Unresolved near-parabolic roots), stopped there.

**Pool rule (fixed before any compiler run):** `tools/research/misiurewicz/auto02_minibrots.py`. For each of the 14 PROB-16/FEATURE-MAP run-1 seeds, take the last three atom-domain partial periods (<= 20000) at the seed centre. Newton-refine the nucleus in mpmath (DEC-21), estimate size = 1/(beta*lambda^2), and frame width = 4*size, offset (+0.2w, -0.15w). Result: 36 views, `auto02_minibrot_pool.txt`. Admission is relaxed to E>0 and I+U>0, with U reported as FIX-04, not as error. Views with width >= 4e-5 (periods 1-3 and 18/26: bulbs and shallow minibrots, BLA well under 5 s) were skipped as unable to be expensive.

**Smallest test.** The compiler decides before rendering (0.1-0.2 s per view). Only an `accelerate` decision needs a full render plus every-pixel `fd compare`. So the decision ran on every deep view first, and BLA renders were kept only where already measured. Frozen AUTO-01 `e42ca0b`, `auto_discover.py` at 960x540, 100000 iterations, Ryzen 9 3900X, 24 threads.

| View | Width | Decision | Reason | BLA E/I/U | BLA s |
|---|---:|---|---|---|---:|
| mb-v0-near-p1 | 4 | decline | even optimistic operator savings cannot amortize the jump | 439085/79311/4 | 0.029 |
| mb-v0-near-p2 | 2 | decline | no numerically usable primitive repelling cycle | 254312/264087/1 | 0.066 |
| mb-v0-deep-p2 | 2 | decline | no numerically usable primitive repelling cycle | 254312/264087/1 | 0.061 |
| mb-v0-deep-p741 | 6.96e-49 | decline | no critical-orbit recurrence at camera scale | 429807/0/88593 | 1.609 |
| mb-v0-deep-p764 | 1.65e-49 | decline | no critical-orbit recurrence at camera scale | 447036/0/71364 | 1.691 |
| mb-eye-ultradeep-p655 | 2.92e-20 | decline | no critical-orbit recurrence at camera scale | 430054/0/88346 | 20.104 |
| mb-eye-ultradeep-p2217 | 1.58e-36 | decline | no critical-orbit recurrence at camera scale | 439488/0/78912 | 3.662 |
| mb-eye-ultradeep-p19205 | 7.75e-163 | decline | no critical-orbit recurrence at camera scale | 0/0/518400 | 0.640 |
| mb-famous-deep-p78 | 4.38e-06 | decline | weakly repelling multiplier outside validated derivative-jump domain | 363322/153631/1447 | 1.366 |
| mb-famous-deep-p998 | 2.5e-15 | decline | no critical-orbit recurrence at camera scale | - | - |
| mb-famous-deep-p8007 | 3.58e-32 | decline | no critical-orbit recurrence at camera scale | - | - |
| mb-published-1-p1 | 4 | decline | even optimistic operator savings cannot amortize the jump | - | - |
| mb-published-1-p2 | 2 | skipped | shallow (width >= 4e-5) | - | - |
| mb-published-1-p3 | 0.0761 | skipped | shallow (width >= 4e-5) | - | - |
| mb-published-3-p329 | 1.61e-30 | decline | no critical-orbit recurrence at camera scale | - | - |
| mb-published-3-p437 | 6.75e-33 | decline | no critical-orbit recurrence at camera scale | - | - |
| mb-published-3-p1111 | 1.76e-59 | decline | no critical-orbit recurrence at camera scale | - | - |
| mb-published-4-p848 | 6.87e-60 | decline | no critical-orbit recurrence at camera scale | - | - |
| mb-published-4-p1595 | 1.15e-80 | decline | no critical-orbit recurrence at camera scale | - | - |
| mb-published-4-p2443 | 1.32e-93 | decline | no critical-orbit recurrence at camera scale | - | - |
| mb-published-6-p26 | 0.000114 | skipped | shallow (width >= 4e-5) | - | - |
| mb-published-6-p495 | 6.99e-27 | decline | no critical-orbit recurrence at camera scale | - | - |
| mb-published-6-p2449 | 1.41e-61 | decline | no critical-orbit recurrence at camera scale | - | - |
| mb-published-7-p93 | 3.02e-25 | decline | no numerically usable primitive repelling cycle | - | - |
| mb-published-7-p594 | 3.39e-77 | decline | no critical-orbit recurrence at camera scale | - | - |
| mb-published-7-p3740 | 1.12e-162 | decline | no critical-orbit recurrence at camera scale | - | - |
| mb-seahorse-valley-p1 | 4 | skipped | shallow (width >= 4e-5) | - | - |
| mb-seahorse-valley-p2 | 2 | skipped | shallow (width >= 4e-5) | - | - |
| mb-two-arm-spiral-p1 | 4 | skipped | shallow (width >= 4e-5) | - | - |
| mb-two-arm-spiral-p2 | 2 | skipped | shallow (width >= 4e-5) | - | - |
| mb-two-arm-spiral-p349 | 5.33e-17 | decline | no critical-orbit recurrence at camera scale | - | - |
| mb-elephant-valley-p18 | 0.00445 | skipped | shallow (width >= 4e-5) | - | - |
| mb-elephant-valley-p74 | 2.72e-07 | decline | weakly repelling multiplier outside validated derivative-jump domain | - | - |
| mb-elephant-valley-p112 | 1.95e-09 | CRASH | compiler found a cycle candidate, then crashed in write_native_model: TypeError (mpf phase in phases_hp) | - | - |
| mb-triple-spiral-p1 | 4 | skipped | shallow (width >= 4e-5) | - | - |
| mb-triple-spiral-p3 | 0.756 | skipped | shallow (width >= 4e-5) | - | - |
| mb-antenna-tip-p1 | 4 | skipped | shallow (width >= 4e-5) | - | - |

**Verdict (AUTO-02 gate: FAIL on speed, no wrong pixel).**
- Run 1: 15 valid views plus 2 rescued (famous-deep at 40k, eye-ultradeep at 80k iterations), all honest declines. v0-deep is invalid: the frame lies inside the period-764 minibrot (1e-85 view, 4e-50 minibrot), so it is all interior that BLA cannot prove.
- Run 2: 22 deep minibrot views (width < 4e-5). 21 declined: 18 "no critical-orbit recurrence at camera scale", 1 "no usable repelling cycle", 2 "weakly repelling multiplier outside validated domain". 1 crashed (mb-elephant-valley-p112). Of the 15 shallow views, 4 had already been decided before the skip rule was applied (all declines) and 11 were skipped.
- 0 jumps on 39 valid blind views (38 declines, 1 crash), so 0 net wins and no per-pixel correctness failure to score. The controls (3 AUTO-01 calibration views) still jump at about 2x with every-pixel compare passing, so the harness can produce a jump.
- The one blind cycle candidate hit a compiler bug, not a cost-model decline. That is a robustness failure on the accept path (FIX-47). It must decline or work, never crash. Do not retune on these views; a fixed compiler needs a fresh pool from the same rule with new seeds.
- Reading: the frozen compiler's precondition (critical-orbit recurrence at camera scale) holds almost only at the Misiurewicz-type spiral centres it was calibrated on, even next to expensive minibrots. This matches the 2026-10-08 assessment that the dynamics fast path pays off on paths aimed at spiral centres. PROB-12 should build zones for spiral-centre paths explicitly, not rely on AUTO-01 to find structure in arbitrary views.

