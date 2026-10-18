# Local Explorer (EXPL-01, sidequest)

A dev tool for looking around, not product UI and not part of the atlas gates.
Code: `crates/fd-cli/src/explore.rs` (server) and `crates/fd-cli/src/explore.html` (page,
embedded in the binary).

```text
fd explore [--port 8737] [--threads N] [--zone FILE] [--exit-with-stdin 1]
```

Three ways in:
- `fd-viewer` (crates/fd-viewer, VIEW-06): the V2 page in its own desktop window. It starts
  `fd explore --exit-with-stdin 1` on a free port as a child, which exits when the viewer
  does. See `viewer/README-v2.md`.
- `viewer/explore.cmd` (Windows) or `viewer/explore.sh` (Linux): builds `fd` if needed,
  starts the server unless it is already running, and opens Chrome on
  `http://127.0.0.1:8737/` (the classic page).
- `viewer/explore-v2.sh`: the same server, opening the V2 stone UI at `/v2/`, which embeds
  the classic page (`/?embed=1`) in canvas-only mode.

## Server

A std-only HTTP server bound to 127.0.0.1. `GET /` returns the page and `GET /v2/` the
V2 shell (`viewer/mockup`, compiled into the binary). Other routes: `/looks`, `/look`
(get and save), `/places`, `/place` (save), and `/capture` (tests).
`GET /render?re=&im=&width=&w=&h=&ss=&iter=&gen=` renders the view with the ordinary
kernel (`render_with`, auto tier, no atlas). By default it shades the result with the
relief look (DEC-07) and returns raw RGB8 rows. `raw=1` returns per-pixel shading
inputs, `raw=2` every sample from the film's cheapest valid kernel (look mode), and
`raw=3` every sample from the fast navigation kernel (the tiles below). The `X-Width` and `X-Height` headers give the size,
and `X-Info` gives the kernel, seconds, iterations and unresolved fraction. Renders run
one at a time. A request whose `gen` is older than the newest one received is answered
`204` and is not rendered.

**Display only:** samples that reach the iteration budget are drawn as interior (black),
the usual max-iteration convention. The page shows their fraction as `unresolved`.

## Page

- The centre is BigInt decimal fixed point, with about 20 digits past the width, so panning
  is exact at any depth. The width is kept as log10.
- **Camera.** The mouse wheel eases the zoom toward its target about the cursor (0.12
  decades per 100 wheel pixels). A drag moves the view at once and glides on release;
  keyboard and button moves glide too. Keys 1-9 and 0 jump to the ten places, which range
  from the whole set to `c = i` at 1e-1000.
- **Tiles (VIEW-08).** Navigation streams tiles, as a map does. Level z cuts the plane
  into squares of width 4·2^-z; each is rendered once as 256x256 samples (`raw=3`) and
  kept as an RGBA32F WebGL2 texture (LRU budget of 400 tiles). Tile centres and widths
  are exact decimals built with BigInt. Each frame draws every cached tile in view,
  finest first, with coarser levels filling the gaps, and the GPU colours each sample
  with the current look. Missing tiles are fetched in priority order: a coarse fill,
  then 1 sample per pixel, then 2x2 per pixel once the view has been still for 150 ms,
  then a prefetch ring and the level above; within each class, tiles nearest where the
  view is heading come first. Tiles already covered by finer cached ones are skipped.
  Coarse tiles drawn while finer ones load are display only; every tile is a genuine
  render.
- **Looks.** The cosine palettes and the studio looks are shaded on the GPU; studio
  looks use `shadeSample`, one GLSL string shared by navigation and look mode.
- **Iteration budget.** The place's budget, raised with depth
  (`1000 + 2000 x decades`, capped at 100000).
- **Reuse probe.** `viewer/probe.sh` runs the page in headless Chrome with `?probe=1`
  (settle, pan half a screen, zoom in 2x) and reports what each step rendered.

## Look mode (EXPL-05)

Press **L**, or the "tune studio look" button, to tune a look on the current view.
- **Frozen view.** The view is frozen and rendered once at full quality: window size,
  2x2 supersampling, the zone fast path where it covers the view (`data/zones/v0.zone` is
  loaded automatically when present, or `--zone FILE`), otherwise per-frame BLA.
  Navigation pauses until you leave with L or Esc.
- **GPU shading.** The server sends every sample (`/render?...&raw=2`: four f32 per
  sample: nu minus `nubase` from `X-Info`, de, normal angle, class). The page uploads
  them once as a float texture. A WebGL2 shader (`shadeSample`) applies the studio look,
  a GLSL port of the WGSL look in `crates/fd-shade/shaders/`, so sliders, colour pickers and flow/breathe
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
- **Parity.** The page's GLSL is a second copy of the studio look (DEC-24 allows only
  the oracle as one), kept until the viewer shades through the film pipeline (DEC-26).
  The parity script (`scripts/explore_look_parity.sh`, headless Chrome against
  `fd shade`) was removed in CLEA cleanup (e7eb7350), so parity is not checked
  automatically now. Its last results:
  - 2026-10-08: mean abs diff 0.002-0.003 levels at 2.7e-38 (ice, coral) and 2.5e-49 (ice).
  - Explorer still vs `fd film --preset` frame: 0.39 with `--chroma 444`. With the default
    4:2:0 it is 7.0, all of it chroma subsampling (film frame vs `fd shade` through
    yuv420p: 0.0).
