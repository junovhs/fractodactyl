# Where We Are (updated 2026-10-17)

Read this first in a new session, then call `ishoo_status`. **Ishoo is the source of
truth for what to do next** (`ishoo_status`, then `ishoo_plan op:next`); this page is a
snapshot and does not override it. The north star is `ishoo_north_star op:show`. Rules
for how we work are in METHOD.md (DEC-14), and every measured result is in its results
log.

Product goals beyond the north star (the commercial feature benchmark and the
web and desktop experience goals) are in [FEATURE-BENCHMARK.md](FEATURE-BENCHMARK.md).

## The aim, in one line

The fastest deep-zoom generator that shows the real Mandelbrot set and looks visually
perfect to humans. Video is the main product: precision a viewer cannot see is cut when
it costs speed (DEC-29). Exact renders (every pixel within 1e-3 px, DEC-17) stay
available for stills and research.

## Where the speed applies

**Start with your own location.** General CPU rendering, keyframes and GPU shading do not require a special destination. A saved spot does not guarantee zone coverage, but lack of a zone does not rule out these general speedups. Zones are a separate, location-specific shortcut.

**CPU rendering without a zone.** BLA skips valid blocks of iteration; a guarded series initializer can skip a useful starting stretch. They can help arbitrary locations, but not every frame. PERF-05 reduced the total cold time for 24 frozen 480×270 views from 23.468 s to 16.582 s: 6.886 s saved, about 29% less time (1.415x). This used eight workers, counted setup and fallback, and selected the initializer on 17 views while leaving seven on their prior route. It is not a whole-film or universal multiplier. PERF-06 separately cut series construction from 0.268 s to 0.040 s in a 24-view check; its roughly 6.7x applies only to that setup stage. Do not multiply these ratios together. [Current-route census and its limits](https://github.com/junovhs/fractodactyl/blob/main/docs/research/2026-10-22-cpu-occupancy/README.md).

**Zones need the right structure.** They pay when a minibrot's orbit circles a nearby repelling cycle many times. The historical FIX-22 probe found 341 laps at v0, versus at most 4.7 at 21 other sampled minibrots. An orbit analysis of Eye of the Universe estimated at most about 1.3x extra from the spiral jump; that was not a timed render. The historical roughly 20x result applies to a covered v0 frame against per-frame BLA, not a whole film or every minibrot. [FIX-22 and orbit evidence](https://github.com/junovhs/fractodactyl/blob/main/docs/spec/METHOD.md#results-log).

**Zone support is broader than M(24,2).** PROB-36 lets `fd zone RE IM -o FILE` accept detected M(q,r) spiral-centre relations with at least 100 laps. Verification includes one covered exterior M(2,1) frame; it does not establish every relation or depth. Each zone still has a location and validity range. PROB-27's `fd zone --suggest RE IM --count N` supplies validated ladder targets. Automatic cost-aware destination selection (PROB-29) remains planned. [PROB-36 / PROB-27 tests](https://github.com/junovhs/fractodactyl/blob/main/crates/fd-cli/tests/zone_build.rs).

**Keyframes and shading work more broadly.** Default `--keyframes 2` reuses samples between zoom frames without needing a zone. Explicit `--ss`, jet-AA or widths outside normal f64 range select direct frames. GPU studio shading works on samples from any location. In the historical v0 1080p60 test on a Ryzen 9 3900X / RX 5700 XT, shading fell from 131 s to 9.8 s; GPU zone plus shading reduced the whole film from 183.6 s to 36.6 s (5.0x). These are different stages and a specific workload, not factors to combine with the CPU results. Optional `--jet-aa on` can reduce costly ss=2 subsample work without a zone; it disables keyframes and leaves zone frames exact. Its historical four-core 960×540 tendril test measured 719.7 s versus 390.5 s (1.84x render time). [Shading benchmark](https://github.com/junovhs/fractodactyl/blob/main/docs/spec/METHOD.md#benc-10-whole-film-race-gpu-against-all-core-cpu-rx-5700-xt-2026-10-16), [jet-AA evidence](https://github.com/junovhs/fractodactyl/blob/main/docs/research/10-11-26/q6-supersampling-from-jets.md#10-film-16-jet-aa-built-into-fd-film-2026-10-15).

**The viewer uses two paths.** Full-quality studio/look renders use the film kernel and can inherit eligible CPU improvements. Pan/zoom tiles currently use the plain preview route, which bypasses the series initializer. We have not measured a GUI latency or smoothness improvement from this work.

**Check the version you install.** The series CPU improvements described here landed after the published v0.9.0 tag. Building current source includes them; an older downloaded binary does not. Check [release notes](https://fractodactyl.dev/changelog/) and [downloads](https://fractodactyl.dev/download/) for your version. Remaining Tim-plan performance work is still open; no completed-plan or overall-film speedup is claimed.

## Foundation snapshot (2026-10-17)

This section is historical. The active plan is **Tim go fast**; use Ishoo for its live status.

**Outcome:** every kernel stage and the studio look exist once (GPU-route stages as
their one WGSL source; everything else in Rust), fd-kernel's tiers are one generic
kernel, and `fd film`'s GPU path runs on wgpu
with an f32-only baseline on NVIDIA, AMD and Intel under Windows and Linux, with the
ash/GLSL code deleted.

**Done so far (highlights):**
- GPU-12: the GPU source route is hand-written WGSL on wgpu (shared Rust/CubeCL was
  1.84-2.25x slower per finish stage on the RX 5700 XT).
- GPU-13: the zone keyframe kernel runs on wgpu (`crates/fd-cli/shaders/zone.wgsl`,
  `zone_finish32.wgsl`); `zone_f64.comp` and the ash zone host are gone.
- GPU-24/25/26 (2026-10-14 research): an f32-only float-float finish is 2.2-2.8x faster
  than binary64 on the RTX 3050 Ti and runs on the no-f64 Intel Iris Xe; native f64 stays
  faster on the RX 5700 XT. Two-level float32 references are killed. Float-float needs a
  reassociation guard and an fma self-test (RADV splits fma).

- SHAD-03..10, FIX-58/59, GPU-14 (2026-10-16): the studio look runs as WGSL on wgpu
  (prepare, shade, tent resample: `fd_shade::gpu`, `crates/fd-shade/shaders/`) for `fd film
  --gpu on` (rotated films, lines, haze, breathing, up to 64 stops) and `fd shade --gpu on`
  (stills at any ss), f32 and integer arithmetic only, on any adapter: hardware Vulkan,
  lavapipe, or DX12 WARP on Windows without a Vulkan driver (DXC-compiled). Palette band
  and line positions reproduce the CPU's f64 exactly via soft f64 in integers. CI runs the
  GPU tests on lavapipe (Linux) and WARP (Windows). The ash/GLSL/shaderc stack and
  `build.rs` are gone.
- SHAD-11, BENC-10 (2026-10-16): the studio look exists only as that WGSL. `fd shade`,
  `fd film` (keyframed or not), `fd play` and `fd_shade::Studio` all shade through it on
  the hardware GPU, else lavapipe/WARP; `--gpu` is accepted and ignored. fd-shade's Rust
  per-sample studio maths and `PreparedStudio` are deleted; tests compare against the CPU
  studio's output frozen beforehand (`crates/fd-shade/src/gpu/frozen`,
  `crates/fd-cli/tests/fixtures`). BENC-10 (METHOD.md): on the RX 5700 XT the GPU-02 film
  takes 36.6 s with GPU zone + GPU shading against 183.6 s all-CPU (5.0x).

- GPU-21, GPU-27, GPU-28 (2026-10-16): GPU zone keyframes stay on the device into
  studio shading. `--gpu-kernel f32only` (zone32.wgsl on GPU-27's float-float library)
  renders zone keyframes with no f64 at all. It passes DEC-29 film-equivalence (0 of
  129,600 shaded pixels over one level at all GPU-02 widths) on the RX 5700 XT and on
  lavapipe, and is chosen automatically without an f64-capable GPU.

- CLEA-06, CLEA-04 (2026-10-17): the zone exists only as WGSL. `fd render`, `fd control`,
  `fd film` and `fd explore` run every `--zone` frame on the wgpu zone kernel (binary64,
  or f32only for extended-range zones and on GPUs without f64; lavapipe/WARP without a
  GPU); a frame it declines renders with the perturbation kernel. fd-kernel's zone pixel
  stages are deleted. `zone_fixtures` holds every kernel to 24 CPU-zone frames frozen
  beforehand (`crates/fd-cli/tests/fixtures/zones`).
- GPU-22, GPU-23 (2026-10-16): mid-band frames and extended-range zones (the ~1e-1000
  rung) run on the GPU zone kernel; extended-range zones use f32only.

- PROB-30..34, PROB-12 (2026-10-17): `fd zone RE IM -o ZONE` builds a complete deep-band
  zone in Rust from its centre (structure, return map, Koenigs chart, tail patch atlas),
  line-for-line identical to the committed v0 zone; 7.8 s for v0, 66 s for the 1e-1000
  rung. PROB-35 also writes the v0 ladder's mid-band constants and reference orbit.
  CLEA-02 freezes the superseded research producers; `fd zone` is the only live producer.

**Also landed (plan "Desktop viewer"):** VIEW-06 `fd-viewer`, the V2 stone explorer in
its own Dioxus window (it runs `fd explore --exit-with-stdin 1` as a child); VIEW-08 the
explorer's navigation streams 256-sample tiles shaded on the GPU, like a map; VIEW-09 on
Linux the page is a GTK widget, so the mouse wheel keeps zooming after a drag.

**Remaining, in plan order (check `ishoo_plan op:show` for the live list):**
BENC-11 (BENC-10's race on the RTX 3050 Ti laptop) · GPU-29, SHAD-12 (the f32-only zone
kernel and the studio look on the laptop's RTX 3050 Ti and no-f64 Intel Iris Xe) ·
GPU-05 (f32-only replacements for every binary64 zone stage).
PROB-35 and CLEA-02 have completed the Rust producer and frozen its research predecessors.

## Decisions to know

Read the full record with `ishoo_decision op:show` before relying on one.
- **DEC-29** (2026-10-15, supersedes DEC-18): films are judged only by what viewers see
  (shaded frames within 1 level of the exact render on >=99.9% of pixels, no extra
  flicker, nothing visible skipped); film speed claims are made in film mode. Exact mode
  keeps DEC-17.
- **DEC-17:** a kept per-pixel shortcut passes every pixel of whole frames (exact mode
  and research acceptance).
- **DEC-24:** each piece of maths is written once and reused everywhere; the Python
  oracle is the one deliberate second copy.
- **DEC-25** (supersedes DEC-20): GPU code is wgpu and works with f32 arithmetic only;
  native GPU f64 is an optional fast path where it measures faster.
- **DEC-27:** product code and dev tooling are Rust, UIs are Dioxus, everything builds
  and runs on Windows and Linux.
- **DEC-28:** a GPU-route kernel exists once, as its WGSL; the CPU zone copy is retired
  after the f32-only path lands; no-GPU machines run the WGSL on a software adapter.
- **DEC-21:** deep centres, widths and zone scales never pass through f64.
- **DEC-14 / DEC-19:** fast probes against fair opponents; production shortcuts run only
  when a pre-render guard admits them.
- **DEC-15 / DEC-16:** cache only what is expensive to rebuild or shared across frames;
  dynamics-derived pixels are genuine under an error contract. Look-alike locations stay
  forbidden (DEC-01).

## How to use (Windows and Linux)

Prerequisites: Rust (stable) and FFmpeg on PATH for `--mp4`. The studio look (the
default) needs a wgpu adapter: any GPU's Vulkan driver, Mesa's lavapipe on Linux, or
DX12 (WARP without a GPU) on Windows. No SDK or shader compiler is needed to build (the
shaders are WGSL, compiled by wgpu at run time).
`fd` below means `<target>/release/fd` (`fd.exe` on Windows).

- Build: `cargo build --release -p fd-cli` (add `-p fd-viewer` for the desktop
  explorer; on Linux it needs WebKitGTK 4.1). `viewer\explore.cmd` expects
  `target\release\fd.exe`.
- Explorer: `target/release/fd-viewer` opens it in its own window. In a browser:
  `viewer\explore.cmd` (Windows, double-click) or `viewer/explore.sh` (Linux) for the
  classic page; `viewer/explore-v2.sh` for the V2 UI (see `viewer/README-v2.md`).
- A short v0 zone film using the committed zone file (the GPU-02 gate's arguments):

  ```
  fd film -0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502 0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922 --from 1e-29 --to 1e-31 --seconds 2 --fps 30 --size 480x270 --keyframes 2 --zone bench/zones/v0-core.zone --preset master-ice --mp4 out/v0.mp4
  ```

  Zone frames run on the wgpu zone kernel by default (binary64, or f32only without an
  f64-capable GPU); `--gpu-kernel f32|mixed|f32only` picks another. Shading is always on
  wgpu. `scripts/check/GPU-02.sh` runs the film-equivalence gate of the f32 or mixed
  kernel against binary64.
- Re-shade stored samples without iterating: `fd shade X.fds --preset ice -o DIR`.
- Other zones: `fd zone RE IM -o FILE` writes a zone from a minibrot centre
  near a detected M(q,r) spiral centre with at least 100 laps (`--period P` skips the period search), including the mid-band constants
  and reference orbit on the v0 ladder. The mid-band validity checks still apply.

## Known loose ends

- Ishoo's registered verification checks are Linux commands for the home box
  (`/home/juno/.fdv/...`, `systemd-run`); run from a Windows session they fail with
  "spawn systemd-run: program not found" (filed in the Ishoo repo as FIX-332). Until
  that is fixed, the owner allows bounded local builds and tests on Windows
  (`CARGO_BUILD_JOBS=4`); run GPU checks on the home runner through
  `.github/workflows/gpu-route-rx5700xt.yml` and general checks through `ci.yml`.
- `docs/spec/NORTH-STAR.md` is generated from `refs/ishoo/store` by
  `.github/workflows/north-star.yml` (DOCS-03) after every push to main, daily, and on
  demand. Do not edit it; change the north star in Ishoo. A bot commit can land on main
  after a north-star change, so pull before pushing.
- `docs/spec/ISHOO-LEDGER.md` is the 2026-10-06 bootstrap ledger, kept as history only;
  the live ledger is the Ishoo store.
- Cloud sessions without Ishoo may push research straight to main. Pull before pushing,
  and file their results as issues (as GPU-24/25/26 were).

## History

Earlier snapshots of this page (Atlas v0 and Gate C, the Misiurewicz/Koenigs fast path,
mid bands, the 2026-10-08 film assessment, the six-agent research roadmap, the CLEAN-01
removal table) are in Git: the 2026-10-10 version is
[`docs/spec/STATE.md` at 65db85f](https://github.com/junovhs/fractodactyl/blob/65db85f4b241d552c9b6920bfe3ce4f9e0164349/docs/spec/STATE.md).
Their measured results live in the METHOD.md results log; research write-ups are in
`docs/research/<date>/`.
