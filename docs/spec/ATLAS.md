# Atlas Chunk Store, Manifests and Byte Budget (ATLA-01..03)

Content-addressed, immutable, semantic chunks (DEC-06), implemented in `crates/fd-atlas`
(depends only on workspace `fd-addr`) and exposed as `fd chunk` and `fd manifest`, with
a per-store byte budget (DEC-03: 2.5 GiB target, 3 GiB hard cap).

## Chunks

A chunk is one semantic unit (an orbit slab, one BLA operator table, one certificate,
one `.fds` sample file...). Boundaries come from meaning, never from content-defined
chunking. Payloads are mathematical data, never raster imagery (DEC-04).

A chunk's **id** is the SHA-256 of its canonical bytes, written as 64 lowercase hex digits.

### Canonical bytes

All integers little-endian; every field written explicitly (no host struct layout);
the payload is zero-padded so the chunk length is a multiple of 8.

| Offset | Type | Field |
|---:|---|---|
| 0 | `[u8; 8]` | magic `FDCHUNK\0` |
| 8 | `u16` | major version (`1`); readers refuse other majors |
| 10 | `u16` | minor version (`0`) |
| 12 | `u16` | kind (below); 0 is invalid |
| 14 | `u16` | encoding: payload layout version for this kind |
| 16 | `u16` | formula: 0 none, 1 Mandelbrot `z^2 + c` |
| 18 | `u8` | rounding contract: 0 exact, 1 nearest (ties to even), 2 outward (enclosing) |
| 19 | `u8` | reserved, 0 |
| 20 | `u32` | precision: significand bits of the working precision (53 = f64; 0 if exact) |
| 24 | `u64` | payload length in bytes |
| 32 | bytes | payload, then zero padding to a multiple of 8 |

The contract fields (kind, encoding, formula, rounding, precision) are part of the
hashed bytes: the same numbers under a different contract are a different chunk.

A reader refuses non-canonical bytes: bad magic or major, kind 0, unknown rounding,
non-zero reserved byte, a length that does not match, or non-zero padding.

Payload fields are written with the canonical builder (`fd_atlas::Builder`):
little-endian integers, `f64`/`f32` as IEEE bits with `-0.0` normalised to `+0.0` and
every NaN written as the single quiet NaN `0x7ff8000000000000` (`0x7fc00000` for f32),
length-prefixed (`u64`) byte strings and UTF-8 text. Already-canonical blobs such as
`.fds` files are stored byte for byte.

### Kinds

| Code | Name | Holds |
|---:|---|---|
| 1 | `orbit-slab` | reference orbit range |
| 2 | `bla` | bilinear-approximation operators |
| 3 | `return-map` | experimental return-map operators |
| 4 | `certificate` | proved claims over an exact region |
| 5 | `exact-sample` | exact-coordinate samples |
| 6 | `samples` | one `.fds` file (SAMPLES.md) |
| 7 | `tile-manifest` | one logical tile (Manifests below) |
| 8 | `frame-manifest` | one video frame (Manifests below) |
| 9 | `orbit-manifest` | one reference orbit bound to its exact centre (Orbit slabs below) |

Other non-zero codes are accepted and shown numerically.

## Store

```text
<root>/FDATLAS                  marker "fd-atlas store 1\n"
<root>/BUDGET                   optional "target <bytes>\ncap <bytes>\n" (Byte budget)
<root>/chunks/<2 hex>/<62 hex>  the chunk's canonical bytes, named by its id
<root>/tmp/                     staging for atomic writes
```

- **Put** hashes the chunk. If an intact copy exists nothing is written
  (`deduplicated`); otherwise the bytes are staged, synced and renamed into place
  (`stored`). A stored copy that fails verification is replaced (`repaired`).
- **Get** re-hashes the file on every read and refuses bytes that do not hash to the
  requested id (`corrupt`) or are not canonical. Corruption is never returned as data.
- **Stats** reports unique chunks and the bytes their files occupy, the quantity the
  byte budget accounts, in total and per kind.
