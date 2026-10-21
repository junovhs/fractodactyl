# Fractodactyl

**Manual:** https://fractodactyl.dev/docs/ · **Download:** https://fractodactyl.dev/download/ · **Changes:** [CHANGELOG.md](CHANGELOG.md)

**Fractodactyl aims to be the fastest deep-zoom generator that shows the real Mandelbrot set and looks visually perfect to humans:** much longer and much deeper zoom films, made much faster than anything else can. The north star is held in Ishoo (`ishoo_north_star op:show`) and mirrored in `docs/spec/NORTH-STAR.md`.

The speed comes from exact dynamics: near a minibrot, theorems (Tan Lei, Koenigs) turn most of each deep pixel's iterations into a few return-map loops and one exact jump, run on the GPU (wgpu). The project began as a path-compiled "mathematical atlas" (`SPEC.md`); that machinery still exists, but Gate C showed it only caches what is cheap to recompute (DEC-15), so the fast per-pixel path is the main line.

The previous twin-minibrot swap prototype is intentionally absent from this branch. It remains in Git history on `ishoo/POC-02` (commit `790be5b`) for reference.

## Start here

0. **Start with `docs/spec/STATE.md`** (where we are, what to do next), then `ishoo_status`.
1. Read `SPEC.md`.
2. Read `docs/spec/ARCHITECTURE.md`.
3. Run `ishoo_status`; Ishoo holds the live issues, plans and ADRs (`docs/spec/ISHOO-LEDGER.md` is the historical bootstrap list).
4. Read the research reports under `docs/research/` (one folder per date).
5. Read `docs/spec/METHOD.md` (how we work).

`fd` (crates/fd-cli) renders, films, compiles and plays genuine deep zooms and builds zones (`fd zone`). `fd-viewer` (crates/fd-viewer) is the explorer in its own desktop window (`cargo build --release -p fd-cli -p fd-viewer`, then `target/release/fd-viewer`; see `viewer/README-v2.md`). The browser explorer is `viewer/explore.cmd` (Windows) or `viewer/explore.sh` (Linux; `--install` adds a desktop icon, `--stop` stops the renderer); `viewer/explore-v2.sh` opens the V2 stone UI in a browser.

What changed between versions is in [CHANGELOG.md](CHANGELOG.md).

The website, https://fractodactyl.dev, lives in `site/` (Astro, with the docs written as dopedocs data in `site/src/docs.ts`); `cd site && npm run deploy` builds and publishes it.

## Make a film

A quick start; the full guide to films, looks and places is the manual at
https://fractodactyl.dev/docs/.

```sh
# once: the v0 zone, built from its centre (about 8 s)
fd zone -0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502 \
        0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922 \
        -o data/zones/v0.zone
# a 1080p60 landing on the v0 minibrot: studio look, flowing bands, half a turn of twist
fd film -0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502 \
        0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922 \
        --from 1e-40 --to 2.5e-49 --rate 0.3 --ease on --twist 0.5 --flow 0.08 \
        --zone data/zones/v0.zone --mp4 landing.mp4
```

`fd film` defaults to `--keyframes 2`: it renders oversized keyframes per 2x zoom
and resamples the intervening frames. Use `--keyframes off` to render every frame
directly. Explicit `--ss`, `--jet-aa on`, or widths outside normal f64 range fall
back to direct rendering with a note on stderr. Keyframes use the cheapest valid kernel: the zone fast path where
`--zone` covers it (about 20x faster; it runs on the wgpu zone kernel, binary64 or f32-only,
on a software adapter if there is no GPU), fd's per-frame BLA elsewhere. It shades on the
GPU (the studio look exists once, as WGSL) and pipes the frames to ffmpeg; `fd filmcheck`
compares a film against an exact render.

`fd zone RE IM -o FILE` writes the deep-band constants and, on the v0 ladder,
the mid-band constants and reference orbit in Rust. Use this command to build zones;
the earlier research producers are frozen historical evidence. Film defaults: the `studio` look with the `ice` preset, 0.15
decades/s, 60 fps, 1920x1080, `--ss 2`, `--aa on`, `--unresolved interior`.

Looks: `--preset NAME` picks `looks/NAME.look` or a built-in (`ice`, `coral`, `steel`,
`zebra`, `smoke`); `--density`, `--terrace`, `--slope`, `--light`, `--lines` and `--line-px`
tweak it, and `--flow`/`--breathe`/`--drift` animate the bands. A `.look` file is a few
`key value` lines (`stops #rrggbb ...`, `density 0.05`, ...). `fd film` with no arguments
lists every flag.
