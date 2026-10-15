# Competitive Analysis of Leading Mandelbrot Fractal Zoom Generators

## Executive assessment

As of October 15, 2026, there is **no single Mandelbrot zoom generator that clearly dominates every important dimension**. Instead, the competitive frontier is split among several specialized leaders:

| Competitive frontier | Current reference point | Why it matters |
|---|---|---|
| Extreme-depth GPU arithmetic | **FractalShark** | GPU-accelerated arbitrary/high-precision reference-orbit calculation, CUDA linear approximation, and a built-in minibrot around \(10^{-650452}\). Its author reports about a 10× reference-orbit advantage over FractalShark's own multithreaded MPIR+AVX2 path on an RTX 4090 at ~158,000 decimal digits. citeturn16view0 |
| Fast CPU deep zoom | **Fractal Zoomer / Imagina-class algorithms** | Fractal Zoomer combines perturbation, BLA/LA, boundary tracing, reference compression and multithreading. In a published cross-tool test by the Kalles maintainer, it rendered a \(5\times10^{-433}\) location in 30 seconds on a 16-thread Ryzen 7 2700X with BLA and boundary tracing. citeturn20view0turn18view3 |
| Heterogeneous CPU/GPU execution | **Fraktaler 3** | Can use conventional CPU code plus OpenCL CPU and GPU devices simultaneously, benchmarks the available numeric/device combinations, and automatically chooses among them through its “wisdom” system. citeturn17view6 |
| Artistic creation and animation | **leading commercial solutions** | The strongest integrated art workflow: layers, gradients, formula extensibility, parameter animation, timeline editing, motion blur, batch rendering, high-resolution disk rendering and distributed network calculations. citeturn17view3turn19search1 |
| Scriptable/scientific rendering | **fractalshades** | Python automation, MPFR arbitrary precision, perturbation plus chained BLA, Numba-parallel CPU loops, sophisticated post-processing and an H.264 movie API. citeturn17view5turn19search2 |
| Established deep-zoom ecosystem | **Kalles Fraktaler 2+** | Still an important interoperability and feature baseline: perturbation, series approximation, Newton zooming, numerous formulas, GLSL coloring, EXR/KFB workflows and extremely high MPFR precision ceilings. Its main technical weakness is the absence of BLA and largely stopped development. citeturn17view0turn17view1 |
| CPU algorithm research | **Imagina** | Influential LA/BLA and reference-compression work has propagated into other leading renderers. The current public application is still labeled pre-release and publishes much less product/benchmark documentation than the others. citeturn17view9turn16view0 |

The central conclusion is therefore stronger than “beat Kalles”: **a new product claiming to be the best available would have to combine capabilities that today are distributed across several programs**. It needs FractalShark-class extreme precision, Fractal Zoomer/Imagina-class CPU algorithms, Fraktaler 3-class heterogeneous execution, leading commercial solutions-class creative workflow, fractalshades-class automation, and Kalles-class deep-zoom interoperability.

There is also an important benchmark caveat. “Iterations per second” by itself is increasingly a poor measure of modern fractal-renderer performance. BLA can skip large blocks of iterations; boundary tracing and Mariani/Silver can avoid calculating pixels; reference reuse can eliminate arbitrary-precision work; and perturbation converts most pixels from expensive arbitrary precision to much cheaper delta arithmetic. BLA specifically works by finding a valid approximation that skips multiple iterations at a time. citeturn14search2turn18view3 **Wall-clock time at fixed image quality and numerical correctness is therefore the primary competitive metric**, supplemented by reference-orbit throughput, pixel-sample throughput, memory use and latency.

### Evidence notation

In the tables below, **Verified** means the capability is explicitly documented in an official project manual, source repository or release record. It does *not* necessarily mean independently benchmarked. **Self-benchmark** means the developer measured their own software. **Cross-tool benchmark** means another renderer developer published the measurement. **Marketing claim** means wording such as “virtually unlimited” or “100 times faster” that lacks enough controlled test detail for an engineering comparison. **Not published** means I did not find a defensible figure in the primary sources reviewed.

## Core rendering and numerical comparison

