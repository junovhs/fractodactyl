# CPU occupancy census, 2026-10-22 (PERF-03 / PERF-07)

PERF-03's reported 9–13 busy threads out of 24 was not reproduced under the
available eight-worker lease. No scheduler or per-pixel maths was changed.
The original 24-thread investigation remains open. PERF-07 adds observability,
not a speedup or a claim that the original occupancy target was achieved.

## Fixed workloads and observed results

Ryzen 9 3900X, Linux; eight renderer workers, CPU affinity 0–7. Reference and BLA
setup, tail work, process startup and serialization are included in the independent
control-process measurements. One cold process per arm/view; not a repeatability
or portable throughput claim. Compilation is excluded. No zone supplied.

The four exact views are indices **0, 10, 17, 23** of
`docs/2026-10-20/series-frames/results/frames.txt`, selected before candidate
measurements, at 480×270, ss1, cap100000, nu/de/normal columns. Plain and per-frame
BLA control processes average **93.3–99.1%** of eight workers. Deepest plain:
14.817037 wall seconds / 117.471678 CPU seconds = 7.928 busy threads; BLA:
1.560651 / 12.121270 = 7.767. These are eight-worker
observations, not evidence about a 24-thread run.

Actual single-output-frame `fd film` runs on the four exact views use automatic
series/BLA routing, direct ss1, CPU shading, ultrafast x264. PERF-07's first green
production census measured:

| Frozen index | Route | Kernel wall s | Process CPU s | Average process busy threads | Whole process before / after s |
|---|---|---:|---:|---:|---:|
| 0 | series-perturbation | 0.0877 | 0.5304 | 6.046 | 0.1723 / 0.1670 |
| 10 | series-perturbation | 0.2741 | 1.8936 | 6.908 | 0.3488 / 0.3542 |
| 17 | bla | 1.1710 | 9.0308 | 7.712 | 1.2513 / 1.2510 |
| 23 | bla | 1.5514 | 11.8863 | 7.662 | 1.6089 / 1.6314 |

These old/new latency differences include noise and instrumentation overhead;
**no performance improvement is claimed**. A fast series route can have lower
occupancy while saving time: optimizing occupancy alone would be misleading.
The clock records a successful kernel call, including reference/BLA/series setup,
chooser probes and retries, ending before subsequent shading/encode. CPU time
includes every overlapping thread in the fd process, excludes encoder children,
and is not worker-specific utilization. Unix process clocks supply CPU time;
unsupported/failed clocks report null. Independent subprocess measurements include
the process tree (including encoder), startup/shutdown and output serialization,
so their denominators must not be mixed with the kernel records.

## Reproduction and evidence

Run through a positive Ishoo lane lease. `perf03-before` runs
`tools/research/cpu-occupancy.py`, which builds the exact snapshot before the
four-view plain/BLA census. `perf07-surface` runs `scripts/check/PERF-07.py`,
which builds the exact snapshot, checks actual direct/keyframe JSON and stderr,
compares two decoded films and four exact-view `.fds` files byte for byte against
the preserved old fd, and records four actual automatic-film views.
Both scripts bound renderer and compiler workers and affinity explicitly.

Initial report proof: job643 failed on the absent `kernel_frame_process_cpu` field;
jobs645 and647 passed. The latter adds independent CPU/wall bounds and the
actual-film census. Four frozen sample frames and two decoded films were
byte-identical. Missing-clock serialization is also checked in the CLI unit tests.
Small extracted measurements are in `census.json`; full JSON, stderr, binaries and
MP4s are outside the checkout in `target/perf03-evidence` and
`target/perf07-evidence`. Exact snapshot bindings remain in Ishoo verification
records; empirical values may vary on gate reruns.
