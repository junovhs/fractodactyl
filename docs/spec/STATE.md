# Where We Are (updated 2026-10-10)

Read this first in a new session, then run `ishoo_status`. Rules for how we work are in
METHOD.md (DEC-14). Every result with its numbers is in the METHOD.md results log.

## In one paragraph

The Atlas v0 machinery is built and correct: kernels, addressing, the chunk store,
manifests, the budget, the planner, scheduler, compiler and player, late shading, the
oracle and a fair benchmark. Gate C's answer was **no**: v0 ties a renderer that builds
its own BLA per frame, because it cached only cheap things (BENCH.md, DEC-15). The
search for something worth caching produced one strong lead.

**Deep zooms are built from exact repeats that complex-dynamics theorems describe.**
- Near a Misiurewicz point, deeper frames are exact rotated and scaled copies of
  shallower ones.
- On the approach to a minibrot, every pixel is "k loops around the minibrot" plus "one
  shared exit function".

**The first real speed win is measured.** A deep pixel inside the minibrot band is now:
- a few cheap loops of the return map;
- 23 approach steps;
- one exact Koenigs jump over the spiral-out;
- 56-342 plain double steps.

There is no table and no multiprecision per pixel. On GitHub Actions (PROB-08), whole
480x270 frames from 1e-35 to 2e-48 render **8.1-25x faster than fd's per-frame BLA,
with 0 wrong pixels out of 777,600**. It is a per-frame technique (DEC-15). The
interpolated exit table (PROB-04) stays rejected; the Koenigs jump replaced it. Full
write-up: `docs/research/10-8-26/misiurewicz-frame-transfer.md`. Conversation context:
`PHILO-HANDOFF.md`.

## Six-agent research (2026-10-10): what to build next

Read `docs/research/10-10-26/six-agent-final-takeaways-and-issue-roadmap.md` first. Six
agents studied acceleration ideas; the synthesis ranks them. Their baselines differ, so the
speed-ups must not be added up.

