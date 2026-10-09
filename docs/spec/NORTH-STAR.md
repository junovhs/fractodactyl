# North Star

Fractodactyl should feel like an attempt to make the impossible cheap.

The goal is not simply to render Mandelbrot fractals faster. It is to rethink the problem until genuinely extreme depth becomes ordinary enough that we can do things with it that currently feel unreasonable: very long, beautiful deep-zoom films, enormous resolutions, rapid experimentation, and eventually perhaps an interactive experience where going deeper does not immediately make the computer grind to a halt.

The fractal should remain real. We should not fake depth by swapping in another location just because it looks similar. If the camera travels somewhere, that is the actual Mandelbrot set at that location and depth.

The project should aggressively exploit one fact: we usually know far more about what the viewer is going to need than a general-purpose fractal renderer does.

For a movie, we know the path in advance. We know what will be on screen hundreds or thousands of frames from now. We know when a tiny feature is still being crushed into two or three pixels, and we know approximately when it will become large enough for its internal detail to matter.

That means Fractodactyl should work ahead.

If something is currently too small for the screen to resolve, do not waste enormous amounts of computation revealing invisible detail. Use the cheapest mathematically valid representation that can produce the pixels we actually need. Before that structure grows large enough to expose more detail, prepare the next level.

In other words: calculate detail before it becomes visible, not long before and not after.

The Google Maps analogy is central to the spirit of the project.

Google Maps does not hold the entire Earth at maximum resolution in memory. It keeps a small useful working set, uses different levels of detail, predicts what will be needed next, reuses previously prepared information, and makes a gigantic world feel cheap to explore.

Fractodactyl should attempt the mathematical equivalent.

But instead of primarily caching pictures, it should cache useful pieces of computation: local coordinate systems, reference orbits, shortcuts through long stretches of Mandelbrot behavior, proven error bounds, and anything else that lets future pixels avoid repeating old work.

A few gigabytes on disk is completely acceptable. Wasteful computation is not.

The atlas should therefore be treated as a compression problem:

**How much genuine Mandelbrot behavior can we represent with the smallest, cheapest, most reusable amount of information?**

A tile that represents millions of ordinary calculations in a few kilobytes is enormously more interesting than simply throwing more GPU power at those millions of calculations.

Hardware friendliness matters too. The smallest representation is not automatically the best representation. Data should be shaped so CPUs and GPUs can consume it quickly and predictably. Storage format, tile size, memory layout, scheduling, precision, and rendering should all be considered part of the same problem.

Fractodactyl should borrow shamelessly from other fields.

Maps. Game engines. Virtual textures. Compilers. Video codecs. Numerical analysis. Scientific computing. GPU architecture. Caching systems. Compression. Dynamical systems.

If another discipline solved an analogous problem under a different name, we should steal the principle and see what happens when it is applied to Mandelbrot rendering.

At the same time, the project should not become trapped by existing fractal-rendering conventions. Prior art is a floor, not the destination. The interesting question is always whether combining these ideas in a new way creates a capability that existing renderers do not have.

The project should prefer experiments that can surprise us.

If a strange idea might turn three million iterations into a tiny reusable operator, test it.

If choosing a slightly different but equally beautiful genuine zoom path cuts computation by 100×, choose it.

If arranging sample points differently lets future frames reuse yesterday’s work, investigate it.

If a mathematical shortcut works beautifully in only 20% of the Mandelbrot set, that can still be valuable. Fractodactyl does not need one universal trick. It can become a system that recognizes what kind of region it is looking at and chooses the cheapest valid strategy.

How we pursue this matters as much as what we pursue. Rendering fractals is slow, so the research loop must not be rendering fractals. Every idea earns an expensive run only by first winning a probe that takes seconds, scored against frozen truth and against the best rival that could use the same trick without us. We kill ideas fast, write the answers down, and look for prior art before inventing. The working rules are in docs/spec/METHOD.md.

The long-term dream is simple:

**Going deeper should stop feeling expensive.**

Not because depth has become fake, and not because infinite complexity somehow disappeared, but because Fractodactyl learned how to reuse, compress, predict, postpone, precompute, and compile the work intelligently.

If we succeed, the payoff should be visible rather than merely academic.

We should be able to make Mandelbrot videos that would previously have been absurdly expensive.

We should be able to explore deeper, longer, richer paths without constantly budgeting around render cost.

And eventually, if the architecture becomes cheap enough, the distinction between “pre-rendered deep zoom” and “interactive deep exploration” may begin to disappear.

That is the spirit of Fractodactyl:

**Keep the fractal real. Make the computation clever. Push depth until it stops being the limiting factor.**
