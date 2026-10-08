# Local Explorer (EXPL-01, sidequest)

A dev tool for looking around, not product UI and not part of the atlas gates.
Code: `crates/fd-cli/src/explore.rs` (server) and `crates/fd-cli/src/explore.html` (page,
embedded in the binary).

```text
fd explore [--port 8737] [--threads N]
```

Or double-click `viewer/explore.cmd`. It builds `fd` if needed, starts the server unless
it is already running, and opens Chrome on `http://127.0.0.1:8737/`.

## Server

A std-only HTTP server bound to 127.0.0.1. `GET /` returns the page.
`GET /render?re=&im=&width=&w=&h=&ss=&iter=&gen=` renders the view with the ordinary
kernel (`render_with`, auto tier, no atlas). It shades the result with the relief look
(DEC-07) and returns raw RGB8 rows. The `X-Width` and `X-Height` headers give the size,
and `X-Info` gives the kernel, seconds, iterations and unresolved fraction. Renders run
one at a time. A request whose `gen` is older than the newest one received is answered
`204` and is not rendered.

**Display only:** samples that reach the iteration budget are drawn as interior (black),
the usual max-iteration convention. The page shows their fraction as `unresolved`.

## Page

- The centre is BigInt decimal fixed point, with about 20 digits past the width, so panning
  is exact at any depth. The width is kept as log10.
- Mouse wheel zooms toward the cursor (0.12 decades per 100 wheel pixels). Drag pans.
  Keys 1-9 and 0 jump to the ten presets, which range from the whole set to `c = i` at 1e-1000.
- Refinement goes coarse to fine, Blender style: 1/8, 1/4 and 1/2 resolution, then full
  resolution, then full resolution with 2x2 supersampling. Levels at 1/2 resolution and finer
  render in row strips sized to about 0.2 s each, so a new view never waits long. While the
  view moves, the last image is drawn scaled and shifted until a new coarse frame lands. That
  stand-in is display only; every image it replaces is a genuine render.
- The iteration budget is the preset's, raised with depth
  (`1000 + 2000 x decades`, capped at 100000).

## Look mode (EXPL-05)

Press **L**, or the "tune studio look" button, to tune a look on the current view.
- **Frozen view.** The view is frozen and rendered once at full quality: window size,
  2x2 supersampling, the zone fast path where it covers the view (`data/zones/v0.zone` is
  loaded automatically when present, or `--zone FILE`), otherwise per-frame BLA.
  Navigation pauses until you leave with L or Esc.
- **GPU shading.** The server sends every sample (`/render?...&raw=2`: four f32 per
  sample: nu minus `nubase` from `X-Info`, de, normal angle, class). The page uploads
  them once as a float texture. A WebGL2 shader applies the studio look, the same maths
  as `crates/fd-shade/src/studio.rs`, so sliders, colour pickers and flow/breathe
  animation re-colour at display rate, with no re-render and no pixelation.
- **Controls:**
  - preset menu;
  - palette stops (click a swatch to edit, × to remove, + to add, 2-16 stops);
  - interior colour, density (log slider), terrace, terrain lines and line width;
  - slope strength and light angle;
  - flow, breathe and breathe rate;
  - filament anti-aliasing and "undecided as interior".
- **Save.** `POST /look?name=NAME` writes `looks/NAME.look`, validated and normalised by
  `fd_shade::Look`. Names are `[A-Za-z0-9_-]`. `GET /looks` lists built-ins plus saved
  presets; `GET /look?name=` returns one. Render the preset with
  `fd film ... --preset NAME` (or `fd shade/play --preset NAME`) from the repo root.
- **Parity.** `scripts/explore_look_parity.sh` runs look mode in headless Chrome
  (SwiftShader WebGL2) and compares its pixels with `fd shade` on the same samples.
  - 2026-10-08: mean abs diff 0.002-0.003 levels at 2.7e-38 (ice, coral) and 2.5e-49 (ice).
  - Explorer still vs `fd film --preset` frame: 0.39 with `--chroma 444`. With the default
    4:2:0 it is 7.0, all of it chroma subsampling (film frame vs `fd shade` through
    yuv420p: 0.0).