| Tool | Rendering performance and acceleration | Zoom depth and precision | Perturbation / approximation | Public benchmark evidence | Principal technical limitations |
|---|---|---|---|---|---|
| **Kalles Fraktaler 2+** | Multithreaded CPU rendering; SIMD controls; OpenCL for perturbation iterations; multithreaded reference calculations for selected numeric types. OpenCL can use fp32/fp64, but the manual says ordinary x86 CPU code is likely substantially faster around \(10^{-300}\)–\(10^{-4900}\) for many built-in formulas because x87 `long double` has useful range that OpenCL lacks. Maximum 64 CPU threads. **Verified.** citeturn20view1turn17view1 | MPFR ceiling documented as **2 gigabits ≈646 million decimal digits**; maximum iteration count 9 exa-iterations and period 2 giga-iterations. Rescaled perturbation allows depths far beyond ordinary floating-point exponent range. **Verified implementation limits.** citeturn17view1turn2view2 | Perturbation + series approximation, including NanoMB variants; reliable glitch handling and multiple lower-precision representations. **No BLA.** The current maintainer explicitly says BLA is “much better” than SA and is unlikely to be added. citeturn17view0 | Official KF benchmark suite uses six power-2 Mandelbrot locations at 512×512 on machines including a Ryzen 7 2700X. The site's cross-tool test says the \(5\times10^{-433}\) location took **~20 min** in current-era KF/NanoMB1 versus ~2 min in fractalshades and 30 s in tuned Fractal Zoomer. **Cross-tool developer benchmark, not independent lab test.** citeturn17view2turn20view0 | Development is “mostly stopped”; no BLA; hard Windows bitmap limit of ~2 GiB/~715 Mpx and 64K longest bitmap dimension for the ordinary bitmap path; 64-thread ceiling. citeturn17view0turn17view1 |
| **leading commercial solutions** | Multicore/multiprocessor calculation and network distribution. OpenGL accelerates **layer compositing/UI**, not documented as a Mandelbrot compute backend. Render jobs can run sequentially or simultaneously. **Verified.** citeturn17view3turn17view4turn20view3 | Vendor describes zoom depth as **“virtually unlimited.”** Precision is automatically increased as zoom depth requires; reviewed documentation does not publish a comparable hard digit ceiling. Treat “virtually unlimited” as a **marketing description, not a numerical maximum**. citeturn19search1turn4search4 | commercial-reference introduced perturbation for deep zooms and permits suitable custom formulas to add perturbation support. No BLA implementation is advertised in the official material reviewed. citeturn20view3 | Vendor example: Mandelbrot around **\(2\times10^{243}\) magnification rendered in 42 s**. Hardware, resolution and all render settings are not included in the public claim, so this is a **self-demo, not a reproducible benchmark**. citeturn20view3 | Deep-zoom compute stack appears behind BLA-based specialist tools; no advertised CUDA/OpenCL compute backend; some formulas requiring whole-image/global operations cannot take full advantage of tiled/network rendering. citeturn3search2turn4search26 |
| **Fractal Zoomer 1.0.9.5** | Java renderer with Boundary Tracing, Mariani/Silver, Successive Refinement and perturbation. Recent versions added work stealing, optimized MPFR/MPIR paths, Linux MPFR multithreading, multithreaded BLA generation and compressed references. **Verified.** citeturn15search1turn17view8turn18view0 | Supports multiple precision engines including double (53-bit), double-double (106-bit), built-in high precision, MPFR and automatic selection; uses extended-exponent representations for deep calculation. Exact maximum decimal zoom depth is not published as a simple hard limit. citeturn15search16 | Perturbation, BLA/LA including newer Zhuoran algorithms, Nanomb1/approximation, reference compression and period-aware optimization. This is one of the broadest CPU deep-zoom stacks reviewed. citeturn18view3turn17view8 | On the Kalles maintainer's 16-thread Ryzen 7 2700X, the common \(5\times10^{-433}\) example rendered in **30 s** with perturbation/BLA + boundary tracing; without boundary tracing it took just over **2 min**. **Cross-tool developer benchmark.** citeturn20view0 | No current official CUDA/OpenCL compute backend is advertised in the project feature/release material reviewed. That makes the very-fast CPU algorithm stack its comparative strength, but also leaves GPU compute as an obvious opportunity for a challenger. citeturn15search1turn18view0 |
| **FractalShark 0.54-series** | NVIDIA CUDA-focused. High-precision reference orbit runs in a persistent fused CUDA kernel using NTT arithmetic, pointwise recurrence operations, signed carry propagation and normalization. Multiple CUDA pixel algorithms; CUDA LA; multithreaded AVX2/MPIR CPU reference path. **Verified implementation.** citeturn16view0 | Uses GPU arbitrary/high precision for reference calculations and extended-range per-pixel arithmetic. Built-in View #34 contains a minibrot at approximately **\(10^{-650452}\)** depth. **Verified as a built-in project example, not an independently certified “world record.”** The author explicitly rejects simplistic “deepest ever” claims. citeturn16view0 | Perturbation plus two CUDA linear-approximation implementations; custom 2×32-bit-plus-exponent representation delivers roughly 48 bits of mantissa without consumer-GPU fp64. Reference-orbit compression and optional cached “perturbed perturbation.” citeturn16view0 | At **16,384 32-bit limbs ≈158,000 decimal digits**, author reports GPU reference-orbit calculation about **10× faster on RTX 4090** than FractalShark's multithreaded MPIR+AVX2 CPU path; RTX 5090 prior test ~9×. For v0.54 the developer reports ~70× vs single-thread AVX2 MPIR, but explicitly says the multithreaded comparator was broken. **Strong self-benchmark, not third-party.** citeturn16view0 | CUDA/NVIDIA-centric; RTX 2xxx+ required for GPU high-precision orbit path; no AMD GPU support; CPU rendering described by its own author as a debugging aid rather than best-in-class; Mandelbrot only; application openly characterized as experimental, buggy and less polished than Fractal Zoomer. citeturn16view0 |
| **fractalshades 1.2.1** | Parallel CPU core loops compiled with Numba; arbitrary precision through an MPFR C extension/Cython. Tiled computation supports large work and recovery. **Verified.** citeturn17view5turn19search10 | Documentation explicitly targets arbitrary-precision Mandelbrot/Burning Ship at **\(10^{-2000}\) and beyond**; showcased Mandelbrot width \(2\times10^{-2608}\). Standard-double deltas use an extra exponent at extreme depth. citeturn17view5 | Perturbation + **chained bilinear approximations**, with rebasing/extended-range deltas. Documentation states chained approximations can skip many iterations and yield speedups of several orders of magnitude in suitable regions; that is a **developer algorithmic claim**, not a fixed benchmark ratio. citeturn17view5turn11search7 | Kalles maintainer reports the common \(5\times10^{-433}\) minibrot in **just over 2 min** on Ryzen 7 2700X, versus ~20 min in KF/NanoMB1. **Cross-tool developer benchmark.** citeturn20view0 | CPU rather than GPU-focused in the documented core; narrower set of fractal formulas than broad creative suites; workflow remains Python/script-centric despite a PyQt6 exploration GUI. citeturn17view5turn20view0 |
| **Fraktaler 3 v3.1** | Conventional CPU plus **OpenCL CPU/GPU devices simultaneously**. Its “wisdom” system benchmarks every device/number-type pairing and automatically selects the fastest available combination for the location. Multi-GPU tiling is supported in the renderer architecture. citeturn17view6turn17view7 | MPFR is part of the build stack; multiple ordinary, extended-range and high-precision numeric representations are selected according to depth/hardware. Exact maximum decimal depth is not published as a simple cap. citeturn18view4turn17view6 | Perturbation plus BLA, reference-period optimizations and reference reuse. Modern implementation follows the deep-zoom BLA/rebasing approach documented by its author. citeturn14search2turn17view7 | Historical cross-tool \(5\times10^{-433}\) result: **just over 2 min** on Ryzen 7 2700X. A current project tuning example shows the same test location improving from **3:00 to 1:34** merely by changing OpenCL tile size from 128×128 to 960×1080, showing how strongly launch/tile configuration affects benchmark results. citeturn20view0turn17view7 | GPU timeouts can occur with oversized tiles; performance needs hardware-specific “wisdom”; maintainer notes native Microsoft Windows is untested even though Windows cross-builds/Wine paths exist. citeturn17view7turn18view4 |
| **Imagina** | C++ fast Mandelbrot explorer; project source contains a high-order/LA evaluator, feature finder, arbitrary-precision infrastructure and task-based computation. Its algorithms have influenced both Fractal Zoomer and FractalShark. citeturn14search4turn16view0 | Arbitrary/high precision and extreme-deep coordinate handling are present in source; current release notes also mention a “pre-modulo” workaround for precision loss. A clean current maximum depth figure is **not published**. citeturn17view9turn14search4 | LA/BLA-family methods, reference saving/compression and pixel reuse are core design themes. FractalShark explicitly credits Imagina for reference-orbit ideas and its Feature Finder inspiration. citeturn16view0 | Current official release materials call it “one of the fastest Mandelbrot set renderer & explorer,” but do not provide a modern controlled timing table. Treat this as a **developer claim, unverified quantitatively**. citeturn17view9 | Current releases are labeled **pre-release**; public product documentation is sparse, so output formats, standardized benchmark figures and hard limits are substantially less transparent than for KF, Fraktaler or leading commercial solutions. citeturn17view9 |

