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
