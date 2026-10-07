# Screen-Space Error Contract (LOD-01)

The rule that decides whether a tile of output pixels is visually resolved (accept) or
needs more work (refine). Governed by DEC-05 (refinement is driven by screen-space
error) and DEC-10 (every approximation layer states validity, error and fallback).
Code: `crates/fd-samples/src/lod.rs`; CLI `fd lod`. Progressive phases (LOD-02) are
`fd render --refine`, below.

## Inputs

Only mathematical columns of a `.fds` file (SAMPLES.md): `class` (kind + evidence),
`de` and `bound`. No palette, gamma or shading enters the rule, so a recolour never
changes a decision.

## Projected error

`sigma_f` is the complex-plane width of one output pixel in frame `f`; `eps_T` is the
tile's complex-domain value error. The projected error is `E_px = eps_T / sigma_f`.

For an escaped sample the value is `nu` and `bound` is its absolute error. Its error is
measured as the equivalent displacement in output pixels, the same measure the oracle
gates on (`tools/oracle.py`): `|∇nu| = 2 / (de ln 2)` per pixel, so

```
e_px(sample) = bound · de · ln 2 / 2        E_px(T) = max over T's samples
```

## Tile states

Judged over every sample in the tile, first failure wins:

| State | When | E_px | Decision |
|---|---|---|---|
| `CERTIFIED_UNIFORM` | every sample `Interior` with `Certified` evidence | 0 | accept |
| `CERTIFIED_APPROXIMATE` | every sample `Escaped`, evidence ≥ `Bounded`, `de` and `bound` present, and `de/4` > half the sample-cell diagonal (`√2 / 2ss` px) | max `e_px` | accept iff `E_px ≤ max_px` |
| `UNRESOLVED_BOUNDARY` | anything else | ∞ | always refine |

Refine reasons reported: `unresolved` (iteration budget ran out), `uncertified`
(`Heuristic` evidence, or interior without `Certified`), `no-bound` (missing or
invalid `de`/`bound`), `near-set` (Koebe lower bound `de/4` on the distance to the set
does not clear the sample cell, so the boundary may cross it), `mixed` (interior and
exterior in one tile: the boundary is inside), and `over-threshold` for an approximate
tile above `max_px`.

The rule treats a sample's evidence as covering its own sample cell. A producer that
writes `Certified` interior or `Bounded` exterior owns that claim, including the
accuracy of `de`.

## Threshold

`max_px` defaults to **0.25 output pixels** (`FINAL_PX`), the initial final/video
target from docs/research/10-6-26/02. It is experimental (DEC-05); LOD-03 measures it.

## Validity, error, fallback (DEC-10)

- **Validity:** a tile is accepted only on `Certified`/`Bounded` evidence as above.
- **Error:** accepted tiles carry `E_px ≤ max_px` (uniform tiles carry 0).
- **Fallback:** every other tile refines: subdivide, supersample, raise precision or
  fall back to exact sampling (LOD-02 schedules these).

## Bounded producer (LOD-05)

`fd render --columns ...,bound` makes `pert-f64/1` and `pert-fx/1` carry rigorous
running error radii next to every escaped sample (`crates/fd-kernel/src/sample.rs`):

- Reference: `q_m >= |A_m - Z_m|`, the stored f64 point's distance from the exact orbit
  `A` of the exact view centre (fixed point: `16 * 2^-bits` defect per step plus f64
  rounding; f64: the rounded centre plus `8u(|Z|^2 + |C|)` per step), propagated as
  `a' = a(2|Z| + a) + defect`.
- Sample: `e >= |z_exact - (A_m + delta)|` with `e' = e(2|w| + e) + rho`, where `rho`
  covers f64 rounding of the delta step, `2 q_m |delta|`, and the f64 offset's distance
  from the exact sample position; rebasing adds `q_m + u|z|`. The derivative carries the
  matching radius on `dz/dc`. The radius computation itself is rounded upward by a
  `1 + 8u` factor per step; underflow is covered by small absolute terms.

An escaped sample is written `Bounded` iff `|z| - e_z > 2` (escape proven) and the
derivative radius is under half `|dz/dc|`; otherwise it stays `Heuristic`. Its columns:

- `bound` = nu error `log2(ln|z| / ln(|z| - e_z)) + 8u(n + 8)`, for nu at the kernel's
  escape step `n`, widened by `de_high / de_low` (so `bound >= ` the nu error).
- `de` = `de_low`: the estimate from `|z| - e_z` and `|dz/dc| + e_d`, times `e^-G`
  (`G <= ln|z| 2^-n`) and `1 - 4/|z|^2`, so the distance to the set is at least `de/4`
  (Koebe: `d >= e^-G de / 4`, finite-`n` terms assume `|c| <= 4`).
- Hence `bound * de * ln2 / 2 >= nu error * de_high * ln2 / 2`, the true displacement.

Interior samples, unresolved samples and `pert-fx-scaled/1` stay `Heuristic`. Renders
without `bound` run the unchanged kernel and stay `Heuristic`. LOD-04 checks accepted
tiles against the oracle.

## Progressive refinement (LOD-02)

`fd render ... --refine B [--max-px E]` (`crates/fd-kernel/src/refine.rs`) renders in
`B x B` output-pixel blocks (row-major, edge blocks smaller) through phases, recording a
per-block bitmask: `1` preview, `2` sparse, `4` dense, `8` final, `16` fallback.
`de` and `bound` are always produced.

1. **Preview:** one sample per block (the sparse sample of its centre pixel). Display
   only; it never certifies a block.
2. **Sparse:** one sample per output pixel, sub-sample `(ss/2, ss/2)`. The rule above
   judges the block with each sample covering its whole pixel: the Koebe disk must clear
   its farthest pixel corner, `reach = sqrt2 (floor(ss/2) + 1/2) / ss` px. Accepted
   blocks are **final**; each pixel's other `ss^2 - 1` sample slots are copies of its
   sparse sample, written `Heuristic` (a copy has no evidence for its own position).
3. **Dense:** the blocks not accepted are compacted into one work list and every sample
   they still lack is computed.
4. **Final/fallback:** each dense block is judged again at full `ss`: accepted is
   **final**, otherwise **fallback** (its exact per-sample results stand, as without LOD).

Every computed sample is bit-identical to a plain render's. Every block ends final or
fallback. Savings come from skipped supersamples, so `--ss 1` skips nothing; kernels
without Bounded evidence (`pert-fx-scaled/1`, LOD-06) fall back everywhere.

Report, after the usual render line:

```
refine block_px B max_px E blocks N
phase preview|sparse|dense blocks n samples S iterations I
blocks.final F
blocks.fallback K
samples S
full_samples S
skipped_samples S
iterations I
block BX BY mask M final|fallback
```

## CLI

```
fd lod in.fds [--tile-px N] [--max-px E]
```

Tiles are `N x N` output pixels (default 128), row-major from the top-left; edge tiles
may be smaller. Output:

```
tile_px N
max_px E
tiles T
accepted A
refine R
state.certified_uniform n
state.certified_approximate n
state.unresolved_boundary n
tile TX TY STATE e_px E accept|refine REASON
```
