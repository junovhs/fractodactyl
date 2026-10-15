# GPU source route probe (GPU-12)

This standalone, unshipped research crate prepares the finish-stage route comparison
for the Foundation plan. It does not port the product renderer.

Current commands:

- `capabilities`: enumerate hardware Vulkan adapters and print `SHADER_F64` and
  subgroup support. Parse a minimal f64 WGSL shader and attempt a live storage/arithmetic pipeline with validation errors captured.
  Software adapters are excluded. This command submits no compute dispatch.
- `smoke`: compile one scalar Rust recurrence for the CPU and CubeCL's wgpu
  SPIR-V backend, then compare eight synthetic samples after eight iterations.
  Input/output buffers total 640 bytes. This is a compiler/runtime prerequisite,
  **not** zone-frame parity or performance evidence.
- `capture`: capture real CPU-zone finish checkpoints on 16x9 grids at six
  GPU parity depths, and confirm the capture pass retains the ordinary renderer's
  exact class, nu, DE and normal columns. No GPU dispatch.
- `zone-smoke`: compare every captured finish input on 16x9 grids at six depths,
  plus a rotated 1e-40 view. Run the shared Rust recurrence and binary64 log/atan
  encoder on CPU and CubeCL/wgpu SPIR-V, the hand-written WGSL arm, and the
  existing ash/GLSL finish in binary64 and guarded-f32 modes. All arms receive
  the same production checkpoints, with at most 20,000 iterations per sample.
  Fail on class differences, displacement >1e-3 px, non-finite output, or DE
  relative error >0.2% / normal error >0.2 degrees outside the reference's
  1e-3-pixel boundary band, matching `fd compare`. Report boundary differences
  separately. Escape-count changes are diagnostic (unavailable from the original
  ash shader). No 1080p or performance verdict is implied.

- `ff-selftest`: GPU-09 float-float arm. On one adapter, checks that TwoSum,
  FastTwoSum, the fma error term and TwoProd are exact, and that one
  float-float z^2 + C step is within 2^-44, for both TwoProds (fma, Dekker),
  scored against binary64 on the host (65,536 cases). Prints a verdict per
  TwoProd; it never fails the command.

Every command has a 30-second runtime cutoff. Building is separate and can take
longer. No command implicitly renders a film or runs a full-frame benchmark.

## Toolchain and safe builds

Pinned Rust 1.95.0, CubeCL 0.11.0, wgpu 30.0.1 and Naga 30.0.0. Dependencies are
locked independently of the product workspace. CubeCL requires Rust 1.95.

On either OS, check available RAM and other running work before building. Keep
Cargo to one worker, keep output outside the checkout, and run one command at a
time. Windows needs its normal Rust MSVC build tools and a Vulkan-capable driver.
The RTX 3050 Ti laptop is a separate hardware target from the Windows test VM.

For a local manual build from this directory:

```text
cargo +1.95.0 build --locked --jobs 1 --target-dir PATH_OUTSIDE_CHECKOUT
```

On Linux, the registered Ishoo checks wrap builds in `systemd-run` with a 4 GiB
memory limit, zero swap, one CPU quota and a 180-second hard cutoff. They retain
the build cache outside the checkout; a cutoff is reported, not bypassed by
raising the limits. Once built, run `gpu-route-probe capabilities`, then the
other tiny commands serially. Check GPU availability before `smoke`.

## Windows/Linux host prerequisite checks

The `gpu-route-build` GitHub Actions workflow builds this standalone crate with
Rust 1.95.0, runs its CPU tests, and captures a 64x36 checkpoint preview on
Windows and Linux. It runs the OS jobs serially with one Cargo/test worker;
build and capture files stay under the runner's temporary directory outside
the checkout. Pushes changing the probe, its kernel dependencies or this
workflow trigger it. Once landed, a manual run is available:

```text
gh workflow run gpu-route-build.yml --ref main
```

These hosted checks establish build and CPU-capture portability. GPU-12 still
requires hardware parity and timing on the RX 5700 XT and RTX 3050 Ti. The
workflow never dispatches a GPU kernel or scores a software adapter.

