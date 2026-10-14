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
- `zone-smoke`: finish eight real CPU-zone landing states through the shared Rust
  recurrence on both CPU and GPU, with at most 20,000 iterations per sample.
  Compare terminal state/iteration/class; full-frame column parity is pending.

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

## Remaining comparison

The full probe still needs finish output encoding, binary64/guarded-f32
variants, an ash baseline fed the same checkpoints, dispatch and end-to-end
medians of three, whole-frame class/nu/DE/normal gates and results from **both**
the RX 5700 XT and the RTX 3050 Ti. Do not pick SHARED-SOURCE or WGSL from these
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
