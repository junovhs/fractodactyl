# Shaded-sample reconstruction (STYL-07)

The original rose-gold preview reduced 2×2 mathematical samples to each 1080p
pixel before bilinearly enlarging those HDR pixels to 1.3×. It could not recover
sample detail discarded before the zoom. Its narrow specular highlights also
needed more samples than its filtered periodic bands.

`StyleRenderer::set_reconstruction` projects each final output pixel's tent
footprint directly onto the stored sample grid. It shades each contributing
sample through the full graph and combines shaded linear-HDR values, rather
than averaging escape counts or normals before nonlinear lighting. The same
footprint/weight helpers serve image and surface/emission graphs. Image passes,
including curved mist, run after reconstruction. Camera magnification converts
DE to final output pixels; band and sample-light filtering retain the underlying
sample footprint. The tent radius has a one-source-sample floor, preserving
constants even when a low-resolution grid is magnified at a fractional pivot.

Omitted camera flags retain the existing box resolve. `--zoom 1` enables quality
reconstruction without motion; `--zoom-end` interpolates exponential
magnification over the sequence, including both endpoints. The styling workspace
uses reconstruction at 1×. `--style-look` opens an existing workspace graph at
launch without replacing it with today's bundled graph. Graph image `zoom`
remains an explicitly separate bilinear operation; do not use it to reproduce
this camera motion after the source resolve.

## Probe evidence

Source camera: re=-0.7453, im=0.1127, width=0.0065, 1920×1080, 5000 iterations,
rotation 0. Original saved graph SHA256:
`e2be1d9b964310e4a4ebf6d67f39056467df8bacc468bff66af42f7c99556ab9`.
Only the preview copy adds 0.06 cycles/s phase motion and scales pixel-sized
controls from the 1024-pixel workspace to 1920 pixels.

Independent 320×320 mathematical crop renders used 8×8 samples at the same
output pixel pitch, including a 32-pixel halo. Comparisons use the central
256×256 pixels. The endpoint reference is newly iterated at the actual 1.3×
camera, rather than enlarged from the initial view. Metrics below are average
absolute errors of decoded PNG RGB8 channels, not full-frame equivalence or a
converged infinite-sampling reference.

| Dense crop | Previous preview | Reconstruction, 4×4 source | Reconstruction, 8×8 source |
|---|---:|---:|---:|
| Start (1532,64), phase 0 | 20.05 | 6.92 | 0.56 |
| End (462,752), phase 0.6 | 13.32 | 7.08 | 2.97 |

Start reference camera: original pixel region (1500,32), size 320×320.
End reference camera: final pixel region (430,720), size 320×320.
The 4×4 probe took 34.4 seconds of CPU math; the 8×8 full source took 138.7
seconds, two CPU threads. On Radeon RX 5700 XT, reconstructed 8×8 warm probe
frames took 92.6–104.6 ms, four dispatches, one sample upload, ten explicit
allocations and 2,372,724,224 estimated GPU buffer bytes. This is an offline
quality setting, not a 60-fps live-rendering or rival-speed claim. Stencil fusion
trades repeated neighboring material evaluation for avoiding multi-gigabyte
supersampled intermediate images; retained resources are unchanged across time
and camera uniform updates.

Artifacts and recipe are under `/home/juno/.fdv/fractodactyl/style-quality/` on
the authoring host. The user's original style and original clip are preserved.
The replacement clip is encoded separately. Sample-grid magnification remains
bounded to 1–2×; adequate source sampling is still required. It cannot reveal
mathematical detail absent from the grid. This is a correction to reconstruction,
not a new fractal kernel or a claim of outperforming competing renderers.

## Regression coverage

A synthetic opposing-normal, power-37 highlight is checked against an independent
CPU integration of shaded-sample energy at 1.3×. Averaging normals before shading
would fail this test. Image and surface/emission routes agree, disabling
reconstruction restores legacy pixels, invalid camera uniforms are rejected,
unchanged frames reuse HDR and resource counts stay fixed. CLI sequence midpoint
replay checks exponential zoom with time, exposure, grain and bloom. Workspace
checks retain six distinct presets, all 21 effective sliders, saved graph replay,
unchanged sample input and darkness where the entire tent footprint is masked.