### What the rendering evidence actually says

The clearest direct CPU comparison available from the reviewed primary material is the Kalles maintainer's common \(5\times10^{-433}\) example on a 16-thread Ryzen 7 2700X:

| Renderer/configuration | Reported time | Relative to tuned Fractal Zoomer | Evidence quality |
|---|---:|---:|---|
| Fractal Zoomer, perturbation + BLA + boundary tracing | **30 s** | 1.0× | Cross-tool developer benchmark |
| Fractal Zoomer, no boundary tracing | **just over 2 min** | ~4× | Cross-tool developer benchmark |
| fractalshades | **just over 2 min** | ~4× | Cross-tool developer benchmark |
| Fraktaler 3, version contemporaneous with test | **just over 2 min** | ~4× | Cross-tool developer benchmark |
| Kalles Fraktaler, NanoMB1 | **about 20 min** | ~40× | Maintainer's own program + cross-tool comparison |

All of these figures come from the same Kalles maintainer page and hardware context. They are unusually useful because they give a common location and machine, but the page does not expose a sufficiently complete modern benchmark manifest—exact renderer build, all precision settings, image size for this particular cross-tool example, AA settings, iteration threshold and correctness test—to treat the numbers as a definitive 2026 ranking. citeturn20view0

The result nevertheless demonstrates something strategically important: **BLA and pixel-level avoidance algorithms matter enormously**. The difference between Fractal Zoomer's 30-second run and its own >2-minute run without boundary tracing is already roughly a fourfold swing without changing the mathematical location; Kalles' lack of BLA leaves it much farther behind on this example. citeturn20view0turn17view0

FractalShark occupies a different benchmark regime. Its most compelling public numbers concern the **arbitrary-precision reference orbit itself**, not end-to-end image rendering. At 16,384 32-bit limbs—approximately 158,000 decimal digits—the developer reports about a 10× advantage for an RTX 4090 over the project's existing multithreaded MPIR+AVX2 CPU path. That is the benchmark a genuinely cutting-edge GPU engine now has to confront. citeturn16view0

## Output quality, visual features and production workflow

