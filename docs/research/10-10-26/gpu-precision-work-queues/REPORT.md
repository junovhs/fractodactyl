# GPU architecture and adaptive precision after mathematical acceleration

**Research date:** 2026-10-10  
**Project:** Fractodactyl  
**Status:** architecture assessment and unmeasured GPU scheduling prototype; **no new GPU speedup or 1e-3-pixel contract pass claimed**.

## Executive assessment

Assume Fractodactyl already removes all mathematical iterations that its validated return-map / Koenigs machinery permits. The remaining problem is how to execute that work efficiently on consumer GPUs.

**Recommendation:** a GPU-first renderer with precision-specialized operator kernels, coarse precision decisions at numerical checkpoints, and optional compaction of exceptional finishing tails. Keep CPU high precision for orbit/chart construction, validated constants, and exceptional fallback. Do not begin with a monolithic kernel that switches arithmetic precision on every operation.

**Critical result remains open:** there is no new timed, complete-frame prototype beating the optimized *GPU* baseline while meeting the 1e-3-pixel benchmark contract. The accompanying queue-only A/B script is an instrument for the first controlled experiment, not proof of a win.

## The fair baseline already exists

The repository has an OpenCL FP64 implementation of the deep return-map -> Koenigs jump -> tail chart -> direct finishing recurrence (tools/research/misiurewicz/koenigs_bench/gpu_bench.py). METHOD.md records BENC-08 on the **RTX 3050 Ti Laptop** at 1920x1080:

| Existing measurement | Recorded result |
|---|---|
| All-FP64 GPU computation, six deep frames | **82–131 ms/frame** |
| Accuracy compared with the project's GPU/CPU class-and-nu reference | **0 errors over approximately 12.4 million samples** |
| Maximum recorded smooth-escape displacement | **1.6e-4 px** |
| Speedup versus the CPU fd per-frame BLA comparison | **about 35x** |
| FP32 tail-patch replacement, PATCH32=1 | **7–274,523 wrong samples/frame, max 3.5 px; small speed benefit** |

These are **historical repository results**, not measurements collected for this report. The 35x figure is **not** the benchmark for a proposed new hardware optimization: the baseline to beat is the already accelerated 82–131 ms FP64 GPU implementation, with identical mathematics, image geometry, output quantities, hardware, compiler options, and accounting.

The existing GPU bench emits **class and smooth escape value nu**, *not* derivative/distance-estimate/normal columns. Therefore those measurements do not, on their own, certify a larger four-output contract. The project also records comparator gaps around nonfinite values and width underflow. Fix correctness instrumentation before accepting new speed claims.

Relevant existing material:

- [BENC-08 results and precision experiments](../../../spec/METHOD.md).
- [Current correctness blockers and accepted decisions](../../../spec/STATE.md).
- [Existing OpenCL kernel](../../../../tools/research/misiurewicz/koenigs_bench/gpu_bench.py).
- [The baseline comparator](../../../../crates/fd-cli/src/compare.rs).
- [Deep-zone kernel](../../../../crates/fd-kernel/src/zone.rs).

## Evidence: why naive FP32 fails

In the mid-band precision probe, float32 changes were applied one stage at a time to 250 points at each of three widths. Reported worst displacements include:

| Arithmetic change | Worst recorded result |
|---|---:|
| All FP64 | 6.6e-11 px |
| Stages 1–4 in FP32 | 6.9e-4 px |
| Koenigs jump with precomputed FP32 multiplier table | 3.9e-4 px |
| Jump via FP32 exp(j log lambda) | 3.8e-1 px |
| Landing addition in FP32 | 5.8e-2 px |
| Finishing iterations in FP32 | 9.3e-2 px |
| Mostly FP32 with FP64 landing/finish | 4.2e-3 px, occasional failures |

These are sampled results, not full-frame passes. The deeper GPU experiment is a stronger negative: the ostensibly local FP32 tail chart failed full 1080p tests.

Reasons include cancellation in a return map, small pixel offsets summed with larger orbit constants, phase error magnified by a large jump count, and weakly expanding finishing cycles. A local FP32 error that looks numerically small can correspond to more than 1e-3 output pixel.

