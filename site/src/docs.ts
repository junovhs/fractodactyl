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
                        { kind: "p", text: "**Plan a trip:** save promising stops with Saved → Save this spot. Each becomes `places/NAME.place`; reopen it to compare destinations. These saved places are the building blocks for a journey. Automatic trips through several stops are planned; the current film workflow zooms toward a chosen destination." },
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
                        { kind: "p", text: "The deep maths produces palette-free samples (smooth escape value, distance estimate, normal). A look is applied afterwards, on the GPU, so `fd shade X.fds --preset ice -o DIR` re-colours stored samples without iterating again." },
                        { kind: "p", text: "`--flow`, `--breathe` and `--drift` animate the bands in films. A `.look` file is a few `key value` lines, such as `stops #rrggbb ...` and `density 0.05`." },
                        { kind: "p", text: "**Sample-aware style zooms:** use `fd style sequence GRAPH INPUT.fds -o DIR --frames 600 --fps 60 --zoom 1 --zoom-end 1.3` for a reconstructed push-in. `--zoom 1` also enables the antialias reconstruction without camera motion. This path evaluates the full material at stored sample locations, combines their colors with a continuous tent filter at each output pixel’s footprint in linear HDR, and only then runs image-space mist and output encoding. It preserves supersample detail that an image `zoom` node would enlarge after reduction. Use input supersampling sufficient for the largest zoom: a 4×4 source grid provides about 3 samples across each final pixel at 1.3×; dense highlights may need more. Smooth palette transitions alone cannot antialias rapidly changing surface highlights. This remains reconstruction over stored data, limited to 1–2×, rather than newly iterated camera frames; omitted flags preserve the existing box resolve. Avoid applying both this camera zoom and graph image zoom to the same push-in." },
                        { kind: "p", text: "**Graph push-ins:** `zoom IMAGE PIVOT_IMAGE MAGNIFICATION_IMAGE` resamples a scalar, vector or color image in linear HDR before output encoding. The vector pivot uses normalized image coordinates (0.5, 0.5 is the centre); magnification is bounded to 1–2×. Time-driven graph signals can animate both magnification and band phase in `fd style sequence`. Zoom the surface, emission, direction and mask images together, then apply output-space mist/glow. This is an image-space push-in over stored data, not a new fractal camera render. Its support is reported as full-frame; neighborhood support remains full-frame through subsequent operations." },
                        { kind: "p", text: "**Styling workspace (source builds):** choose Options → Styling workspace in the desktop viewer, launch `fd-viewer --style --samples INPUT.fds` (add `--style-look LOOK.style` to reopen a saved look at launch), or use `fd-viewer --style` and Open samples. The native viewer offers six starting points: Luminous tide, Etched bronze, Violet vapor, Rose glass, Polar ice and Obsidian. Shape controls fold frequency/depth and creases; Light & material controls illumination and sheen; Color & emission separates cool backlight from warm edges; Mist spreads the cool emission along curved tangents. Raise Cool backlight before increasing Mist brightness. Pin reference and hold Compare to judge changes, Reset this preset to recover its starting point, Save PNG to keep an image, and Save look/Open look to preserve and reopen workspace settings. Saved `.style` graphs also work with `fd style render`. The workspace uses shaded-sample tent reconstruction; CLI exports match with `--zoom 1`. Slider changes reuse sample uploads and GPU pipelines. This workspace styles stored samples rather than the explorer's live camera; unresolved samples are displayed black. It is an experimental 2D appearance model, not volumetric fog or physical depth." },
                        { kind: "p", text: "**Experimental style graphs (source builds):** `fd style inspect GRAPH` validates a version-1 `.style` file and reports reachable nodes, shared operations, uniforms and pass requirements. `-o BODY.wgsl` saves the generated pointwise body. Render with `fd style render GRAPH INPUT.fds -o OUTPUT.png`. Use `--set strength=4,width=1.5` for named graph uniforms, `--time T` for animation, and `--runs N` to inspect warm resource reuse and frame costs. Add `--bloom 1 --bloom-radius 12` for HDR glow or `--grain 0.025 --grain-size 1 --grain-seed 1` for optional photographic texture. `--exposure N` makes grain reproducible per exposure; `--grain-color` mixes color variation, and `--grain-shadow`, `--grain-mid`, `--grain-highlight` shape its tonal response. `--dither on` separately reduces quantization bands. Bloom reports its bounded pixel support; tiled styling is not yet supported. `fd style sequence GRAPH INPUT.fds -o DIR --frames 48 --fps 24` reuses the same samples and GPU resources across animated PNG frames. Times and exposure indices are explicit for replay. The included `celestial.style` and `etched-copper.style` graphs show different compositions; celestial includes an experimental bounded `flow FIELD VECTOR DISTANCE` operator (five taps, at most eight output pixels, one flow node per graph). Connected image programs use `output IMAGE` and `image FIELD`, with typed scalar/vector/color ports for `warp`, `blur`, `relight`, `mix`, addition and multiplication. Signals can control warp distance, blur width and blend masks. Compare `looks/styles/modular-light.style` and `modular-light-swapped.style`: their wiring changes operation order. `inspect` reports fused passes and reusable buffer slots. Connected graphs wire glow explicitly; `--bloom` belongs to the surface/emission compatibility route. Grain is optional and off by default; further grain development is parked. This opt-in path does not replace `.look` presets, the live explorer camera or production films. See `looks/styles/scaffold.style` in the repository for an example." },
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
                            { title: "Check the GPU", text: "wgpu needs a Vulkan driver, Mesa's lavapipe on Linux, or DX12 (WARP without a GPU) on Windows. So far it has been run on an AMD RX 5700 XT, lavapipe and WARP." },
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
                        { kind: "p", text: "**Optional acceleration:** for a supported minibrot near M(24,2), build `fd zone <re> <im> -o data/zones/v0.zone` once, then add `--zone data/zones/v0.zone` to the film command. A zone is specific to its location and validity range. You can render without one; see Zones for current coverage." },
                        { kind: "steps", items: [
                            { title: "Choose the destination and style", text: "Explore in the viewer, save a spot and copy its exact real and imaginary coordinates. Pick a look from `looks/`. In the commands below, replace `<re>` and `<im>` with those coordinates; do not type the angle brackets." },
                            { title: "Make a short preview", text: "`fd film <re> <im> --from 0.0065 --to 0.003 --seconds 5 --size 640x360 --fps 30 --preset ice --mp4 preview.mp4` is a starter recipe for a Seahorse Valley destination. Choose widths that suit your own location, then inspect the framing, pace and colours." },
                            { title: "Render the final", text: "Reuse the preview command with `--size 1920x1080 --fps 60 --mp4 final.mp4`. Set `--seconds` to the desired duration. Larger sizes and more samples increase render time; export a short segment before committing to a long film." },
                            { title: "Inspect and validate", text: "Watch the encoded MP4 for framing, banding and flicker. When evaluating an accelerated route against a reference, run `fd filmcheck reference.mp4 candidate.mp4` with matching cameras, looks and encoding settings; it compares every frame. A visual review alone does not establish numerical accuracy." },
                        ] },
                        { kind: "table", head: ["Default", "Value"], rows: [
                            ["Look", "studio, ice preset"],
                            ["Speed", "0.15 decades per second"],
                            ["Frames", "60 fps, 1920x1080"],
                            ["Sampling", "`--keyframes 2`, `--aa on`, unresolved drawn as interior"],
                        ] },
                        { kind: "p", text: "Explicit `--ss`, `--jet-aa on`, or widths outside the normal f64 range switch to direct per-frame rendering with a one-line stderr note. Direct rendering defaults to `--ss 2`. The `fd-film/1` report records `keyframes: 2` (or your chosen multiplier), or `keyframes: null` for direct rendering." },
                        { kind: "p", text: "`fd film` with no arguments lists every flag. Keep the exact coordinates, widths, command and `.look` file alongside the MP4 so you can reproduce the film." },
                    ],
                },
                {
                    id: "zones",
                    title: "Zones",
                    question: "What is a zone in Fractodactyl?",
                    answer: "A Fractodactyl zone is a file of fast-path constants for one minibrot: its return map, Koenigs chart and tail patches, built once by `fd zone` and passed to `fd film`, `fd render` or `fd explore` with `--zone`. Frames the zone covers run on the GPU zone kernel.",
                    blocks: [
                        { kind: "p", text: "`fd zone RE IM -o FILE` finds the minibrot's structure and writes a complete deep-band zone in Rust: 7.8 s for the v0 minibrot (period 764), 66 s for a rung near 1e-1000." },
                        { kind: "callout", tone: "warn", text: "Today `fd zone` covers minibrots near the Misiurewicz point M(24,2), and the v0 path's mid-depth constants still come from the research script `make_zone.sh`. Making zones for any film is the next big step." },
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