- A non-empty directory without the marker is refused rather than turned into a store.

## CLI

```text
fd chunk put --store DIR --kind K [--encoding N] [--formula none|mandelbrot]
             [--precision BITS] [--rounding exact|nearest|outward] <file>
fd chunk get --store DIR <id> -o out      # writes the verified payload
fd chunk show --store DIR <id>            # verified header fields and path
fd chunk verify --store DIR               # exit non-zero if any chunk fails
fd chunk stats --store DIR
fd chunk budget --store DIR [--target BYTES] [--cap BYTES]   # BYTES: N, NKiB, NMiB, NGiB
```

Defaults for `put`: encoding 1, formula `mandelbrot`, precision 53, rounding `nearest`.
Output is `name value` lines (`id`, `result stored|deduplicated|repaired`, `bytes`, ...).

## Byte budget

Each store has a budget over the sum of its chunk file sizes (DEC-03). Without a
`BUDGET` file it is the default: target 2.5 GiB (2684354560), hard cap 3 GiB
(3221225472). `fd chunk budget` persists another (target <= cap; a cap below current
usage is allowed and simply admits nothing new). A malformed `BUDGET` file is refused.

Policy (the simple one; choosing what to drop is the compiler's job):

- A put of a **new** chunk that would take stored bytes over the cap is refused with
  `atlas over budget ... hard cap ...; nothing written` and a non-zero exit. This covers
  every build path: `fd chunk put`, `fd manifest tile|frame`.
- Deduplicated puts write nothing and always succeed; repairs restore already-admitted
  bytes and are allowed.
- Exceeding the target is allowed but reported: puts print `atlas_bytes N` and
  `over_target 0|1`, and warn on stderr when over.
- Within one process the check and write are serialised; concurrent writer processes
  each check against their own count and may together overshoot by their in-flight
  chunks.

`fd chunk stats` reports `target`, `cap`, `headroom` (cap minus bytes, floored at 0),
`over_target`, one `kind.<name> <chunks> <bytes>` line per stored kind, and bytes per
allocation class. The research allocation guide (tuning targets only, not enforced):

| Class | Kinds | Guide share |
|---|---|---|
| `class.operators` | orbit-slab, bla, return-map | 55-70% |
| `class.evidence` | certificate, exact-sample, samples | 10-20% |
| (previews) | no kind yet | 10-20% |
| `class.manifests` | tile-manifest, frame-manifest, orbit-manifest | rest |
| `class.other` | unknown kinds | - |

## Orbit slabs (REF-01)

A reference orbit `Z_0 = 0, Z_{n+1} = Z_n^2 + C` is stored as fixed semantic slabs
(DEC-13): slab `k` of size `S` holds `Z_{kS} .. Z_{kS+len-1}`, `1 <= len <= S`, and only
the orbit's last slab is short. Boundaries depend only on the iteration index, so the
same orbit prefix always cuts into the same chunks. Kind 1 (`orbit-slab`), encoding 1:

| Field | Type | Meaning |
|---|---|---|
| `size` | u32 | points per full slab `S` |
| `len` | u32 | points in this slab |
| `start` | u64 | iteration index of the first point, a multiple of `S` |
| `re[len]`, `im[len]` | f64 | the points, structure-of-arrays, finite |

Contract: formula `mandelbrot`, rounding `nearest` (each point is the f64 nearest the
orbit computed at the working precision), `precision_bits` = 53 for the f64 tier or the
fixed-point fraction bits of the deep tiers. Decoding re-encodes and refuses
non-canonical bytes. A slab says nothing about which centre `C` it belongs to; the
orbit manifest (kind 9, encoding 1, REF-02) binds an orbit to its exact centre:

| Field | Type | Meaning |
|---|---|---|
| `center_re`, `center_im` | str | the exact decimal centre the orbit was computed at |
| `n` | u64 | slab count, at least 1 |
| `slabs[n]` | 32-byte id | the orbit's slabs in order from `Z_0` |

Its contract carries the orbit's `precision_bits` (formula `mandelbrot`, rounding
`nearest`); every slab it names must have the same precision.

