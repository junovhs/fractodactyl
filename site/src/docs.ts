// The Fractodactyl documentation, as dopedocs data: one source for every page under
// /docs/ and for the machine-readable set (JSON-LD, sitemap, llms.txt, questions.json).
// Numbers come from the results log in docs/spec/METHOD.md; the facts registry holds
// each one once, with the date it was last checked.
import { defineDocs, defineFacts } from "dopedocs";

const REPO = "https://github.com/junovhs/fractodactyl";
const SPEC = `${REPO}/blob/main/docs/spec`;

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
        version: "0.1.0",
        channel: "Work in progress",
        maker: { name: "junovhs", href: REPO },
        mark: "/favicon.png",
    },
    title: "How Fractodactyl works",
    lead: "Fractodactyl is a renderer for deep zooms into the Mandelbrot set, built in public. We aim for films that go much deeper and run much longer, made much faster, with every frame showing the real set. These docs cover what works today, and mark plans as plans.",
    facts,
    sections: [
        {
            id: "start",
            title: "Start here",
            children: [
                {
                    id: "what-it-is",
                    title: "What Fractodactyl is",
                    question: "What is Fractodactyl?",
                    answer: "Fractodactyl is an open-source, work-in-progress renderer for deep Mandelbrot zoom films, written in Rust with GPU code in WGSL and built in public. Its aim is the fastest deep-zoom generator that shows the real set and looks visually perfect.",
                    keywords: ["Mandelbrot", "deep zoom", "fractal renderer", "Rust", "wgpu"],
                    blocks: [
                        { kind: "p", text: "A deep zoom dives into the Mandelbrot set until the screen is a tiny fraction of an atom wide, and keeps going. The set never runs out of detail, but the arithmetic gets harder the deeper you go, which is why long, deep zoom films are rare and slow to make." },
                        { kind: "p", text: "Fractodactyl aims to attack that cost directly, and is built in the open with every result published. **Video is the main product:** precision a viewer cannot see is a cost, not a feature. Exact renders with full numerical accuracy stay available for stills, for maths and science use." },
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
                    id: "the-rules",
                    title: "The three tests",
                    question: "What rules does Fractodactyl follow?",
                    answer: "Every Fractodactyl idea must pass three tests in order: the frame shows the real Mandelbrot set at that place and depth; viewers cannot tell a film frame from an exact render, measured rather than eyeballed; and it makes zooms faster, deeper or longer.",
                    keywords: ["north star", "genuine depth", "no faking"],
                    blocks: [
                        { kind: "steps", items: [
                            { title: "Real", text: "The frame shows the actual Mandelbrot set at that location and depth." },
                            { title: "Looks perfect", text: "Viewers cannot tell a film frame from an exact render of the same frame. `fd filmcheck` measures it on every frame." },
                            { title: "Moves the outcome", text: "It makes zooms faster, deeper or longer, or unblocks something that does. Ideas that stop paying off are dropped." },
                        ] },
                        { kind: "callout", text: "**Depth is never faked.** No swapping in a look-alike location, and no skipping work a viewer would see. An early prototype that swapped deep regions for shallow look-alikes was retired for exactly this reason." },
                    ],
                },
                {
                    id: "how-its-fast",
                    title: "How it gets its speed",
                    question: "How does Fractodactyl render deep zooms so fast?",
                    answer: "Near a minibrot, exact theorems (Tan Lei's and Koenigs's) govern each pixel's orbit, so Fractodactyl replaces most of the step-by-step iteration with a few loops of a small map and one exact jump, in ordinary floating point, and checks the result against full-precision renders.",
                    keywords: ["Koenigs linearization", "return map", "perturbation", "minibrot"],
                    blocks: [
                        { kind: "compare",
                            before: { title: "A usual deep-zoom renderer", text: "Follows each pixel's orbit step by step, often for hundreds of thousands of steps, with tricks (perturbation, BLA) to make each step cheaper." },
                            after: { title: "Fractodactyl's fast path", text: "Near a minibrot, most of those steps become a few loops of a small return map plus one exact Koenigs jump, then a short finish. The pixel is still computed, not guessed." } },
                        { kind: "p", text: "That fast path is packaged as a **zone**: the constants for one stretch of a zoom path, built once by `fd zone`. Before a frame uses a zone, a guard checks the zone is valid there; if not, the frame falls back to perturbation with BLA." },
                        img("spiral", "A spiral near the Misiurewicz point M(24,2) at width 1e-24", "The spiral around M(24,2) at width 1e-24, on fd's perturbation kernel: 4.1 s for 3200x1800 samples."),
                        img("minibrot", "A period-764 minibrot at width 1e-48", "Twenty-four decades deeper: the period-764 minibrot at width 1e-48, on the GPU zone kernel, 0.25 s for the same 3200x1800 samples."),
                    ],
                },
                {
                    id: "results",
                    title: "What it has achieved",
                    question: "How much faster is Fractodactyl?",
                    answer: "Fractodactyl's zone fast path renders deep frames {fact:deepSpeedup} faster than its own BLA renderer with {fact:deepPixels} pixels, about 1000x faster at a depth of 1e-1000, and a whole 1080p60 film on the GPU {fact:filmSpeedup} faster than on all CPU cores.",
                    keywords: ["benchmark", "speedup", "results"],
                    blocks: [
                        { kind: "metrics", items: [
                            { value: "{fact:deepSpeedup}", label: "Deep frames near a minibrot", detail: "Against fd's own BLA renderer, {fact:deepPixels}." },
                            { value: "{fact:hdSpeedup}", label: "At 1080p", detail: "{fact:hdPixels}." },
                            { value: "{fact:deep1000Frame}", label: "Per frame at 1e-1000", detail: "Against {fact:deep1000Rival} for fd's best renderer there, every pixel correct." },
                            { value: "{fact:filmSpeedup}", label: "Whole film on the GPU", detail: "{fact:filmGpu} against {fact:filmCpu} for 10 s of 1080p60, passing the film check on all 600 frames." },
                        ] },
                        { kind: "callout", tone: "note", text: "These are measurements on the zoom paths tested so far, not on every zoom. Every claim is against a fair opponent: the best renderer that could use the same trick. At 1e-1000 that opponent is fd's plain scaled perturbation, because fd has no BLA at that depth yet. The full log, with commands, is in How we work." },
                    ],
                },
            ],
        },
        {
            id: "use",
            title: "Using fd",
            children: [
                {
                    id: "build",
                    title: "Build it",
                    question: "How do I build Fractodactyl?",
                    answer: "Fractodactyl builds with stable Rust: `cargo build --release -p fd-cli` produces the `fd` tool, and adding `-p fd-viewer` builds the desktop explorer. FFmpeg on PATH is needed for MP4 output, and the shaders need no SDK because wgpu compiles them at run time.",
                    blocks: [
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
                    title: "Make a film",
                    question: "How do I make a zoom film with Fractodactyl?",
                    answer: "Fractodactyl makes films with `fd film RE IM --to WIDTH --mp4 FILE`, which renders each frame with the cheapest valid kernel, shades it on the GPU and pipes it to FFmpeg. Adding `--zone FILE` uses a zone's fast path where it covers the frame.",
                    blocks: [
                        { kind: "steps", items: [
                            { title: "Build the zone once", text: "`fd zone <re> <im> -o data/zones/v0.zone` builds the fast-path constants for a minibrot near M(24,2) in about 8 s." },
                            { title: "Render the film", text: "`fd film <re> <im> --from 1e-40 --to 2.5e-49 --rate 0.3 --ease on --twist 0.5 --flow 0.08 --zone data/zones/v0.zone --mp4 landing.mp4`" },
                            { title: "Check it", text: "`fd filmcheck` compares the film against an exact render, frame by frame." },
                        ] },
                        { kind: "table", head: ["Default", "Value"], rows: [
                            ["Look", "studio, ice preset"],
                            ["Speed", "0.15 decades per second"],
                            ["Frames", "60 fps, 1920x1080"],
                            ["Sampling", "`--ss 2`, `--aa on`, unresolved drawn as interior"],
                        ] },
                        { kind: "p", text: "`fd film` with no arguments lists every flag." },
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
                {
                    id: "looks",
                    title: "Looks",
                    question: "How do I change the colours of a Fractodactyl render?",
                    answer: "Fractodactyl colours renders with looks: `--preset NAME` picks `looks/NAME.look` or a built-in (ice, coral, steel, zebra, smoke), and flags such as `--density`, `--terrace`, `--slope`, `--light` and `--lines` tweak it. Recolouring never redoes the deep maths.",
                    blocks: [
                        { kind: "p", text: "The deep maths produces palette-free samples (smooth escape value, distance estimate, normal). A look is applied afterwards, on the GPU, so `fd shade X.fds --preset ice -o DIR` re-colours stored samples without iterating again." },
                        { kind: "p", text: "`--flow`, `--breathe` and `--drift` animate the bands in films. A `.look` file is a few `key value` lines, such as `stops #rrggbb ...` and `density 0.05`." },
                    ],
                },
                {
                    id: "explorer",
                    title: "The explorer",
                    question: "How do I explore the Mandelbrot set with Fractodactyl?",
                    answer: "Fractodactyl's explorer, `fd-viewer`, is an early-build desktop window over the real renderer: drag to pan, scroll to zoom toward the cursor, and use Explore, Places, Saved and Snapshot to jump to landmarks, save spots for films and export PNGs.",
                    blocks: [
                        { kind: "keys", rows: [
                            ["Drag", "Pan (glides on release)"],
                            ["Wheel", "Zoom toward the cursor"],
                            ["+ / -", "Zoom in or out"],
                            ["Arrow keys", "Pan"],
                            ["1-9, 0", "Jump to a landmark"],
                            ["L", "Open the studio look editor"],
                        ] },
                        { kind: "p", text: "Saved spots are written to `places/NAME.place`, so `fd film --place NAME` can zoom to them. A native Dioxus version, drawing through the same pipeline as the films, is planned." },
                        { kind: "gallery", columns: 2, label: "fd-viewer at four places in four looks", images: [
                            { src: "/img/viewer-elephant-1280.webp", alt: "fd-viewer showing Elephant Valley in the master-ice look", caption: "Elephant Valley, master-ice.", width: 1280, height: 1082 },
                            { src: "/img/viewer-antenna-1280.webp", alt: "fd-viewer showing the antenna minibrot in the ice look", caption: "The antenna minibrot, ice.", width: 1280, height: 1082 },
                            { src: "/img/viewer-deepvalley-1280.webp", alt: "fd-viewer showing a deep valley at zoom 1.3e12 in the smoke look", caption: "A deep valley at zoom 1.3e12, smoke.", width: 1280, height: 1082 },
                            { src: "/img/viewer-coral-1280.webp", alt: "fd-viewer showing Seahorse Valley in the coral look", caption: "Seahorse Valley, coral.", width: 1280, height: 1082 },
                        ] },
                    ],
                },
            ],
        },
        {
            id: "inside",
            title: "How it works",
            children: [
                {
                    id: "exact-coordinates",
                    title: "Exact coordinates",
                    question: "How does Fractodactyl keep deep coordinates exact?",
                    answer: "Fractodactyl never rounds a deep centre or width to a 64-bit float: centres are exact high-precision values and widths and scales carry their own exponent end to end, so a frame at 1e-1000 is placed exactly where it was asked to be.",
                    blocks: [
                        { kind: "p", text: "A 64-bit float runs out of digits around 1e-16 and out of range around 1e-308. Fractodactyl keeps a high-precision reference point and works in small offsets from it (perturbation), with explicit exponents where values would otherwise underflow." },
                    ],
                },
                {
                    id: "maths-and-colour",
                    title: "Maths apart from colour",
                    question: "Why does Fractodactyl separate rendering from colouring?",
                    answer: "Fractodactyl keeps the deep maths and the colouring apart: the kernel produces palette-free samples, and a look is applied afterwards on the GPU. Recolouring or re-lighting a film never repeats the expensive part.",
                    blocks: [
                        { kind: "list", items: [
                            "**Samples:** class (escaped, interior, unresolved), smooth escape value, distance estimate and surface normal.",
                            "**Looks:** palette stops, band density, terraces, slope lighting, contour lines and animation, all in one WGSL implementation.",
                        ] },
                    ],
                },
                {
                    id: "gpu",
                    title: "One source on the GPU",
                    question: "Which GPUs does Fractodactyl run on?",
                    answer: "Fractodactyl writes its GPU stages once in WGSL on wgpu, so the same code is meant to run on NVIDIA, AMD and Intel GPUs under Windows and Linux, with an f32-only path for GPUs without 64-bit floats. So far it is verified on an AMD RX 5700 XT and on software adapters.",
                    blocks: [
                        { kind: "p", text: "Each piece of maths is written once and reused everywhere; the only deliberate second copy is an independent high-precision oracle used to check results. Native GPU 64-bit floats are an optional fast path, used only where they measure faster." },
                    ],
                },
                {
                    id: "guards",
                    title: "Guards and fallbacks",
                    question: "How does Fractodactyl know a shortcut is safe to use?",
                    answer: "Fractodactyl authorises every shortcut before rendering: a guard checks the frame lies where the shortcut is valid, and the shortcut can decline, sending the frame to perturbation with BLA. A shortcut is kept only after matching an exact render on every pixel of whole frames.",
                    blocks: [
                        { kind: "p", text: "For stills and research the bar is every pixel within 1e-3 px of the exact render, with no class mismatches. Films are judged by what viewers see: shaded frames within one colour level of the exact render on at least 99.9% of pixels, with no extra flicker." },
                    ],
                },
                {
                    id: "atlas",
                    title: "The atlas, and what it taught",
                    question: "What happened to Fractodactyl's mathematical atlas?",
                    answer: "Fractodactyl began as a path-compiled mathematical atlas that cached reusable work across frames. The full machinery was built, and the decisive benchmark found it only tied per-frame rendering because what it cached was cheap to rebuild, so the project turned to per-pixel theorems.",
                    blocks: [
                        { kind: "p", text: "The lesson became a rule: cache only what is expensive to rebuild or inherently shared across frames. The atlas tools (`fd compile`, `fd play`, `fd schedule`) remain, and cross-frame reuse returns wherever it measurably pays." },
                    ],
                },
            ],
        },
        {
            id: "project",
            title: "The project",
            children: [
                {
                    id: "how-we-work",
                    title: "How we work",
                    question: "How does the Fractodactyl project test ideas?",
                    answer: "Fractodactyl tests ideas with seconds-scale probes before any expensive run, measures every speedup against a fair opponent, keeps a shortcut only after whole frames match the exact render, and writes every result, positive or negative, into a public results log.",
                    blocks: [
                        { kind: "steps", items: [
                            { title: "Probe", text: "Under 30 s: a few thousand points or a tiny frame. Kill or keep." },
                            { title: "Promote", text: "Under 5 min: 20-50 small frames, with honest wall-clock time." },
                            { title: "Benchmark", text: "The real path at real size, only to confirm what the first two showed." },
                        ] },
                        { kind: "p", text: `Read [the method and results log](${SPEC}/METHOD.md) for every measured result, with its command and numbers.` },
                    ],
                },
                {
                    id: "roadmap",
                    title: "Where it is going",
                    question: "What is next for Fractodactyl?",
                    answer: "Fractodactyl's planned next steps are fast paths for any film rather than one family of places, a trip planner that strings minibrots, spirals and embedded Julia sets into directed films, a native Dioxus explorer on the film pipeline, and fair races against other deep-zoom renderers.",
                    blocks: [
                        { kind: "callout", tone: "note", text: "Plans, not promises: this is what is being worked on, in the open." },
                        { kind: "list", items: [
                            "**Fast everywhere:** find and build the right shortcuts automatically for any zoom path.",
                            "**Directed films:** map the set's features and plan journeys with deliberate climaxes.",
                            "**Native explorer:** `fd-viewer` in Rust and Dioxus, drawing through the film pipeline.",
                            "**Head-to-heads:** published races against rival renderers on common locations.",
                        ] },
                        { kind: "p", text: `Read [where we are](${SPEC}/STATE.md) for the current state, decisions and work order.` },
                    ],
                },
                {
                    id: "specs",
                    title: "Specifications",
                    question: "Where are Fractodactyl's full specifications?",
                    answer: "Fractodactyl's full specifications live in the repository under `docs/spec`: the north star, the current state, the research method and results log, the architecture, the feature map and one document per subsystem.",
                    blocks: [
                        { kind: "cards", items: [
                            { title: "North star", text: "The promise and the three tests.", href: `${SPEC}/NORTH-STAR.md`, label: "Open" },
                            { title: "Where we are", text: "Done, next, and how to run it.", href: `${SPEC}/STATE.md`, label: "Open" },
                            { title: "Method", text: "How ideas are tested, and every result.", href: `${SPEC}/METHOD.md`, label: "Open" },
                            { title: "Architecture", text: "Tiles, storage, GPU and refinement.", href: `${SPEC}/ARCHITECTURE.md`, label: "Open" },
                            { title: "Feature map", text: "Every kind of feature in the set, and how to find it.", href: `${SPEC}/FEATURE-MAP.md`, label: "Open" },
                            { title: "Explorer", text: "The explorer's server, tiles and look mode.", href: `${SPEC}/EXPLORE.md`, label: "Open" },
                        ] },
                    ],
                },
            ],
        },
    ],
});