| Tool | Still-image output | Video / animation | Visual-quality stack | Workflow and navigation | Scripting / extensibility / distributed features |
|---|---|---|---|---|---|
| **Kalles Fraktaler 2+** | PNG, JPEG, TIFF and EXR; EXR can contain raw iteration channels and metadata and is the recommended iteration-data format. Ordinary Windows bitmap path has ~2 GiB/~715 Mpx and 64K-dimension limits; tiled/EXR workflows provide alternatives for very large renders. citeturn20view2turn17view1 | Generates zoom-out frame sequences in JPEG/PNG/TIFF plus KFB/EXR for assembly by **third-party software** such as Zoomasm; no integrated modern video-codec pipeline. Resume sequence support exists, although “Examine Zoom sequence” is documented as probably broken. citeturn20view2 | Smooth iteration data, analytic distance-estimation options, jitter, supersampling/downsampling, custom GLSL coloring, multiple coloring modes and a “Best” preset that disables guessing and tightens glitch/approximation quality. citeturn20view1turn20view2 | Direct scroll-wheel zoom, Newton-Raphson zoom to deep features, undo/redo, extensive formula/bailout/color dialogs, custom hybrid formula editor. citeturn17view1 | Command-line rendering, GLSL coloring and custom hybrid formulas. No active distributed-rendering facility documented. citeturn17view1turn20view2 |
| **leading commercial solutions** | “Any size and resolution” through Render to Disk; JPEG, PSD, TIFF, PNG, BMP, Targa and AVI. PSD can support layered creative workflows. The “any size” wording should be interpreted as a streamed/render-to-disk architecture claim rather than literally infinite output dimensions. citeturn17view3turn4search1 | **Strongest integrated animation system reviewed.** Every parameter can be animated; arbitrary animation length and desired frame rate; timeline editor; motion blur; image sequence or AVI output in Extended Edition. citeturn17view3 | Sophisticated gradient editor, transparency, layers/masks/groups, anti-aliasing, linear-light AA improvements, motion blur and extensive community/custom coloring formulas. citeturn19search1turn20view3 | Mature GUI, real-time Explore parameter previews, integrated file browsers, high-DPI support and polished timeline/render queues. OpenGL makes layer compositing and GUI work more fluid. citeturn19search1turn20view3 | Optimizing formula compiler, complex types/arrays/classes/plugins, formula database, background/batch render queues and **distributed LAN/Internet calculations**. Vendor says networking can “easily double or triple” processing power—this specific magnitude is a marketing claim rather than controlled scaling data. citeturn17view3turn17view4 |
| **Fractal Zoomer** | Release 1.0.9.1 reworked width/height handling to allow “images of any dimension”; also documents BMP, JPG, PPM and PGM support in addition to its established renderer ecosystem. KFR/KFB interoperability is increasingly emphasized. No physical maximum is stated. citeturn18view0turn18view1 | Dedicated **Zoom Sequence** renderer, saved render settings/zoom factors, reference-orbit save/load and KFB output. The current release notes reviewed are frame/sequence-oriented and do not advertise a native modern movie-codec encoder. citeturn18view2 | Very broad: many coloring algorithms, palettes, generated palettes, domain coloring, normal mapping/distance estimation, image filters, post-processing, 3D heightmaps, pixel jitter and multiple AA sampling strategies. 1.0.9.5 adds texture mapping and lighting effects. citeturn15search1turn18view0turn17view8 | One of the most feature-rich traditional fractal GUIs; recent FlatLaf UI, quick-draw/preview mechanisms, selection zoom/rotation, statistics and task/thread metrics. citeturn18view0turn15search16 | More than **500 fractal-generating functions**, user formulas/custom functions and substantial import compatibility. Batch/minimal renderer is strong, but active network-distributed rendering is not documented. citeturn15search1 |
| **FractalShark** | Current public README does not provide a clean maximum-resolution/export-format matrix comparable with KF or commercial tools. It includes CLI infrastructure and PNG-related code/dependencies, but output production is not its competitive focus. citeturn16view0 | No polished integrated animation/timeline/video system is documented. An older distributed version was once used for zoom movies, but its network/distributed feature is explicitly **broken and will not be revived**. citeturn16view0 | GPU supersampled AA, palette coloring and parallel per-frame statistics; CPU equivalent exists. Visually much narrower than leading commercial solutions/Fractal Zoomer. citeturn16view0 | Basic box zoom/hotkeys, Feature Finder and experimental autozoom. Developer explicitly warns that some controls can crash/misbehave and calls the product experimental and rough. citeturn16view0 | CLI, extensive source/test infrastructure and cutting-edge CUDA numerical experiments. Windows and Linux are maintained at feature parity. No AMD support, no general formula language and Mandelbrot only. citeturn16view0 |
| **fractalshades** | PNG-oriented image output plus memory-mapped raw/post-processing databases; high-quality rendering is tiled so it can limit disk/RAM pressure and can resume interrupted renders. citeturn19search3turn19search10 | Native movie API produces **MP4/H.264**, accepts an arbitrary `(width,height)` tuple and integer FPS, supports yuv420p/yuv444p, and includes specialized pan, exponential zoom and custom sequences. citeturn19search2turn19search9 | Final rendering supports **2×2 through 7×7 supersampling**, i.e. 4–49 calculated samples per output pixel, plus jitter; rich distance-estimation and mathematical post-processing are central design goals. citeturn19search3turn17view5 | PyQt6 interactive GUI can create template scripts, but serious animation and advanced production remain Python-oriented. citeturn17view5turn19search9 | Excellent Python automation and reusable data pipeline. Multithreading is configurable; specialized database/projection system makes scripted movie generation powerful. No distributed or GPU backend is advertised in the core documentation reviewed. citeturn19search10turn17view5 |
| **Fraktaler 3** | EXR, PNG and JPEG with embedded metadata. Current v3 documents PNG RGB8, JPEG, and EXR RGB32F; with one subframe, batch EXR can also contain raw calculation data. Example recommended Zoomasm export is **12,288×1,360**; width/height are configurable integer parameters rather than tied to display size. citeturn18view4turn18view6 | Creates zoom-out sequence/keyframe data for external **Zoomasm** assembly rather than providing the strongest integrated movie editor. Reference reuse can accelerate sequences. citeturn17view7turn18view6 | Editable GLSL coloring, linear RGB, brightness/contrast/gamma/exposure, histogram display, metadata-preserving output and subframe-based AA. Tiles can be recolored without recomputing the fractal. citeturn18view7turn18view4 | SDL/OpenGL/Dear ImGui graphical explorer plus CLI batch mode; mouse/keyboard/touch navigation and Newton-based feature zooming; image size decoupled from GUI window. citeturn17view6turn18view4 | Batch CLI, editable GLSL and hardware benchmark/autotuning. Particularly notable for using **multiple heterogeneous CPU/OpenCL devices concurrently**. Builds target Linux, Windows, Android and Web, although Windows testing caveats remain. citeturn17view6turn18view4 |
| **Imagina** | Current release notes document raw pixel-data saving and reference-saving options, but no sufficiently complete current format/resolution specification was found in the primary release material reviewed. citeturn17view9 | No documented production-grade timeline/video subsystem in the release notes reviewed. | Performance/exploration rather than an leading commercial solutions-class creative stack is the focus. | Fast explorer, feature-finding architecture and pixel reuse. Transformation disables pixel reuse in the documented v0.2.0 release, illustrating an optimization trade-off. citeturn17view9turn14search4 | Open-source algorithm-focused application; reference saving/compression concepts are important enough to have been adopted or extended in FractalShark and Fractal Zoomer. citeturn16view0turn17view8 |

