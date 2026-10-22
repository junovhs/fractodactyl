# GPU-33 guarded affine blocks

This explicit experimental `fd render/control --gpu-perturb bla` route is Heuristic,
uses GPU-32's unchanged final output guards and original plain CPU zero-start fallback,
and never selects a default/film route. `on` remains the no-LA control; `off` is default.
It is actual LA integration, not a claim of beating the fastest landed CPU route.

## Shared contract

`GuardedBla` reuses the ordinary CPU `Block` layout/alignment and one nominal affine
composition implementation. It does not change serialized atlas BLA or CPU defaults.
For mathematical reference point Z, stored reference W with error q, and |delta|<r,
the exact-reference perturbation is `(2Z+delta)delta+dc`, while the parameter derivative
has multiplier `2Z+2delta` and additive1. Thus the single-step state coefficient error
is `r+2q`, and derivative coefficient error is `2r+2q`; neither is obtained by
formally differentiating the state remainder bound. Every participating reference
point has `|W|+q<=2`; epsilon remains2^-50, below the existing2^-41 cap.

For x then y, nominal `A=Ay Ax`, `B=Ay Bx+By`. An operator envelope `(a,b)` means
`|true_result-(A input+B forcing)| <= a|input|+b|forcing|`.
Both state and derivative envelopes compose with the same triangle inequality:

```
a = |Ay| ax + ay (|Ax|+ax)
b = |Ay| bx + ay (|Bx|+bx) + by
```

The derivative forcing is1, not dc. L1 coefficient magnitudes are upper bounds on
complex Euclidean magnitudes. State envelopes shrink the common radius with
`min(rx,(ry-(|Bx|+beta_x)dc_max)/(|Ax|+alpha_x))` so all intermediate exact deltas
remain in their single-step domains. Bounds receive64epsilon inflation and a tiny
normal underflow floor. Host binary64 composition separately charges32epsilon times
the L1 magnitudes of its product/sum terms, covering the small fixed RN operation
sequence without dividing by a cancellation-prone output. This is a CPU RN numerical
envelope, not a new certified or portable GPU arithmetic guarantee.

Input uncertainty e must satisfy `|delta|+e<r`; parameter magnitude plus input error
must fit dc_max. Lookup preserves the existing longest aligned2^j blocks and stays
within reference/iteration budget. Radii are packed inward (1e-4 margin; dc_max1e-5),
error envelopes outward (1e-4); actual transport tests assert those directions.
Exact CE coefficient residuals are added separately to both paired envelopes.

With a=state alpha,b=state beta, da/db=derivative envelope, the shader charges
`(|A|+a)e + a|delta| + b|dc| + (|B|+b)rc`, and
`(|A|+da)ed + da|D| + db`, plus the existing conditional CE evaluation estimate.
That CE estimate remains empirical/conditional as documented in ce-error-bound.md;
runtime FF tests and exact-frame gates do not make it a portable certificate.

All skipped intermediate states stay below the escape radius (blocks require radius
at least4). A block's **landing** state can escape or require rebasing, so both plain
steps and blocks call the same `perturb_finish` immediately. Endpoint rebase transport
is charged into e. Incomplete dispatch, capability or final guard failures preserve
GPU-32's exact-camera original CPU restart; bound evidence requests decline entirely.
No lossy state is resumed on CPU.

## Reproduce and measured limits

```sh
cargo test --jobs 8 -p fd-kernel --release guarded_bla_direct_derivative_windows -- --nocapture
cargo test --jobs 8 -p fd-cli --release gpu_perturb -- --include-ignored --nocapture
cargo build --jobs 8 -p fd-cli --release
python3 scripts/check/GPU-33.py --fd target/release/fd --out target/gpu33-six
```

Run under a positive verification lease; recorded tests used8CPU workers and RX5700XT
RADV25.1.5. CPU probe798 checks1845 direct state/derivative windows, including583where
state-only envelopes understate derivative error. GPU windows808 check80cases across
a long valley reference and1+0i, with alignment/parameter/budget/end refusals and an
actual block landing atZ3=5/radius4 requiring immediate escape. The first window
harness incorrectly used a compile-time FF seed; checks802/803 caught f32-like loss.
Runtime storage seed (same as production) restores the intended reassociation guard.

Whole-frame805/810 use the same six cameras frozen in GPU-32.py; no tuning/fixture
change. Cusp/deep-seahorse/valley still fall back and that is reported.
GPU BLA is faster than the prior GPU path, **slower than CPU BLA**. Series/film timing has a different
pipeline/output boundary and is not a denominator here. Fresh corrected whole-frame
gates are required for landing; parentGPU11 still needs resident/profitability/film,
≥4× and two-device proof. No release is implied.
