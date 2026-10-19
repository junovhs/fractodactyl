# Connected styling engine POC

The work and acceptance contract live in Ishoo STYL-01 through STYL-04; DEC-33
and DEC-34 govern reuse and HDR. These notes describe the implemented architecture.
The references and externally supplied proposal are preserved beside this file.

## Two connected levels

A `.style` file is a typed dependency graph. Definitions may appear in any textual
order. Wires determine evaluation order; changing the wires changes the program.
Scalar signals, 2D vectors and linear colors can modulate compatible field inputs.
`param` nodes are named runtime uniforms, distinct even when defaults are equal.

`image FIELD` crosses from a sample field to a scalar/vector/color HDR image.
`output IMAGE` chooses a connected image program instead of the compatibility
`surface FIELD` / `emission FIELD` route. Image controls are typed images too:
spatial or time-varying signals can drive masks, warp distances and blur widths.
Global scalar/vector controls occupy one value, not a full raster.

```text
height_image = image height_field
light_image = image sun_vector
albedo_image = image albedo
vector_image = image tangent
control_image = image animated_mask
warped = warp albedo_image vector_image control_image
lit = relight warped height_image light_image
halo = blur emission_image control_image
final = add lit halo
output final
```

The same `animated_mask` can control several effects. Replacing the two middle
operations with `prelit = relight albedo_image height_image light_image` and
`lit = warp prelit vector_image control_image` gives different pixels. Saved examples:
`looks/styles/modular-light.style` and `modular-light-swapped.style`.

## Implemented vocabulary

- Sources: relative smooth iteration, precise iteration phase, DE, analytic gradient,
  normal, tangent, output-pixel position, time and escaped/interior/unresolved masks.
- Constants/uniforms: `scalar`, literal `vector`, `color #rrggbb`, `param`; `compose`
  connects two scalar signals into a vector.
- Field shaping: add/subtract/multiply/safe divide, power, min/max, sin/cos, abs,
  exp/log, fract, mix, smoothstep, clamp, length, normalize, dot.
- Structure: `boundary DE WIDTH`, antialiased `band FREQUENCY PHASE`, analytic
  `light NORMAL SLOPE DIRECTION`; experimental bounded field `flow`.
- Image operations: add/multiply/max, mask-driven mix, warp, blur and height-driven
  relight. Inputs and outputs carry scalar/vector/color image types.

Colors decode from sRGB to linear light. Tone mapping and display encoding happen
once at the output boundary. Spatial operations consume linear HDR values. The
existing `.look`/production film route remains compatible and uses its original
render pipeline; the new CLI route does not rerun any fractal iterations.

`phase` keeps the existing soft-f64 precision helpers. `band` and analytic lighting
reuse the existing studio footprint-filtering functions. `source nu` is relative
to a finite escaped origin; use precise phase for periodic detail at depth.
Ordinary spatial fields default to escaped-region masks. Explicit classification
sources let a composition deliberately style interior/unresolved regions.

## GPU work and memory

Compilation walks only reachable nodes, rejects cycles/types, and shares identical
pure field/image expressions. All reachable sample fields share one fused raster
pass. Neighborhood operations have explicit passes and support. Connected image
arithmetic fuses into kernels; fanout intermediates remain shared. Fusion respects
a seven-read-buffer limit for portable storage binding counts.

Image lifetimes assign a small transient buffer pool. A buffer is never reused
while it is an input to the current pass. The real connected example lowers to five
image passes, three transient image slots, plus field rasterization and final
encoding. Repeated frames retain device, sample uploads, pipelines, bind groups,
immutable pass uniforms and allocated buffers. Unchanged inputs reuse computed HDR
images. New graph topology compiles again; parameter/time changes update uniforms.

Within a dynamic frame, cheap static expressions feeding dynamic fields can still
be evaluated; there is no promise of a fully staged static/dynamic field cache.
Introduce further staging only with measured savings against its memory/dispatch
cost. Direct GPU-kernel buffer adoption, HDR camera/keyframe integration and viewer
integration remain future consumers of this shared engine, not replacement kernels.

Stats report renderer-created buffers/pipelines and estimated buffer bytes, not
all driver allocations or GPU timestamp duration. Frame wall time includes the
single final readback. Sample loading, initialization, first-frame shader warmup,
PNG writing and encoding are reported separately. No paid service or new package.

## Bounds and current limits

Image warp clamps displacement to eight output pixels. Image blur uses five
bilinear taps per axis and a signal-controlled radius clamped to 0..8 pixels.
It lowers to horizontal/vertical passes. Relighting differentiates a separate
height image at adjacent pixels. Support bounds accumulate through the graph;
`inspect` shows the output support and buffer plan. Whole-frame rendering only:
unsupported device buffer sizes are rejected rather than silently tiled with seams.

The field-flow experiment permits one non-nested five-tap node, bounded to eight
pixels. It is local tangent-directed sampling, not a global flow coordinate or
stateful fluid simulation. Source fields are limited to 16 raster outputs and 64
scalar uniforms; image plans to 128 passes, graphs to 1024 definitions / 256 KiB.
These are explicit POC bounds, not a finished universal material system.

The current images demonstrate composition, illumination and sculpted structure;
they do not reproduce the references' sweeping silk, selective optical glints or
full pearlescent material response. The engine has no new orbit-trap/depth data.
Procedural noise/fBm, richer gradients/curves, specular/roughness/iridescence and
feature-stable glints are useful next primitives, not secretly simulated here.
A node editor and production viewer/camera film integration are outside this POC.

Owner steering: grain is lowest priority. The basic optional implementation is
retained, defaults off, and is off in the main demo. All further grain work is parked
until the other visual capabilities are stronger.

## Reproduce

From a source build, render samples once, then inspect/reconnect styles:

```sh
fd render --re -0.7453 --im 0.1127 --width 0.0065 --size 1024x640 --ss 2 --iter 5000 --columns nu,de,normal -o valley.fds
fd style inspect looks/styles/modular-light.style
fd style render looks/styles/modular-light.style valley.fds -o connected.png --unresolved interior
fd style render looks/styles/modular-light-swapped.style valley.fds -o swapped.png --unresolved interior
fd style sequence looks/styles/modular-light.style valley.fds -o frames --frames 48 --fps 24 --unresolved interior
```

`--set transport_strength=7,height_gain=18,glow_gain=0.6` modulates named inputs
without rebuilding pipelines. Connected graphs wire blur/add themselves; `--bloom`
is a convenience on the older surface/emission compatibility graph only.

The reproducible local demo is `tools/research/looks/styling-poc.sh`, with
`FD_STYLE_BIN` pointing to the built fd, `FD_STYLE_OUTPUT` to an external artifact
folder, and the usual Cargo target/jobs settings. It reuses successfully rendered
sample inputs with matching render arguments; all styling variants share their
sample hashes. It saves inspection output, commands, per-frame costs, stills,
48 animated frames, an MP4, decoded frames and deterministic replay evidence.
