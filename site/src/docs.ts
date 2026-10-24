// The Fractodactyl documentation, as dopedocs data: one source for every page under
// /docs/ and for the machine-readable set (JSON-LD, sitemap, llms.txt, questions.json).
// Numbers come from the results log in docs/spec/METHOD.md; the facts registry holds
// each one once, with the date it was last checked.
import { defineDocs, defineFacts } from "dopedocs";
import { readFileSync } from "node:fs";
import { researchPages } from "./research";

// SITE-12: the manual states the version the workspace builds (Cargo.toml), not a copy.
const cargo = readFileSync(new URL("../../Cargo.toml", import.meta.url), "utf8");
const VERSION = /\[workspace\.package\][^[]*?\nversion = "([^"]+)"/.exec(cargo)?.[1];
if (!VERSION) throw new Error("no [workspace.package] version in Cargo.toml");

const REPO = "https://github.com/junovhs/fractodactyl";

const facts = defineFacts({
    deepSpeedup: { value: "8-25x", reviewed: "2026-10-17", note: "PROB-08, GitHub Actions EPYC, 480x270" },
    deepPixels: { value: "0 wrong out of 777,600", reviewed: "2026-10-17" },
    hdSpeedup: { value: "20-54x", reviewed: "2026-10-17", note: "PROB-09, 1080p" },
    hdPixels: { value: "0 wrong out of 12.4 million", reviewed: "2026-10-17" },
    deep1000Frame: { value: "0.023 s", reviewed: "2026-10-17", note: "PROB-19, k=7676 rung" },
    deep1000Rival: { value: "23-42 s", reviewed: "2026-10-17" },
    filmGpu: { value: "36.6 s", reviewed: "2026-10-17", note: "BENC-10, RX 5700 XT" },
    filmCpu: { value: "183.6 s", reviewed: "2026-10-17" },
    filmSpeedup: { value: "5x", reviewed: "2026-10-17" },
    license: { value: "MIT or Apache-2.0", reviewed: "2026-10-17" },
});

const img = (name: string, alt: string, caption: string) => ({
    kind: "image" as const,
    src: `/img/${name}-1600.webp`,
    srcset: `/img/${name}-800.webp 800w, /img/${name}-1600.webp 1600w`,
    sizes: "(max-width: 760px) 100vw, 700px",
    width: 1600,
    height: 900,
    alt,
    caption,
});

