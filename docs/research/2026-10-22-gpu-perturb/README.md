# GPU-31: bounded generic perturbation checkpoints

This is GPU-11's numerical foundation, **not a new film route or a speedup**.
`fd film` without a zone still uses the CPU renderer. GPU-11 retains its original
≥4× per-frame and two-device film-equivalence targets.

`shaders/cf.wgsl` is the single complex f32-mantissa/shared-i32-exponent library,
now consumed by the existing production zone shaders and the generic stepper.
`shaders/perturb32.wgsl` exposes `PerturbState` and `perturb_step`: the caller
supplies the CPU-computed reference points Z[m], Z[m+1], exact-camera offset in
mantissa/exponent form, and whether this is the reference's final step. The state
returns delta, reference index, iteration count, rebase count and terminal status:
0 active, 1 radius-2 escape, 2 invalid input/arithmetic/count, 3 exponent-range
refusal. Nonzero statuses stay terminal. Inputs must be zero or normalized finite
mantissas, with exponents in [-16000,16000]. This is a supported diagnostic range,
not a claim that every such orbit is accurate. Reference-array bounds belong to
the caller; the dispatch test asserts invalid-index refusal explicitly.

Reproduce on the selected adapter (`FD_ADAPTER` works as for the renderer):

```sh
cargo test -p fd-cli --release gpu_perturb -- --include-ignored --nocapture
FD_REQUIRE_GPU=1 FD_ZONE_KERNEL=f32only cargo test -p fd-cli --release --test zone_fixtures zone_fixtures -- --nocapture
cargo +1.90.0 clippy -p fd-cli --all-targets --locked -- -D warnings
```

The first command must dispatch; missing adapters fail rather than silently pass.
It requests no optional device features and validates WGSL with no optional Naga
capabilities. Fixed CPU references come from five exact decimal cameras (shallow
seahorse, cusp, deep valley, i, tip), three pixels and checkpoints 0/32/128 where
the CPU has not already escaped, followed by eight GPU steps. Separate cases
assert exponents outside f32 and f64 physical range and both range boundaries,
NaN and invalid reference index. The CPU calculation is test-only reference math.

The local diagnostic budget is prescribed before measurement, with u=2^-24:
`e' = (2|Z| + 2|delta| + e)e + 64u(2|Z||delta| + |delta|² + |dc|)`.
Initial-state quantization and rebase addition also receive 64u times their input
magnitudes. The fixed factor covers the small sequence's f32 operations and input
quantization; it is not fitted to observed errors and is **not a certified bound**.

On RX 5700 XT (RADV), the first 42-camera-checkpoint run passed, worst
error/local-budget ratio 0.01546. However, the tip's late delta error divided by
initial sample spacing reached about 1e74. That ratio is **not a screen-space
position error** (which requires the derivative), and a passing local budget
never authorizes DEC-17 output. No render or film timing was measured.

Before production frame admission, GPU-11 must add derivative/escape encoding,
interior/unresolved semantics, accumulated error in output-pixel units, a saved
pre-loss checkpoint and CPU restart, then whole-frame and film proof against the
best current CPU route. LA and profitable route selection follow that guarded
frame path. The RTX 3050 Ti and no-f64 hardware remain untested here; requesting
no f64 on RX is capability independence, not proof on every device.
