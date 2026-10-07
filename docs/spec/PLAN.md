# Known-Path Footprint Planner (SCHED-01)

`fd plan` turns a known camera path into per-frame logical-tile demand (ADDRESS.md) and
the frame where each tile is first used: the first-use deadlines a build scheduler
(SCHED-02) orders work by. Implemented in `crates/fd-cli/src/plan.rs`.

## Path file

One frame per non-blank line, `#` starts a comment:

```
re im width [rotation]
```

The same file feeds `fd reuse` (ATLAS.md "Orbit reuse") and `fd control`, the
independent-frame control renderer (SAMPLES.md "Independent-frame control").
Fields mean exactly what `fd render --re --im --width --rotation` means (SAMPLES.md View);
`re`/`im` are exact decimals of any length, so deep paths stay exact.

## Tile level

Each frame uses the coarsest level `L` whose tile side `2^(2-L)` is at most `--tile-px`
output pixels (default 128; ARCHITECTURE.md logical tiles 64/128/256). Microblock demand
is the same query with `--tile-px 8` or `16`.

## Footprint

A frame needs every level-`L` tile containing one of its samples (the renderer's own
grid, `fd_kernel::Plane`). The planner places the centre exactly with
`Tile::locate` at level `L+32` (error <= 2^-33 tile side), intersects each candidate tile
with the rotated rectangle spanned by the corner samples (separating axes, padded by
1e-6 tile), and drops tiles outside the root square. It then locates every sample of
every frame and counts those whose tile was not predicted: `unpredicted` must be 0, or
`fd plan` exits non-zero.

## Output

```
frames N
tile_px P
tiles T                               distinct tiles over the path
demand D                              sum over frames of tiles per frame
frame F level L tiles n new k         k = tiles first used at frame F
tile KEY first F last G frames c      sorted by first use, then key
checked_samples S
unpredicted 0
```

Not here: build cost and scheduling (SCHED-02), fallback ancestors and envelopes, and
byte estimates against the atlas budget.
