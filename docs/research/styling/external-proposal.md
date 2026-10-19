# Proposal: Ethereal Light Shader
## Modular, luminous, flowing fractal shading

### Objective

Create a highly expressive shader capable of producing the visual appearance shown in the two reference images: intricate Mandelbrot geometry surrounded by swirling, luminous, ethereal light, electric filaments, iridescent surfaces, atmospheric depth, and brilliant star-like glints.

The shader should be capable of producing a wide variety of appearances rather than reproducing only these two images.

All effects operate on existing fractal data. **The underlying mathematical geometry remains unchanged.**

## 1. Visual capabilities

### A. Luminous boundary lighting

Generate bright, finely controlled illumination along fractal boundaries and filaments.

- Narrow, intensely luminous edge highlights
- Golden, white, cyan, violet, or multicolored emission
- Brightness varying with local geometric structure
- Sharp incandescent filaments surrounded by soft glow
- Multiple independently colored emission bands
- Adjustable light width, falloff, intensity, and contrast

Light should naturally accentuate the fractal's real structures rather than appearing as a uniform outline.

### B. Ethereal swirling light

Generate smooth, flowing, semi-transparent light fields that appear to curl around and through fractal structures.

- Flow guided by fractal-derived normal and tangent fields
- Curling ribbons and spiral-shaped illumination
- Multiple overlapping currents moving at different speeds
- Smooth color transitions within flowing fields
- Adjustable turbulence, distortion, softness, and direction
- Flowing light that can spread beyond the brightest boundaries

This is the shader's signature effect.

The appearance should resemble luminous silk, flowing plasma, auroras, or translucent energy.

### C. Glints and sparkling highlights

Create small, brilliant flashes at selected locations in the fractal's detail.

- Point-like highlights with intense central brightness
- Four-point, six-point, and anisotropic starbursts
- Subtle diffraction streaks
- Color-dependent flares
- Adjustable rarity, scale, threshold, and intensity
- Temporal shimmer and occasional flashes

Glints must be spatially deterministic and attached to stable image features. They should not randomly jump between positions from frame to frame.

### D. Iridescent and pearlescent surfaces

Produce richly colored, sculpted surfaces resembling illuminated minerals, shells, metallic films, or translucent materials.

- Multi-hue interference-like gradients
- Angle-dependent color variation
- Colored specular highlights
- Fine surface striations
- Smooth transitions between glossy and matte regions
- Local contrast and apparent relief

Surface shading should support both delicate, naturalistic appearances and extreme psychedelic coloration.

### E. Atmospheric light and bloom

Give emitted light a sense of depth and space.

- Multiple bloom radii
- Fine glow around tiny filaments
- Broad halos around luminous formations
- Colored haze and soft diffusion
- Directional light scattering
- Controlled highlight roll-off
- High-dynamic-range accumulation before tone mapping

A small bright feature might have a razor-sharp center, a colored surrounding glow, and a much larger barely visible halo.

### F. Apparent depth and topography

Create the impression of looking down at a richly detailed three-dimensional landscape.

- Directional relief lighting
- Adjustable apparent surface steepness
- Ambient-style shading
- Highlight and shadow shaping
- Terraced or smoothly sculpted surfaces
- Fine structural contrast
- Depth-like atmospheric separation

These are appearance effects derived from the available mathematical fields, not modifications to the Mandelbrot geometry or claims of actual three-dimensional geometry.

## 2. Modular shader primitives

The shader must not be a fixed sequence of hardcoded effects.

Instead, it should expose a small collection of composable operations.

| Category | Primitives |
|---|---|
| Sources | Smooth iteration, distance estimate, normal, interior mask, screen coordinates, time |
| Signal shaping | Remap, curves, smoothstep, threshold, clamp, power, repeat, invert |
| Procedural fields | Noise, fractal noise, turbulence, periodic waves, radial and angular fields |
| Spatial effects | Warp, tangent-directed displacement, vector-field advection, directional blur |
| Color | Multistop gradient, color mix, hue transform, iridescence |
| Illumination | Relief light, rim light, emission, specular response |
| Optical effects | Bloom, halos, glints, streaks |
| Combining | Add, multiply, screen, max, alpha mask, layered compositing |
| Animation | Time, oscillators, phase, speed, modulation curves |

Every compatible output should be usable as an input or modulation signal elsewhere in the shader.

For example:

