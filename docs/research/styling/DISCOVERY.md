# Styling discovery and viewer controls

The broad numerical search did not establish a match for the owner's references.
The owner steered toward direct control: meaningful primitives, diverse presets,
and sliders in the existing viewer. The native styling workspace uses the shared
`fd-shade::style::render::StyleRenderer`; there is no second production shader,
Python renderer backend, browser application, or extra Mandelbrot recurrence.

## Negative experiment

`tools/research/looks/style_search.py` is an offline research prototype. It uses
already-installed NumPy/SciPy/Pillow and real `.fds` samples, with deterministic
seeds, input hashes, per-trial parameters, bounded streamline/horizon probes and
contact sheets. Freeze or delete it if any experimental maths is ported.

Three completed batches evaluated 3,840 variants (94,371,840 trial pixels):

* First valley batch: 1,280 variants, 42.996 seconds including finalists.
* Revised valley batch: 1,280 variants, 44.365 seconds including finalists.
* Revised whole-set batch: 1,280 variants, 57.608 seconds including finalists.

These are CPU research times, not GPU throughput measurements. Fifty variants
across ten related construction families are not fifty independent discoveries.
The distribution score is not reference similarity. Its preferred results could
still be visibly flat, embossed, or covered in cyan. Small previews also lose fine
structure, so their rankings are unreliable at full resolution. The first batch's
violet coverage and softened height field were rejected by visual inspection.

Full outputs/manifests remain under `/home/juno/.fdv/fractodactyl/style-search/`.
The first report/contact sheets are retained in `rejected-v1/`; the revised valley
contact sheets are in `second-pass-valley/`. `valley/` and `whole/` contain the
completed per-view galleries. Do not treat any of them as approved presets.

## Deterministic building blocks

`looks/styles/workbench.style` is the one material graph. Its six presets are
parameter maps in `workbench-presets.json`, not copies of the shading program.
They combine footprint-filtered continuous-escape folds and creases, a signed
pseudo-slope, directional illumination, groove darkening, colored surface sheen,
separate cool backlight and warm boundary emission, and optional curved spreading.
This is stylized 2D relief; it does not recover actual depth or scattering.

`stream IMAGE VECTOR_IMAGE RADIUS_IMAGE` is a typed graph primitive in the shared
WGSL image kernel. It takes six midpoint steps in each direction, evaluates the
vector field along the path, and averages with fixed positive weights. Radius is
bounded to 24 output pixels. Its declared support is 26 pixels, including midpoint
and bilinear guide sampling. Zero radius is identity. It is stateless convolution,
not advection. Uniform/time changes retain pipelines and buffers.

The emission seed depends on orientation and lighting, not just the scalar whose
contours define transport. Convolving an unchanged scalar along its own contours
would not create new structure. The final escaped mask protects fully masked pixels
from transported light; mixed edge pixels retain sample coverage. Unresolved is
displayed black in this workspace and is not a mathematical interior claim.

The distance column is already in output pixels. Dividing it by fractal pixel pitch
again would be wrong. The offline prototype and shared renderer use those units.

## Native workspace

The workspace is a Dioxus component in `fd-viewer`, available from the desktop
explorer's Options → Styling workspace menu. It may also be launched directly with
`fd-viewer --style --samples INPUT.fds`. The official manual documents the controls.
The workspace currently uses stored samples, independently of the explorer camera.

One worker owns a persistent renderer and coalesces queued slider requests. The UI
discards stale responses. Presets are uniform updates; samples upload once per
loaded file. Pin/compare, reset, look export/import and PNG saving support visual
exploration. Saved looks are ordinary standalone `.style` graphs for the CLI.

This is an exploratory tool, not a claim that the material matches the references.
The remaining aesthetic question is whether controlled shape, lighting and emission
are sufficient, and which additional graph primitives real exploration warrants.
