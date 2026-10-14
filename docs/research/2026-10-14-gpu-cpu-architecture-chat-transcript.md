# GPU/CPU Architecture Discussion — Chat Transcript

**Repository:** [junovhs/fractodactyl](https://github.com/junovhs/fractodactyl)  
**Conversation date:** 2026-10-14  
**Scope:** The four most recent completed user-prompt/assistant-response pairs preceding the request to save this transcript. Chronological order.

> **Transcription note:** Assistant wording and links have been preserved in Markdown. Chat-only visual elements (images, badges, cards, and the interactive performance chart) have been converted to equivalent text or tables so a coding agent can read them. This is a record of discussion, not a fresh verification of the repository, and some observations are time-sensitive. No Ishoo issue shards were modified or available for decompression during the conversation.

---

## Exchange 1 — Should the project remain CPU-only?

### User prompt

> Sounds disappointing perhaps we should stay cpu only, I just feel GPU is the one that could go fastest potentially

### Assistant response

I wouldn't abandon GPU yet. In fact, I think the latest results are more encouraging than they initially sound.

There's an important distinction:

**The GPU implementation was not 2× slower than the CPU. It was 2× slower than another GPU implementation.**

On the RX 5700 XT:

- **Shared Rust/CubeCL GPU:** Nearly as fast as handwritten WGSL during actual GPU computation.
- **WGSL GPU:** Much faster when including memory allocation, uploads, and readback.
- **Correctness:** Both passed all twelve parity checks.

That suggests the GPU mathematics aren't the problem. The overhead surrounding the computation is.

### Where I think the real opportunity lies

Fractodactyl currently has a sophisticated CPU renderer. But GPUs can process enormous numbers of independent operations simultaneously, which is particularly attractive for fractal rendering.

There are three important considerations:

1. **Keep the CPU renderer as the reference implementation.** Its correctness and determinism are valuable.
2. **Don't insist on shared Rust/CubeCL if WGSL is faster.** A little duplicated code might be worth a significant performance advantage.
3. **Benchmark the entire rendering pipeline.** The current GPU experiment accelerates the finish stage, not everything. CPU preparation, sampling, shading, and transfers can limit the overall benefit.

There's also a genuine obstacle: Fractodactyl's deep zooms require numerical precision that consumer GPUs don't necessarily handle efficiently. That makes the performance opportunity more complicated than ordinary parallel pixel rendering.

### My recommendation

- **CPU remains the production baseline.**
- **GPU research continues, with strict parity requirements.**
- **Prioritize finding the fastest practical GPU route, not proving CubeCL wins.**

I would make one more concentrated effort: benchmark the existing WGSL route against CPU-only rendering on identical representative scenes, measuring actual complete-frame performance. Then repeat on the Windows NVIDIA GPU.

If GPU delivers a substantial end-to-end advantage, keep pursuing it. If not, defer it without compromising the CPU architecture.

**The thing I'd most want to avoid is abandoning GPU because one GPU programming approach is slower than another.**

We haven't established that CPU is faster. And I agree with your instinct: GPU could still offer the largest remaining performance upside, provided we can move enough work onto it without compromising deep-zoom precision.

---

## Exchange 2 — What changed in the repository?

### User prompt

> See what's changed now

### Assistant response

# Fractodactyl — What's changed since the last check

October 14, 2026 · Checked at 1:27 PM Pacific

**The biggest development: the two-GPU comparison is complete, and the project has selected WGSL rather than shared Rust/CubeCL.**

| New commits on `main` | Selected GPU implementation | GPU models now evaluated |
| --- | --- | --- |
| 3 | WGSL | 2/2 |

## 1. GPU architecture decision is now settled

Actual hardware benchmarks are available for both the AMD RX 5700 XT on Linux and NVIDIA RTX 3050 Ti on Windows.

**Shared Rust/CubeCL whole-stage time relative to WGSL**

| Depth | AMD RX 5700 XT | NVIDIA RTX 3050 Ti |
| --- | ---: | ---: |
| 1e-37 | 1.87× | 1.09× |
| 1e-40 | 2.00× | 1.15× |
| 1e-43 | 2.09× | 1.253× |
| 1e-46 | 2.05× | 1.09× |
| 2e-48 | 1.84× | 1.22× |
| 2e-49 | 2.25× | 1.15× |

Lower is better. 1.0× means equal execution time.

The findings are now much clearer:

- **AMD:** CubeCL is 1.84–2.25× slower end-to-end than WGSL.
- **NVIDIA:** CubeCL is much closer, approximately 1.09–1.25× slower.
- **GPU computation itself:** Both implementations perform similarly. Most of CubeCL's disadvantage comes from host-side transport.
- **Correctness:** All 18 binary64 comparison rows passed strict parity checks.

The project applied its previously defined performance threshold rather than choosing an architecture arbitrarily.

Source: [GPU-12 verdict and measurements](https://github.com/junovhs/fractodactyl/commit/7350a0cbe91db3fdaea8561a9d3e8604bfad5e04).

## 2. A Windows GPU driver problem was solved

A binary64 `frexp` instruction generated by Naga was crashing the NVIDIA driver's compute-pipeline creation.

The shader now extracts exponent and mantissa bits directly, preserving the required mathematical behavior. The corrected implementation passed the Windows comparisons.

A new self-hosted AMD GPU workflow also confirmed that the modified WGSL implementation remains correct on Linux:

**All six 1080p AMD binary64 parity tests passed.**

This eliminates a significant portability obstacle.

Sources: [GPU-20 workflow](https://github.com/junovhs/fractodactyl/commit/f097ad753aafbf38a4fee4f0126dc523608ea164), [successful hardware run](https://github.com/junovhs/fractodactyl/actions/runs/38616843242).

## 3. GPU-13 — the production renderer is being ported

**In progress · Not merged**

A separate branch, [`ishoo/GPU-13`](https://github.com/junovhs/fractodactyl/tree/ishoo/GPU-13), now contains four commits implementing the next step.

It replaces the existing GLSL/ash zone-rendering path with WGSL/wgpu, while keeping the CPU fallback.

Early testing is promising:

| Test | Result |
|---|---|
| Full-precision f64 on both GPUs | Passed recorded parity tests |
| Mixed/f32 precision | Some errors still exceed tolerance |
| AMD 480×270 dispatch + readback | 6–12 ms |
| NVIDIA 480×270 dispatch + readback | ~31 ms |

The new WGSL implementation is reproducing the earlier GLSL behavior on both GPUs. However, those timings are not full-frame CPU-versus-GPU comparisons.

The new implementation has also passed dedicated AMD hardware runs, including [this GPU-13 validation](https://github.com/junovhs/fractodactyl/actions/runs/38630967035).

## 4. CI issues remain

Not everything is green.

- The latest completed main CI run failed on Linux because an unrelated shading test encountered a `Text file busy` error. Windows passed. [View run](https://github.com/junovhs/fractodactyl/actions/runs/38616794091).
- The GPU-13 development branch has encountered Clippy errors involving redundant `u64` conversions.
- A newer GPU-13 build workflow has passed its Windows job, while Linux was still running at the time of inspection. [View workflow](https://github.com/junovhs/fractodactyl/actions/runs/38631123708).

## What this means for CPU versus GPU

The situation has improved substantially since our earlier discussion.

**The main architectural uncertainty has been resolved:** WGSL is the preferred route, and implementation work has already begun.

But one crucial question remains unanswered: **How much faster is complete rendering on the GPU than on the CPU?**

The latest benchmarks compare different GPU implementations, not total CPU and GPU rendering times.

I would continue GPU development through GPU-13, get its CI passing, and then require a representative full-render comparison before committing further resources to optimization.

The project has made meaningful progress toward GPU acceleration without yet demonstrating its ultimate speed advantage.

---

## Exchange 3 — Does the GPU now match the CPU's precision?

### User prompt

> So... my bet is just GPU will always be able to be the fastest over cpu, because, it just has more hardware and threads or whatnot, it can be sci-fi basically. But cpu seems to have a 64 bit advantage, is that right? But now the GPU matches it?

### Assistant response

Yes. Your intuition is broadly right, but there's an important twist: **the GPU now matches the CPU's required numerical accuracy in the tested rendering paths, but that doesn't mean it matches the CPU's performance or all of its precision capabilities.**

And I think the distinction actually makes the GPU opportunity more interesting.

## 1. GPUs have enormous parallel-processing potential

**CPU**

A smaller number of powerful cores, excellent at complex, sequential calculations.

**GPU**

Many parallel execution lanes, excellent at performing similar calculations across huge amounts of data.

Fractals are attractive GPU workloads because millions of pixels can potentially be calculated independently.

But GPUs aren't universally faster. Divergent calculations, sequential dependencies, and memory transfers can waste their parallel capacity.

## 2. You're right about the 64-bit advantage

The important term is **FP64**, or double-precision floating-point arithmetic.

Both CPUs and some GPUs support it. The difference is how efficiently.

| Capability | CPU | Consumer GPU |
|---|---|---|
| 32-bit floating point | Fast | Extremely fast |
| 64-bit floating point | Generally efficient | Can be much slower than FP32 |
| Precision beyond 64-bit | Software libraries | Possible, but generally expensive |
| Millions of parallel pixels | Limited parallelism | Major strength |

Your NVIDIA RTX 3050 Ti, for example, has FP64 arithmetic throughput around **1/64 of its FP32 throughput**.

That is an enormous penalty.

The AMD RX 5700 XT and NVIDIA RTX 3050 Ti both support FP64, but this doesn't make their 64-bit throughput equivalent to their 32-bit throughput.

## 3. Has Fractodactyl overcome that?

**It has demonstrated FP64 correctness, not eliminated the FP64 performance penalty.**

The new GPU code performs genuine 64-bit calculations. It isn't quietly substituting less accurate 32-bit arithmetic.

The recent GPU tests show:

- CPU-reference parity passed at all six tested zoom depths.
- Both AMD and NVIDIA GPU hardware validated.
- No class mismatches in the full-precision comparison.

These are strict numerical-tolerance matches, not a guarantee that every floating-point output is bit-for-bit identical.

And there's another subtlety: a zoom width of `1e-49` doesn't mean ordinary FP64 can represent every absolute coordinate at that scale. Fractodactyl uses carefully prepared high-precision information and local calculations to make those depths workable.

## 4. The potentially powerful architecture

**CPU: High-precision preparation**

Reference orbits, deep coordinates, precision-sensitive setup

↓

**GPU: Massively parallel rendering**

Thousands or millions of pixel calculations using prepared inputs

↓

**GPU: Shading and compositing**

Minimize unnecessary transfers back to the CPU

That's the architecture I would be most interested in pursuing.

The CPU handles the difficult precision-sensitive work; the GPU handles whatever can be distributed efficiently. The existing CPU renderer remains a correctness reference and fallback.

## 5. Where the real breakthrough could happen

The existing mixed-precision experiments are particularly interesting.

What if Fractodactyl could do most GPU work in FP32, only switching to FP64 when necessary, while maintaining its strict error tolerance?

On an RTX 3050 Ti, that could unlock substantially more throughput. But the existing guarded-FP32 experiments still have accuracy failures, so that optimization has not been proven safe.

**My conclusion:** GPU absolutely has the potential to be Fractodactyl's fastest rendering path, possibly by a large margin. The fact that FP64 parity now works is an important milestone. But it hasn't yet demonstrated a whole-render speedup over CPU, and consumer-GPU FP64 throughput is a real limitation.

The exciting challenge now isn't getting the GPU to calculate accurately enough in FP64. It's getting it to perform the maximum amount of work in parallel, preferably with less expensive arithmetic, **without losing the accuracy Fractodactyl has worked so hard to establish.**

---

## Exchange 4 — Is this architecture planned in the repository?

### User prompt

> 4. The potentially powerful architecture  
> CPU: High-precision preparation  
> Reference orbits, deep coordinates, precision-sensitive setup  
> GPU: Massively parallel rendering  
> Thousands or millions of pixel calculations using prepared inputs  
> GPU: Shading and compositing  
> Minimize unnecessary transfers back to the CPU  
> That's the architecture I would be most interested in pursuing.< is this planned anywhere in the repo or mentioned? Try to decompress some of the zstd shards those are the issues (don't modify them though)

### Assistant response

**Yes. The CPU-preparation → GPU-rendering → GPU-shading architecture is explicitly described in Fractodactyl's research documents.** It is more than an idea: significant pieces have already been implemented, although they are not yet one fully integrated GPU pipeline.

I found a particularly relevant document:

[GPU Architecture and Adaptive Precision — October 10, 2026](https://github.com/junovhs/fractodactyl/blob/main/docs/research/10-10-26/gpu-precision-work-queues/REPORT.md)

### 1. Almost exactly the architecture we discussed

The section titled **Proposed CPU/GPU division of labor**, lines 118–131, specifies:

| Stage | Explicitly proposed responsibilities |
|---|---|
| CPU preparation | High-precision reference orbits, fixed-point camera coordinates, return maps, Koenigs coefficients, error bounds |
| GPU rendering | Evaluate mathematical operators, select precision, run many pixel calculations in parallel |
| GPU fallback | Reprocess exceptional pixels through GPU work queues where practical |
| GPU shading | Produce class, smooth escape, distance estimation and normals, then shade on GPU when beneficial |

The report also specifically advises avoiding transfers of per-pixel intermediate states back to the CPU and reserving CPU fallback for rare high-precision repairs.

That is a remarkably close match to the architecture you're imagining.

**One qualification:** this is a research proposal, not evidence that every stage has been integrated or has passed its performance gates.

### 2. The broader design has anticipated this for some time

The repository's [architecture specification](https://github.com/junovhs/fractodactyl/blob/main/SPEC.md) explicitly separates expensive mathematical preparation from rendering and appearance. It also proposes CPU-generated numerical certificates that GPUs can consume cheaply.

The [North Star document](https://github.com/junovhs/fractodactyl/blob/main/docs/spec/NORTH-STAR.md) emphasizes caching mathematical computation rather than finished images and designing data structures that CPUs and GPUs can consume efficiently.

These ideas are longstanding, although some of the older atlas-specific plans were superseded when benchmarks showed their caching strategy wasn't providing enough benefit.

### 3. What has actually been implemented?

| Component | Status | Evidence |
|---|---|---|
| CPU precision-sensitive setup and zone validation | Implemented | `zone_gpu.rs` keeps decimal arithmetic and validation on CPU |
| GPU mathematical rendering | Implemented, being migrated | GPU-03 GLSL path; GPU-13 WGSL/wgpu branch |
| GPU film shading and compositing | Implemented | GPU-06 |
| GPU-only intermediate data flow | Not yet integrated | Rendering reads sample columns back to CPU |
| GPU work queues and adaptive precision | Research / partial prototypes | GPU-04, GPU-05, guarded FP32 experiments |

GPU shading is further along than I initially appreciated. [GPU-06](https://github.com/junovhs/fractodactyl/blob/main/docs/spec/METHOD.md#gpu-06-vulkan-film-shading-2026-10-11-rx-5700-xt-results) already demonstrated approximately 7.78 ms per frame for 1080p shading and 33.1 ms per frame at 4K, meeting its performance targets on the RX 5700 XT.

There is, however, one important inefficiency in the current design.

**Current GPU rendering + film shading data flow**

    CPU prepares precision-sensitive inputs
                   ↓
        GPU calculates fractal samples
                   ↓
      Readback to CPU, then upload again
                   ↓
          GPU shades and composites
                   ↓
        CPU receives RGB for FFmpeg encoding

The GPU-13 branch still maps its output buffers into CPU-side `Samples`. GPU-06's film-shading implementation then uploads those samples using a separate Vulkan path.

Sources: [GPU-13 source](https://github.com/junovhs/fractodactyl/blob/ishoo/GPU-13/crates/fd-cli/src/zone_gpu.rs) and [GPU-06 source](https://github.com/junovhs/fractodactyl/blob/main/crates/fd-cli/src/film_gpu.rs).

A future improvement could keep the samples on the GPU between rendering and shading, avoiding that intermediate round trip. That particular end-to-end integration is **not something I can verify as an accepted live issue**.

### 4. What happened with the zstd issue shards?

I inspected the connected GitHub repository's complete `main` tree (470 entries) and checked the repository's older tree and branch inventory.

I couldn't find any tracked `.zst` or `.zstd` issue shards.

The reason is probably visible in the repository's [`.gitignore`](https://github.com/junovhs/fractodactyl/blob/main/.gitignore):

    .ishoo/
    **/.ishoo/*.local.toml
    **/.ishoo/cache/
    **/.ishoo/worktrees/

The project documentation also identifies Ishoo, not the historical Markdown issue ledger, as the live issue source of truth.

This means I could inspect the checked-in architecture and issue references, but **I couldn't decompress the live issue shards because their contents aren't available through the connected GitHub repository**.

No files were modified.

If you provide a copy of the `.ishoo` zstd shards or a containing archive, I can decompress copies for inspection and read the actual issue definitions, dependencies, priorities, and decisions without altering the originals.

## My conclusion

The architectural direction is already there. I wouldn't propose a new architecture from scratch.

What I would look for in the live Ishoo store is whether **GPU-14 or another existing issue explicitly covers keeping GPU-rendered samples resident for GPU shading**, followed by full-pipeline benchmarking.

That seems like the natural connection between two substantial pieces Fractodactyl has already built. It could be valuable, but it needs measurement rather than an assumed speedup.