## RX 5700 XT check on the self-hosted runner

`gpu-route-rx5700xt` runs on the owner's `homebox-linux` runner (labels
`self-hosted, gpu, rx5700xt`). It is manual only, because the repository is
public and the runner is a home desktop: no push or pull-request trigger.

```text
gh workflow run gpu-route-rx5700xt.yml --ref main
```

It builds the dev profile with one Cargo worker. Build and capture files stay
under `~/.cache/fractodactyl-gpu-route-ci`, and each probe command runs in a
`systemd-run --user` scope (4 GiB, no swap, one CPU), or under `nice -n 19` when
there is no user systemd bus. It runs `capabilities`, `zone-smoke`, and then
`capture-frame` plus `frame <width> wgsl f64` at all six depths. The run fails
unless every binary64 row reports `parity=PASS`. It then runs `ff-selftest`
(guarded and unguarded) and `frame <width> wgsl ff` at all six depths; ff rows
must complete, but their parity verdict is a recorded result, not a job gate.
`glslc` comes from the optional `glslc`
dispatch input, then PATH, then the product shaderc build under the runner's
home. Logs are uploaded as an artifact. Timings in these logs are informational;
METHOD.md tables are not replaced by them.

## GLSL baseline compiler

`zone-smoke` needs `glslc` on PATH or `FD_GPU_ROUTE_GLSLC` pointing to an existing
executable (on Windows, `glslc.exe`). The registered Linux check uses the compiler
already built by the product's shaderc dependency; it does not rebuild shaderc.
The probe compiles the original `zone_f64.comp` source with only `main` replaced
by an identical-checkpoint entry point. GPU-13 ported the product kernel to WGSL
and deleted the GLSL from `crates/fd-cli`; `src/zone_f64.comp` is a frozen copy
of it (main at 3df7fd0), kept only as this research arm's baseline (DEC-24 rule 4). Finish/guard/encoding math stays in the
original shader. The source is piped to the compiler and SPIR-V read from stdout,
with no generated source or binaries inside the checkout.

The native f64 encoders need `SHADER_INT64` in addition to `SHADER_F64`: CubeCL
extracts a binary64 exponent through bits; pinned Naga's f64 `frexp` generates
an i64 exponent. All GPU arms emit the same packed 16-byte columns on the device: f64 nu,
f32 DE and u32 normal/class. GPU terminal iteration counts are unavailable in
this packed format; the report labels that diagnostic explicitly.

## Remaining comparison

The harness is tracked independently as GPU-15. GPU-12 recorded the RTX 3050 Ti
Windows rows and the verdict, **WGSL**: shared Rust/CubeCL is within 1.25x of the
fastest arm on the 3050 Ti but 1.84-2.25x on the RX 5700 XT, because of host
transport rather than compute. See METHOD.md under "GPU-12: RTX 3050 Ti rows".
Pinned Naga's f64 `frexp` crashes the NVIDIA Windows pipeline compiler, so both
WGSL sources extract the exponent from `bitcast<u64>` bits (bit-identical here). Linux 1080p results at all
six depths are in METHOD.md: all binary64 arms pass; every mixed arm fails
DE/normal parity, with class mismatches also found at 2e-49. Mixed rows are
ineligible for a speed claim even when their pixel-displacement test passes. Do not pick SHARED-SOURCE or WGSL from these
prerequisites. The pinned Naga/wgpu stack accepts a native f64 WGSL pipeline on the RX 5700 XT
Vulkan adapter, despite the published support matrix describing f64 as SPIR-V
only. This native extension does not establish WebGPU portability. Probe the
actual target pipeline rather than infer eligibility from parser acceptance or
documentation alone. A lossy f32 replacement is not equivalent arithmetic.

The `gpu-route-probe` feature exposes CPU checkpoints only for research. Its
capture hook observes the existing zone pipeline; product builds retain the
same arithmetic and do not enable the capture API. The capture is limited to
the ordinary-number deep zone, ss=1, at most 1080p and 20,000 iterations.