### Visual-quality bar

For pure image fidelity, a new entrant should not treat “smooth coloring” as a differentiator; it is table stakes. Kalles exposes smooth iteration data and analytic distance estimation, Fractal Zoomer has numerous smoothing/statistical/normal-map methods, Fraktaler exposes programmable GLSL coloring and raw channels, fractalshades emphasizes distance-estimation-based post-processing, and leading commercial solutions has an exceptionally mature gradient/layer system. citeturn20view1turn18view0turn18view7turn17view5turn19search1

Likewise, ordinary 2× or 4× supersampling is no longer impressive. fractalshades explicitly exposes up to **7×7 = 49 samples per output pixel**, FractalShark performs supersampled AA on the GPU, Fraktaler supports configurable subframes, and Fractal Zoomer continues adding AA sampling strategies. citeturn19search3turn16view0turn18view6turn18view0

A credible new leader should therefore target **at least 8×8/64-sample supersampling or a demonstrably better adaptive reconstruction algorithm**, jittered sampling, linear-light downsampling, temporal AA for animation, float/high-bit-depth rendering and raw-data recoloring without recomputing the orbit. The 64-sample figure is a proposed engineering threshold specifically intended to exceed the clearest fixed competitor figure—fractalshades' 49 samples/pixel—not a claim about an existing industry standard. citeturn19search3

## The benchmark thresholds a new “best available” renderer should meet

A product should not make a credible “best Mandelbrot zoom generator” claim merely because it reaches a large exponent. FractalShark's own author makes essentially this point when warning that the Internet contains misleading “deepest ever” claims, despite shipping an approximately \(10^{-650452}\) built-in example himself. citeturn16view0

The meaningful target is a **multi-dimensional performance envelope**.

### Rendering engine

The minimum modern architecture should include **perturbation plus rebasing/glitch handling plus BLA/LA**. Series approximation by itself is no longer state of the art for quadratic Mandelbrot deep zoom: even the Kalles maintainer now describes BLA as much better than the SA/NanoMB family that powers KF. citeturn17view0turn14search2

For CPU performance, the new renderer should beat **Fractal Zoomer under a strict same-machine A/B test**. The historical reproducibility target is the \(5\times10^{-433}\) example: ≤30 seconds on the same Ryzen 7 2700X configuration would at least equal the published tuned result. On modern CPUs, the better criterion is **≥25% lower end-to-end wall time than the current Fractal Zoomer build across a basket of deep locations**, not one cherry-picked image. The 25% margin is a recommended claim threshold; small single-digit differences are too sensitive to JIT warmup, tile choice, precision heuristics and operating-system noise. The known 30-second reference result itself is documented. citeturn20view0

For GPU extreme precision, FractalShark sets the higher bar. A new GPU path should benchmark against its **View #30 at 16,384 32-bit limbs / ≈158,000 decimal digits** and beat the current GPU reference-orbit implementation on identical RTX hardware. FractalShark's reported ~10× RTX 4090 advantage over its multithreaded CPU path shows that simply moving low-precision pixel calculations to a GPU is insufficient; the expensive **arbitrary-precision reference calculation itself** now has to be accelerated. citeturn16view0

A best-in-class implementation should also automatically choose among CPU, GPU and numeric representations based on measured performance. Fraktaler 3 already does this via per-device/per-number-type “wisdom,” so a manually configured CPU/GPU toggle would be a regression in usability. citeturn17view6

**Recommended performance targets for a new leader:**

