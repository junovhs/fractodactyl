# Atlas Chunk Store (ATLA-01)

Content-addressed, immutable, semantic chunks (DEC-06), implemented in `crates/fd-atlas`
(no dependencies) and exposed as `fd chunk`. Manifests (ATLA-02) and the byte budget
(ATLA-03, 3 GiB hard cap per DEC-03) build on this layer.

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
| 7 | `tile-manifest` | reserved for ATLA-02 |
| 8 | `frame-manifest` | reserved for ATLA-02 |

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