export const docs = defineDocs({
    entity: {
        name: "Fractodactyl",
        url: "https://fractodactyl.dev",
        tagline: "Fractodactyl aims to be the fastest deep-zoom generator that shows the real Mandelbrot set. Built in public.",
        notToBeConfusedWith: ["Pterodactyl (the game server panel)", "Kalles Fraktaler", "Fractal Zoomer"],
        sameAs: [REPO],
        logo: "https://fractodactyl.dev/favicon.png",
    },
    identity: {
        name: "Fractodactyl",
        version: VERSION,
        channel: "Work in progress",
        maker: { name: "junovhs", href: REPO },
        mark: "/favicon.png",
    },
    title: "Fractodactyl user manual",
    lead: "Explore locations and design trips and styles in the live viewer. Render high-resolution videos on your desktop. Study the mathematics, numerical methods and experiments behind the renderer. Choose the part of the manual that matches your work; current capabilities and planned features are identified throughout.",
    facts,
    sections: [
        {
            id: "viewer",
            title: "Live viewer: explore and design",
            children: [
                {
                    id: "what-it-is",
                    title: "What Fractodactyl is",
                    question: "What is Fractodactyl?",
                    answer: "Fractodactyl is an open-source Mandelbrot project: a live viewer for exploring locations and designing trips and styles, a desktop pipeline for high-resolution video, and a research project in complex dynamics and numerical computation. It is built in public and still in development.",
                    keywords: ["Mandelbrot", "deep zoom", "fractal renderer", "Rust", "wgpu"],
                    blocks: [
                        { kind: "p", text: "A deep zoom dives into the Mandelbrot set until the screen is a tiny fraction of an atom wide, and keeps going. The set never runs out of detail, but the arithmetic gets harder the deeper you go, which is why long, deep zoom films are rare and slow to make." },
                        { kind: "p", text: "Start with the **live viewer** to find places, compose a journey and try colours. Use the **desktop video pipeline** to turn a chosen destination and look into a finished film. Read **Research, mathematics and science** for the algorithms, numerical contracts and experimental evidence. Video is the main product; numerical stills remain available for research." },
                        img("seahorse", "Seahorse Valley rendered in the master-ice look", "Seahorse Valley, width 0.0065. Every image here was rendered and coloured by `fd` itself."),
                        { kind: "facts", title: "At a glance", href: REPO, mark: "/favicon.png", rows: [
                            ["Language", "Rust; GPU code in WGSL on wgpu"],
                            ["Targets", "Windows and Linux; any modern GPU, or a software adapter"],
                            ["Tested on", "AMD RX 5700 XT, Mesa lavapipe, DX12 WARP"],
                            ["Status", "Work in progress, built in public"],
                            ["Main tool", "`fd`: render, film, check, explore, build zones"],
                            ["Licence", "{fact:license}"],
                        ] },
                    ],
                },
                {
                    id: "explorer",
                    title: "Explore and design in the live viewer",
                    question: "How do I explore the Mandelbrot set with Fractodactyl?",
                    answer: "Use `fd-viewer` to explore the Mandelbrot set, find locations for a trip, and design its visual style. Drag to pan, scroll to zoom toward the pointer, choose landmarks with Places, save destinations with Saved, and export a PNG with Snapshot.",
                    blocks: [
                        { kind: "p", text: "**Find a destination:** open Places for a known landmark, or Explore for a random one. Follow a shape with the wheel and drag to frame it. Use the overview thumbnail to return to the whole set. Stop on details you would like a film to reveal." },
                        { kind: "p", text: "**Plan a trip:** save promising stops with Saved → Save this spot. Each becomes `NAME.place` in your personal data folder: `%APPDATA%/fractodactyl/places` on Windows, or `$XDG_DATA_HOME/fractodactyl/places` on Linux (default `~/.local/share/fractodactyl/places`). The folder is created on the first save. Set `FD_PLACES` to choose another save folder. Reopen a saved place to compare destinations; `fd film --place NAME` uses the same folder. Curated places beside the executable or in a source checkout are also readable; a personal save with the same name takes precedence. These saved places are the building blocks for a journey. Automatic trips through several stops are planned; the current film workflow zooms toward a chosen destination." },
                        { kind: "p", text: "**Keep your location:** use Edit to copy exact coordinates. Keep those decimal digits intact when moving a destination into a render command; a rounded coordinate can land somewhere else at high zoom." },
                        { kind: "keys", rows: [
                            ["Drag", "Pan (glides on release)"],
                            ["Wheel", "Zoom toward the cursor"],
                            ["+ / -", "Zoom in or out"],
                            ["Arrow keys", "Pan"],
                            ["1-9, 0", "Jump to a landmark"],
                            ["L", "Open the studio look editor"],
                        ] },
                        { kind: "p", text: "Launch `target/release/fd-viewer` from the repository root after building (on Windows, `target\\release\\fd-viewer.exe`). The current Dioxus desktop window hosts the existing explorer; a native drawing surface on the shared film pipeline is planned." },
                        { kind: "gallery", columns: 2, label: "fd-viewer at four places in four looks", images: [
                            { src: "/img/viewer-elephant-1280.webp", alt: "fd-viewer showing Elephant Valley in the master-ice look", caption: "Elephant Valley, master-ice.", width: 1280, height: 1082 },
                            { src: "/img/viewer-antenna-1280.webp", alt: "fd-viewer showing the antenna minibrot in the ice look", caption: "The antenna minibrot, ice.", width: 1280, height: 1082 },
                            { src: "/img/viewer-deepvalley-1280.webp", alt: "fd-viewer showing a deep valley at zoom 1.3e12 in the smoke look", caption: "A deep valley at zoom 1.3e12, smoke.", width: 1280, height: 1082 },
                            { src: "/img/viewer-coral-1280.webp", alt: "fd-viewer showing Seahorse Valley in the coral look", caption: "Seahorse Valley, coral.", width: 1280, height: 1082 },
                        ] },
                    ],
                },
                {
                    id: "looks",
                    title: "Design a style",
                    question: "How do I change the colours of a Fractodactyl render?",
                    answer: "In the viewer, choose a look in the Looks sidebar and press L to open the Studio Look editor. Try a palette, adjust the bands and lighting, and compare the result at several saved locations. Changing the style keeps your camera in place and reuses the computed detail.",
                    blocks: [
                        { kind: "p", text: "**Shape the appearance:** start with ice, coral, steel, zebra or smoke. Band density controls how tightly colours repeat; terraces change their stepped appearance; slope and light change the relief; lines add contours. Use Snapshot to keep a visual reference while you compare styles." },
                        { kind: "p", text: "**Carry the look into a film:** `--preset NAME` reads `looks/NAME.look` or a built-in preset. Film flags such as `--density`, `--terrace`, `--slope`, `--light` and `--lines` adjust that look. Keep the preset file with your saved destination so the design is reproducible." },
                        { kind: "p", text: "**Find named presets:** `fd shade` and `fd film` choose the looks directory in this order: `FD_LOOKS` if set; a source checkout found from the executable, then the current working directory; finally `looks/` beside the executable in an unpacked release. Git worktrees use the main checkout's `looks/`. This selects one directory, rather than searching every directory for each name. You can also pass a `.look` file path directly; built-in presets remain available when no matching file is found." },
                        { kind: "p", text: "**Use an unpacked release from another folder:** keep its `looks/` beside `fd` (`fd.exe` on Windows). From your project folder containing `tiny.fds`, run `/path/to/fractodactyl/fd shade tiny.fds --preset master-ice -o shaded` (PowerShell: `& 'C:\\path\\to\\fractodactyl\\fd.exe' shade tiny.fds --preset master-ice -o shaded`). Replace the executable path with your unpacked location. With `FD_LOOKS` unset and no source checkout found, this reads the bundled preset and writes the image into your project's `shaded/` folder. For your own saved presets, set `FD_LOOKS` to a personal writable folder." },
                        { kind: "p", text: "The deep maths produces palette-free samples (smooth escape value, distance estimate, normal). A look is applied afterwards, on the GPU, so `fd shade X.fds --preset ice -o DIR` re-colours stored samples without iterating again." },
                        { kind: "p", text: "`--flow`, `--breathe` and `--drift` animate the bands in films. A `.look` file is a few `key value` lines, such as `stops #rrggbb ...` and `density 0.05`." },
                        { kind: "p", text: "**Sample-aware style zooms:** use `fd style sequence GRAPH INPUT.fds -o DIR --frames 600 --fps 60 --zoom 1 --zoom-end 1.3` for a reconstructed push-in. `--zoom 1` also enables the antialias reconstruction without camera motion. This path evaluates the full material at stored sample locations, combines their colors with a continuous tent filter at each output pixel’s footprint in linear HDR, and only then runs image-space mist and output encoding. It preserves supersample detail that an image `zoom` node would enlarge after reduction. Use input supersampling sufficient for the largest zoom: a 4×4 source grid provides about 3 samples across each final pixel at 1.3×; dense highlights may need more. Smooth palette transitions alone cannot antialias rapidly changing surface highlights. This remains reconstruction over stored data, limited to 1–2×, rather than newly iterated camera frames; omitted flags preserve the existing box resolve. Avoid applying both this camera zoom and graph image zoom to the same push-in." },
                        { kind: "p", text: "**Graph push-ins:** `zoom IMAGE PIVOT_IMAGE MAGNIFICATION_IMAGE` resamples a scalar, vector or color image in linear HDR before output encoding. The vector pivot uses normalized image coordinates (0.5, 0.5 is the centre); magnification is bounded to 1–2×. Time-driven graph signals can animate both magnification and band phase in `fd style sequence`. Zoom the surface, emission, direction and mask images together, then apply output-space mist/glow. This is an image-space push-in over stored data, not a new fractal camera render. Its support is reported as full-frame; neighborhood support remains full-frame through subsequent operations." },
                        { kind: "p", text: "**Styling workspace (source builds):** choose Options → Styling workspace in the desktop viewer, launch `fd-viewer --style --samples INPUT.fds` (add `--style-look LOOK.style` to reopen a saved look at launch), or use `fd-viewer --style` and Open samples. The native viewer offers six starting points: Luminous tide, Etched bronze, Violet vapor, Rose glass, Polar ice and Obsidian. Shape controls fold frequency/depth and creases; Light & material controls illumination and sheen; Color & emission separates cool backlight from warm edges; Mist spreads the cool emission along curved tangents. Raise Cool backlight before increasing Mist brightness. Pin reference and hold Compare to judge changes, Reset this preset to recover its starting point, Save PNG to keep an image, and Save look/Open look to preserve and reopen workspace settings. Saved `.style` graphs also work with `fd style render`. The workspace uses shaded-sample tent reconstruction; CLI exports match with `--zoom 1`. Slider changes reuse sample uploads and GPU pipelines. This workspace styles stored samples rather than the explorer's live camera; unresolved samples are displayed black. It is an experimental 2D appearance model, not volumetric fog or physical depth." },
                        { kind: "p", text: "**Experimental style graphs (source builds):** `fd style inspect GRAPH` validates a version-1 `.style` file and reports reachable nodes, shared operations, uniforms and pass requirements. `-o BODY.wgsl` saves the generated pointwise body. Render with `fd style render GRAPH INPUT.fds -o OUTPUT.png`. Use `--set strength=4,width=1.5` for named graph uniforms, `--time T` for animation, and `--runs N` to inspect warm resource reuse and frame costs. Add `--bloom 1 --bloom-radius 12` for HDR glow or `--grain 0.025 --grain-size 1 --grain-seed 1` for optional photographic texture. `--exposure N` makes grain reproducible per exposure; `--grain-color` mixes color variation, and `--grain-shadow`, `--grain-mid`, `--grain-highlight` shape its tonal response. `--dither on` separately reduces quantization bands. Bloom reports its bounded pixel support; tiled styling is not yet supported. `fd style sequence GRAPH INPUT.fds -o DIR --frames 48 --fps 24` reuses the same samples and GPU resources across animated PNG frames. Times and exposure indices are explicit for replay. The included `celestial.style` and `etched-copper.style` graphs show different compositions; celestial includes an experimental bounded `flow FIELD VECTOR DISTANCE` operator (five taps, at most eight output pixels, one flow node per graph). Connected image programs use `output IMAGE` and `image FIELD`, with typed scalar/vector/color ports for `warp`, `blur`, `relight`, `mix`, addition and multiplication. Signals can control warp distance, blur width and blend masks. Compare `looks/styles/modular-light.style` and `modular-light-swapped.style`: their wiring changes operation order. `inspect` reports fused passes and reusable buffer slots. Connected graphs wire glow explicitly; `--bloom` belongs to the surface/emission compatibility route. Grain is optional and off by default; further grain development is parked. Fine detail stays smooth: where a `sin`, `cos` or `fract` argument, or a lit normal, changes faster than one sample (dense filaments near the set), each sample is shaded four times with that value spread over its cycle and averaged, so the region shows its average colour instead of speckle. Graphs without such values are shaded once. `source stripe` reads the stripe average, a combed fibre texture from the angle of the last iterations before escape (0 to 1): render its samples with `fd render ... --columns +stripe` (add `--stripe-window K` for the iterations averaged, default 16 up to 64, and `--stripe-freq k` for the comb density, default 5). It comes from the CPU perturbation kernel, so `--zone`, `--refine` and `--gpu-perturb` refuse it. See `tools/research/looks/probes/metallic.style` for an example. This opt-in path does not replace `.look` presets, the live explorer camera or production films. See `looks/styles/scaffold.style` in the repository for an example." },
                    ],
                },
            ],
        },
        {
            id: "video",
            title: "Desktop video: from design to final render",
            children: [
                {
                    id: "build",
                    title: "Build it",
                    question: "How do I build Fractodactyl?",
                    answer: "Fractodactyl builds with stable Rust: `cargo build --release -p fd-cli` produces the `fd` tool, and adding `-p fd-viewer` builds the desktop explorer. FFmpeg on PATH is needed for MP4 output, and the shaders need no SDK because wgpu compiles them at run time.",
                    blocks: [
                        { kind: "p", text: "**Ready-built packages:** [Download](https://fractodactyl.dev/download/) has the Windows ZIP and Linux tar.gz, runtime requirements and checksums. Extract the package and keep fd, fd-viewer, looks/ and places/ together. See [Changelog](https://fractodactyl.dev/changelog/) for release history." },
                        { kind: "steps", items: [
                            { title: "Get the source", text: "Clone `github.com/junovhs/fractodactyl`." },
                            { title: "Build", text: "`cargo build --release -p fd-cli -p fd-viewer` (on Linux the viewer needs WebKitGTK 4.1)." },
                            { title: "Install FFmpeg", text: "Put `ffmpeg` on PATH so `fd film --mp4` can encode." },
                            { title: "Choose the GPU", text: "Run `fd adapters` to list index, backend, vendor:device ID, name and native f64 support. GPU commands (`film`, `render`, `shade`, `style`, `play`, `control`, `bench` and `explore`) accept `--adapter N`; for example `fd shade INPUT.fds --preset ice --adapter 0 -o OUT`. The flag overrides `FD_ADAPTER`. Set `FD_ADAPTER=0` before launching `fd-viewer` to use the same index in its explorer and styling workspace (PowerShell: `$env:FD_ADAPTER='0'`). Indices are stable for the current adapter inventory; list again after changing drivers or hardware. Without a selection, fd keeps its hardware-first ranking. Selecting an adapter never substitutes a different GPU; a zone may use the f32-only kernel on that same adapter when native f64 is unavailable. wgpu needs a Vulkan driver, Mesa's lavapipe on Linux, or DX12 (WARP without a GPU) on Windows. So far it has been run on an AMD RX 5700 XT, lavapipe and WARP." },
                        ] },
                    ],
                },
                {
                    id: "first-film",
                    title: "Render a high-resolution video",
                    question: "How do I make a zoom film with Fractodactyl?",
                    answer: "Fractodactyl makes films with `fd film RE IM --to WIDTH --mp4 FILE`. It defaults to `--keyframes 2`: oversized keyframes per 2x zoom supply the intervening frames. Use `--keyframes off` to render every frame directly. Adding `--zone FILE` uses a zone's fast path where it covers the frame.",
                    blocks: [
                        { kind: "p", text: "This is an offline desktop workflow: the renderer computes the final frames and FFmpeg encodes an MP4. The target is high-resolution video that is visually indistinguishable from an exact render. Live navigation previews and final video exports have different quality requirements." },
                        { kind: "p", text: "**Control the camera:** `--from` and `--to` are frame widths in the complex plane; smaller means deeper. Use `--seconds` for a fixed duration or `--rate` for decades of zoom per second. `--ease on` softens the start and stop, `--twist` adds rotation, and `--flow`, `--breathe` and `--drift` animate the look." },
                        { kind: "p", text: "**Depth range:** film camera widths stay as decimal strings through parsing, easing, saved places and keyframes, including widths such as `1e-1000`; exact destination centers are preserved. `--depth-cap D` limits the requested depth to D decades (width at least `1e-D`). It defaults to 1000 and accepts a smaller positive cap up to 1000. Deeper requests are refused before rendering; extreme depths remain still-only. This fixes the former floating-point width ceiling, not the cost of deep rendering. Start with a small selected-frame preview at your own location." },
                        { kind: "p", text: "**Optional acceleration:** for a supported minibrot near a detected M(q,r) spiral centre with enough laps, build `fd zone <re> <im> -o data/zones/v0.zone` once, then add `--zone data/zones/v0.zone` to the film command. A zone is specific to its location and validity range. You can render without one; see Zones for current coverage." },
                        { kind: "steps", items: [
                            { title: "Choose the destination and style", text: "Explore in the viewer, save a spot and copy its exact real and imaginary coordinates. Pick a look from `looks/`. In the commands below, replace `<re>` and `<im>` with those coordinates; do not type the angle brackets." },
                            { title: "Make a short preview", text: "`fd film <re> <im> --from 0.0065 --to 0.003 --seconds 5 --size 640x360 --fps 30 --preset ice --mp4 preview.mp4` is a starter recipe for a Seahorse Valley destination. Choose widths that suit your own location, then inspect the framing, pace and colours." },
                            { title: "Render the final", text: "Reuse the preview command with `--size 1920x1080 --fps 60 --mp4 final.mp4`. Set `--seconds` to the desired duration. Larger sizes and more samples increase render time; export a short segment before committing to a long film." },
                            { title: "Inspect and validate", text: "Watch the encoded MP4 for framing, banding and flicker. When evaluating an accelerated route against a reference, run `fd filmcheck reference.mp4 candidate.mp4` with matching cameras, looks and encoding settings; it compares every frame. A visual review alone does not establish numerical accuracy." },
                        ] },
                        { kind: "p", text: "**Preview selected frames:** `--frames N` selects one frame, `--frames A..B` selects A through B−1, and `--frames A..=B` includes both endpoints. Indices start at zero in the full film: for a film of C frames, N and inclusive endpoints must be below C; the exclusive endpoint B may equal C. For example, `--frames 0..2` and `--frames 0..=1` both select the first two frames. `--frames 0..0` is empty and fails with guidance to use `--frames 0` for a single frame." },
                        { kind: "p", text: "Try a tiny single-frame preview: `fd film -0.75 0.1 --from 4 --to 2 --seconds 1 --fps 2 --size 16x16 --iter 32 --threads 1 --x264 ultrafast --look umber --ss 1 --keyframes off --frames 0 --mp4 frame-0.mp4`. This selects the first frame of a two-frame camera path. Replace `--frames 0` with `--frames 0..=1` to encode both frames; omitting `--frames` renders the whole film. FFmpeg must be installed." },
                        { kind: "table", head: ["Default", "Value"], rows: [
                            ["Look", "studio, ice preset"],
                            ["Speed", "0.15 decades per second"],
                            ["Frames", "60 fps, 1920x1080"],
                            ["Sampling", "`--keyframes 2`, `--aa on`, unresolved drawn as interior"],
                            ["BLA tolerance", "`--bla-eps 1.4901161193847656e-8` (2^-26, film); `8.881784197001252e-16` is the exact tolerance"],
                        ] },
                        { kind: "p", text: "Explicit `--ss`, `--jet-aa on`, or widths outside the normal f64 range switch to direct per-frame rendering with a one-line stderr note. Direct rendering defaults to `--ss 2`. The `fd-film/1` report records `keyframes: 2` (or your chosen multiplier), or `keyframes: null` for direct rendering." },
                        { kind: "p", text: "**CPU timing (source builds):** Each actual kernel frame prints a `kernel process CPU` line. `fd film --json` also records these measurements in `kernel_frame_process_cpu`, with the width, route and requested thread count. `process_cpu_seconds` counts all CPU time used by the fd process during that kernel call; `average_process_busy_threads` is that time divided by the record's `wall_seconds`. The interval includes reference/BLA/series setup, chooser probes and retries, and ends before subsequent shading and encoding. It includes any overlapping work by other fd threads, excludes the encoder child process, and is not a measurement of individual render workers or GPU occupancy. Interpolated output frames add no kernel record. Unix process clocks supply CPU time; unsupported or failed clocks report `null`. Whole-film wall time includes work outside these intervals, so a kernel's busy-thread average alone does not establish film speed." },
                        { kind: "p", text: "`fd film` with no arguments lists every flag. Keep the exact coordinates, widths, command and `.look` file alongside the MP4 so you can reproduce the film." },
                        { kind: "p", text: "**Style-graph films:** `fd film ... --style looks/styles/celestial.style --mp4 F` shades the film with a style graph (the same engine as `fd style render` and the styling workspace) instead of a `--look`. `--set name=value,...` sets graph parameters and `--bloom A --bloom-radius R` adds glow. Graph distances (edge widths, flow) stay in output pixels on keyframe films. A graph that does not read `time` is shaded once per keyframe and reused; one that does is shaded every frame. A graph reading `source stripe` gets the stripe samples automatically (`--stripe-window`, `--stripe-freq`). Grain is not available in films yet." },
                    ],
                },
                {
                    id: "where-speed-applies",
                    title: "Where the speed applies",
                    question: "Which speedups help at my chosen location?",
                    answer: "General CPU shortcuts, keyframes and GPU shading can help ordinary locations. Zones require a matching location and validity range. Measured gains are specific to their workloads; source additions after v0.9.0 need an updated build.",
                    blocks: [
                        { kind: "p", text: "**Start with your own location.** General CPU rendering, keyframes and GPU shading do not require a special destination. A saved spot does not guarantee zone coverage, but lack of a zone does not rule out these general speedups. Zones are a separate, location-specific shortcut." },
                        { kind: "p", text: "**CPU rendering without a zone.** BLA skips valid blocks of iteration; a guarded series initializer can skip a useful starting stretch. They can help arbitrary locations, but not every frame. PERF-05 reduced the total cold time for 24 frozen 480×270 views from 23.468 s to 16.582 s: 6.886 s saved, about 29% less time (1.415x). This used eight workers, counted setup and fallback, and selected the initializer on 17 views while leaving seven on their prior route. It is not a whole-film or universal multiplier. PERF-06 separately cut series construction from 0.268 s to 0.040 s in a 24-view check; its roughly 6.7x applies only to that setup stage. Do not multiply these ratios together. [Current-route census and its limits](https://github.com/junovhs/fractodactyl/blob/main/docs/research/2026-10-22-cpu-occupancy/README.md)." },
                        { kind: "p", text: "**Zones need the right structure.** They pay when a minibrot's orbit circles a nearby repelling cycle many times. The historical FIX-22 probe found 341 laps at v0, versus at most 4.7 at 21 other sampled minibrots. An orbit analysis of Eye of the Universe estimated at most about 1.3x extra from the spiral jump; that was not a timed render. The historical roughly 20x result applies to a covered v0 frame against per-frame BLA, not a whole film or every minibrot. [FIX-22 and orbit evidence](https://github.com/junovhs/fractodactyl/blob/main/docs/spec/METHOD.md#results-log)." },
                        { kind: "p", text: "**Zone support is broader than M(24,2).** PROB-36 lets `fd zone RE IM -o FILE` accept detected M(q,r) spiral-centre relations with at least 100 laps. Verification includes one covered exterior M(2,1) frame; it does not establish every relation or depth. Each zone still has a location and validity range. PROB-27's `fd zone --suggest RE IM --count N` supplies validated ladder targets. Automatic cost-aware destination selection (PROB-29) remains planned. [PROB-36 / PROB-27 tests](https://github.com/junovhs/fractodactyl/blob/main/crates/fd-cli/tests/zone_build.rs)." },
                        { kind: "p", text: "**Keyframes and shading work more broadly.** Default `--keyframes 2` reuses samples between zoom frames without needing a zone. Explicit `--ss`, jet-AA or widths outside normal f64 range select direct frames. GPU studio shading works on samples from any location. In the historical v0 1080p60 test on a Ryzen 9 3900X / RX 5700 XT, shading fell from 131 s to 9.8 s; GPU zone plus shading reduced the whole film from 183.6 s to 36.6 s (5.0x). These are different stages and a specific workload, not factors to combine with the CPU results. Optional `--jet-aa on` can reduce costly ss=2 subsample work without a zone; it disables keyframes and leaves zone frames exact. Its historical four-core 960×540 tendril test measured 719.7 s versus 390.5 s (1.84x render time). [Shading benchmark](https://github.com/junovhs/fractodactyl/blob/main/docs/spec/METHOD.md#benc-10-whole-film-race-gpu-against-all-core-cpu-rx-5700-xt-2026-10-16), [jet-AA evidence](https://github.com/junovhs/fractodactyl/blob/main/docs/research/10-11-26/q6-supersampling-from-jets.md#10-film-16-jet-aa-built-into-fd-film-2026-10-15)." },
                        { kind: "p", text: "**Experimental generic GPU samples.** Current source accepts `--gpu-perturb on|bla` on `fd render` and `fd control` (default `off`). It uses shared hi/lo binary32 mantissas with carried exponents and an empirical, conditional round-to-nearest error model; admitted output is Heuristic, never certified. Runtime float-float self-tests and all requested output guards must pass. Bound-evidence requests decline to the CPU. Rejected/interior/unfinished samples restart from the exact camera using the original CPU reference. Incomplete dispatches discard the entire attempt with a visible reason. Long dispatches use at most 16,384 samples per completed submission (5-second submission and 120-second attempt limits). Counts and setup/GPU/fallback/total timings are printed. This route is currently slower than landed CPU BLA. Explicit zone/BLA/refinement routes take precedence; unsupported ranges stay on the CPU. `bla` adds shared CPU-built affine blocks with distinct derivative guards and block/skipped-step counters. It is also currently slower than CPU BLA; fallback remains original plain CPU. This is experimental groundwork, with no film integration or automatic selection." },
                        { kind: "p", text: "**The viewer uses two paths.** Full-quality studio/look renders use the film kernel and can inherit eligible CPU improvements. Pan/zoom tiles currently use the plain preview route, which bypasses the series initializer. We have not measured a GUI latency or smoothness improvement from this work." },
                        { kind: "p", text: "**Check the version you install.** The series CPU improvements described here landed after the published v0.9.0 tag. Building current source includes them; an older downloaded binary does not. Check [release notes](https://fractodactyl.dev/changelog/) and [downloads](https://fractodactyl.dev/download/) for your version. Remaining Tim-plan performance work is still open; no completed-plan or overall-film speedup is claimed." },
                    ],
                },
                {
                    id: "zones",
                    title: "Zones",
                    question: "What is a zone in Fractodactyl?",
                    answer: "A Fractodactyl zone is a file of fast-path constants for one minibrot: its return map, Koenigs chart and tail patches, built once by `fd zone` and passed to `fd film`, `fd render` or `fd explore` with `--zone`. Frames the zone covers run on the GPU zone kernel.",
                    blocks: [
                        { kind: "p", text: "`fd zone RE IM -o FILE` finds the minibrot's structure and writes a complete deep-band zone in Rust: 7.8 s for the v0 minibrot (period 764), 66 s for a rung near 1e-1000." },
                        { kind: "p", text: "If `fd zone -o` refuses a detected relation, its error gives the minibrot period, relation M(q,r), and the measured laps around that cycle when available. If detection stops before laps are measured, payoff has not been assessed. Writing a zone requires at least 100 laps: fewer laps are unlikely to repay the setup cost. This is a conservative payoff estimate, not a measured speedup for your film. Render without a zone when it declines." },
                        { kind: "callout", tone: "warn", text: "Today `fd zone` writes the deep-zone constants and, on the v0 ladder, the mid-depth constants and reference orbit too. The mid-depth operator keeps its existing validity checks; other ladders do not gain mid-depth coverage. Making zones for any film is the next big step." },
                    ],
                },
            ],
        },
        {
            id: "research",
            title: "Research, mathematics and science",
            children: researchPages,
        },
    ],
});
