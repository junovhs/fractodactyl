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

## GLSL baseline compiler

`zone-smoke` needs `glslc` on PATH or `FD_GPU_ROUTE_GLSLC` pointing to an existing
executable (on Windows, `glslc.exe`). The registered Linux check uses the compiler
already built by the product's shaderc dependency; it does not rebuild shaderc.
The probe compiles the original `zone_f64.comp` source with only `main` replaced
by an identical-checkpoint entry point. Finish/guard/encoding math stays in the
original shader. The source is piped to the compiler and SPIR-V read from stdout,
with no generated source or binaries inside the checkout.

The native f64 encoders need `SHADER_INT64` in addition to `SHADER_F64`: CubeCL
extracts a binary64 exponent through bits; pinned Naga's f64 `frexp` generates
an i64 exponent. All GPU arms emit the same packed 16-byte columns on the device: f64 nu,
f32 DE and u32 normal/class. GPU terminal iteration counts are unavailable in
this packed format; the report labels that diagnostic explicitly.

## Remaining comparison

The harness is tracked independently as GPU-15. GPU-12 still needs the RTX 3050 Ti
Windows results and the final source-route verdict. Linux 1080p results at all
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
