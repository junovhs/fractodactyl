# Growing-order series initializer on real PROB-22 frames (PROB-41)

2026-10-20, Pacific time. A research probe that feeds FIX-09's go/no-go. The question:
on the frames Tim's films actually render, does PROB-40's growing-order parameter series
(order 8→16→32→64, carried remainder) beat plain perturbation and today's per-frame BLA
cold, with setup counted? And is the output pixel-correct?

**Answer.** It is correct and fast where pixels do moderate work, but it reaches 3x in
only one bin. With the final guard, the initializer passes `fd compare` on all 15 frames
of the three shallow bins. There it is 3.33x, 2.43x and 1.73x faster than plain cold, and
2.1–5.2x faster than today's BLA. In the two deep bins it is only 1.31x and 2.03x faster
than plain, and it flips 78 interior samples to unresolved. One frame-wide series stops
at the same iterate (845) on nearly every frame, so its share of the work shrinks as
depth grows. **FIX-09 cannot reach 3x per bin with the initializer alone; staged LA is
still needed after it.** Go/no-go is at the end.

## Frames

`freeze_frames.py` reruns the PROB-22 planner (`tools/research/variety/decades.py`,
`MODE=target` / `plan2`, default Seahorse start -0.75+0.1i, width 0.5, 24 candidates,
seed 1, at least 20 decades). It reproduced the recorded PROB-22 result: a period-2535
nucleus of size 7.6e-29, 27 decades, planned in 34.6 s. It then freezes FIX-09's 24
views on the straight zoom into that nucleus: five 3-decade bins over 1e-14..1e-29,
5+5+5+5+4 frames at log-even depths, the same rule as the FIX-09 branch's harness.
The views are 480x270, ss 1, 100,000 iterations, columns nu,de, and they are in
`results/frames.txt`. These are not the path-atlas-v0 frames.

Because every frame is centred on the nucleus, the deepest bin looks into the minibrot
itself: 1.3e-28 and 2.4e-29 are mostly unresolved samples at 100,000 iterations. Those
two frames dominate the deep bin's seconds.

## What runs

