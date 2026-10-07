# Sample file format (`.fds`) — v1.0

Palette-independent renderer output (BASE-01, DEC-07). A render writes mathematical
results once; any number of appearance passes read them without recomputing anything.
Code: `crates/fd-samples` (schema + codec, no dependencies).

## Design rules

- **Math only.** No colour, palette, lighting or gamma ever enters this file.
- **Structure-of-arrays.** One dense array per quantity. A consumer reads only the
  columns it uses (the reader seeks straight to them); a producer computes and stores
  only the columns requested. Example: `--columns de,normal` skips `nu` entirely, and
  `--columns nu` also lets the kernel skip the `dz/dc` derivative.
- **Canonical bytes.** Little-endian, fixed field order, explicit zero padding, no host
  struct layout. The same samples always encode to the same bytes, so a later
  content-addressed store (ATLA-01) can hash them directly.
- **Uncertainty is explicit.** "Ran out of iterations" is never written as "inside".
- **Exact view.** Coordinates are kept as the decimal strings given, never rounded
  through a float, because a deep reference may need thousands of bits.

## Layout

All sections start on an 8-byte boundary; padding bytes are zero.

| Offset | Type | Field |
|---:|---|---|
| 0 | `[u8; 8]` | magic `FDSAMPLE` |
| 8 | `u16` | major version (`1`) — a reader refuses other majors |
| 10 | `u16` | minor version (`0`) — additive changes only |
| 12 | `u32` | column bitmask (below); bit 0 must be set |
| 16 | `u32` | `nx`: samples per row |
| 20 | `u32` | `ny`: rows |
| 24 | `u32` | `ss`: samples per output pixel per axis; `nx`, `ny` divisible by it |
| 28 | `u32` | reserved, 0 |
| 32 | `u64` | `max_iter` |
| 40 | `f64` | escape radius |
| 48 | `f64` | rotation, radians counter-clockwise |
| 56 | str | centre real part (decimal) |
| … | str | centre imaginary part (decimal) |
| … | str | grid width in the complex plane (decimal) |
| … | str | kernel id and numeric contract, e.g. `pert-f64/1` |

`str` = `u16` byte length + UTF-8 bytes. The header is then padded to 8 bytes, and the
columns follow in bit order, each `nx * ny` elements padded to 8 bytes. Sample `k` is
row `j = k / nx`, column `i = k % nx`. Column offsets are implied by the bitmask, so
there is no directory to keep consistent. New columns always take higher bits, so
older readers still find every column they know at the same offset.

## Sample positions

Sample `(i, j)` is at `c = centre + e^{i·rotation} · h · (x − i·y)` with
`x = i + 0.5 − nx/2`, `y = j + 0.5 − ny/2`, `h = width / nx`. Row `j` grows downward on
screen and the imaginary axis points up. Output pixel `(px, py)` is the `ss × ss` block
of samples starting at `(px·ss, py·ss)`. Exact dyadic tile addresses (ADDR-01) are a
separate, axis-aligned form: see ADDRESS.md.

## Columns

| Bit | Name | Type | Bytes | Meaning |
|---:|---|---|---:|---|
| 0 | `class` | `u8` | 1 | always present; see below |
| 1 | `nu` | `f64` | 8 | smooth escape value `n + 1 − log2(log2 |z_n|)` |
| 2 | `de` | `f32` | 4 | exterior distance estimate `2|z_n| ln|z_n| / |∂z_n/∂c|`, in output pixels |
| 3 | `normal` | `u16` | 2 | screen-space angle of `z_n / (∂z_n/∂c)`, in turns × 65536 (x right, y down) |
| 4 | `bound` | `f32` | 4 | absolute error bound on `nu`; only with evidence ≥ Bounded |

`n` is the number of iterations taken when `|z_n|` first exceeds the escape radius.
`nu`, `de` and `normal` are meaningful only for `Escaped` samples; writers store 0
for the others. `nu` is `f64` because deep zooms reach millions of iterations, where
`f32` would leave too few bits for the fractional part. `de` and `normal` drive shading
only, so `f32` and a 16-bit angle (≈1e-4 rad) are well below what a pixel can show.
All columns together cost 15 bytes per sample; omitted columns cost nothing.

The `de` value is the standard derivative estimate. Asymptotically the true distance
lies within `[de/4, de]`, but in floating point it is a **heuristic**: it may guide
refinement, never certify that a pixel misses the set (docs/research/02).

### Class byte

| Bits | Field | Values |
|---|---|---|
| 0–1 | kind | 0 `Escaped`, 1 `Interior`, 2 `Unresolved` (budget ran out), 3 invalid |
| 2–3 | evidence | 0 `Heuristic` (plain floating point, no bound), 1 `Bounded` (see `bound`), 2 `Certified` (classification proven), 3 invalid |
| 4–7 | reserved | 0 |

Evidence follows the certification hierarchy in docs/research/02. A reader rejects
invalid class bytes.

## Producers

| Kernel id | Contract |
|---|---|
| `pert-f64/1` | Perturbation with rebasing against one f64 reference orbit at the f64-rounded centre (at most 2^20 points; past the end a sample rebases to `Z_0`). Used only while rounding the centre moves it by under 1/1024 of a sample. Interior: closed-form main cardioid/period-2 bulb test, then a Brent near-return check confirmed by Newton on the cycle multiplier. |
| `pert-fx/1 bits=B` | Same perturbation, but the reference orbit is iterated in `B`-bit fixed point from the exact decimal centre (`B` = 128 + depth in bits). f64 deltas; used down to sample spacings of 2^-900. No closed-form shortcut (f64 cannot place the sample). |
| `pert-fx-scaled/1 bits=B` | Fixed-point reference; the delta and `dz/dc` are f64 values times exact powers of two, so no depth underflows. Interior only by exact cycle return (no Newton yet): deep interior samples may stay `Unresolved`. |

All three write `Heuristic` evidence. Periodicity checks run only once a sample's
delta is resolvable next to the reference point in f64; before that `z == Z` in f64
and a periodic reference (e.g. the Misiurewicz point `i`) would fake a cycle.
`fd render` picks the cheapest valid kernel; `--kernel f64|fx|scaled` forces one where
its contract holds (deeper kernels are valid at any depth, for cross-checking).

Correctness is checked by `tools/oracle.py` (direct mpmath iteration at depth + 128
bits, no perturbation) over the locations in `bench/locations.txt`; see
`scripts/locations.sh`.

## Command surface

```text
fd render --re X --im Y --width W [--size WxH] [--ss N] [--iter N]
          [--columns nu,de,normal] [--threads N] [--rotation R]
          [--kernel auto|f64|fx|scaled] -o out.fds
fd shade <palette|relief> in.fds out.png
fd info in.fds
```

`palette` reads `class` and `nu`; `relief` reads `class`, `de` and `normal`. They are
proof passes for this format; the real appearance pipeline is SHADE-01.
