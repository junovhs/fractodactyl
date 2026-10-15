# Where We Are (updated 2026-10-15)

Read this first in a new session, then call `ishoo_status`. **Ishoo is the source of
truth for what to do next** (`ishoo_status`, then `ishoo_plan op:next`); this page is a
snapshot and does not override it. The north star is `ishoo_north_star op:show`. Rules
for how we work are in METHOD.md (DEC-14), and every measured result is in its results
log.

## The aim, in one line

The fastest deep-zoom generator that shows the real Mandelbrot set and looks visually
perfect to humans. Video is the main product: precision a viewer cannot see is cut when
it costs speed (DEC-29). Exact renders (every pixel within 1e-3 px, DEC-17) stay
available for stills and research.

## Active plan: "Foundation: one source, portable 32-bit-first GPU"

**Outcome:** every kernel stage and the studio look exist once (GPU-route stages as
their one WGSL source; everything else in Rust), and `fd film`'s GPU path runs on wgpu
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

**Remaining, in plan order (check `ishoo_plan op:show` for the live list):**
SHAD-02 (studio look once, in f32-only WGSL) → GPU-14 (delete the ash/shaderc stack),
BENC-10 (whole-film GPU vs all-core CPU race), GPU-21 (zone output stays on the GPU into
shading) · GPU-05 (f32-only zone kernel, gated by film-equivalence) → GPU-22 (mid-band
frames on the GPU), GPU-23 (extended-range zones on the GPU) → CLEA-04 (CPU zone copy
deleted) · PROB-12 (`fd zone` in Rust) → CLEA-02 (retire research copies of the zone
pipeline).

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

Prerequisites: Rust (stable), FFmpeg on PATH for `--mp4`, and, until GPU-14 removes
the GLSL film shaders, CMake, Ninja and Python on PATH: shaderc-sys builds shaderc from
source when no system shaderc is found. `fd` below means `<target>/release/fd`
(`fd.exe` on Windows).

- Build: `cargo build --release -p fd-cli`. On Windows, keep the target directory
  short (for example `CARGO_TARGET_DIR=%USERPROFILE%\.fdt`): shaderc's source build
  exceeds the path-length limit under a deep target such as
  `target/ishoo-worktrees/shared` (`fatal error C1083: Cannot open compiler generated
  file`). `viewer\explore.cmd` expects `target\release\fd.exe`.
- Explorer: `viewer\explore.cmd` (Windows, double-click) or `viewer/explore.sh`
  (Linux); `viewer/explore-v2.sh` opens the V2 UI (Linux, see `viewer/README-v2.md`).
- A short v0 zone film using the committed zone file (the GPU-02 gate's arguments):

  ```
  fd film -0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502 0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922 --from 1e-29 --to 1e-31 --seconds 2 --fps 30 --size 480x270 --keyframes 2 --zone bench/zones/v0-core.zone --preset master-ice --mp4 out/v0.mp4
  ```

  Add `--gpu-kernel mixed` (zone keyframes on the wgpu zone kernel) and `--gpu on` (GPU
  shading). Until SHAD-02 lands, GPU shading declines looks with lines, haze, breathing
  or more than 32 stops (master-ice among them) and shades on the CPU, saying so on
  stderr.
  `scripts/check/GPU-02.sh` runs the full CPU-vs-GPU film-equivalence gate.
- Re-shade stored samples without iterating: `fd shade X.fds --preset ice -o DIR`.
- Other zones: `tools/research/misiurewicz/make_zone.sh` generates zone files (Python
  research generator until PROB-12 lands `fd zone`).

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
