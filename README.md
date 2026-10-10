# Fractodactyl

Fractodactyl is reset around one goal:

**Make genuine deep Mandelbrot rendering dramatically cheaper by compiling a known zoom path into a bounded, reusable mathematical atlas.**

The previous twin-minibrot swap prototype is intentionally absent from this branch. It remains in Git history on `ishoo/POC-02` (commit `790be5b`) for reference.

## Start here

0. **Start with `docs/spec/STATE.md`** (where we are, what to do next), then `ishoo_status`.
1. Read `SPEC.md`.
2. Read `docs/spec/ARCHITECTURE.md`.
3. Run `ishoo_status`; Ishoo holds the live issues, plans and ADRs (`docs/spec/ISHOO-LEDGER.md` is the historical bootstrap list).
4. Read the research reports under `docs/research/` (one folder per date).
5. Read `docs/spec/METHOD.md` (how we work).

`fd` (crates/fd-cli) renders, compiles and plays genuine deep zooms; `viewer/explore.cmd` (Windows) or `viewer/explore.sh` (Linux; `--install` adds a desktop icon, `--stop` stops the renderer) opens a local browser explorer.

## Make a film

```sh
# once: the v0 zone (needs python3 with mpmath and numpy; about 15 s)
tools/research/misiurewicz/make_zone.sh data/zones/v0.zone
# a 1080p60 landing on the v0 minibrot: umber look, flowing bands, half a turn of twist
fd film -0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502 \
        0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922 \
        --from 1e-40 --to 2.5e-49 --rate 0.3 --ease on --twist 0.5 --flow 0.08 \
        --zone data/zones/v0.zone --mp4 landing.mp4
```

`fd film` renders each frame with the cheapest valid kernel: the zone fast path where
`--zone` covers it (about 20x faster), fd's per-frame BLA elsewhere. It shades in memory and
pipes the frames to ffmpeg. Film defaults: the `studio` look with the `ice` preset, 0.15
decades/s, 60 fps, 1920x1080, `--ss 2`, `--aa on`, `--unresolved interior`.

Looks: `--preset NAME` picks `looks/NAME.look` or a built-in (`ice`, `coral`, `steel`,
`zebra`, `smoke`); `--density`, `--terrace`, `--slope`, `--light`, `--lines` and `--line-px`
tweak it, and `--flow`/`--breathe`/`--drift` animate the bands. A `.look` file is a few
`key value` lines (`stops #rrggbb ...`, `density 0.05`, ...). `fd film` with no arguments
lists every flag.