- **Factored ladder returns (Agent #3), highest upside, unmeasured.** A long minibrot
  period on the v0 ladder is "entry + a power of the 2-cycle's Koenigs multiplier + exit".
  If the whole return operator can be built without stepping through all P iterations,
  the 1e-1000 rung (P = 16,116) gets cheap. → FACT-01, then FACT-02 (full frames).
- **Automatic cycle compiler (Agent #1), working prototype.** Branch
  `research/auto-misiurewicz-discovery` finds a repelling cycle from the camera, builds a
  Koenigs jump and declines when unprofitable: 1.44x (c = i) and 1.17x over per-frame BLA,
  cold, at 960x540. Its acceptance used a retrospective BLA frame, and it rounds deep
  centres to f64. → AUTO-01 (guards, exact coordinates), AUTO-02 (blind corpus).
- **GPU (Agent #5), nothing new measured.** The all-FP64 OpenCL zone kernel (82-131 ms per
  1080p frame, ~35x CPU BLA) is the opponent. → GPU-03 (freeze it on both GPUs), GPU-04
  (queue A/B, keep at >1.25x), GPU-05 (float-float, keep at >=1.5x).
- **Near-parabolic transits (#2), exterior fields (#4), film atlas (#6):** falsifiable
  spikes only. → PARA-01, FIEL-01, ATLA-04 (parked until operator build cost is material).
- New rules: DEC-19 (shortcuts are authorized by a pre-render guard and may abstain),
  DEC-20 (lossy GPU precision keeps an FP64 checkpoint; GPU work is scored against our FP64
  GPU kernel), DEC-21 (deep centres and widths never pass through f64).

## Mid bands (2026-10-09, evening): a strong sampled lead

Read `docs/research/10-9-26/HANDOFF-mid-band-jump.md` before working on PROB-14.

- Measuring each pixel's offset from its **own** cycle point p(c) (a closed-form shift)
  fixes the shared-tail failure. A mid-band pixel becomes 24 perturbation steps, one
  Koenigs jump and about 40 plain steps: about 64 steps at every width from 1e-6 to
  1e-24, against 172-747 direct, in doubles, with ν, de and normal.
- **Sampled only** (250 random pixels per width, Python). The next step is the whole-frame
  Rust bench against `fd --bla per-frame`; until then this is not a kept result.
- Estimate if it holds: 5-10x on the whole v0 film against per-frame BLA (2.5x today).
- It was written in a cloud session without Ishoo: the handoff lists the issues to file.

## Films (2026-10-08, afternoon): what exists now

Plan "Films: fast 60 fps deep zooms with effects" (active).

**Landed:**
- **KERN-01:** the zone fast path in fd (`--zone`). The v0 band is 24x faster, every
  pixel is scored, and the whole film runs 2.5x faster.
- **FX-01:** animated bands, filament anti-aliasing, unresolved-as-interior, threaded
  shading.
- **FILM-01:** `fd film`, one command to an mp4.
- **FX-02:** the studio look (linear-nu bands, terraces, slope light, terrain lines) as
  `.look` presets: ice, coral, steel, zebra, smoke. It is the film default.
- **EXPL-05:** look mode in the explorer (L), with a GPU re-colour of a frozen
  full-quality render and saved presets.

**How to use:**
- `viewer/explore.sh`, navigate, press L, tune, save NAME.
- `fd film RE IM --to W --preset NAME --zone data/zones/v0.zone --mp4 out.mp4`, with
  `--chroma 444` for exact thin coloured lines.
- `data/zones/v0.zone` is generated by `tools/research/misiurewicz/make_zone.sh`.

**Measured** (desktop, 1080p60, ss 2): the v0 minibrot landing, 28.7 s of film, rendered
in 891 s against an estimated 6.3 h with per-frame BLA (25.4x on the compared frames).

**Owner taste** (2026-10-08): umber was rejected as "ugly". They want KF-style crisp cool
palettes, terraces and terrain lines; their favourite is ice with lines.

**Open in this plan:**
- PROB-12 (`fd zone`: Rust zone constants plus new spiral locations, so films can go
  beyond v0);
- PROB-14 (mid-band speed);
- FIX-04 (deep interior without a zone; zoned views are already solved, see its comment).

## Assessment (2026-10-08): when do films get faster?

- **The fast path is real, but it isn't usable yet.**
  - It runs 20-54x faster than fd at 1080p and 22-28x faster than fraktaler-3, with
    0 wrong pixels.
  - It lives only in `tools/research/.../koenigs_bench`, with per-zone constants from
    Python, for one hand-found zone, on frames narrower than about 1e-29.
- **Amdahl.** On the BENC-01 timings (960x540), frames at 1e-29 and deeper are 59% of
  v0 render time (284 s of 478 s). 20-50x on that band gives **about 2.3x on the whole
  v0 film**.
- **It only helps paths aimed at a spiral centre.** v0 skips 89% of the orbit. Random
  minibrots get almost nothing, and "Eye of the Universe" about 24% (PROB-12 seed tests).
  Shallow films such as the owner's 4e-3 → 2.5e-5 seahorse zoom get nothing.
- **Bigger levers:**
  - **Mid bands (PROB-14):** if the co-moving chart works from about 1e-9 down, the
    estimate is about 8x on the whole v0 film.
  - **GPU (GPU-03 → GPU-04/GPU-05):** the only lever that speeds up every frame. The
    laptop GPU kernel ran about 35x faster than fd.
- **Parameter-transfer report**
  (`docs/research/10-8-26/parameter-dependant-transfer-maps-rendering.md`), read
  2026-10-08:
  - Keep its parameter-dependent Poincaré chart (λ(c) = 4(c+1) for the 2-cycle;
    F_c^m(L_c(w)) = L_c(λ(c)^m w)) → PROB-14.
  - Its Taylor-model remainder r' ≤ 2Br + r² + τ → PROB-13.
  - Its "direct bivariate transfer patches" amount to higher-order BLA → ACC-03, with
    modest gains expected.

## Do next (2026-10-10): follow Ishoo

Ishoo is the source of truth for order: `ishoo_status`, then `ishoo_plan op:next`. Plans
are organised by outcome, with dependency edges. In priority order:

0. **Correctness gate first** (external review 2026-10-09; all findings confirmed).
   GATE-01 (`fd compare` passes NaN/inf and compares widths as f64) → GATE-02 (score de
   and normal) → FIX-35 (scaled kernel declares Interior without a contraction check) →
   FIX-36 (.fds bound column and reader caps), in plan "Proving suite". Almost every
   speed issue below depends on GATE-01/GATE-02. KERN-04 (zones at any depth) and FILM-09
   (film widths past 1e-308) apply DEC-21.
   **Can run now, in parallel:** FACT-01 (factored-return probe, no blockers).
1. **Fast path everywhere** (active). PROB-14 (mid-band jump, whole frames in Rust; read
   `docs/research/10-9-26/HANDOFF-mid-band-jump.md`) → PROB-10 (v0 deep band, whole
   frames) → PROB-17 (1e-100 and 1e-1000 ladder rungs, conventional construction) →
   PROB-11 → PROB-16 (census of famous zooms) → KERN-02 (mid-band path in `fd --zone`;
   this is what makes films faster) → PROB-12 (`fd zone`, now waits on AUTO-02) →
   PROB-15, KERN-03, KERN-04, PROB-13.
2. **Compiled dynamics: depth and generality** (new). FACT-01 → FACT-02 (correct
   ~1e-1000 frames from a factored operator; lands after FIX-03 so the opponent is real
   BLA) · AUTO-01 → AUTO-02 (blind corpus) · PARA-01 · FIEL-01.
3. **GPU: beat our own FP64 kernel** (new). GPU-03 (absorbs BENC-09) → GPU-04 → GPU-05.
   GPU-02 (GPU film kernel, in Films) now waits on GPU-05.
4. **Films.** FILM-07 → FIX-04 → EXPL-09 → FX-05 → GPU-02 → EXPL-02.
5. **Feature map and trip design.** MAP-01 → MAP-03 → MAP-02 → MAP-04. Waits on PROB-12.
6. **Proving suite.** The gate issues above, then TRUT-01 → PROB-01 → FILM-08 → FIX-10 →
   BENC-02 → BENC-03.
7. **BLA must-haves: fallback and honest opponent** (was "Cold renderer"). TRUT-01 →
   PROB-01, FIX-09 (cheap form: per-frame plain-or-BLA pick) → FIX-03 (scaled-tier BLA;
   the fallback for every declined pixel below 1e-300 and the fair opponent at 1e-1000;
   until it lands, 1e-1000 speed-ups are against plain perturbation) → RESE-02.
8. **Credibility.** BENC-08 (FractalShark; its float32 part moved to GPU-05), BENC-05,
   BENC-07, BENC-06.
9. **BLA extras: maybe later.** ACC-03 (higher-order BLA), ACC-02 (certified BLA bounds).
   Start only if a probe shows BLA fallback is still a material share of film time after
   the fast path and the must-haves land.
10. **Parked: atlas ideas.** ATLA-04, PROB-05, PROB-02, REF-05, LOD-03, LOD-04, LOD-06.
   Revisit only if a probe earns it.

Retired on 2026-10-10 (kept in Ishoo as knowledge): RET-01, RET-02 → FACT-01; GPU-01 →
GPU-04; BENC-09 → GPU-03; CERT-01 → PROB-13; PROB-06 → AUTO-01. Declined as atlas-era
work DEC-15 rules out: TILE-01, SAMP-01, SAMP-02, REF-03, REF-04, INTE-01.

Key findings from 2026-10-09 (all in the METHOD.md results log):
- **Mid bands:** the jump at each pixel's own cycle point passed probes A-F (sampled, not
  yet whole frames). About 64 steps per pixel at every width from 1e-6 to 1e-24, in
  doubles, with ν, de and normal. The finish must stay double; all-float32 is about
  1e-2 px.
- **Ladders (probe G):** v0's minibrot is rung 0 of an exact ladder on M(24,2). Rungs at
  1e-100 and 1e-1000 were found by Newton in seconds (`ladder_rungs.txt`): exact film
  destinations at any depth.
- **Cost down a ladder (probe H):** returns per pixel grow like log2(n) (about 4 at v0, 5
  at 1e-100, 8 at 1e-1000), not flat. The "universal landing" lookup is rejected.
- **Long cycles (probe E):** the jump works on period-197/655 cycles in "Eye of the
  Universe", but its radius is small; nested minibrots need return maps there.
- **Killed:** hex sample lattices; OpenAI's 2026-10-06 math release as a speed lever.

## Decisions to know

- DEC-01 to DEC-10: the atlas principles. DEC-13 (orbit slabs) is accepted as the
  atlas store format; DEC-11 (tile sizes) and DEC-12 (offline certificates) are
  superseded by DEC-15 and DEC-19.
- DEC-14: fast probes against fair opponents.
- DEC-15: Gate C's lesson. The atlas must hold work that is expensive to rebuild or
  inherently cross-frame.
- DEC-16 (accepted 2026-10-10): pixels derived from shallower data at the same location
  by proven dynamics are genuine, under an error contract with fallback. Look-alikes stay
  forbidden (DEC-01).
- DEC-17: a per-pixel shortcut is kept only after it passes every pixel of whole frames.
- DEC-18: films may use shortcuts viewers cannot tell apart (measured by FILM-08);
  benchmarks keep 1e-3 px.
- DEC-19, DEC-20, DEC-21 (accepted 2026-10-10): see "Six-agent research" above.

## Things that exist for the owner

- `viewer/explore.cmd`: double-click to open the browser explorer (EXPLORE.md; follow-up ideas are in EXPL-02).
- `fd play --mp4` and `tools/research/twist_zoom_path.py`: zoom videos with speed ramps
  and twists. Three demo videos are in the owner's Downloads/mandelbrot.
- Deep-research reports: `docs/research/<date>/`. Process: before a new investigation,
  the agent drafts 2-3 questions, the owner runs them, and the answers are filed there.

## Removed code (CLEAN-01)

Only disconnected one-off or rejected work was removed. Historical source remains available at [the last commit containing every item](https://github.com/junovhs/fractodactyl/tree/83c908dcc7833e0b70c12224be59342a8d074f90); no `.ishoo/` content was changed.

| Removed item | Abandoned/superseded evidence | Last containing commit |
|---|---|---|
| `.github/workflows/f3-bench.yml` | BENC-04: completed Windows-only head-to-head; METHOD.md BENC-04 rows | [`83c908d`](https://github.com/junovhs/fractodactyl/blob/83c908dcc7833e0b70c12224be59342a8d074f90/.github/workflows/f3-bench.yml) |
| `.github/workflows/prob17.yml` | PROB-17: branch-specific one-off; METHOD.md says superseded by PROB-19 | [`83c908d`](https://github.com/junovhs/fractodactyl/blob/83c908dcc7833e0b70c12224be59342a8d074f90/.github/workflows/prob17.yml) |
| `scripts/explore_look_parity.sh` | EXPL-05: finished manual Chrome parity run; studio look shipped (STATE.md) | [`83c908d`](https://github.com/junovhs/fractodactyl/blob/83c908dcc7833e0b70c12224be59342a8d074f90/scripts/explore_look_parity.sh) |
| `tools/research/sampling/hex_sample_lattice_probe.py` | 2026-10-09 METHOD.md: hex lattice killed (no sample-efficiency win) | [`83c908d`](https://github.com/junovhs/fractodactyl/blob/83c908dcc7833e0b70c12224be59342a8d074f90/tools/research/sampling/hex_sample_lattice_probe.py) |
| `tools/research/misiurewicz/naive_z2_map.py` | Research README: first failed quadratic parameter-map hypothesis | [`83c908d`](https://github.com/junovhs/fractodactyl/blob/83c908dcc7833e0b70c12224be59342a8d074f90/tools/research/misiurewicz/naive_z2_map.py) |
| `tools/research/misiurewicz/tanlei_one_return.py` | Research README: earlier one-return map fails deeper; replaced by returns_exit_tail | [`83c908d`](https://github.com/junovhs/fractodactyl/blob/83c908dcc7833e0b70c12224be59342a8d074f90/tools/research/misiurewicz/tanlei_one_return.py) |
| `tools/research/misiurewicz/compare_fds_basic.py` | Research README: earlier comparator; fd compare now scores complete frames | [`83c908d`](https://github.com/junovhs/fractodactyl/blob/83c908dcc7833e0b70c12224be59342a8d074f90/tools/research/misiurewicz/compare_fds_basic.py) |
| `tools/research/misiurewicz/de_histogram.py` | Research README: analytic patch hypothesis killed; DEC-15 | [`83c908d`](https://github.com/junovhs/fractodactyl/blob/83c908dcc7833e0b70c12224be59342a8d074f90/tools/research/misiurewicz/de_histogram.py) |
| `tools/research/misiurewicz/float32_stages.py` | PROB-14 F: all-float32 misses the 1e-3 px bar (METHOD.md) | [`83c908d`](https://github.com/junovhs/fractodactyl/blob/83c908dcc7833e0b70c12224be59342a8d074f90/tools/research/misiurewicz/float32_stages.py) |
| `tools/research/misiurewicz/f32_frame.py` | PROB-14 F follow-up: all-float frame not eligible for DEC-17 promotion | [`83c908d`](https://github.com/junovhs/fractodactyl/blob/83c908dcc7833e0b70c12224be59342a8d074f90/tools/research/misiurewicz/f32_frame.py) |
| `tools/research/misiurewicz/f32_compare.py` | Only the removed all-float frame comparison consumes it (PROB-14 F) | [`83c908d`](https://github.com/junovhs/fractodactyl/blob/83c908dcc7833e0b70c12224be59342a8d074f90/tools/research/misiurewicz/f32_compare.py) |
| `tools/research/looks/proto.py` | FX-02: appearance prototype superseded by shipped fd shade studio | [`83c908d`](https://github.com/junovhs/fractodactyl/blob/83c908dcc7833e0b70c12224be59342a8d074f90/tools/research/looks/proto.py) |
| `tools/research/looks/fds.py` | Only the superseded looks/proto.py imports this prototype reader | [`83c908d`](https://github.com/junovhs/fractodactyl/blob/83c908dcc7833e0b70c12224be59342a8d074f90/tools/research/looks/fds.py) |
| `tools/research/misiurewicz/koenigs_bench/f3_bench.sh` | BENC-04 finished; invoked only by removed f3-bench workflow | [`83c908d`](https://github.com/junovhs/fractodactyl/blob/83c908dcc7833e0b70c12224be59342a8d074f90/tools/research/misiurewicz/koenigs_bench/f3_bench.sh) |
| `tools/research/misiurewicz/koenigs_bench/compare_f3.py` | BENC-04 one-off jittered EXR comparator, called by f3_bench.sh | [`83c908d`](https://github.com/junovhs/fractodactyl/blob/83c908dcc7833e0b70c12224be59342a8d074f90/tools/research/misiurewicz/koenigs_bench/compare_f3.py) |
| `tools/research/misiurewicz/koenigs_bench/report_f3.py` | BENC-04 one-off reporter, called by f3_bench.sh | [`83c908d`](https://github.com/junovhs/fractodactyl/blob/83c908dcc7833e0b70c12224be59342a8d074f90/tools/research/misiurewicz/koenigs_bench/report_f3.py) |
| `tools/research/misiurewicz/koenigs_bench/truth_f3.py` | BENC-04 one-off adjudication; result preserved in METHOD.md | [`83c908d`](https://github.com/junovhs/fractodactyl/blob/83c908dcc7833e0b70c12224be59342a8d074f90/tools/research/misiurewicz/koenigs_bench/truth_f3.py) |

### Atlas v0 usage (owner decision; no deletion)

All eight parked Atlas commands remain available through `fd` and its tests. “Not on the live frame/film path” means the path is opt-in, **not** that its command is dead. Gate C was 0.996x (BENC-01; DEC-15).

| Atlas v0 piece | Reachable today | Not on the live frame/film path |
|---|---|---|
| `crates/fd-atlas` chunk/store/slab/BLA | `fd chunk`, `fd orbit`, stored `fd render`, `fd compile`/`play`; tested by CI | `fd film` does not consume an Atlas v0 store |
| `fd chunk` | Public CLI for store put/get/verify/stats/budget; also internal store users above | Not automatically called by `fd film` |
| `fd manifest` | CLI tile/frame/show/walk; CI walks compiler output | No manifest playback in `fd film` |
| `fd plan` | CLI and integration tests for path planning | No call from normal `render`, `control`, or `film` |
| `fd reuse` | CLI, exercised on `bench/path-valley.txt` by CI (REF-02) | No call from `fd film` |
| `fd schedule` | CLI and CI deadline test (SCHE-02) | No call from `fd film` |
| `fd compile` | CLI and CI v0 path compile (VIDE-01) | Separate ahead-of-time route, not `fd film` |
| `fd lod` | CLI inspection and integration tests | `fd render --refine` has its own live path; `fd film` does not call `fd lod` |
| `fd play` | CLI, integration tests, and owner-facing `fd play --mp4` (STATE.md) | `fd film` directly renders/shades/encodes instead |
| `fd addr` / `fd orbit` | CLI and stored-orbit render/CI paths | Not required by default independent `fd render` or `fd film` |

### Considered but kept

- **Rust surface:** workspace dispatch and cross-crate call review found no provably dead independent CLI/module to delete. All `fd` subcommands remain dispatched; public Atlas APIs are retained for the owner instead of inferring dead code from test-only callers.
- **Bench inputs:** `bench/path-atlas-v0.txt` feeds the v0 compiler and film checks; `bench/path-valley.txt` and `bench/locations.txt` feed CI; `bench/zones/v0-core.zone` is embedded by kernel/film tests; `bench/benc-01-results.json` preserves the BENC-01 baseline (METHOD.md). None is an unused disposable fixture.
- **Live/future research:** `make_zone.sh`, `koenigs_bench`, `auto_cycle_bench`, `auto_discover.py`, `run_rungs.sh`, `factored_return.py`, `gpu_bench.py`, and the corresponding workflows stay for PROB-14/19, AUTO-01/02, FACT-02, and GPU-03/04/05. BENC-08 `single_precision.py` stays as a GPU-05 baseline, not an approved shortcut.
- **CI and manual parity:** `.github/workflows/ci.yml` and `scripts/{check,bench,locations}.sh` guard shipped paths; `koenigs-bench.yml` and `auto-misiurewicz.yml` exercise active research. The browser-driving tool remains for live explorer investigations.
- **Historical reports:** METHOD.md and `docs/research/` retain measured results and historical commands. Removed sources remain in the Git parent above, even when old historical reproduction commands now require that revision.

## Known loose ends

- On Windows, the `bench.rs`/`control.rs` tests expect Linux peak-RSS fields (FIX-10).
- The ACC-01 worktree leaked (`.ishoo/worktrees/ACC-01`); clean it with Ishoo.
- `docs/spec/ISHOO-LEDGER.md` is the 2026-10-06 bootstrap ledger, kept as history only;
  the live ledger is the Ishoo store.
- The Ishoo binary is behind its source (`ishoo reinstall`).
- Cloud sessions without Ishoo push research straight to main. Pull before pushing, and
  file their results on the matching issues.
- Pushes occasionally fail an LFS lock check; it cleared on retry. If it persists:
  `git config lfs.https://github.com/junovhs/fractodactyl.git/info/lfs.locksverify false`
  (needs the owner's OK).
