# Tile Addresses (ADDR-01)

Exact dyadic quadtree addresses, implemented in `crates/fd-addr` and exposed as `fd addr`.
They give logical tiles (ARCHITECTURE.md) a cache identity that stays exact at any depth.

## Model

- The root tile (level 0) is the square `[-2, 2] x [-2, 2]`.
- A tile at level `L` has side `2^(2-L)` and integer indices `0 <= x, y < 2^L`.
  `x` grows to the right (real part up); `y` grows downward (imaginary part down), like
  sample rows in SAMPLES.md.
- Tile `(L, x, y)` covers `re in [-2 + x·s, -2 + (x+1)·s)` and
  `im in (2 - (y+1)·s, 2 - y·s]` with `s = 2^(2-L)`. Every point of the root square
  has exactly one tile per level.
- Indices are integers of any size, so all mappings below are exact. A 1e-1000 view is
  about 2^-3322 wide, so its tiles sit near level 3330 with 3330-bit indices.

## Text key

`level/xhex/yhex`: decimal level, lowercase hex indices, no leading zeros (`0` for zero),
e.g. `3/1/6`. Only this canonical form parses, so equal tiles have equal keys.

## Mappings

| Mapping | Definition |
|---|---|
| parent | `(L-1, x>>1, y>>1)`; none for the root |
| child `q` | `(L+1, 2x + (q&1), 2y + (q>>1))`; `q` = 0 upper-left, 1 upper-right, 2 lower-left, 3 lower-right |
| locate | `x = floor((re+2)·2^(L-2))`, `y = floor((2-im)·2^(L-2))`, computed exactly from the decimal strings |
| centre | `re = -2 + (2x+1)·2^(1-L)`, `im = 2 - (2y+1)·2^(1-L)`, printed as exact decimals |
| sample | sample `(i, j)` of a tile's `2^k x 2^k` grid is the descendant cell `(L+k, x·2^k + i, y·2^k + j)`; its position is that cell's centre |
| owner | inverse of sample: `(L-k, x>>k, y>>k)` and `(i, j)` = the low `k` bits |

Because sample cells are themselves tiles, a sample keeps one identity across zoom levels:
sample `(i, j)` in a `2^k` grid of tile T is sample `(i + (x&1)·2^k, j + (y&1)·2^k)` in
the `2^(k+1)` grid of T's parent. There is no approximation here, so no
validity/error contract (DEC-10) applies; consumers that round a centre to f64 or fixed
point own that error.

## CLI

```
fd addr locate --re X --im Y --level L   # -> key
fd addr show <key>                       # key, level, re, im, side, parent, children
fd addr sample <key> --grid K --at I,J   # -> sample cell key and exact centre
fd addr owner <cell-key> --grid K        # -> owning tile key and I,J
```
