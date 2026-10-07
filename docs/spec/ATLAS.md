# Atlas Chunk Store and Manifests (ATLA-01, ATLA-02)

Content-addressed, immutable, semantic chunks (DEC-06), implemented in `crates/fd-atlas`
(depends only on workspace `fd-addr`) and exposed as `fd chunk` and `fd manifest`. The
byte budget (ATLA-03, 3 GiB hard cap per DEC-03) builds on this layer.

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

Other non-zero codes are accepted and shown numerically.

## Store

```text
<root>/FDATLAS                  marker "fd-atlas store 1\n"
<root>/chunks/<2 hex>/<62 hex>  the chunk's canonical bytes, named by its id
<root>/tmp/                     staging for atomic writes
```

- **Put** hashes the chunk. If an intact copy exists nothing is written
  (`deduplicated`); otherwise the bytes are staged, synced and renamed into place
  (`stored`). A stored copy that fails verification is replaced (`repaired`).
- **Get** re-hashes the file on every read and refuses bytes that do not hash to the
  requested id (`corrupt`) or are not canonical. Corruption is never returned as data.
- **Stats** reports unique chunks and the bytes their files occupy, the quantity the
  byte budget will account.
- A non-empty directory without the marker is refused rather than turned into a store.

## CLI

```text
fd chunk put --store DIR --kind K [--encoding N] [--formula none|mandelbrot]
             [--precision BITS] [--rounding exact|nearest|outward] <file>
fd chunk get --store DIR <id> -o out      # writes the verified payload
fd chunk show --store DIR <id>            # verified header fields and path
fd chunk verify --store DIR               # exit non-zero if any chunk fails
fd chunk stats --store DIR
```

Defaults for `put`: encoding 1, formula `mandelbrot`, precision 53, rounding `nearest`.
Output is `name value` lines (`id`, `result stored|deduplicated|repaired`, `bytes`, ...).

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
`fd chunk put`. Filling manifests from a zoom path is the compiler's job (not here).
