# Build-Cost-Aware Deadline Scheduler (SCHE-02)

The atlas compiler's scheduling core (SPEC.md "Atlas compiler": schedule preparation by
first-use deadline and build cost; ARCHITECTURE.md "Scheduling"). Given a known camera
path it derives the chunks the path needs as a dependency DAG of build jobs, estimates
each job's cost before building, builds the chunks into the store with real builders
on `--workers` threads, least slack first, and logs whether every chunk was ready
before the frame that first uses it. Governed by DEC-02 (compile known paths offline;
cold may stay expensive) and DEC-05 (frame deadlines drive refinement; thresholds
experimental). Code: `crates/fd-cli/src/schedule.rs`. The full compiler `fd compile`
(VIDE-01, COMPILE.md) runs the same jobs, estimates and dispatcher, and adds the BLA
sharing mode `level` (one table per run of tile levels); interactive prefetch is out of scope.

```text
fd schedule PATH --store DIR [--size WxH] [--ss N] [--iter N] [--columns C] [--kernel K]
            [--fps F] [--lead S] [--workers N] [--policy slack|edf|first-use]
            [--bla frame|group|none] [--slab N] [--on-miss fail|report]
```

`PATH` is a camera path (PLAN.md). Render flags mean what they mean for `fd render`
and decide each frame's tier and precision. Defaults: `--fps 30`, `--lead 0`,
`--workers 1`, `--policy slack`, `--bla frame`, `--slab 4096`, `--on-miss fail`.

## Jobs

- **Orbit**: one per exact-centre group, grouped as `fd reuse` does (ATLAS.md "Orbit
  reuse"; f64-tier frames apart from deep ones), computed for the group's deepest frame
  and stored as slabs plus an orbit manifest (`fd orbit put`). Name `orbit.G`. No
  dependencies. First use: the group's first frame.
- **BLA table** over its group's orbit (`fd orbit bla`, default `eps`), depending on the
  orbit job. `--bla frame`: one per frame, for that frame's `dc_max`, name `bla.F`, first
  use frame `F`. `--bla group`: one per group at the largest `dc_max` of its frames
  (valid for all of them), name `bla.gG`, first use the group's first frame. `none`: no
  tables. Scaled-tier frames get none (`bla_skipped_frames`). A table with no valid
  block (shallow f64 frames) is still a 96-byte chunk the renderer would load.
- **Certificate**: no producer exists yet (CERT-01), so none is derived
  (`jobs.certificate 0`). The scheduler takes any DAG: a job is a kind, its
  dependencies, first-use frame and cost.

Parents always have lower indices than their dependents.

## Time

- **now**: wall-clock seconds since the compile clock started, which is after the path
  is read and the store opened and before any probe. Probes, dispatch and builds all
  count against deadlines.
- **deadline** `d_j = lead + first_use_j / fps`: when the first frame that reads the
  chunk is presented, with playback starting `--lead` seconds after the clock starts.
  `--lead 0` is just-in-time compilation (playback starts with the compile); a lead is
  the compile-ahead time of DEC-02. The time to render that frame from the chunk is not
  budgeted here (VIDE-01).
- **ready**: the chunk (and for an orbit its manifest) is stored. `finish_j` is when the
  builder returns; a job is **met** iff `finish_j <= d_j`.
- **effective deadline** `D_j = min(d_j, min over dependents k of (D_k - est_k))`: a
  parent must be done in time for each dependent to be built by its own effective
  deadline. Computed dependents first.
- **slack** at dispatch `s_j = D_j - now - est_j` (`slack_at_start`); at finish the log
  shows `slack = d_j - finish_j`.

## Cost

Estimated before anything is built (`est_seconds`), then measured (`actual_seconds =
finish - start`); `est_error = (est - actual) / actual`.

- **Store**: puts of a 64-byte and a 1 MiB chunk into a scratch store
  `DIR.schedule-probe-PID` beside the store (same filesystem, removed afterwards),
  median of 3 each, give seconds per chunk and per byte (each put hashes, checks and
  fsyncs). Probe directories of runs that were killed (their PID is not a live process,
  `/proc/PID` absent) are removed first; a live run's probe is left alone. Printed as `store_cost.per_chunk_seconds`, `store_cost.per_mib_seconds`.
- **Orbit**: compute the first 8192 points of the same orbit (same centre, same
  precision) and time it. Predicted length: the probe's if it escaped, else
  `min(max_iter, 2^20 - 1) + 1`. Compute = length x seconds per probe point. Bytes =
  16 per point + 96 per chunk (slabs + manifest).
- **BLA**: build the job's table over the probe orbit, scale its time by the predicted
  length. Bytes = 56 per block of the probe table's levels at the predicted length + 96.
- Estimate = compute + chunks x per-chunk + bytes x per-byte (`est_bytes` beside the
  real `bytes`). Not modelled: encoding, and contention between parallel workers.

## Policies

Whenever a worker is free the dispatcher starts the best job whose dependencies are
done (non-preemptive list scheduling); each job runs on its own thread.

- `slack` (default): least `s_j`. `now` is common to all candidates, so this is least
  `D_j - est_j` (latest start time), ties to the earlier `D_j`, then index.
- `edf`: earliest effective deadline `D_j`, cost ignored.
- `first-use`: earliest own deadline `d_j` (first-use frame order), the naive baseline.

Every deadline shifts by the lead, and no key depends on it, so the order does not
depend on the lead: `min_lead_seconds = max(0, max_j (finish_j - first_use_j / fps))`
is the smallest lead with which these finish times meet every deadline.

The default stays `slack`, the rule the scope asks for, but it is not the best rule
everywhere:

- On one worker, EDF on effective deadlines is optimal for maximum lateness (Lawler's
  rule for precedence-constrained jobs released together); least slack is not. Measured
  on the valley path at `--size 640x360 --iter 1000000`, one worker: least slack needed
  a 3.05 s lead, EDF and first-use 2.85 s. Least slack starts the long deep-group jobs
  first, and frame 0's chunks wait (unit test
  `on_one_worker_least_slack_can_lose_to_first_use`).
