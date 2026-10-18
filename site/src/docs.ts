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
                    answer: "Fractodactyl makes films with `fd film RE IM --to WIDTH --mp4 FILE`, which renders each frame with the cheapest valid kernel, shades it on the GPU and pipes it to FFmpeg. Adding `--zone FILE` uses a zone's fast path where it covers the frame.",
                    blocks: [
                        { kind: "p", text: "This is an offline desktop workflow: the renderer computes the final frames and FFmpeg encodes an MP4. The target is high-resolution video that is visually indistinguishable from an exact render. Live navigation previews and final video exports have different quality requirements." },
                        { kind: "p", text: "**Control the camera:** `--from` and `--to` are frame widths in the complex plane; smaller means deeper. Use `--seconds` for a fixed duration or `--rate` for decades of zoom per second. `--ease on` softens the start and stop, `--twist` adds rotation, and `--flow`, `--breathe` and `--drift` animate the look." },
                        { kind: "p", text: "**Optional acceleration:** for a supported minibrot near M(24,2), build `fd zone <re> <im> -o data/zones/v0.zone` once, then add `--zone data/zones/v0.zone` to the film command. A zone is specific to its location and validity range. You can render without one; see Zones for current coverage." },
                        { kind: "steps", items: [
                            { title: "Choose the destination and style", text: "Explore in the viewer, save a spot and copy its exact real and imaginary coordinates. Pick a look from `looks/`. In the commands below, replace `<re>` and `<im>` with those coordinates; do not type the angle brackets." },
                            { title: "Make a short preview", text: "`fd film <re> <im> --from 0.0065 --to 0.003 --seconds 5 --size 640x360 --fps 30 --preset ice --mp4 preview.mp4` is a starter recipe for a Seahorse Valley destination. Choose widths that suit your own location, then inspect the framing, pace and colours." },
                            { title: "Render the final", text: "Reuse the preview command with `--size 1920x1080 --fps 60 --ss 2 --mp4 final.mp4`. Set `--seconds` to the desired duration. Larger sizes and more samples increase render time; export a short segment before committing to a long film." },
                            { title: "Inspect and validate", text: "Watch the encoded MP4 for framing, banding and flicker. When evaluating an accelerated route against a reference, run `fd filmcheck reference.mp4 candidate.mp4` with matching cameras, looks and encoding settings; it compares every frame. A visual review alone does not establish numerical accuracy." },
                        ] },
                        { kind: "table", head: ["Default", "Value"], rows: [
                            ["Look", "studio, ice preset"],
                            ["Speed", "0.15 decades per second"],
                            ["Frames", "60 fps, 1920x1080"],
                            ["Sampling", "`--ss 2`, `--aa on`, unresolved drawn as interior"],
                        ] },
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
            children: [
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
                    answer: "Fractodactyl accelerates selected minibrot neighbourhoods using parameter-dependent return maps, local Koenigs conjugacy and exit-tail evaluation. Perturbation and BLA supply fallback routes. Finite-series and floating-point errors are checked against high-precision reference data.",
                    keywords: ["Koenigs linearization", "return map", "perturbation", "minibrot"],
                    blocks: [
                        { kind: "p", text: "**Dynamical model:** let `f_c(z) = z² + c`, with critical orbit `z₀ = 0` and `zₙ₊₁ = f_c(zₙ)`. For a period-P structure, a local return map approximates `f_c^P` in normalized state and parameter coordinates. Its validity domain limits where a truncated series may replace P direct iterations." },
                        { kind: "p", text: "**Koenigs conjugacy:** near a repelling fixed point α of a return map F, with multiplier `λ = F′(α)` and `|λ| > 1`, a local chart φ satisfies `φ(F(z)) = λ φ(z)`. Within the chart and inverse-chart domains, `F^k(z) = φ⁻¹(λ^k φ(z))`. The implementation uses finite series, domain guards and derivative propagation; the theorem alone does not certify floating-point truncation error." },
                        { kind: "p", text: "**Parameter dependence:** neighbouring pixels have different c. Return-map coefficients, chart evaluation and the exit tail must retain the parameter dependence required by the error budget. Tan Lei similarity motivates structure near Misiurewicz points; it is not permission to replace a frame with an image of a different location." },
                        { kind: "compare",
                            before: { title: "A usual deep-zoom renderer", text: "Follows each pixel's orbit step by step, often for hundreds of thousands of steps, with tricks (perturbation, BLA) to make each step cheaper." },
                            after: { title: "Fractodactyl's fast path", text: "Near a minibrot, most of those steps become a few loops of a small return map plus one guarded Koenigs jump, then a short finish. The pixel is still computed, not guessed." } },
                        { kind: "p", text: "That fast path is packaged as a **zone**: the constants for one stretch of a zoom path, built once by `fd zone`. Before a frame uses a zone, a guard checks the zone is valid there; if not, the frame falls back to perturbation with BLA." },
                        img("spiral", "A spiral near the Misiurewicz point M(24,2) at width 1e-24", "The spiral around M(24,2) at width 1e-24, on fd's perturbation kernel: 4.1 s for 3200x1800 samples."),
                        img("minibrot", "A period-764 minibrot at width 1e-48", "Twenty-four decades deeper: the period-764 minibrot at width 1e-48, on the GPU zone kernel, 0.25 s for the same 3200x1800 samples."),
                    ],
                },
                {
                    id: "exact-coordinates",
                    title: "Exact coordinates",
                    question: "How does Fractodactyl keep deep coordinates exact?",
                    answer: "Fractodactyl preserves deep centres as high-precision values and carries explicit exponents for widths and offsets, avoiding binary64 rounding and underflow in camera coordinates. Orbit and sample accuracy are checked separately.",
                    blocks: [
                        { kind: "p", text: "**Perturbation recurrence:** write `c = C + δc` and `zₙ = Zₙ + δzₙ`, where `Zₙ₊₁ = Zₙ² + C`. Then `δzₙ₊₁ = 2Zₙδzₙ + δzₙ² + δc`. The reference orbit carries high precision; local offsets avoid subtracting nearby absolute coordinates in machine precision." },
                        { kind: "p", text: "**Derivative state:** `Dₙ₊₁ = 2zₙDₙ + 1`, with `D₀ = 0`, propagates sensitivity to c. Accelerated jumps must also propagate the derivative used for distance estimation and normals. Mantissa/exponent representations extend range at extreme depth; they do not by themselves increase significand precision." },
                        { kind: "p", text: "A 64-bit float runs out of digits around 1e-16 and out of range around 1e-308. Fractodactyl keeps a high-precision reference point and works in small offsets from it (perturbation), with explicit exponents where values would otherwise underflow." },
                    ],
                },
                {
                    id: "maths-and-colour",
                    title: "Maths apart from colour",
                    question: "Why does Fractodactyl separate rendering from colouring?",
                    answer: "Fractodactyl keeps the deep maths and the colouring apart: the kernel produces palette-free samples, and a look is applied afterwards on the GPU. Recolouring or re-lighting a film never repeats the expensive part.",
                    blocks: [
                        { kind: "p", text: "**Sample interface:** the kernel emits class, smooth escape value ν, distance estimate and normal, independent of palette. The shading stage consumes these columns plus the look and animation time. Displaying unresolved samples as interior is a film presentation choice; scientific consumers must retain the unresolved class." },
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
                        { kind: "p", text: "**Admission contract:** a production shortcut must pass a pre-render validity-domain and profitability guard. A rejection is an ordinary outcome: use the appropriate perturbation/BLA or direct route. Post-render agreement is validation evidence, not a substitute for prospective admission." },
                        { kind: "p", text: "**Numerical contract:** whole-frame comparisons require equivalent pixel displacement ≤1e-3 px and zero class mismatches. Distance estimates and normals are scored separately; the results log specifies relative/angle tolerances and conditioning exclusions. Escaped, interior and unresolved are distinct states: exhausting an iteration budget is not a proof of membership." },
                        { kind: "p", text: "**Evidence boundary:** passing sampled points or tested whole frames is empirical validation on that corpus. It is not interval certification of every input in a domain. Films additionally use a perceptual contract: compare shaded output and temporal behaviour against the exact route. See the method log for the actual commands and measured limits." },
                        { kind: "p", text: "For stills and research the bar is every pixel within 1e-3 px of the exact render, with no class mismatches. Films are judged by what viewers see: shaded frames within one colour level of the exact render on at least 99.9% of pixels, with no extra flicker." },
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
                {
                    id: "how-we-work",
                    title: "How we work",
                    question: "How does the Fractodactyl project test ideas?",
                    answer: "Fractodactyl tests ideas with seconds-scale probes before any expensive run, measures every speedup against a fair opponent, keeps a shortcut only after whole frames match the exact render, and writes every result, positive or negative, into a public results log.",
                    blocks: [
                        { kind: "p", text: "**Research protocol:** freeze coordinates, widths, iteration budgets, sampling, output columns and reference data before comparing candidates. Report the hardware, precision tier, cold construction cost, per-frame execution, fallback coverage and end-to-end time. Reduced iteration counts alone do not establish a speedup." },
                        { kind: "p", text: "**Independent validation:** frozen truth points screen candidates; promotion requires whole-frame checks at each tested depth and high-precision adjudication of the worst disagreements. Preserve negative results and distinguish an implemented algorithm, a passing experiment and a proved bound." },
                        { kind: "steps", items: [
                            { title: "Probe", text: "Under 30 s: a few thousand points or a tiny frame. Kill or keep." },
                            { title: "Promote", text: "Under 5 min: 20-50 small frames, with honest wall-clock time." },
                            { title: "Benchmark", text: "The real path at real size, only to confirm what the first two showed." },
                        ] },
                        { kind: "p", text: `Read [the method and results log](${SPEC}/METHOD.md) for every measured result, with its command and numbers.` },
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
                {
                    id: "roadmap",
                    title: "Where it is going",
                    question: "What is next for Fractodactyl?",
                    answer: "Fractodactyl's planned next steps are fast paths for any film rather than one family of places, a trip planner that strings minibrots, spirals and embedded Julia sets into directed films, a native Dioxus explorer on the film pipeline, and fair races against other deep-zoom renderers.",
                    blocks: [
                        { kind: "p", text: "**Open research questions:** can parameter-dependent local maps cover arbitrary paths with useful prospective error bounds? When do chart construction and derivative propagation amortize across frames? Which precision tiers preserve the sample contract on each GPU? These are experimental questions, not established capabilities." },
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
