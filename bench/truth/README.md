# Frozen oracle truth pack v1

Run the current production CPU kernels against 5,120 independent frozen observations:

```
cargo run -p fd-kernel --release --example truth-pack -- bench/truth/v1 8
```

The reusable loader/comparison lives in `crates/fd-kernel/examples/support/mod.rs`.
This development surface is available to the future `fd probe`; it adds no product
command. Pack loading, reference/table setup, plain and requested BLA routes, and
mandatory exact-center rechecks are all included in the reported comparator time.
Compilation and oracle generation are excluded. On Ryzen 9 3900X/Linux, 8 workers,
the initial full comparison took 0.0394 s (112,199 actual BLA blocks).

## Frozen selection and semantics

Each case has 1,024 fixed sample positions; no failing positions were removed:

| Case | Exact camera / grid | Iteration cap | Independent census |
|---|---|---:|---|
| exterior | 0.5 + 0.5i, width0.1, 32x32 | 200 | 1024 escaped |
| filament | -0.7453 + 0.1127i, width0.0065, 32x32 | 2000 | 994 escaped,30 finite-cap nonescape |
| period764 | Exact center from `bench/path-atlas-v0.txt`, width2e-49,32x32 | 8000 | 883 escaped,141 finite-cap nonescape |
| parabolic | 0.25 + 0i, width1e-3,1024x1 | 2000 | 509 escaped,512 analytic Interior,3 finite-cap nonescape |
| extreme | 0 + 1i, width1e-300,32x32 | 4000 | 1024 escaped |

The filament/minibrot views target difficult boundaries. The near-parabolic case
samples the real axis deliberately: a square grid misses its very thin escaping
channel. All grids use ss1, rotation0, escape radius1e10, and exact decimal cameras.

TSV observations record class evidence, escape iteration, nu, DE in output pixels,
normal radians, and DE/normal variation rates per output pixel. `E` means escaped
within the cap. `A` means the independent analytic main-cardioid/period2 test passed.
`U` means only nonescape by the finite cap; it is **not** a claim of set membership.
Production Interior versus Unresolved for `U` is counted separately; either is
nonescape evidence. An escape/nonescape disagreement remains a raw mismatch.

The comparator uses tools/oracle.py's existing 1e-3 pixel displacement gate, with
1e-2 relative DE/1e-3 radian normal raw tolerances and storage quantization allowance.
These are regression tolerances, not certification. Sparse truth does not replace
DEC-17 whole-frame promotion checks. No renderer/kernel tolerances are changed.

BLA is actually executed on supported tiers, with block usage reported per case.
Current scaled rendering has no BLA implementation (FIX-03): extreme explicitly
reports `bla-unavailable-plain-fallback`; this is **not** scaled-BLA coverage.
The same frozen case is ready to check that future implementation.

## Two explicit finite-precision edges

Raw automatic-F64 comparisons have **two** mismatched filament points, in both plain
and BLA (four reported observations). They are preserved, not rounded into truth:

| Index | Exact sample center | Oracle escape | Oracle DE(px) | Actual F64 parameter placement shift(px) |
|---:|---|---:|---:|---:|
|318| -0.7423546875 +0.1140203125i |1628|5.052427416275048e-15|1.865674617339006e-13|
|988| -0.7427609375 +0.1097546875i |1371|1.444043873387348e-18|1.868415124508964e-13|

Both automatic routes return **Unresolved**, not Interior. The exact decimal oracle
agrees at depth+160,+416,+672 bits. Exact-center Fixed plain/BLA rechecks of the whole
filament grid give zero class/scalar mismatches. Rust Plane input bits match an
independent exact-binary reconstruction. Parameter shifts exceed local DE estimates
by about37x/129,000x, but DE is not a rigorous distance bound. Moreover, directly
iterating the rounded inputs still escapes at1992/1583. Thus placement alone does
**not** explain the automatic results: reference arithmetic/finite precision remains
unresolved. These are empirical precision-budget edges, not a certified error proof.

Only the two versioned `precision-edges.txt` identities are recognized: exact camera,
grid, cap, oracle class/iteration, actual binary center/offset bits, and observed
OracleEscaped→ProductionUnresolved pattern. Raw counts and diagnostics always remain
visible. Every run must also pass both whole-filament Fixed rechecks. A changed
identity/pattern, any new class mismatch, or any Fixed/scalar failure fails the check.
There is no general DE-based exception and no production admission change.

Reproduce the precision investigation:

```
python3 bench/truth/diagnose.py
cargo run -p fd-kernel --release --example truth-diagnose
```

## Generation and provenance

`generate.py` imports the independent direct iteration and analytic tests from
`tools/oracle.py` (mpmath), with no production renderer, perturbation, BLA or saved
reference orbit. Observations use depth+160 bits; 33 fixed positions per case are
rechecked at depth+416 bits. The two sensitive edges have the additional checks above.
Generation took53.5 s with an eight-worker Ishoo lease; comparison needs no Python.

```
python3 bench/truth/generate.py --out /external/output/v1 --workers 8
python3 bench/truth/generate.py --out bench/truth/v1 --workers 8 --validate
```

The pack records exact camera strings, precision, sample indices and iteration caps.
`provenance.json` records Python/mpmath versions and oracle/generator source hashes;
`data-sha256.json` fixes the five data-file hashes. Data uses schema `fd-truth/1`.
Treat v1 as frozen: changed selection/semantics/oracle provenance belongs in a new
version directory, reviewed with its own numerical results.

The actual surface check takes explicit compiled-binary and external-output paths:

```
TRUTH_BIN=/external/build/release/examples/truth-pack OUT=/external/evidence bash scripts/check/TRUT-01.sh
```

It checks provenance, all routes and runtime, and proves six deliberately corrupted
class/scalar/edge inputs fail through the compiled comparator itself.
