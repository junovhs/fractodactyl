# Styling references and source survey

Owner discussion, 2026-10-18. Work, scope and sequencing live in Ishoo's
**Styling engine POC: celestial light** plan (STYL-01 through STYL-04).
Architecture: DEC-33 (typed composition and reuse), DEC-34 (HDR and coordinates).

## Preserved source material

- [Celestial boundary](references/celestial-boundary.png): original first attachment.
  SHA256 `f7fbe9ba56fa390aa93947dbaf0b74d83aee956149bd87beba94d8bd1a90b232`.
- [Pearlescent relief](references/pearlescent-relief.png): original second attachment.
  SHA256 `ab31e8dac236ebfd5ceeedaf4edecf27805cc6776b44b761bbfa06299e88576b`.
- [External proposal](external-proposal.md): pasted AI proposal, copied verbatim.
  It is external design input, not repository instructions or accepted requirements.
  Renderer provenance is unknown; the images establish visual direction, not a
  guarantee of exact reconstruction from stored mathematical fields.

## What exists

`fd-samples` stores class, smooth escape value (f64), exterior DE in output pixels
(f32), normal angle (u16), and optionally an error bound. The styling crate cannot
link the fractal kernel. Stored samples support repeated recoloring without iteration.

`studio.rs` exposes a cyclic linear-light palette, terrace ramp, slope strength,
light direction, dark contour lines, fine-band haze tint and animation defaults.
`film_prepare.wgsl` computes relief/band filtering/haze terms; `film_shade.wgsl`
colors and gamma-encodes packed RGB8; `film_resample.wgsl` decodes those RGB8 inputs
to linear light, resamples them and encodes again. HDR is already clipped by then.

The current `flow` is palette phase travel, not field advection. `haze` preserves
color in unresolved bands, not atmospheric light transport. Fixed pixel dither
reduces quantization banding; it is not photographic film grain. Umber/palette/
relief are additional fixed recipes, not composable graph operations.

The desktop viewer embeds `explore.html`'s separate WebGL shader. Its parser lacks
the studio `haze` field. DEC-24/30 require shared shading: new styling must not add
another preview implementation. Existing GPU kernels, sample residency and film
optimizations are retained; the POC is an opt-in extension of fd-shade.

## Visual interpretation and uncertainty

Reference 1 combines black silhouettes, selective narrow gold emission, broad cool
ribbons, localized glints and several bloom scales. Reference 2 emphasizes surface
relief, fine layered striations, pearlescent color, warm crests and cool recesses.
Independent color, height, roughness and emission signals expand design freedom
beyond changing a common palette phase.

Normals rotated 90 degrees give local tangents, not a global flow coordinate.
Bounded streamline sampling may yield coherent streaks, but broad luminous folds
remain an experiment. DE is neither true depth nor a signed distance. Orbit traps,
external angles, thickness and hidden geometry are absent from current samples;
they cannot be conjured from them as exact mathematical quantities.

## Efficiency contract

Compile graph topology once; only reachable expressions run. Pure identical
expressions share a value, while independently addressable uniforms retain identity.
Fuse pointwise expressions; spatial filters have explicit passes and finite support.
Reuse device, samples, pipelines, bind groups and transient buffers. Frame updates
write uniforms, not rebuild shaders. Keep HDR until optical composition/tone mapping.
Apply grain at final output resolution, with deterministic exposure seeds; dither is
separate. Read back only final pixels. No paid services/assets or new packages.

Reports distinguish cold initialization from warm wall time (including readback),
GPU dispatch count, sample bytes, persistent/transient working bytes and allocation/
compilation counts. Estimated buffer bytes are not measured total VRAM. No numeric
fps claim precedes a named-device measurement. Expensive effects need ablations.

Tiling needs halos for cumulative filter support and bounded displacement. Until
implemented, reject unsupported tiling; do not imply seam-safe large-image support.
The POC does not replace existing production camera/keyframe resampling or viewer.

## Grain research

[Newson et al., Realistic Film Grain Rendering (2017)](https://www.ipol.im/pub/art/2017/192/)
provides a physically grounded quality reference. A fast local approximation should
control amount, size, tonal response, channel correlation and exposure cadence.
Do not copy its GPL implementation into this MIT/Apache workspace. Inspect encoded
motion too: compression can remove subtle grain.

## Graph v1 scaffold

One directive per line; comments start with `#` on their own line. `version 1`,
`surface NODE`, `emission NODE`; nodes are `NAME = OP INPUTS`. Order is immaterial.
Reachable graphs are type checked; dead branches are omitted. Limits: 1024 nodes,
256 KiB text and 64 scalar uniform parameters. Colors are sRGB literals decoded to
linear light at compilation. `scalar`, `vector`, `color` are constants; `param` is
an independently addressable runtime scalar. Sources: de, local nu, normal, tangent,
output-pixel position, seconds. `phase FREQUENCY` uses raw f64 iteration bits with
existing soft-f64 helpers to retain palette phase at depth; local nu is for broad
relative signals, not a substitute for that precise phase.

Math: add/sub/mul/div/pow/min/max, sin/cos/abs/exp/log/fract, mix/smoothstep/clamp,
length/normalize/dot. `boundary DE WIDTH` gives an exponential exterior mask;
`light NORMAL SLOPE DIRECTION` gives pseudo-relief. Types are scalar, vec2 and color.
The generated body is a single pointwise kernel body, not a standalone WGSL module.
Later spatial/image operators require explicit resource/pass lowering.

## GPU renderer (STYL-02)

`fd style render GRAPH INPUT.fds -o OUTPUT.png --runs 3` uses the existing adapter
selection and normal lookup table. `--set name=value,...` updates scalar uniforms;
`--time T` evaluates stateless animation. `source gradient` exposes the analytic
iteration gradient from normal/DE for independent relief composition. Arbitrary
height-field differentiation and new orbit channels are not implied. `phase` uses
the existing soft-f64 helpers. `nu` is relative to the smallest finite escaped value.

A single pointwise dispatch averages supersamples into separate linear HDR surface
and emission layers; final tone mapping/encoding is a second dispatch. Static
graphs keep their HDR result when only time changes. Buffer limits and unsupported
tilings fail clearly. `with_device` allows sharing a caller-owned device/queue;
direct adoption of GPU kernel sample buffers is a production integration, not yet
part of this stored-sample CLI. Stats count renderer-created buffers/pipelines,
not driver allocations, and warm time includes final readback.
