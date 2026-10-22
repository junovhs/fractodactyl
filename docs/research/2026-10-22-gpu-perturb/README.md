# GPU-32 experimental guarded frame integration

GPU-31 supplied the shared stepper; GPU-32 adds an explicit `fd render/control --gpu-perturb on` route (default off). It is useful transport/correctness groundwork for GPU LA, **currently slower**, and never automatically selected. No film integration or GPU-11 ≥4×/two-device completion is claimed.

Admitted output is **Heuristic**. Runtime FF capability tests, orbit/derivative/output uncertainty guards and bounded dispatch completion are required. The RN arithmetic estimate is conditional, not a portable rigorous bound; see [ce-error-bound.md](ce-error-bound.md). Bound evidence requests decline to the original CPU renderer. GPU full-escape Float reference construction is separate from the original CPU fallback reference: the earlier shared Float fallback caused two nu regressions. Rejected samples restart from zero at the exact camera, never from lossy GPU state. An incomplete/error attempt discards all GPU output visibly. Batches contain at most16,384 samples, reuse device/pipeline/buffers, wait at most5 seconds per submission and120 seconds per attempt, and do not retry. Existing unbatched zone behavior is preserved.

## Reproduce

Run under the repository's positive verification lease (the recorded machine used8 CPU workers). Exact six cameras are frozen in the script, including Tim's arbitrary accidental save; no special nucleus assumption is made.

```sh
cargo build --jobs 8 -p fd-cli --release
python3 scripts/check/GPU-32.py --fd target/release/fd --out target/gpu32-surface
python3 scripts/check/GPU-32-timing.py --fd target/release/fd --size 1280x720 --indices 5 --out target/gpu32-timing
cargo test --jobs 8 -p fd-cli --release gpu_perturb -- --include-ignored --nocapture
FD_REQUIRE_GPU=1 cargo test --jobs 8 -p fd-shade --release batched_dispatch_preserves_partial_chunk_ids -- --nocapture
FD_REQUIRE_GPU=1 FD_ZONE_KERNEL=f32only cargo test --jobs 8 -p fd-cli --release --test zone_fixtures zone_fixtures -- --nocapture
FD_REFERENCE_REPORT=target/reference.tsv cargo test --jobs 8 -p fd-kernel --release full_escape -- --include-ignored --nocapture
python3 scripts/check/GPU-32-reference.py --input target/reference.tsv
```

The timing script includes plain, landed BLA and GPU complete control processes with matching camera/size/iteration/AA/output columns, plus a separately labeled film-series compute observation. **Do not compare film compute-only time with complete control-process time.** Generator validation uses mpmath; independent GPU checkpoints use CPU binary64 windows, not implementation-mirror expected states.

## Recorded evidence (RX5700XT, RADV25.1.5)

- Check780:5 shared CF/CE/norm/negative tests pass;42 CE windows plus8 exponent/invalid states, worst error/model ratio0.0019804. Actual incomplete-output negative logs completed=0/64,entered_only=64, discards the entire attempt, and proves byte-identical original CPU restart after diagnostic kernel-name normalization. Corruption and runtime capability negatives also restart.
- Check774: six128×72 frames (55,296 samples),0 comparator failures. GPU acceptance: seahorse8193, cusp0, deep seahorse0, valley0, exterior9216, Tim8033. Guards are unchanged; zero coverage is reported rather than hidden. Later final surface check also asserts actual Heuristic bytes and bound refusal.
- Check778: partial final chunk67 samples preserves every original index and sums completed iterations2278.
- Check779: actual1280×720 Tim has921,600 completed samples,803,150 admitted and118,450 original CPU fallbacks,0 invalid/unfinished. Whole-frame comparison to original plain CPU has0 class/nu/DE/normal failures; maxnu1.2551e-10, displacement7.1781e-10px, scored DErelative1.1392e-7, normal0. Near-boundary samples also have zero DE/normal difference versus plain. Landed BLA comparison passes standard gates, but BLA itself differs at near-boundary samples; do not attribute those raw BLA differences to GPU error.
- Complete cold process costs: plain7.093279s, landed BLA1.484947s (check775), batched GPU9.661005s (check779). GPU setup/batches/readback8.264363s, CPU fallback1.024411s, total reported frame9.597544s. One cold run per arm,8 CPU workers; this is a clear slowdown, not a speed claim. Separate existing series film compute.101234s / complete filmprocess.248408s (776) has a different boundary and is context only.
- Original unbatched720p long dispatch returned unwritten output despite successful validation/map;777 proved buffers/groups under device limits and working trivial/short/prefix controls. A watchdog cause was **not proven**. Batching plus explicit entered/completed markers fixes silent unusable output, not shader numerical thresholds.
- Check705 independently validated26,916 Float reference points/radii. Earlier stopped prototypes, including2 Float-fallback nu errors and failed CF/no-coverage attempts, remain in checkpoint JSON and research notes. Attribution invalidated the early rebase hypothesis: accumulated f32 error dominated. CE addresses mantissa precision, not a fitted tolerance.

Raw logs and frames are recorded by Ishoo checks774–780 and referenced in issue comments; checked-in scripts regenerate every final surface/camera. Historical diagnostic scripts/patch and JSON preserve the failed experiments. GPU-11 still needs GPU LA, resident/film integration, profitability and ≥4×/two-device evidence. No release follows this foundation alone.