| Metric | Competitive threshold for a credible “best” claim |
|---|---|
| Deep CPU end-to-end wall time | Beat current Fractal Zoomer by **≥25% geometric mean** over a public multi-location suite on the same CPU |
| Legacy \(5\times10^{-433}\) reference test | ≤30 s on the original Ryzen 7 2700X-class configuration if reproducing the historical test; preferably substantially below |
| Extreme GPU reference orbit | Beat FractalShark's current View #30 implementation on the **same RTX model**, precision and iteration count |
| Scaling | ≥80% useful scaling from 1→8 CPU cores on compute-heavy reference/pixel stages; publish 1/2/4/8/16/32-thread curves rather than only the fastest figure |
| Multi-GPU | Near-linear scaling where orbit/reference sharing does not dominate; at least two GPUs should be supported without manual image splitting |
| Navigation preview | First useful preview **<100 ms** after ordinary pan/zoom input; full shallow redraw should target 60 Hz where workload permits |
| Deep-navigation response | New low-resolution preview within **<500 ms** even while a reference/BLA structure is still being prepared; progressive refinement thereafter |
| Memory | Reference compression and streamed/tiled pixels; no requirement to hold the complete final image in RAM |

The latency and scaling percentages above are **recommended product thresholds**, not measurements claimed by competitors.

### Precision and numerical correctness

For an **overall** leadership claim, arbitrary precision must be genuinely arbitrary from the user's perspective, not “double-double plus a large exponent.” The reference orbit should use MPFR/GMP-class or custom arbitrary-precision arithmetic, while per-pixel deltas can use lower-precision extended-exponent representations where error bounds permit. That is broadly the architecture used by the best current deep zoomers. citeturn17view5turn16view0turn20view1

The new renderer should demonstrate all of the following:

**A public successful render at approximately \(10^{-650452}\) or deeper.** That matches FractalShark's strongest documented depth demonstration. A deeper number alone should not be marketed as a record unless coordinates, precision, reference period, iteration count and verification methodology are supplied. citeturn16view0

**At least ~650,000 decimal digits of practical coordinate/reference precision.** This is enough to demonstrate the above depth, even though Kalles' theoretical MPFR ceiling of ~646 million decimal digits is much larger. The important distinction is practical rendering speed: a huge library precision ceiling does not prove the renderer can do useful work there. citeturn17view1

**Stable rebasing/glitch correction and extended-exponent perturbations.** Deep zooms must not silently produce incorrect blobs, missing filaments or underflow artifacts. fractalshades explicitly carries an extra exponent for extreme-depth deltas, while Kalles and FractalShark use comparable extended-range techniques. citeturn17view5turn16view0

**Cross-validation against a slow high-precision reference path.** Every benchmark image should include a correctness pass: randomly selected pixels recomputed without approximation at enough MPFR precision to establish that the fast BLA/perturbation output is within the specified iteration/color error tolerance. This is a proposed benchmark requirement and would make performance claims substantially more rigorous than most existing public demos.

### Still images and video

A new overall leader should treat **64K as a compatibility floor, not a headline feature**. Kalles already reaches a 64K longest bitmap dimension before Windows bitmap limitations intervene, while Fractal Zoomer advertises software-level arbitrary width/height and leading commercial solutions claims disk rendering at any size/resolution. citeturn17view1turn18view1turn17view3

A more meaningful high-end demonstration would be a **streamed/tiled 100,000×100,000 render** that never requires the full image in memory, with crash-resume capability. That is a proposed stress test rather than an existing competitor benchmark.

For data preservation and post-production, the target format set should be at least:

**PNG + JPEG + TIFF + OpenEXR**, with EXR supporting float channels for continuous iteration, distance estimate, phase/angle, derivative/normal data, validity/glitch masks and full fractal metadata. Kalles already uses EXR specifically to preserve iteration data and metadata, while Fraktaler writes float EXR and raw calculation data. citeturn20view2turn18view4turn18view6

For video, the competitive bar is surprisingly open. leading commercial solutions has the best integrated keyframe/timeline workflow but its documented direct format remains AVI/image sequences, Kalles and Fraktaler rely heavily on sequence + Zoomasm workflows, and fractalshades provides direct MP4/H.264 generation but through Python. citeturn17view3turn20view2turn17view7turn19search2

That creates a clear opportunity for a new product to lead with:

| Capability | Recommended “best available” threshold |
|---|---|
| Interactive timeline | Arbitrary keyframes for position, log-zoom, rotation, iterations, palette, formula parameters, lighting and post-processing |
| Resolution | Native **4K60** production workflow; **8K30/8K60 image-sequence** support |
| Codec | H.264, H.265/HEVC and AV1 plus lossless/high-bit-depth image sequences |
| Color | At least 10-bit video path; 16/32-bit float EXR master path |
| Motion quality | Motion blur + temporal supersampling + deterministic interpolation |
| Sequence reliability | Pause/resume, crash recovery, frame checksums and ability to regenerate only missing/invalid frames |
| Deep-zoom efficiency | Reuse reference orbit/BLA structures across adjacent frames whenever mathematically valid |

Unlike zoom depth, this is a field where a new product could surpass nearly every competitor at once without inventing a new fractal algorithm.

## Workflow and advanced-capability bar

The hardest standard to beat on **creative workflow** is leading commercial solutions rather than the extreme-depth engines. leading commercial solutions can animate individual parameters through a timeline, queue multiple renders, organize formulas and parameters, use layers/masks, and distribute calculations across networked computers. citeturn17view3turn19search1