```text
fd orbit put --store DIR <render view flags> [--slab N]   # default N = 4096
fd render <flags> --store DIR --orbit ID                   # load instead of compute
```

`put` computes the reference `fd render` would use, stores its slabs and its orbit
manifest and prints `points`, `precision`, `slab_size`, one `slab START LEN ID RESULT
BYTES` line per slab, `slabs`, `slab_bytes`, `bytes_per_iter` (slab chunk bytes over
points: 16 plus the 48-byte header and fields per slab), `orbit` (the orbit manifest id)
and the atlas byte lines. `render --orbit` refuses an orbit whose centre is not the
view's centre as an exact decimal (`1.0` and `1` agree), joins the slabs (contiguous from
`Z_0`, one size), and checks precision: the f64 tier needs exactly 53 bits, the deep
tiers take any precision at least the view's own (the header's `bits=` then records the
orbit's). With the view's own precision the samples are bit-identical to computing the
orbit; with more they are those of a render whose reference was computed at that
precision, which is at least as accurate.

### Orbit reuse (REF-02)

The reference orbit depends only on the centre `C` and the working precision, not on
the frame's width, size or iteration limit (a longer orbit is valid, a shorter one just
rebases sooner). So every frame of a zoom path at one exact centre can render from one
stored orbit: the one computed for its deepest frame.

```text
fd reuse PATH --store DIR [--size WxH] [--ss N] [--iter N] [--columns C]
         [--threads N] [--kernel K] [--slab N]
```

`PATH` is a camera path as in PLAN.md (`re im width [rotation]` per line). Frames are
grouped by exact centre, f64-tier (53-bit) frames apart from deep ones. Each group's
orbit is computed once for its deepest frame and stored (`orbit G centre RE IM precision
B points P frames n lead F seconds S id ID`). Every frame is then rendered twice, from
its own computed orbit and from the stored one (loaded through the manifest, which
checks the centre), and compared sample by sample: `frame F orbit G ROLE need_bits b
orbit_bits B own_reference_seconds .. own_seconds .. load_seconds .. reuse_seconds ..
iterations OWN REUSED differing D`. ROLE is `lead` (the orbit was computed for this
frame), `reused` (it was computed for another frame) or `fallback` (no other frame shares
the centre, so the frame uses its own orbit). Totals: `frames`, `orbits`, `reused`,
`fallback`, `reuse_ratio` (frames per orbit), `reference_seconds.per_frame` (orbit time
with one orbit per frame), `reference_seconds.shared`, `load_seconds`,
`reference_seconds.saved` (per-frame minus shared minus loads), `seconds.per_frame` and
`seconds.reuse` (whole-path wall time each way), `identical_frames`,
`differing_samples`, then the atlas byte lines. The command fails if any frame whose
precision equals its orbit's renders differently. Frames off the shared centre (pans)
need an off-centre reference and are not reused yet. Compact encodings (shared exponents, compression) are a follow-up; encoding 1 is
the 16 bytes/iteration baseline.

## BLA tables (ACC-01)

Bivariate linear approximation, the first reusable acceleration operator. Perturbation
steps `delta_{k+1} = 2 Z_k delta_k + delta_k^2 + dc` against one reference orbit are
replaced, while `delta` is small next to `Z`, by affine blocks
`delta_{m+l} = A delta_m + B dc` (`dz/dc` likewise: `d_{m+l} = A d_m + B`). Code:
`crates/fd-kernel/src/bla.rs` (build, lookup), `crates/fd-atlas/src/bla.rs` (chunk).

**Blocks.** The build starts from one single step per index `k = 1 .. points-2`:
`A = 2 Z_k`, `B = 1`, `r = alpha = 2 eps |Z_k|`, `beta = 0`, and no block where
`|Z_k| > 2` (the reference is escaping; those steps always run one by one). Level `j`
merges pairs of level `j-1` (`x` then `y`): `A = A_y A_x`, `B = A_y B_x + B_y`, and with
`Ahat = |A_x| + alpha_x`, `Bhat = |B_x| + beta_x`:

