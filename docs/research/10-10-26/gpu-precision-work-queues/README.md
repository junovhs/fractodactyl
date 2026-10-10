# Fractodactyl hardware architecture: experimental GPU queue ablation

This is an **unmeasured test prototype**, not an established whole-frame GPU speedup.

## Source and reproducibility

It is a drop-in PyOpenCL benchmark that imports `gpu_bench.py` from the checked-out `junovhs/fractodactyl` repository (default branch `main` as inspected October 10, 2026). It uses the *exact* existing OpenCL kernel as one arm. The second arm retains the same prefix, Koenigs jump, polynomial chart, and floating-point recurrence, except that it executes a fixed number of tail iterations, atomically appends surviving pixels into a compact queue, and completes their tails in a second kernel. **No precision or mathematical-iteration changes.**

Place `gpu_queue_ab.py` in `tools/research/misiurewicz/koenigs_bench/`, next to `gpu_bench.py`, then use the same constants as `gpu_bench.py`:

```bash
python gpu_queue_ab.py \
  /path/to/zone-consts.json /tmp/gpu-queue-ab 1920x1080 7 \
  1e-35 1e-38 1e-40 1e-43 1e-46 2e-48 \
  --chunks 4,16,32,64
```

Note: the constants argument must point to the existing BENC-08 loadable constants file, **not** necessarily a JSON file; use whatever argument works with `gpu_bench.py` locally. The example path is illustrative.

Set `DEVICE=AMD` or `DEVICE=NVIDIA` to choose the OpenCL GPU. `pyopencl`, `numpy` and the existing repository scripts are prerequisites. The script deliberately validates that both kernels produce identical class and `nu` results for each whole frame before considering timings; any difference aborts.

The CSV contains per-frame median end-to-end device-submit/synchronize time, compacted queue fraction, raw timings and baseline speedup. One-time compilation and host readback are excluded from the timed section. The extra host synchronization required by compaction **is included**. This experiment answers whether tail-queueing, by itself, beats the repository's already optimized FP64 GPU kernel. It does **not** change reference construction, rendering precision or mathematical skip counts.

## Decisive benchmark criteria

* Use the existing FP64 GPU kernel—not CPU BLA—as the primary comparison on the same card, resolution, scene, compiler flags, clock regime and emitted fields.
* Score every pixel of each whole frame, not samples. Compare classes, escaped smooth `nu`, relative distance estimate (`de`), and normal direction against a frozen high-precision truth set. Score the `nu` shift as `|delta_nu| * de_truth * ln(2) / 2` in pixel units under the current project convention. Reject nonfinite numbers, ambiguous sign/underflow and unexpected differences. Validate difficult near-boundary points at high precision.
* **This particular BENC-08 ablation only outputs class and nu.** Bitwise-equivalent output establishes that queue scheduling did not change its output on the tested device. It does NOT establish a strict full Fractodactyl `nu/de/normal` contract. A second benchmark must add full output columns to both GPU arms and use a strengthened comparator, especially because the present `fd compare` lacks nonfinite and deep-width protections (see `docs/spec/STATE.md`).
* Gate 1: a queue-only, bitwise-equivalent GPU frame speedup >1.25x on multiple frames, including launch and synchronization overhead. If it is below this, kill queue splitting or tune chunk sizes rather than layering adaptive precision on top.
* Gate 2: add an FP32x2 (compensated `float2`) tail and early-fallback path; demand class equality, no pixel displacement >1e-3, error-safe `de` and normal, measured complete-frame throughput >1.5x versus *optimized GPU baseline* (predeclared substantial threshold), and no severe regressions on hostile scenes.
* Gate 3: reproduce at least on a contemporary NVIDIA consumer GPU and an AMD consumer GPU. The best precision mix is vendor/device-dependent; neither native FP64 ratio nor FP32 expansion cost is uniform.

## Precision controller to implement after the scheduling gate

At each operator, maintain an upper estimate `e` of absolute complex-state error from previous operations and a stable derivative `D = dz/dc` or `D_pixel = h*D`. For a quadratic step:

`e_next <= (2*|z_approx| + e)*e + e_c + rounding_bound`.

For a return map `R`, `e_next <= sup|R'| * e + e_operator` on the validated state disk. For a Koenigs jump `K(lambda^j phi(...))`, multiply incoming error by a local derivative bound (including the jump-count and parameter dependence), then add truncation and evaluation errors. Use compensated evaluation near cancellation and guard patch-domain acceptance. Compare predicted error with a reserved pixel budget `e/|D_pixel|` and monitor an independent `nu`/`de` output error allowance. Conservatively escalate if derivative vanishes, interval straddles a bailout/interior domain, a recurrence diverges, exponent normalization loses low limbs, or the budget is exceeded.

This is a **screening model**, not a proof until the rounding terms and derivative/radius bounds are validated and applicable to the actual GPU compiler/FTZ/FMA semantics. The controller must route unsafe pixels to unmodified FP64, with re-execution from a checkpoint that predates the inaccurate operation.

## Measurements that are already known, not new

Repo `docs/spec/METHOD.md` records for the RTX 3050 Ti Laptop GPU BENC-08 at 1920x1080, six v0 deep frames, all-FP64 `82–131ms` per frame, zero pixel errors over 12.4M samples in its class/nu test, max `1.6e-4 px`. The PATCH32 arm had 7–274,523 wrong pixels per frame, max 3.5 px, and little gain. Mid-band FP32 stage probe F (`docs/research/10-9-26/binary32-precision-analysis-of-the-mid-band-pixel.md`) found all-float32 finishes can be ~0.09 px off and sampled combinations of floats and doubles still occasionally exceeded 1e-3. Do not extrapolate a speedup from these.

## What is not established

No OpenCL/CUDA GPU is available in the present evaluation environment, so this script has not rendered or timed a GPU frame here. Python syntax and an extracted OpenCL prototype syntax were checked locally, but **the complete kernel has not been compiled against a real GPU OpenCL driver**. Benchmark numbers will only be valid once run on a target GPU and checked against the truth pack.