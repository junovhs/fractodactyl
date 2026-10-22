# Changelog

What changed in Fractodactyl, for people who use it. Newest first.

How we keep it (DEC-32): when something you can see or use lands, a line goes under
**Unreleased**. When a version is bumped, Unreleased becomes that version's section, and
the release built from it uses that section as its notes. Releases and the download page
at https://fractodactyl.dev/download/ follow automatically.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

- Film camera paths, saved places and keyframes now support decimal widths down to `1e-1000`, preserving exact destination centers. `--depth-cap` allows a smaller limit (default and maximum 1000 decades); deeper film requests are refused before rendering.

- Film reports now include process CPU seconds and average busy threads for each actual kernel frame, including setup and retries, with explicit measurement boundaries and unavailable-clock handling.

### Added

- Experimental, default-off `fd render/control --gpu-perturb on` integrates guarded Heuristic hi/lo GPU samples with visible dispatch refusal, bounded batches and exact-camera CPU restart. This groundwork is currently slower than CPU BLA; it has no automatic selection or film integration.

- `fd zone -o` now writes the v0 ladder's mid-depth constants and reference orbit
  in Rust, so its generated zones use the GPU mid-band renderer without a Python
  generation step. The existing mid-band validity checks still apply.

- `fd adapters` lists GPU indices, backends, vendor/device IDs, names and native
  f64 support. GPU commands accept `--adapter N`; `FD_ADAPTER` selects the viewer's
  adapter and supplies a CLI default. The flag takes precedence over the environment.

### Changed

- README, project state and the public manual explain where general CPU, keyframe, shading and zone speedups apply, with measured limits and source-versus-release distinctions.

- Zone generation instructions and the KERN-02 gates now use `fd zone`; the
  superseded research producers are frozen and their manual CI workflow is retired.

- The public style manual explains bundled preset lookup, `FD_LOOKS` precedence,
  and using an unpacked release from another working directory.

- The public film manual explains zero-based single-frame, half-open and inclusive
  `--frames` selections, with a runnable preview and empty-range guidance.

- `fd zone -o` refusal messages explain the detected period, relation, laps around
  the cycle, and why the existing guard declines to write a zone.

- Saved places use your personal data folder (or `FD_PLACES`) in installed
  packages and source builds. Packaged places remain readable without being
  overwritten by saves.

- CPU film keyframes use a guarded series initializer when a small timing probe
  predicts a net speedup over the existing renderer. Unprofitable frames retain
  their previous path, and unresolved initialized samples retry through the
  ordinary kernel. `FD_CPU_SERIES=off` enables reproducible before/after comparisons.

## [0.9.0] - 2026-10-20

### Changed

- `fd film` defaults to `--keyframes 2`. Use `--keyframes off` for direct per-frame
  rendering. Explicit `--ss`, `--jet-aa on`, and widths outside normal f64 range
  fall back to direct rendering with a short stderr note. The film report records
  the active keyframe multiplier (`null` for direct rendering).

### Added

- The website downloads page shows the latest Windows and Linux packages, checksums,
  setup instructions and release notes, with a full changelog at /changelog/.
- `fd --help` and `fd-viewer --help` print their usage and a link to the manual.

### Fixed

- Release packages no longer include two personal, unverified saved places
  (`journey_various_places`, `my_discovery`); `places/` ships only curated places.
- The website hero has a clearer headline and mascot placement, without a slogan
  or decorative window controls.

- The manual's header shows the version you are reading about (it said 0.1.0).

## [0.5.0] - 2026-10-18

The first public version. Everything below is what Fractodactyl can do today.

### Added

- **Downloads.** Ready-built packages for Windows and Linux (fd-viewer, fd, looks and
  places in one folder, with the MIT and Apache-2.0 licence texts), published
  automatically for each version. `fd --version` says which one you have.
- **One menu across fractodactyl.dev.** Every page, including the manual, has the same
  bar: Home, Docs, Explore, Download and GitHub.
- **Zoom films in one command.** `fd film` goes from a location to a 1080p60 MP4: an
  eased camera that can twist, flowing or breathing colour bands, and variable zoom speed
  from a speed-curve file. Keyframe films (`--keyframes`) render a few frames per zoom
  step and fill the rest in, and spin files add placed, eased rotations.
- **The fast path.** Near a minibrot, a zone file (`fd zone`) lets most of each deep
  pixel's work be skipped exactly, about 20x faster than per-frame rendering. It runs on
  the GPU through wgpu, in 32-bit arithmetic where the GPU has no 64-bit, and on a
  software adapter when there is no GPU at all. Elsewhere `fd film` uses per-frame BLA.
- **GPU shading.** The studio look (cyclic palettes, terraced bands, slope light, contour
  lines, haze control in dense regions) is shaded on the GPU for films and stills, with
  8-bit dither so slow gradients do not show rings.
- **Looks.** Named presets (`ice`, `coral`, `steel`, `zebra`, `smoke`, `master-ice`) and
  your own `looks/NAME.look` files, used the same way by films, stills and the explorer.
- **Places.** Save an exact location as `places/NAME.place` from the explorer and film it
  with `fd film --place NAME`.
- **Stills and checks.** `fd shade` renders studio stills on the GPU; `fd filmcheck`
  compares a film against an exact render of the same frames.
- **The desktop explorer.** `fd-viewer` opens the explorer in its own window on Windows
  and Linux: drag to pan with glide, wheel to zoom toward the cursor, places and looks in
  the sidebar, snapshots, and a tile engine that only computes what comes into view.
- **fractodactyl.dev.** The project's website, with a user manual covering the explorer,
  making videos, and the maths and research behind them.
- **Product goals.** The a leading commercial solution feature benchmark and the web and desktop goals
  we aim for (docs/spec/FEATURE-BENCHMARK.md).

### Fixed

- In the explorer, the mouse wheel keeps zooming after a drag-pan (it stopped after the
  first drag in the Linux window).
- Saved looks are found at run time, so builds made elsewhere still see your presets.