```
r     = min(r_x, (r_y - Bhat dc_max) / Ahat)        (block invalid, all zero, unless r > 0)
alpha = |A_y| alpha_x + alpha_y Ahat
beta  = |A_y| beta_x + alpha_y Bhat + beta_y
```

Level `j` has `(points - 2) >> j` blocks of `2^j` steps, block `i` starting at index
`1 + i 2^j`. Only levels `j >= 1` are stored: a one-step block saves no step and costs
more than the plain step (measured: ~15 ns per applied block against ~8 ns per plain
step in the same regime). The table ends before the first level with no valid block,
so it holds under `points - 2` blocks.

**Lookup.** At reference index `m` the renderer takes the longest valid block starting
there (`k = m - 1` divisible by `2^j`). Because `r` of a level-`j` block is at most that
of its first half, validity only shrinks going up, so the lookup walks up from level 1
and stops at the first failure. Odd `k` (and `m = 0`, where `Z_0 = 0`) take a plain step.

**Validity, error, fallback (DEC-10).**

- *Validity:* a block applies to a sample at reference index `m` only while
  `|delta_m| < r`, `|dc| <= dc_max`, it lands at most on the orbit's last point and
  within the iteration budget. Then, in exact arithmetic, every skipped step `k` had
  `|delta_k| <= 2 eps |Z_k| <= 4 eps` (so its dropped `delta_k^2` is at most `eps` times the
  kept `2 Z_k delta_k`, and `|z_k| <= 2 (1 + 2 eps)` with `|z_k| > |delta_k|`): a block
  never jumps over an escape (escape radius >= 4, checked) or a rebase. Every skipped
  iterate also has `|delta_k| < 2 eps / (1 - 2 eps) |z_k|`, and `eps` is capped at
  `EPS_MAX = 2^-41` so that this stays at most the `RESOLVABLE |z| = 1e-12 |z|` from which
  the plain kernel judges periodicity (one shared constant in `sample.rs`, checked by a
  compile-time assert in `bla.rs`): a block never covers a periodicity-judged iterate,
  so blocks run only where the plain kernel judges nothing either. Larger `eps` is
  refused when a table is built (`fd orbit bla --eps`) and when one is loaded
  (`Bla::new`, which every decoded table goes through). The default is `eps = 2^-50`.
  A block also ends at the next Brent save point at the latest (its length is capped at
  `chk - n`), so BLA saves exactly the iterates the plain kernel saves and judges
  periodicity at the same iterates against the same save points: classes match the plain
  kernel's (FIX-02; letting blocks land past a save point and saving there instead
  compared against other iterates and classed some samples differently). The cap costs
  a few blocks per save point (17 save points up to 2^17 iterates).
- *Error:* `|delta_{m+l} - (A delta_m + B dc)| <= alpha |delta_m| + beta |dc|`
  (exact arithmetic, proved by induction over merges; `crates/fd-kernel/src/bla.rs`
  tests it against plain perturbation). A deviation `e` in `z` at iterate `n` moves the
  rest of the orbit, to first order, like moving `c` by `e / |dz_n/dc|`. With `dz/dc`
  tracked (`de` or `normal` columns) the renderer sums, per sample, `(alpha |delta_m| +
  beta |dc|) / |d_{m+l}|` over the blocks it applies and reports the largest sum over
  the frame in output pixels (`bla.shift_px.max`): the BLA term of the screen-space
  error, comparable with the oracle's displacement measure and with LOD.md's
  `FINAL_PX`. A blanket form: each skipped step drops at most about `2 eps |dc|` in these
  units, so the shift is at most about `2 skipped eps |dc|` (default `eps`, 5e4 skipped
  steps and a 128x72 frame: ~7e-9 px). `dz/dc` drops `2 delta_k d_k`, at most `2 eps`
  of the kept `2 Z_k d_k` per step. The estimate is first order and leaves out f64
  rounding of the coefficients and of the block evaluation (the plain kernel's rounding,
  of the order `U / eps = 1/8` of the eps term). Certified bounds are ACC-02: BLA
  samples stay `Heuristic`, and `--columns bound` is refused with `--bla`.