**Fractal normal → Tangent field → Flow distortion → Blue emission → Bloom**

Simultaneously:

**Distance estimate → Boundary mask → Gold emission → Glint selection**

Both results can then be composited over a dark iridescent surface.

A signal can also control another operation's intensity, width, hue, speed, or direction.

The same primitives should permit substantially different styles without introducing a new hardcoded shader for every appearance.

## 3. Reference appearance: Celestial Currents

The two supplied images provide a concrete first target.

The initial shader configuration should reproduce their principal characteristics:

**Base surface**
- Nearly black fractal interiors
- Deep midnight-blue and violet surroundings
- Richly sculpted fractal detail

**Boundary illumination**
- White-gold incandescent edges
- Fine illuminated branching structures
- Selectively intense intersections and narrow filaments

**Flowing light**
- Cyan and blue currents wrapping around spirals
- Violet and lavender translucent streaks
- Gold light flowing through selected channels
- Smooth, sweeping transitions between colors

**Optical effects**
- Fine localized sparkles
- Subtle elongated glints
- Soft multiscale bloom
- Atmospheric light shafts and broad luminous gradients

**Surface treatment**
- Pearlescent highlights
- Iridescent blue-violet-gold reflections
- Fine, closely packed relief lines
- Deep shadows with preserved structural detail

The goal is not to paint over the fractal with arbitrary decoration. The light should appear to originate from and respond to the underlying mathematical structures.

## 4. Animation

Every meaningful appearance parameter should be animatable.

In particular:

- Light currents travel along stable fractal-derived paths.
- Colors shift gradually through gradients.
- Emission intensity pulses or ripples.
- Glints brighten and fade.
- Iridescent highlights shimmer.
- Atmospheric halos expand, contract, or drift.
- Multiple modulation signals create complex but controlled motion.

Animation must preserve spatial coherence. Motion should resemble light traveling across a stable fractal, not randomly changing textures.

Procedural randomness must be deterministic across frames.

## 5. Implementation constraints

The implementation should remain confined to the shading subsystem.

- Consume existing sample channels wherever possible.
- Derive additional appearance signals in GPU shading passes.
- Do not modify fractal positions, membership, or iteration results.
- Use WGSL and the existing wgpu architecture.
- Compile connected operations into an efficient execution graph.
- Support multipass operations where required for blur, bloom, or advection.
- Process large images in tiles with correct overlap for effects crossing tile boundaries.
- Maintain consistent appearance between stills and movies.
- Avoid visible seams, color discontinuities, aliasing, and temporal flicker.
- Preserve the existing looks and their output as backward-compatible options.

A node may require neighboring pixels or earlier passes. Those dependencies should be explicit rather than assuming every effect can run independently on one pixel.

## 6. Acceptance criteria

The shader is successful when it can:

1. Produce both reference appearances convincingly from genuine fractal samples.
2. Generate bright, fine boundary illumination without destroying detail.
3. Create recognizable swirling fields of light following fractal structures.
4. Render selective, visually convincing glints and soft multiscale bloom.
5. Produce rich iridescent surfaces and strong apparent depth.
6. Animate the effects smoothly without random flicker or jumping features.
7. Produce noticeably different appearances by recombining existing primitives.
8. Render high-resolution stills and video without introducing image seams or unstable artifacts.

## 7. Recommended development sequence

**Stage 1 — Light foundation**

Introduce HDR emission, boundary masks, independently colored emission fields, compositing, and multiscale bloom.

**Stage 2 — Flowing light**

Introduce normal/tangent-derived vector fields, signal warping, directional blur, and controlled advection to produce the characteristic swirling ethereal illumination.

**Stage 3 — Optical detail**

Add stable glint selection, starburst shapes, diffraction-style streaks, and iridescent surface responses.

**Stage 4 — Modulation**

Allow shader primitives and their parameters to drive one another, including time-dependent signals, while maintaining predictable and reproducible results.

**Stage 5 — Reference matching**

Tune and validate the shader against both supplied images, then create several dramatically different looks using the same primitives.

---

## Intended result

One reusable, modular appearance shader capable of producing everything from subtle illuminated topography to spectacular cosmic fractal lightscapes.

The shader should make the fractal appear simultaneously mathematically intricate, three-dimensional, luminous, fluid, and alive.

**The defining effect is light that seems to flow through, curl around, and emerge from the fractal's real geometry.**