A technically faster renderer with a modal, fragile UI would therefore not credibly be “best available” overall. FractalShark demonstrates this trade-off directly: it arguably has the most adventurous GPU numerical engine in this comparison, yet its own README describes the application as experimental, buggy and substantially less polished than Fractal Zoomer. citeturn16view0

A new leader should combine five workflow properties.

### Immediate exploration

Navigation should support continuous wheel/pinch zoom, pan and rotation; fast low-resolution previews; undo/redo/history; bookmarks; automatic precision selection; and a **Newton/Feature Finder** that can lock onto minibrots or periodic points rather than making the user manually copy coordinates. Kalles, Fraktaler and FractalShark already offer Newton/feature-finding mechanisms, so this is no longer an advanced niche feature. citeturn17view1turn16view0

### Decoupled calculation and coloring

Recoloring should not rerun the orbit calculation. Fraktaler 3 caches tiles specifically so GLSL coloring can be changed without recomputing the fractal, while Kalles stores iteration data in EXR and Fractal Zoomer has increasingly reusable processing pipelines. citeturn18view7turn20view2

The ideal architecture would store a high-fidelity intermediate buffer/database containing raw continuous iteration, derivative/distance and auxiliary channels, with real-time GPU color/post-processing on top. That separates expensive mathematics from inexpensive artistic iteration.

### Automation as a first-class interface

A graphical interface alone is insufficient. fractalshades demonstrates the power of a Python-native model, while Fraktaler 3 exposes batch CLI operation and Kalles supports command-line rendering. citeturn19search9turn17view6turn20view2

For a new leader, **every GUI action should be reproducible through a CLI/API**, ideally with a versioned JSON/TOML project format. Batch jobs should expose machine-readable timing, iteration/skip statistics, precision chosen, device used, peak RAM/VRAM and output hashes. This would also make third-party benchmarking far easier.

### Real extensibility

There are two high standards to combine. leading commercial solutions allows users to author formulas, transformations and coloring code in its own compiled formula system, while Kalles/Fraktaler expose programmable coloring, including GLSL. citeturn17view3turn17view1turn18view7

An overall leader should expose:

- a safe high-level fractal/formula language that can compile to CPU and GPU;
- GLSL/WGSL/SPIR-V-class programmable coloring and post-processing;
- Python or another mainstream scripting API;
- plugin hooks for new precision types, approximation engines, color modules and exporters.

Most importantly, **custom formulas should still be able to participate in perturbation/BLA when their derivatives and critical-point behavior make that mathematically valid**. leading commercial solutions already allows suitable user formulas to opt into perturbation, establishing a useful baseline. citeturn20view3

### Distributed and heterogeneous rendering

leading commercial solutions is notable because current specialist deep-zoom engines have largely neglected network distribution; commercial tools distributes calculations over LAN/Internet, while FractalShark explicitly says its old distributed facility is broken and will not return. citeturn17view3turn16view0

Fraktaler 3, meanwhile, shows the desirable local-device model: conventional CPU and OpenCL CPU/GPU devices can work simultaneously, guided by hardware benchmarks. citeturn17view6

A genuine “best available” system should combine both:

**heterogeneous local compute + multi-GPU + remote worker nodes**, with automatic task sizing according to measured throughput.

CUDA-only should not be sufficient for an overall claim. FractalShark is very fast on NVIDIA but explicitly excludes AMD GPUs. A new cross-vendor implementation should ideally cover NVIDIA, AMD and Apple GPU hardware through CUDA plus Vulkan/OpenCL/Metal or another portable compute abstraction. citeturn16view0

## Recommended benchmark suite for substantiating a “best available” claim

The biggest weakness in the existing market is not lack of performance—it is **lack of standardized, reproducible performance reporting**. Kalles has a documented benchmark suite and FractalShark publishes useful reference-orbit comparisons, but the products generally do not publish SpecCPU-style cross-renderer results with identical inputs, output correctness requirements and hardware. citeturn17view2turn16view0

A new renderer could establish itself as the technical reference simply by publishing a serious open benchmark suite.

### Workload corpus

| Workload | Purpose | Suggested configuration |
|---|---|---|
| **Shallow baseline** | Measures ordinary SIMD/GPU pixel throughput without arbitrary precision dominating | Mandelbrot power 2, 3840×2160, fixed 1,000-iteration cap, fp64-quality reference |
| **Moderate deep zoom** | Tests perturbation setup and ordinary reference calculation | Depth around \(10^{-1000}\), 4K, fixed published coordinates |
| **Legacy KF/FZ deep point** | Connects to existing comparative evidence | The published \(5\times10^{-433}\) minibrot used in the Kalles comparison. citeturn20view0 |
| **High-period deep point** | Stresses BLA table generation, reference storage/compression and memory | Period in the hundreds of millions; FractalShark reports a 600,000,000-period case in which compression saved multiple GB. citeturn16view0 |
| **Extreme-reference test** | Measures arbitrary-precision engine directly | FractalShark View #30, 16,384 32-bit limbs / ~158k decimal digits. citeturn16view0 |
| **Extreme correctness test** | Establishes meaningful practical depth | FractalShark-class View #34 / approximately \(10^{-650452}\). citeturn16view0 |
| **Complex boundary scene** | Defeats overly optimistic boundary tracing/guessing | Dense filaments with exact-pixel validation and guessing disabled |
| **High-AA production frame** | Measures real art-render cost | 8K, ≥64 samples/pixel or equivalent adaptive AA, float intermediate |
| **Movie workload** | Tests reference reuse, temporal consistency and pipeline throughput | 60-second 4K60 logarithmic zoom with fixed start/end coordinates and palette |
| **Huge-image workload** | Tests streaming rather than RAM capacity | 100k×100k tiled EXR, crash/restart at 50%, then completion |