The arms run in `probe/` (`series-probe`, built with the fd workspace's release profile).
Each arm renders every frame cold. Each run builds its own reference, table and series
inside its own clock and shares nothing. There are 3 runs per frame, the arm order
rotates every run, the median is reported, and 8 threads are used.

| Arm | Setup in the clock | Per-pixel path |
|---|---|---|
| `plain` | reference orbit | `render_with` (fd control's plain arm) |
| `bla` | reference + BLA table at fd's eps 2^-50 for the frame's largest \|dc\| | `render_bla`, or plain if the table is empty (fd control `--bla per-frame`) |
| `series` | reference + growing-order series | series start, then the plain kernel |
| `series-bla` | reference + series + BLA table | series start, then BLA blocks |
| `series-k64` | reference + fixed order-64 series (no growth) | series start, then plain |

The series continues **the existing kernel loop**, not a copy of it. `fd-kernel`'s
`iterate` gained a start state `(n, delta, dz/dc)`. With the default zero start it is
byte-identical to before: the precision-tier byte tests and all 38 fd-kernel tests pass.
`crates/fd-kernel/src/series.rs` holds the series. `render_series` exists only for this
probe; no fd command calls it.

## Series and guard (DEC-10 / DEC-19 contract)

With dc = rho x and |x| ≤ 1 over the frame (rho = the frame's largest |dc|), the series
uses a **general complex reference** (the frame's own f64 or fixed-point orbit):

    b[1]' = 2 Z b[1] + rho,   b[k]' = 2 Z b[k] + sum(i+j=k) b[i] b[j]
    E' = 2(|Z|+M)E + E^2 + T,   F' = 2(|Z|+M+E)F + 2ED + T'

These are PROB-38/40's exact-arithmetic majorants. The active order doubles once E
passes 1e-4 of the step's budget, and E and F are carried through every doubling.

The guard checks the whole frame before any pixel is rendered:

- **Validity:** every intermediate |Z|+M+E < 2.
- **Derivative:** dz/dc stays bounded away from zero, with Dmin = |b1| − Σk≥2 k|b_k| − F > 0.
- **Error:** E·rho/Dmin is at most `shift_px` output pixels, and F ≤ 1e-6·Dmin.
- **Fallback:** if no step passes, n = 0 and the frame renders with the ordinary kernel.
- **Trimming:** high orders are dropped only when their norms fit in half the remaining
  slack.

On the near-2 question from PROB-38: fd's escape radius is 1e10, so a sample's
binary64 crossing of |z| = 2 does not move fd's escape index. The guard abstains
instead of compensating; it never needed to on these frames. The arithmetic is
round-to-nearest. Rounding of the reference, coefficients and evaluation is not
enclosed.

## Results (final guard: shift_px = 1e-8)

`results/bins.csv` holds the per-bin cold seconds summed over each bin's frames, median
of 3 runs, 8 threads. The machine is an Intel family 6 model 154 with 20 logical CPUs,
Windows 11, rustc 1.90.0.

| Bin | plain s | bla s | series s | series-bla s | series vs plain | series-bla vs plain | best series vs bla | Failed frames (bla / series) |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1e-14..1e-17 | 2.195 | 3.416 | 0.658 | 1.115 | **3.33x** | 1.97x | 5.19x | 0 / 0 |
| 1e-17..1e-20 | 2.891 | 4.069 | 1.189 | 1.875 | **2.43x** | 1.54x | 3.42x | 0 / 0 |
| 1e-20..1e-23 | 4.572 | 5.489 | 2.639 | 3.868 | **1.73x** | 1.18x | 2.08x | 0 / 0 |
| 1e-23..1e-26 | 8.022 | 6.655 | 6.107 | 5.762 | 1.31x | **1.39x** | 1.15x | 1 / 3 |
| 1e-26..1e-29 | 35.061 | 9.016 | 17.308 | 7.363 | 2.03x | **4.76x** | 1.22x | 1 / 2 |

**Where the time goes.** Series setup is cheap:

- The series build takes 0.9–56 ms on 23 of the 24 frames. The exception is 2.4e-29
  (472 ms), which is mostly interior: the build runs for 99,999 steps there.
- The reference orbit costs 11–25 ms per frame and is shared by every arm.
- The growth schedule pays where orders get high. At 1e-14 it reaches order 28 with
  0.32M coefficient pairs, against 1.78M for fixed order 64 (`series-k64`, 2.96x
  instead of 3.33x). Deeper, the series is nearly linear (order 1–4), so growth hardly
  matters.

**Why the gain falls with depth.** The admitted prefix is 845 on 20 frames, 827 at
1e-14, 1690 at 4.2e-27, and 2535 at 7.5e-28 and 1.3e-28. Mean plain work per sample
climbs from about 1,100 to 2,700 iterations across the shallow bins, so a fixed
845-step skip covers less of it.

845 is a third of the target period, 2535. At that step the reference passes close to
0: the path's minibrot sits in the decorations of a period-845 one. Just after it, the
map from x to the state folds, |b1| collapses, and Dmin turns negative across the
disk. The guard can never admit again; building past 845 did not re-admit on any frame.
This is a property of one frame-wide polynomial, not of the order: fixed order 64
admits the same prefix.

**BLA after the series.** In the shallow bins `series-bla` is slower than `series`,
because today's per-frame BLA is itself slower than plain there (0.64–0.83x; the
lookup and table cost outweigh the blocks it applies). In the deep bins BLA is
what pays (3.89x on its own), and the series adds about 20% on top (4.76x).

## Correctness

`fd compare plain ARM` was run on all 24 frames, using its 1e-3 px nu contract and its
0.2% de tolerance (`results/compare.csv`, `compare-*.jsonl`).

- **Shallow bins (15 frames): series, series-bla and series-k64 pass every frame.**
  There are zero class mismatches, nu displacement is at most 5.9e-7 px, and de error
  is at most 3.7e-6. This meets DEC-17's ≤1e-3 px / 0 class mismatches on whole frames
  at every tested depth in those bins.
- **Deep bins (9 frames): 5 frames fail, all on class.** 78 samples go
  interior→unresolved and 1 goes unresolved→escaped; nu and de stay within tolerance
  on every frame. The interior→unresolved flips have one cause. The plain kernel's
  nucleus-contraction heuristic (FIX-04) multiplies 2z over steps 1..p, so a sample
  that starts at n > 0 has skipped those factors and must decline the heuristic. Its
  Brent/Newton checks then leave some minibrot-interior samples unresolved. A port
  must carry that product, for example as a second series ∂z_n/∂z_1, or move interior
  detection after the start. The single unresolved→escaped sample at 4.2e-27 also
  appears under BLA, which fails two deep frames the same way: one escaped↔unresolved
  sample each.
- **Recorded loss, `results/guard-1e-4/`.** With shift_px = 1e-4, PROB-40's budget
  scale, the series failed 13/24 frames, 8 of them on nu/de alone. The worst
  displacement was 7e-3 px, and de errors reached 5%. The guard's first-order shift
  measures error against dz/dc at the start iterate. The fold just after step 845
  amplifies that error about 10^4x relative to the final dz/dc. Tightening to 1e-8
  removed every nu/de failure at the same prefix and speed. **This is the valley-style
  endpoint failure from PROB-38/40 appearing on real frames.** It is cured here by
  margin, not by a bound that sees the later fold.

## Open questions from docs/2026-10-19 and PROB-38/39, answered per bin

| Bin | Initializer alone enough for 3x? | Staged LA still needed? | Valley endpoint failure? |
|---|---|---|---|
| 1e-14..1e-17 | Yes, 3.33x | No for 3x; today's BLA only slows it | Only at the 1e-4 guard (fixed by 1e-8) |
| 1e-17..1e-20 | No, 2.43x | Yes, after step 845 | Only at the 1e-4 guard (fixed by 1e-8) |
| 1e-20..1e-23 | No, 1.73x | Yes | Only at the 1e-4 guard (fixed by 1e-8) |
| 1e-23..1e-26 | No, 1.31x | Yes; BLA is barely ahead | Only at the 1e-4 guard; class flips remain |
| 1e-26..1e-29 | No, 2.03x | Yes; BLA carries this bin (4.76x combined) | Class flips remain (nucleus heuristic) |

## Go / no-go for FIX-09

**No-go for porting the initializer as FIX-09's 3x answer. Go for porting it as FIX-09's
first stage.** It is the cheapest correct win measured on these frames: 1.7–3.3x cold
across 1e-14..1e-23 with zero `fd compare` failures, against today's BLA at 0.64–0.83x.

Reaching 3x in every bin needs work that continues past the reference's near-zero
passages:

- staged / multi-reference LA that restarts after each fold, which is what the rival LA
  implementations do;
- a BLA that is profitable in the shallow bins;
- carrying the nucleus contraction across the start, so the deep bins stay class-exact.

The guard must keep a margin over the first-order shift (1e-8 here) until a bound that
sees later folds exists.

## Reproduce

From the repository root, with Python 3.10+ (mpmath, NumPy) and a Rust toolchain:

```sh
FD=target/release/fd python docs/2026-10-20/series-frames/freeze_frames.py --out docs/2026-10-20/series-frames/results
python docs/2026-10-20/series-frames/run.py --out /tmp/prob41 --threads 8 --runs 3
python docs/2026-10-20/series-frames/run.py --out /tmp/prob41-loose --threads 8 --runs 3   # after editing --shift-px 1e-4 into the probe call, for the loss
cargo test -p fd-kernel series
```

`run.py` uses an already-built `fd` if one exists (`--fd`); this issue does not change
`fd compare`. It always builds the probe. On this Windows machine, relinking `fd` failed
because the local Visual Studio install lacks `atls.lib` (needed by a GPU dependency),
so the shared prebuilt `fd` ran the comparisons.

The full run took 473.7 s. It prints the per-bin table and writes `bins.csv`,
`frames.csv` (per frame and arm: setup split, series prefix, order, coefficient pairs,
growth steps, guard values, BLA use, classes) and `compare.csv`. Timings vary by
machine; the arm ratios and the 845 prefix are the result.
