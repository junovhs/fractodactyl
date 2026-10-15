# Jet-AA probe (frontier question 6: supersampling from jets)

Notes and conclusions: `docs/research/10-11-26/q6-supersampling-from-jets.md`.
CPU only. Standalone crate, outside the workspace (like `tools/research/twolevel/zone_finish/capture`).

- `refgen.py RE IM WIDTH MAX_ITER OUT`: reference orbit at the exact decimal centre (gmpy2),
  stored as hi+lo doubles so the truth arm can run in double-double.
- `render REF WIDTH CROPW CROPH OX OY MAX_ITER OUT [ss4]`: a crop of a 1920x1080 frame. Per
  output pixel it stores:
  - the centre sample with dz/du, d2z/du2 and d3z/du3 (binary64, u in output pixels);
  - the four exact ss 2 samples (binary64, production arithmetic);
  - with `ss4`, the 4x4 truth samples, re-run in double-double where DE < 0.01 px.
- `check DUMP...`: accuracy of jet-predicted nu / DE / normal at the ss 2 offsets, by jet
  order and DE band.
- `score DUMP [ORDERS] [TAUS]`: shades every arm with the project's looks (fd-shade:
  umber, palette, relief, the five studio presets, master-ice; aa on, unresolved as
  interior) and scores against exact ss 2 (DEC-29) and the 4x4 truth. Arms:
  - centre scheme, exact jet (`3`) or neighbour-stencil jet (`n3`), DE flag `tau`;
  - `CORNER="m...;tau0..."`: the corner scheme with the consistency flag (`QUANT=1`
    rebuilds the real samples from their stored columns only);
  - `SS4=1`: virtual ss 4 against the 4x4 truth;
  - `--seq DUMP...`: the flicker test on a zoom sequence.
  - Env `MAXIT` is the iteration count charged to an unresolved sample (default 50000).
- `zonejet WIDTH [M...]`: end to end on the zone fast path. It renders the v0 frame with
  `fd_kernel::render_zone` at 1920x1080 ss 2 and rebuilds three of each pixel's four samples
  from the stored columns of the fourth.
- `costbench REF WIDTH MAX_ITER`: per-iteration cost of the exact third-order jet against
  z + dz/dc.
- `run_views.sh SCRATCH`: renders every view in `views.txt` (480x270 crops at the frame
  centre, with ss4).
- `run_seq.sh SCRATCH NAME RE IM WIDTH MAX_ITER FRAMES`: zoom sequence at 0.5 decades/s and
  30 fps (320x180 crops), then `score --seq`.

Build: `cargo build --release` (needs `pip install gmpy2` for refgen.py).