### Metrics that should be reported

**End-to-end wall clock time should be the headline metric.** It automatically rewards a renderer for BLA, boundary tracing, period detection, reference reuse, GPU acceleration and efficient scheduling rather than rewarding it for executing unnecessary iterations quickly.

Behind that headline, publish:

| Metric | Why it is necessary |
|---|---|
| Reference-orbit generation time | Separates arbitrary-precision bottleneck from pixel rendering; essential for FractalShark-class extreme depth |
| Actual reference iterations/s | Useful when comparing the same precision and numeric representation |
| Pixel samples/s | Normalizes rendering across resolution and AA level |
| Actual perturbation iterations executed | Reveals whether a result came from arithmetic throughput or better skipping |
| BLA/LA skip ratio | Quantifies approximation effectiveness |
| Pixels explicitly calculated vs inferred/guessed | Prevents boundary-tracing renderers from being compared as though every pixel were computed |
| CPU/GPU utilization | Detects serialization and load-balancing problems |
| Peak host RAM and VRAM | Critical at high period/depth |
| Time to first preview / p50 / p95 navigation latency | Measures actual exploration quality rather than final-render speed |
| Output error against MPFR reference | Prevents a fast-but-wrong renderer from winning |
| Power/energy per rendered frame | Useful for GPU-versus-CPU and cluster comparisons |
| Cold versus warm-cache time | Prevents hidden reference/BLA caches from contaminating comparisons |

This is especially important because Fraktaler's own documented test changes from **3:00 to 1:34 simply by adjusting tile size**, while Fractal Zoomer's cross-tool result changes by roughly 4× when boundary tracing is enabled. A benchmark that does not record tile size, pixel-skipping mode and cache state is not trustworthy. citeturn17view7turn20view0

### Evidence standard for the marketing claim

The strongest defensible wording would not be “deepest fractal ever” or “100× faster.” Kalles historically advertised “100 times faster than commercial programs,” but the current site presents that as the original author's claim rather than as a modern reproducible benchmark. citeturn17view0

A materially stronger claim would be:

> **Fastest tested Mandelbrot renderer across our published benchmark suite, measured on identical hardware against current releases of Fractal Zoomer, FractalShark, Fraktaler 3, Kalles Fraktaler, fractalshades and leading commercial solutions, with source locations, settings, output hashes and raw timing logs published.**

To justify **“best available” rather than merely “fastest,”** the renderer should additionally meet or exceed this capability checklist:

| Area | Threshold to credibly lead |
|---|---|
| Deep-zoom algorithm | Perturbation + robust rebasing/glitch handling + modern BLA/LA |
| CPU | Beats Fractal Zoomer/Imagina-class implementations across a multi-location suite |
| GPU | Accelerates both pixels **and arbitrary-precision reference orbit**, matching or beating FractalShark |
| Practical depth | Verified render at \(\sim10^{-650452}\) or deeper with published coordinates/correctness validation |
| Precision | ≥650k practical decimal digits; arbitrary-precision backend with no normal-user depth ceiling |
| Heterogeneous compute | Simultaneous CPU + GPU; multi-GPU; automatic hardware/number-type tuning |
| Hardware coverage | NVIDIA + AMD + modern Apple/portable GPU path rather than CUDA-only |
| Still output | Streamed/tiled arbitrary-size architecture; PNG/JPEG/TIFF/EXR; rich metadata and raw scientific channels |
| Production stress test | Demonstrated ≥100k×100k tiled render with restart/resume |
| AA | ≥64 spp or superior adaptive equivalent, jitter and linear-light filtering |
| Video | Integrated keyframe timeline, 4K60, 8K sequence output, modern codecs, motion blur/temporal AA |
| Creative tools | Layers, gradients, programmable coloring, distance-estimation/normal/lighting effects |
| Exploration | <100 ms ordinary preview target, progressive deep refinement, Newton/Feature Finder |
| Automation | CLI + documented scripting/API + resumable batch queue |
| Formula extensibility | User formulas compilable to CPU/GPU and able to opt into perturbation/BLA where mathematically valid |
| Distributed rendering | Local multi-device plus remote-worker/cluster rendering |
| Reproducibility | Complete project metadata, deterministic renders, benchmark logs and output hashes |

The decisive competitive opportunity is therefore **not another renderer that merely reaches an enormous exponent**. That problem has already been pushed to extraordinary levels by tools such as FractalShark, while Kalles demonstrates that theoretical precision ceilings can be vastly greater still. citeturn16view0turn17view1

The strongest new product would instead unify the currently fragmented frontier: **FractalShark's GPU high-precision engine, Fractal Zoomer's BLA and CPU optimization, Fraktaler 3's heterogeneous autotuning, leading commercial solutions's polished creative/animation environment, fractalshades' programmable production pipeline, and Kalles' raw-data/interchange ecosystem**. On that standard, the clearest technical whitespace in 2026 is a polished, cross-vendor, CPU/GPU/cluster renderer that accelerates the high-precision reference orbit as well as pixels, offers a genuine professional timeline and high-bit-depth output pipeline, and ships with a public reproducible benchmark suite. citeturn16view0turn18view3turn17view6turn17view3turn19search2turn20view2