- *Fallback:* every step no block covers is the plain perturbation kernel, and the
  render reports how much that was. The plain kernel without `--bla` carries no BLA
  code (a compile-time switch).

A table is valid for any frame at its orbit's centre whose largest `|dc|` (half the
sample grid's diagonal) is at most `dc_max`: one table built for a frame serves every
deeper frame at that centre (with shorter blocks than a table of their own). The scaled
tier (spacings below 2^-900) is not supported: `orbit bla` and `render --bla` refuse it.

Kind 2 (`bla`), encoding 1, contract formula `mandelbrot`, precision 53 (the
coefficients are f64; the orbit's own precision is in its manifest), rounding
`nearest`; decoding re-encodes and refuses non-canonical bytes:

| Field | Type | Meaning |
|---|---|---|
| `orbit` | 32-byte id | the orbit manifest (kind 9) the table was built over |
| `eps` | f64 | relative tolerance of dropped terms; the chunk accepts `0 < eps < 1`, the kernel refuses `eps > 2^-41` on load |
| `dc_max` | f64 | largest `|dc|` the radii hold for, `> 0` |
| `points` | u64 | points of that orbit |
| `n` | u64 | stored levels (`j = 1 .. n`) |
| per level `j` | 7 x f64[`(points-2) >> j`] | `a.re`, `a.im`, `b.re`, `b.im`, `r`, `alpha`, `beta`, structure-of-arrays, finite |

Canonical form: every value finite; a block is valid (`r > 0`) or all zero; every
stored level holds at least one valid block. The table names its orbit manifest, so it
is bound to that orbit's exact centre, precision and length: `render --bla` loads the
orbit through the manifest (centre-checked as for `--orbit`) and refuses a table whose
`points` differ from the orbit's. Up to 56 bytes per block, under 56 bytes per orbit
point (about 3.5 times the orbit's 16 bytes per point).

```text
fd orbit bla --store DIR <render view flags> [--orbit ID] [--eps E] [--slab N]
fd render <view flags> --store DIR --bla ID -o out.fds
```

`orbit bla` builds the table of the stored orbit `--orbit` (else computes and stores the
view's own orbit) for the view's `dc_max` and prints `points`, `precision`, `eps`,
`dc_max`, `levels`, `blocks`, `valid_blocks`, `build_seconds`, `bytes`,
`bytes_per_iter`, `orbit_slab_bytes` (the orbit's slab chunks), `bytes_over_orbit`,
`result`, `orbit`, `bla` (the table's id) and the atlas byte lines. `render --bla` loads
and verifies the table and its orbit before the render clock starts, refuses a view
outside the contract (`dc_max`, escape radius, scaled tier, `bound` column, `--refine`),
writes kernel id `... bla/1` and prints `bla.load_seconds`, `bla.blocks`, `bla.skipped`
(steps replaced), `iterations` (plain steps plus blocks), `iterations.equivalent`
(steps a plain render takes for the same samples), `skip_fraction`,
`speedup.iterations`, `fallback.iteration_fraction` (share of equivalent steps run by
the plain kernel), `closed_form.samples` (samples settled by the closed-form
main-cardioid/period-2 test before any iteration, f64 tier: neither BLA nor fallback
work), `fallback.samples` and `fallback.sample_fraction` (samples that iterated but took
no block, over the samples that iterated), and `bla.shift_px.max` (`untracked` without
`dz/dc`). An escape radius below 4 is refused with its own error.

Measured (CPU, 1 thread, valley 1e-28, 128x72, 5e4 iterations): 2.96x fewer iterations
but ~1.5x less wall time, because the 29% of steps left to the fallback (the resolvable
regime, with periodicity checks) cost ~3x a step in the BLA regime. At shallow f64 views
(|dc| ~ 1e-3) no block is valid at `eps = 2^-50` and the fallback is 100%. Only the CPU
path exists; the GPU comparison (prior work found GPU BLA slower than plain series
approximation, docs/research/01) waits for a GPU kernel.

## Manifests

Manifests are ordinary chunks (kinds 7 and 8, encoding 1) that name other chunks by id,
so the atlas is a Merkle DAG: frame manifest -> tile manifests -> child tile manifests
and math chunks. A frame owns no mathematics; thousands of frames naming one orbit slab
repeat only its 32-byte id, while the slab's bytes are stored once. Both payloads are
written with the canonical builder; a decoder re-encodes and refuses any chunk whose
bytes differ (unsorted or duplicate references, unknown bits, trailing bytes).

Strings are `u64` length + UTF-8; ids are 32 raw bytes.

### Tile manifest (kind 7)

Contract: formula Mandelbrot, precision 0, rounding exact.

| Field | Type | Meaning |
|---|---|---|
| tile | string | canonical key `level/xhex/yhex` (ADDRESS.md) |
| evidence | `u8` | bits: 1 heuristic, 2 bounded, 4 certified; others 0 |
| children | `u8` | bit `q` set when child quadrant `q` (ADDRESS.md) has a manifest; others 0 |
| child ids | id each | one per set bit, ascending `q` |
| refs | `u64` n, then n x (`u16` kind, id) | math chunks, sorted by (kind, id), no duplicates |

A child id must name the tile manifest of exactly `tile.child(q)`. Refs name
non-manifest chunks and carry the referenced chunk's kind.

### Frame manifest (kind 8)

Contract: formula Mandelbrot, precision 53, rounding nearest. The camera is local to an
exact anchor tile (DEC-08): the deep chart transform is precomposed, so a frame never
walks the tile hierarchy to place itself.

| Field | Type | Meaning |
|---|---|---|
| anchor | string | tile key of the chart the camera is expressed in |
| offset re, im | `f64`, `f64` | view centre minus anchor centre, in anchor sides (im up) |
| width | `f64` | view width in anchor sides, > 0 |
| rotation | `f64` | radians, as `fd render --rotation` |
| size | `u32`, `u32` | output pixels, > 0 |
| ss | `u32` | supersampling per axis, > 0 |
| iter | `u64` | iteration limit |
| columns | `u32` | sample column mask (SAMPLES.md `ColumnSet` bits) |
| tiles | `u64` n, then n ids | tile manifests read, sorted, no duplicates |

All `f64` fields are finite. A frame is ~150-200 bytes.

### Walk

`fd manifest walk` loads every frame given (default: every frame manifest in the
store), re-verifies every chunk it reaches, checks reference kinds and child
addresses, and reports:

| Field | Meaning |
|---|---|
| `frames`, `tiles` | distinct frame and tile manifests walked |
| `math_chunks`, `math_bytes` | distinct math chunks reached and their stored bytes |
| `math_refs` | sum over frames of the distinct math chunks each frame reaches |
| `math_bytes_per_frame` | bytes if every frame owned private copies of what it reaches |
| `manifest_bytes` | bytes of all distinct manifests reached |
| `sharing` | `math_bytes_per_frame / math_bytes` |
| `store_chunks`, `store_bytes` | `fd chunk stats` for the whole store |

### Manifest CLI

```text
fd manifest tile --store DIR --tile KEY [--evidence heuristic,bounded,certified]
                 [--children Q:ID,...] [--refs ID,...]
fd manifest frame --store DIR --anchor KEY [--offset U,V] [--width W] [--rotation R]
                  [--size WxH] [--ss N] [--iter N] [--columns nu,de,normal] --tiles ID,...
fd manifest show --store DIR <id>
fd manifest walk --store DIR [frame-id...]
```

`tile` and `frame` refuse references that are missing, corrupt, of the wrong kind, or
(for children) for the wrong address, then print `id`, `result`, `bytes` like
`fd chunk put`. Filling manifests from a zoom path is the compiler's job: `fd compile` (COMPILE.md)
emits them from `fd plan`'s prediction (PLAN.md) of which tiles each frame needs.
