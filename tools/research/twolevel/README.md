# Two-level references for float32 pixels (frontier question 4)

Notes and results: `2026-10-14-q4-two-level-float32-references-notes.md` (repo root); generated tables and logs in `results/`.

A CPU simulation of the GPU arithmetic. It is not a GPU run.

- `ref.py LOC LOG10W OUT.ref [NMAX]` computes, with gmpy2:
  - the exact centre: `eye` (eye-of-universe), `mis` (Misiurewicz M(24,2), off centre) or `nuc` (a nucleus near M(24,2) whose size is about W/20 to W/70);
  - the global reference orbit, rounded to binary64;
  - K, with pixel spacing h = 2^K for a 1920-wide frame.
- `tile32.c`, built with `gcc -O2 -fopenmp -ffp-contract=off -o tile32 tile32.c -lm`:
  - `tile32 truth REF NMAX S OUT` runs binary64 perturbation with explicit exponent and rebasing (the fd scaled tier). It covers 1/S of the 64x64 blocks of a 1920x1080 frame, chosen by hash.
  - `tile32 f32 REF NMAX S TILE TRUTH [centre|maxit] [tol] [bla]` builds one binary64 secondary reference per TILE x TILE tile. Each pixel then iterates its offset in binary32 mantissa x 2^k against that tile orbit, with rebasing, an optional per-tile BLA, a linear error bound and a variance error model.
  - `REC=file` writes per-pixel records. `ALT=1` adds a second float32 run on an offset reference (the cross-check experiment). `DUMP=1` prints the bad pixels.
- `check.py LOC LOG10W x,y ...` adjudicates pixels by direct iteration at about 3.3·digits + 400 bits.
- `analyze.py`, `project.py` and `summarize.py` handle guard sweeps, the RDNA1 operation-count projection and the report tables.
- `run_matrix.sh SCRATCH` runs the whole width x location x tile matrix (about 1-2 h on 4 cores).

Float32 is emulated with real C `float` operations (FMA contraction off, round to nearest). Exponents are carried explicitly, and every rescale is by an exact power of two. That matches a GPU float32 kernel using `ldexp`, except that GPUs flush denormals to zero, which only affects terms already below 2^-126 relative.