References: [CubeCL support matrix](https://github.com/tracel-ai/cubecl/blob/v0.11.0/cubecl-book/src/core-features/features.md),
[wgpu SHADER_F64](https://docs.rs/wgpu/30.0.1/wgpu/struct.Features.html#associatedconstant.SHADER_F64).


## Whole-frame commands

Run from the repository checkout (Git binds inputs to its exact source tree):

```text
gpu-route-probe capture-grid 1e-40 64 36
gpu-route-probe grid 1e-40 64 36
gpu-route-probe capture-frame 1e-40
gpu-route-probe frame 1e-40 shared f64
gpu-route-probe frame 1e-40 wgsl mixed
gpu-route-probe frame 1e-40 ash f64
```

`capture-frame` freezes 1920x1080 production checkpoints and the CPU-zone `.fds`
reference once. All arms reuse these files, without rebuilding front stages or
references. `FD_GPU_ROUTE_CACHE` must point to an absolute directory outside the
checkout; the default is under the OS temporary directory. Cache filenames bind
the Git source tree, width and dimensions. Incomplete files are not committed
as reusable inputs. Use one of 1e-37, 1e-40, 1e-43, 1e-46, 2e-48 or 2e-49.

Omit arm/precision to run every row on a cached frame, or choose `shared|wgsl|ash`
and `f64|mixed` to keep a hardware check smaller. Warm-up is excluded. Each row
uses three GPU-timestamp runs and three **separate unprofiled** wall-clock runs.
`dispatch_ms` is the compute-pass median; `finish_frame_ms` measures allocation,
upload, dispatch and readback of encoded columns for the entire finish stage.
Reference capture, comparison, diagnostic decoding, shading, resampling and
encoding a film are excluded. These are not product film frame times.

`frame_probe=COMPLETE` means the experiment ran successfully, not that every
candidate passed parity. `frame_result ... parity=FAIL` is a measured negative
result, not an eligible timing. Invocation/driver/cache failures exit nonzero;
completed experiments exit zero and leave the source verdict pending.

Every command retains its 30-second runtime limit, including explicit 1080p
commands. Run preview first, and keep all resource limits from the safe-build
section. Native GPU timestamp queries are required for the timed WGSL path;
absence is an error, never reported as zero time or replaced with CPU timing.


`host_stages` for the shared arm reports CPU allocation/upload preparation,
launch, and submission/readback (including pending GPU work). The parts come
from the run with median whole-stage wall time, so they add to that frame time;
they are not independent per-component medians. GPU-16 switches this arm to
`Client::create(Bytes::from_elems(...))` for typed owned inputs, avoiding the
redundant slice clones and erased-byte alignment reallocation. Kernel maths,
packed columns, GPU timestamps and parity thresholds are unchanged.

## Corrected six-depth Linux comparison

After the typed upload fix, all six 1080p binary64 shared/WGSL pairs pass the
existing class/displacement/DE/normal gate (jobs 200–217, source tree
536ed33782806882fb6bedc03781d15d9cd20bec). Timing uses separate GPU-timestamp
and unprofiled wall medians of three, with warm-up excluded.

| Width | Shared GPU dispatch (ms) | WGSL GPU dispatch (ms) | Shared whole stage (ms) | WGSL whole stage (ms) | Shared / WGSL whole stage |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1e-37 | 35.425 | 33.364 | 104.736 | 55.985 | 1.87x |
| 1e-40 | 22.188 | 20.633 | 90.639 | 45.329 | 2.00x |
| 1e-43 | 35.686 | 32.214 | 113.190 | 54.099 | 2.09x |
| 1e-46 | 36.240 | 34.035 | 117.507 | 57.454 | 2.05x |
| 2e-48 | 39.331 | 41.234 | 112.848 | 61.277 | 1.84x |
| 2e-49 | 37.160 | 37.134 | 108.748 | 48.354 | 2.25x |

The shared compute pass is within 1.11x of WGSL; its complete stage remains
1.84–2.25x slower. These replace the original shared-client timings as the
current Linux comparison. Full parity counts, boundary-band caveats and source
provenance are in METHOD.md under GPU-18. The previous mixed precision failures
remain disqualifying. GPU-12 combines these with the RTX 3050 Ti rows for the WGSL verdict.

## Float-float arm (GPU-09/GPU-05)

```text
gpu-route-probe ff-selftest
gpu-route-probe frame 1e-40 wgsl ff
FD_GPU_ROUTE_ADAPTER=iris gpu-route-probe frame 1e-40 wgsl ff
```

`frame <width> wgsl ff` runs the float-float finish (`src/ff.wgsl`), a port of
`finff` in `tools/research/twolevel/zone_finish/zfin.c`, on the same frozen
inputs and parity gate as the other arms. The kernel has no f64: naga
validates it with no capabilities (`ff_wgsl_validates_without_f64`), and the
device is opened with only `TIMESTAMP_QUERY`, so the arm also runs on adapters
without `SHADER_F64` (Intel Iris Xe). Without a CubeCL f64 client, `frame ...
wgsl ff` never needs an f64 device.

- **Arithmetic.** z is float-float (TwoSum adds; TwoProd by fma or Dekker);
  dz/dc is an f32 mantissa with an i32 exponent, renormalised every step.
  Escape at |z|^2 > 1e20 on the hi words; at most 20,000 iterations.
- **Host.** The host stays binary64. It rounds each landing state to
  float-float (hi = f32(z), lo = f32(z - hi)) and dz/dc to mantissa/exponent,
  directly into the mapped upload buffer. This conversion is inside the timed
  `finish_frame_ms`.
- **Encoding.** nu, DE and the normal are encoded in f32 on the device, with
  log2/atan2 from the IEEE bits and short polynomials (no transcendental
  builtins, no frexp/ldexp). nu leaves as an exact (integer, fraction) f32 pair.
  The host sums the pair in binary64 after the clock stops, as the other arms
  exclude column decoding. Output is still 16 bytes per input.
- **Reassociation guard.** Every TwoSum/FastTwoSum intermediate and the TwoProd
  product pass through `opq`, an XOR with a runtime zero from the camera
  buffer. Without it, the NVIDIA and Intel drivers rewrite the error terms to
  zero and parity fails badly. `FD_GPU_ROUTE_FF_NOGUARD=1` strips the guard, for
  diagnosis only.
- **TwoProd selection.** Vulkan lets a driver split `fma` into a multiply and
  an add, and RADV (RX 5700 XT) does. At start-up the runner self-tests the fma
  TwoProd on the device and uses it only if it is exact; otherwise it uses
  Dekker/Veltkamp splitting. Each row prints the choice as `two_prod` and
  `two_prod_selection` (`auto`, `auto_fma_not_fused` or `forced`).
  `FD_GPU_ROUTE_FF_TWOPROD=fma|dekker` forces one.
- **Adapter.** `FD_GPU_ROUTE_ADAPTER=<name substring>` (case-insensitive) picks
  the adapter; otherwise a discrete GPU is preferred. Software adapters are
  excluded.
- **Diagnostics.** `FD_GPU_ROUTE_FF_STAGES=1` prints each run's host stages to
  stderr. `FD_GPU_ROUTE_DUMP_CLASS=1` (any arm) prints every class mismatch
  with its frozen landing state.

`frame_result` lines for every arm also carry `report_only_de_wrong_ge1px` and
`report_only_normal_wrong_ge1px`: DE/normal failures counted only where the
reference DE is at least 1 px. These are report-only. The pass/fail contract
(class, displacement <= 1e-3 px, DE <= 0.2% and normal <= 0.2 degrees outside
the 1e-3 px boundary band) is unchanged.

Results (RTX 3050 Ti, Iris Xe, RX 5700 XT) and the GPU-05 kill-line verdict
are in METHOD.md under "GPU-09/GPU-05: float-float finish arm".
