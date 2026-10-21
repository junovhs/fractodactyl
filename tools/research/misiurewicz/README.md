# Frozen zone-pipeline evidence

The following prototypes are frozen historical evidence, superseded by
`fd zone RE IM -o FILE` and the production zone loader / WGSL kernel
(DEC-24, DEC-28, CLEA-02). Do not maintain, benchmark, or use them to produce
new zones. Their old commands are preserved solely to interpret METHOD.md's
PROB-07/08/09/14/17/19/20 and BENC-04/08 results.

- `koenigs_bench/`: the Rust pixel pipeline, OpenCL copy, Python drivers,
  comparisons and historical benchmark runners.
- `make_zone.sh`, `koenigs_bench_consts.py`, `tail_patches.py`, `mid_consts.py`:
  the retired zone writer and its constants generators.
- Their shared mathematical research dependencies: `koenigs_tail.py`,
  `returns_exit_tail.py`, `psi_series.py`, `shared_consts_double.py`.
- `run_rungs.sh` and `single_precision.py`: historical measurements using
  the superseded zone producer or pixel pipeline.

These files remain in their original locations so recorded evidence and relative
imports retain their meaning. The koenigs-bench workflow has been removed; live
KERN-02 gates use `fd zone`. For current use see the public Zones manual at
https://fractodactyl.dev/docs/ and `fd zone --help`.

Other research here is outside this freeze. `tools/oracle.py` remains the
independent Python oracle by design. The frozen zone/frame fixtures under
`crates/fd-cli/tests/fixtures/zones` remain independent expected results and must
not be overwritten by the Rust producer.
