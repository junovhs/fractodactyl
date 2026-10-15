# Zone-finish probe: two-level float32 and float-float finish (GPU-09 design input)

Notes and conclusions: `docs/research/2026-10-14-q4-two-level-float32-references-notes.md`, Part B.
This is CPU only: it emulates GPU float32 arithmetic and is not a GPU measurement.

- `capture/` is a standalone Rust tool, outside the workspace. It calls
  `fd_kernel::Zone::probe_finish_inputs` (the `gpu-route-probe` feature, as GPU-12 does) and
  dumps the real v0-zone finish checkpoints: landing z, dz/dc and iteration, plus the shared
  finish parameter C and the pixel size.
  - usage: `zone-finish-capture WIDTH NX NY OUT.zfin`
  - At 1920x1080 the six GPU-12 depths give 12,169,808 finish inputs, the same count as
    GPU-12/15.
- `zfin.c` scores every pixel against the binary64 finish (production arithmetic) with these
  arms:
  - `ff`: float-float z (2×binary32, about 48 bits) with binary32-mantissa+exponent dz/dc.
    This is the f32-only path that DEC-25/GPU-05 allow.
  - `tl`: two-level float32 finish.
    - Per T×T tile, a float-float reference orbit W starts from one pixel's landing state.
    - Each pixel iterates e' = 2We + e² in binary32 mantissa+exponent. There is no c term,
      because C is shared by the fixed-C tail.
    - It rebases onto the critical orbit of C when |z| < |e| or when W ends.
    - A variance guard sends flagged pixels to the `ff` fallback.
  - Kept-only stats are equivalent to an optional native-binary64 fallback arm.
  - Two DE contracts are scored. A: DE/normal gated where DE_ref > 1e-3 px (current).
    B: gated where DE_ref ≥ 1 px.
  - Env vars: `LAMDE=` sets λ for the DE rule; `FFD64=1` gives the ff arm a binary64 dz/dc
    (diagnostic); `DUMP=1` and `DUMPFF=1` print failing pixels.
- `run.sh SCRATCH` reproduces everything (about 6 minutes on 4 cores).
- `results/zfin-1080p-final.log` is the final run.
  `results/zfin-1080p-before-gate-margin-fix.log` is the earlier run whose DE gate had no
  margin (kept a few artefacts; superseded). `results/ff-contractA-failures-2e-48.txt` lists
  the 354 float-float contract-A failures at 2e-48.
- `capture/src/bin/shadecmp.rs`: the shading proof (§22 of the notes).
  - usage: `shadecmp WIDTH DUMP LOOKSDIR`, where DUMP comes from `zfin` with `DUMPOUT=`.
  - It renders the binary64, float-float, hybrid and double-double (truth) frames with every
    look, and reports changed pixels and colour levels by distance to the edge.
  - `FFGUARD=16` turns the hybrid slot into float-float plus a targeted re-run (re-run share
    printed); `DNU=` sets the Δν threshold, default 0.01.
  - Results: `results/shading/`.
