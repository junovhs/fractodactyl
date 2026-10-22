# Fractodactyl

**Manual:** https://fractodactyl.dev/docs/ · **Download:** https://fractodactyl.dev/download/ · **Changes:** [CHANGELOG.md](CHANGELOG.md)

**Fractodactyl aims to be the fastest deep-zoom generator that shows the real Mandelbrot set and looks visually perfect to humans:** much longer and much deeper zoom films, made much faster than anything else can. The north star is held in Ishoo (`ishoo_north_star op:show`) and mirrored in `docs/spec/NORTH-STAR.md`.

General CPU shortcuts and reuse between zoom frames help ordinary destinations. Near a suitable minibrot, location-specific dynamics (Tan Lei, Koenigs) can turn much of each deep pixel's iteration work into a few return-map loops and a jump on the GPU (wgpu). The project began as a path-compiled "mathematical atlas" (`SPEC.md`); that machinery still exists, but Gate C showed it only caches what is cheap to recompute (DEC-15), so the fast per-pixel path is the main line.

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

## Where the speed applies

**Start with your own location.** General CPU rendering, keyframes and GPU shading do not require a special destination. A saved spot does not guarantee zone coverage, but lack of a zone does not rule out these general speedups. Zones are a separate, location-specific shortcut.

**CPU rendering without a zone.** BLA skips valid blocks of iteration; a guarded series initializer can skip a useful starting stretch. They can help arbitrary locations, but not every frame. PERF-05 reduced the total cold time for 24 frozen 480×270 views from 23.468 s to 16.582 s: 6.886 s saved, about 29% less time (1.415x). This used eight workers, counted setup and fallback, and selected the initializer on 17 views while leaving seven on their prior route. It is not a whole-film or universal multiplier. PERF-06 separately cut series construction from 0.268 s to 0.040 s in a 24-view check; its roughly 6.7x applies only to that setup stage. Do not multiply these ratios together. [Current-route census and its limits](https://github.com/junovhs/fractodactyl/blob/main/docs/research/2026-10-22-cpu-occupancy/README.md).

**Zones need the right structure.** They pay when a minibrot's orbit circles a nearby repelling cycle many times. The historical FIX-22 probe found 341 laps at v0, versus at most 4.7 at 21 other sampled minibrots. An orbit analysis of Eye of the Universe estimated at most about 1.3x extra from the spiral jump; that was not a timed render. The historical roughly 20x result applies to a covered v0 frame against per-frame BLA, not a whole film or every minibrot. [FIX-22 and orbit evidence](https://github.com/junovhs/fractodactyl/blob/main/docs/spec/METHOD.md#results-log).

**Zone support is broader than M(24,2).** PROB-36 lets `fd zone RE IM -o FILE` accept detected M(q,r) spiral-centre relations with at least 100 laps. Verification includes one covered exterior M(2,1) frame; it does not establish every relation or depth. Each zone still has a location and validity range. PROB-27's `fd zone --suggest RE IM --count N` supplies validated ladder targets. Automatic cost-aware destination selection (PROB-29) remains planned. [PROB-36 / PROB-27 tests](https://github.com/junovhs/fractodactyl/blob/main/crates/fd-cli/tests/zone_build.rs).

**Keyframes and shading work more broadly.** Default `--keyframes 2` reuses samples between zoom frames without needing a zone. Explicit `--ss`, jet-AA or widths outside normal f64 range select direct frames. GPU studio shading works on samples from any location. In the historical v0 1080p60 test on a Ryzen 9 3900X / RX 5700 XT, shading fell from 131 s to 9.8 s; GPU zone plus shading reduced the whole film from 183.6 s to 36.6 s (5.0x). These are different stages and a specific workload, not factors to combine with the CPU results. Optional `--jet-aa on` can reduce costly ss=2 subsample work without a zone; it disables keyframes and leaves zone frames exact. Its historical four-core 960×540 tendril test measured 719.7 s versus 390.5 s (1.84x render time). [Shading benchmark](https://github.com/junovhs/fractodactyl/blob/main/docs/spec/METHOD.md#benc-10-whole-film-race-gpu-against-all-core-cpu-rx-5700-xt-2026-10-16), [jet-AA evidence](https://github.com/junovhs/fractodactyl/blob/main/docs/research/10-11-26/q6-supersampling-from-jets.md#10-film-16-jet-aa-built-into-fd-film-2026-10-15).

**The viewer uses two paths.** Full-quality studio/look renders use the film kernel and can inherit eligible CPU improvements. Pan/zoom tiles currently use the plain preview route, which bypasses the series initializer. We have not measured a GUI latency or smoothness improvement from this work.

**Check the version you install.** The series CPU improvements described here landed after the published v0.9.0 tag. Building current source includes them; an older downloaded binary does not. Check [release notes](https://fractodactyl.dev/changelog/) and [downloads](https://fractodactyl.dev/download/) for your version. Remaining Tim-plan performance work is still open; no completed-plan or overall-film speedup is claimed.

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
`--zone` covers it (it runs on the wgpu zone kernel, binary64 or f32-only,
on a software adapter if there is no GPU), guarded CPU series/BLA/plain rendering elsewhere in current source. It shades on the
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