- On the CI path (`--size 128x72 --iter 50000`, two workers) all three policies tie, and
  at the heavier settings with four workers they also tie (1.16 s).
- On several workers least slack can win: it starts a long job early where EDF queues it
  behind short ones (unit test `least_slack_starts_the_long_job_first_on_two_workers`).
  No measured path shows this yet.

## Compile log

Header: `frames`, `fps`, `lead_seconds`, `workers`, `policy`, `bla_mode`,
`bla_skipped_frames`. Then one line per job in start order:

```text
job NAME kind orbit|bla deps PARENT,..|- first_use F deadline D effective E est_seconds C
    start S slack_at_start X finish T actual_seconds A est_error R slack Y ready yes|no
    worker W points P est_bytes B' bytes B chunk ID
```

`chunk` is the orbit manifest id (for `fd render --orbit`) or the BLA table id (for
`--bla`). Then, for each policy, the same jobs list-scheduled in simulation with this
run's measured costs and with the estimates, starting when the real first dispatch did:
`compare POLICY costs measured|estimated missed M max_lateness_seconds L
makespan_seconds X min_lead_seconds Y`. Totals: `jobs`, `jobs.orbit`, `jobs.bla`,
`jobs.certificate`, `met`, `missed`, `max_lateness_seconds` (max `finish - deadline`;
negative is the smallest margin), `makespan_seconds`, `min_lead_seconds`,
`estimate_seconds` (all probes), the store cost lines, `cost.estimated_seconds`,
`cost.actual_seconds`, `cost.total_error`, `cost.mean_abs_error` (mean |est_error|),
`stored_bytes` (new bytes this run), then the atlas byte lines. Seconds are printed
with 6 decimals.

Exit codes: 0 when every deadline is met (or with `--on-miss report`); 1 when a chunk
missed its deadline (stderr names the minimum lead); 2 for any other failure (usage,
a builder error or panic, I/O), as for every `fd` command.

## Measured

CPU workstation (Ryzen 3900X), `bench/path-valley.txt` (9 frames, 2 orbits, 9 tables),
`--size 128x72 --iter 50000 --columns nu,de --fps 30 --workers 2`, as in CI:

| lead | met | missed | makespan s | min lead s | est total error |
|---|---|---|---|---|---|
| 0 | 8 | 3 | 0.136 | 0.056 | +0.117 |
| 2 | 11 | 0 | 0.135 | 0.033 | -0.374 |

With lead 0 the chunks of frames 0 and 1 cannot be ready: frame 0 is due at t = 0 and
the probes alone take 0.03-0.05 s. The three policies tie on this path (the DAG is two
orbits with tables; both orbits are ready at once). Heavier settings (`--size 640x360
--iter 1000000`, 16 MB of orbit, 56 MB per deep table): one worker, least slack needs a
3.05 s lead, EDF and first-use 2.85 s (it builds the deep group first and frame 0
waits); four workers, all three tie at 1.16 s. Estimates are within a few percent in
total for those large jobs and off by up to 3x for millisecond jobs, where fsync jitter
dominates; four parallel workers once took 5x the estimated total on a loaded
machine (contention is not modelled).