The source is [the repository's 2026-10-09 FP32 stage investigation](../../10-9-26/binary32-precision-analysis-of-the-mid-band-pixel.md).

## Arithmetic candidates

| Representation | Helps with | Main risk / decision |
|---|---|---|
| Native FP32 | High throughput for screened, well-conditioned operators | ~24-bit mantissa fails on sensitive stages |
| Native FP64 | Reference and dangerous operators | Low throughput relative to FP32 on many consumer NVIDIA devices |
| Compensated FP32 (float-float / two-float) | Improved significand precision using fast FP32 FMAs | Many instructions, register pressure, and non-IEEE behavior |
| FP32 mantissa plus integer exponent | Very small or large perturbation offsets | **Range, not extra mantissa precision** |
| FP64 mantissa plus integer exponent | Extreme ranges with native double accuracy | Slower arithmetic and normalization |
| Block-floating-point | Coherent states with similar magnitudes | Shared exponent can lose tiny but critical components |
| CPU multiprecision or validated balls | Reliable coefficients, references and exceptional repairs | Too expensive as the default per-pixel GPU representation |

### Compensated FP32: first new arithmetic experiment

Encode a value as hi+lo, where both parts are FP32. Use compensated sum and FMA-based product splitting to retain lower residuals. Effective precision can approach ~48 bits in favorable, normalized cases, **not** correctly rounded FP64. Costs include several FP32 operations, longer dependency chains, extra registers and potential compiler re-association.

A controlled microbenchmark must measure two-float complex multiply-add, complex square-add, derivative recurrence, and transcendental/phase operations separately. Disable unsafe fast-math transformations in the correctness arm; verify actual GPU FMA and flush-to-zero behavior. FP32x2 is attractive only when the all-in cost per operator and its propagated output error are both better than FP64.

Start on repeated finishing recurrences: z' = z^2+c, D' = 2zD+1. Keep the FP64 implementation as a precise checkpoint/fallback. Precomputed jump factors may benefit from split representations, but FP32-only phase calculation is unsafe.

### Floating mantissa plus exponent

Store x = m*2^e with a separately carried integer exponent; normalize m after arithmetic. This avoids underflow at depths below the FP64 range. It does **not** create low-order bits that were discarded from m. Do not derive a deep pixel's parameter by forming a rounded absolute c and subtracting its center. The repository already has a CPU scaled implementation to adapt: [scaled.rs](../../../../crates/fd-kernel/src/scaled.rs).

For exceptionally deep views such as 1e-1000, accurate center addressing and operator construction still require arbitrary precision on the CPU (or a separately validated precision library), irrespective of GPU exponent range.

### Block floating-point

Allow a warp, return-map batch, or tile to share an exponent only when its dynamic range and cancellation bounds are small enough. The most plausible use is on normalized return-map work packets. Never choose a shared exponent for an arbitrary group of adjacent pixels without screening: near-zero offsets can be lost, and a small local error can be large in output pixels.

## Cheap dynamic precision selection

**Yes in principle; not yet shown profitable or fully safe.** Select precision per operator checkpoint or per pixel *before* executing an expensive stage, not via a divergent condition inside every floating-point instruction.

For an approximate quadratic state \(\hat z_n\), parameter uncertainty E_c, roundoff allowance eta_n, and a bound E_n on state error, one conservative propagation form is

\[
E_{n+1}\le(2|\hat z_n|+E_n)E_n+E_c+\eta_n.
\]

For an analytic operator R, use a verified upper bound on |R'| over the admitted state disk, plus operator truncation/evaluation errors. For a Koenigs jump, also include uncertainty in the chart, multiplier, derivative, parameter dependency, phase reduction and potentially discontinuous integer jump count.

A **heuristic** first-order pixel displacement for regular escaped pixels is

\[
E_{\rm px}\approx \frac{E_z}{h\,|dz/dc|},
\]

where h is the complex-plane width per output pixel. For production acceptance, use error propagation for **nu, DE, and normal individually**; reject nonfinite or badly conditioned cases, ensure classification/escape decisions cannot change inside the admitted error balls, and use a conservative safety budget well below 1e-3 px. The simple expression is neither a universal rigorous bound nor sufficient near the boundary.

The promotion rule should be:

1. Calculate or reuse an inexpensive numerical sensitivity/error envelope for the upcoming operator.
2. Run FP32 or FP32x2 only when the full downstream allowance and chart-domain test passes.
3. Route marginal cases to FP64 or explicit-exponent FP64.
4. On uncertainty or failure, **restart at a checkpoint preceding the lossy arithmetic**. Converting a corrupted FP32 result to FP64 cannot recover discarded bits.

The forward-error gate itself must be included in GPU timings. To be a rigorous certificate, eta_n and operator/domain bounds need validation against the target compiler, rounding, FMA and FTZ semantics.

## Proposed CPU/GPU division of labor

**CPU (amortized):** build high-precision reference orbits, fixed-point camera coordinates, return maps, Koenigs/chart coefficients, reliable domain/roundoff bounds, optional power/phase tables and reusable constants. Generate per-zone or per-frame descriptors that can be transferred in compact contiguous buffers.

**GPU (per frame):**

1. Form normalized offsets without catastrophic absolute-coordinate subtraction.
2. Classify candidate operator/precision; group by chart, map and numerical range where it pays.
3. Execute return-map and Koenigs work in warp-coherent packets, using FP64 for sensitive cancellation and independently validated alternatives elsewhere.
4. Complete short finishing tails inline. Optionally append exceptional survivors to a compact work queue.
5. Execute queue kernels specialized by precision and coarse remaining-work class, not unrelated per-lane branches.
6. Produce class, nu, DE, normal and an error/precision diagnostic; shade on GPU if frame delivery warrants it.

Avoid transferring per-pixel intermediate states to the CPU. The CPU should not sit on the critical path for ordinary pixel fallback; reserve it for rare high-precision repairs or offline reference truth.

## GPU work queues and warp coherence

The current baseline maps a pixel to one GPU lane and runs a direct finishing loop. Some lanes finish quickly while others do many iterations. That motivates a **measurable**, not assumed, compaction opportunity:

- Run a small fixed number of finishing iterations inline.
- Append surviving lanes to a device-resident queue via atomic reservation (later use warp-aggregated reservations or prefix scans if atomic contention matters).
- Continue the remaining tails in a separate kernel whose active lanes are densely packed.
- Where possible, regroup by operator/chart before repeated map evaluations.

**Trade-off:** extra state stores, queue traffic, launch overhead, host synchronization and lost locality can cost more than the saved idle-lane work. The optimized zone path's tails may already be short enough for queuing to lose. Test 4, 16, 32 and 64 inline iterations plus controls. Keep state persistent and use device-side queue counts/indirect dispatch if supported and if it materially helps.

Warp-coherent return-map execution groups by **same operator and chart**, not necessarily by neighboring display coordinates. Screen tiles are useful for initial locality; compacted worklists help when different lanes have distinct operators or long-tail lengths.

## Experimental artifacts committed with this report

- [gpu_queue_ab.py](gpu_queue_ab.py): imports the repository's exact baseline OpenCL source at runtime, creates a split-tail kernel from it, and measures full-frame baseline versus compacted-tail variants on the same GPU. All arithmetic and mathematical skip counts remain unchanged. It checks complete-frame output agreement (class and nu) for each queue setting.
- [README.md](README.md): running instructions, existing measurement provenance, stages of the correctness/performance gate and limitations.

The script must currently be placed next to the existing gpu_bench.py to import it. It has **not** been executed on an OpenCL GPU in this environment. Syntax-level inspection is not an actual driver compilation or performance result. It writes raw class/nu outputs and CSV timing data when run on a suitable system.

This queue-only prototype is a scheduling **ablation**, not a compensated-precision prototype. It cannot validate DE/normal until both compared GPU arms implement those output columns. The report's full acceptance gate must not be silently replaced by the queue-only equality test.

## Decisive acceptance plan

**Gate 0 — correct benchmark:** freeze representative camera frames, parameters, supersampling, class/nu/DE/normal outputs, a truthful error metric and a fast, optimized FP64 GPU baseline. Reject NaN/inf and underflowed or mismatched camera widths. Check all pixels; adjudicate ambiguous near-boundary outliers against high precision. Include warm-up, GPU launch, synchronization, transfers required by the intended frame pipeline, CPU preprocessing where per-frame, and all fallback work.

**Gate 1 — pure scheduling:** compare original baseline OpenCL with the queue-only ablation on the same hardware, rotating run order. Record median complete-frame time, per-frame speed ratio, queue fraction, occupancy, register use and divergence. An exploratory target of >1.25x on multiple frames merits retention; losses mean kill or retune without pretending the queue adds value.

**Gate 2 — arithmetic:** introduce FP32x2 on a specific expensive recurrence, then an error-screened FP64 fallback. Compare against optimized all-FP64 GPU execution (not CPU BLA), include guard overhead, enforce zero class failures and no sample exceeding 1e-3 px, and separately enforce specified DE and normal tolerances. Test deep, mid-band, boundary-heavy and hostile unrelated locations.

**Gate 3 — practical substantial win:** predeclare at least **1.5x median whole-frame speedup** (preferably >=2x), no serious outlier-frame regressions, full-accuracy acceptance, and reproduce on at least one FP64-constrained NVIDIA consumer GPU and one AMD consumer GPU. This speed threshold is a proposal for the new study, not an existing Fractodactyl decision.

Amdahl's law matters: if only one-third of baseline time is improvable, that part alone can never yield 1.5x even with infinite acceleration once nonzero overhead remains. Stage profiling should determine the ceiling before architecture work.

## Open questions and recommended order

1. Instrument existing FP64 OpenCL code: per-stage time, executed finishing iterations, per-lane tail distribution, active-lane fraction, and memory/register limits.
2. Run queue-only A/B. Retain only if measured benefit survives full-frame accounting.
3. Microbenchmark float-float complex arithmetic against native FP64 on each target GPU.
4. Add checkpoint-scoped error estimation with escalation to exact FP64 checkpoint state, initially for the finish.
5. Validate complete frames against fixed high-precision truth; add DE/normal to the GPU benchmark and strengthen the comparator.
6. Re-evaluate workload partitioning for non-v0 scenes and deep exponent ranges.
7. Keep only combinations that outperform the fair best GPU baseline with the full contract.

## Conclusion

Better mathematics and better hardware execution can compound. Fractodactyl's previous CPU-to-GPU and return-map wins are real evidence for that direction. But **the incremental hardware-layer gain is still unmeasured**, and the repository already rejects naive FP32 substitutions at full resolution.

The most promising next precision experiment is **compensated FP32 finishing arithmetic with a cheap, conservative FP64 fallback**. The attached **queue-only** prototype isolates scheduling first. Neither is yet proof of a >=1.5x whole-frame win under the strict 1e-3-pixel benchmark.
