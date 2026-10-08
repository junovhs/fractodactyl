"""Look prototypes (KF-style slope shading + crisp cyclic palettes), rendered from stored
.fds samples so many variants can be compared in seconds before porting one to fd-shade.

Model (per sample, then averaged over ss x ss in linear light):
- palette index t = density * nu + phase (linear in nu, as KF; log-nu bands were far too
  sparse at depth: nu spans only ~0.24 log units across a 2.7e-38 frame);
- slope light from the analytic gradient grad nu = -(2 / (de ln 2)) * normal (checked
  against finite differences: cos -1.000, ratio 1.000), in palette cycles per pixel:
  s = density * |grad nu|; shade = 1 + strength * dir_term * (2/pi) atan(s / s0)
  with dir_term = dot(uphill direction, light direction): ridges facing the light are
  brighter, the far side darker (KF "slope shading");
- crevice darkening near the set from de (dark seams between filaments);
- interior: flat colour (or a glossy gradient later).
Usage: python proto.py FDS NX NY SS OUT_PREFIX [crop]
"""
import sys, numpy as np
from PIL import Image
import fds

def hexrgb(h):
    h = h.lstrip('#'); return np.array([int(h[i:i + 2], 16) / 255 for i in (0, 2, 4)])

def lin(c): return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)

PALETTES = {
    # image 5: navy -> royal -> sky -> white -> warm grey -> slate
    'ice': ['#08152e', '#0b3d91', '#1e7fe0', '#7cc4ff', '#eef6fc', '#c9c3bc', '#7d7a78', '#2a3140'],
    # image 2: black/grey/white ribs with indigo
    'steel': ['#0a0a0c', '#2e2e34', '#76767e', '#d9d9de', '#ffffff', '#aeb2d4', '#4d5299', '#191b3a'],
    # image 3: zebra white/black with blue + teal accents
    'zebra': ['#f4f8ff', '#0b0f19', '#5b9dff', '#f4f8ff', '#12222c', '#22d1c3', '#dfe9f7', '#0a1220'],
    # owner's image 6: coral, salmon, sand, pale cyan, sky, teal, deep teal (terrain lines)
    'coral': ['#e2553c', '#f2845a', '#f7d58c', '#bfe8e4', '#7cc9dc', '#1f8fb3', '#0f5e87', '#c9483a'],
    # image 4: smoky greys (pair with a blue interior)
    'smoke': ['#1e1e1f', '#4a4744', '#8f8a84', '#d9d4ce', '#f2eee9', '#a39d96', '#5c5854', '#2a2928'],
}

def palette_lookup(stops, t):
    """Smooth cyclic interpolation (cosine-eased) through linear-light stops."""
    st = np.array([lin(hexrgb(s)) for s in stops]); n = len(st)
    x = (t % 1.0) * n; i = np.floor(x).astype(int) % n; f = x - np.floor(x)
    f = 0.5 - 0.5 * np.cos(np.pi * f)
    return st[i] * (1 - f[..., None]) + st[(i + 1) % n] * f[..., None]

def render(path, nx, ny, ss, pal='ice', density=0.05, phase=0.0, strength=0.9, s0=0.25,
           light_deg=135.0, crevice=0.6, interior='#05070d', aa=0.7, terrace=0.0, tdensity=None,
           steep_mode='atan', lines=0.0, line_px=1.3, line_space=4.0):
    k, nu, de, ux, uy = fds.load(path, nx, ny)
    t = density * nu + phase
    col = palette_lookup(PALETTES[pal], t)
    if terrace:
        # Sawtooth relief per terrace band: bright at the band's uphill edge, darkening
        # toward its downhill edge, then a crisp step: stacked scales / contour terraces.
        td = tdensity or density * len(PALETTES[pal])
        f = (td * nu + phase) % 1.0
        ramp = 1.0 - terrace * (1.0 - f) ** 1.5
        col = col * ramp[..., None]
    g = 2.0 / (np.maximum(de, 1e-30) * np.log(2))          # |grad nu| per output px
    if lines:
        # Contour ("terrain") lines on every band edge, a constant width in pixels: the
        # distance to the nearest edge is |frac| / (bands per px). They fade where edges
        # crowd closer than line_space px, so dense areas do not turn to mush.
        td = density * len(PALETTES[pal])
        c = td * nu + phase * len(PALETTES[pal])
        gc = td * g
        dpx = np.abs(c - np.round(c)) / np.maximum(gc, 1e-30)
        half = line_px / 2
        line = np.clip((half + 0.5 - dpx) / 1.0, 0, 1)
        vis = np.clip((1 / np.maximum(gc, 1e-30) - line_space) / line_space, 0, 1)
    s = density * g                                          # palette cycles per px
    a = np.radians(light_deg)
    lx, ly = np.cos(a), -np.sin(a)                           # screen y down; 135 deg = from upper left
    up_x, up_y = -ux, -uy                                    # uphill (increasing nu) direction
    dirt = up_x * lx + up_y * ly                             # facing the light: +1
    if steep_mode == 'atan':
        steep = (2 / np.pi) * np.arctan(s / s0)
    else:  # log: strong everywhere, still saturating near the set
        steep = np.tanh(np.log1p(s / s0) * 0.6)
    # Sub-sample filaments: their normals are noise; fade the light there (FX-01 aa).
    fade = np.tanh(np.maximum(de, 0) * ss / aa) if aa else 1.0
    shade = 1.0 + strength * dirt * steep * fade
    seam = 1.0 - crevice * np.exp(-np.maximum(de, 0) * ss * 1.5)
    out = col * (shade * seam)[..., None]
    if lines:
        out = out * (1 - lines * line * vis)[..., None]
    out[k == 1] = lin(hexrgb(interior)); out[k == 2] = lin(hexrgb(interior))
    out = out.reshape(ny // ss, ss, nx // ss, ss, 3).mean(axis=(1, 3))
    return (np.clip(out, 0, 1) ** (1 / 2.2) * 255 + 0.5).astype(np.uint8)

VARIANTS = {
    'coral_bold': dict(pal='coral', density=0.05, terrace=0.4, strength=1.0, steep_mode='log', s0=0.02, lines=1.0, line_px=2.2, line_space=3.0),
    'ice_bold': dict(pal='ice', density=0.05, terrace=0.5, strength=1.1, steep_mode='log', s0=0.02, lines=1.0, line_px=2.2, line_space=3.0),
    'ice_lines': dict(pal='ice', density=0.04, terrace=0.55, strength=1.1, steep_mode='log', s0=0.02, lines=0.85),
    'coral_lines': dict(pal='coral', density=0.04, terrace=0.35, strength=1.0, steep_mode='log', s0=0.02, lines=0.9),
    'ice_lines_dense': dict(pal='ice', density=0.07, terrace=0.45, strength=1.1, steep_mode='log', s0=0.02, lines=0.85),
    'coral_lines_dense': dict(pal='coral', density=0.07, terrace=0.3, strength=1.0, steep_mode='log', s0=0.02, lines=0.9),
    'ice_terrace': dict(pal='ice', density=0.04, terrace=0.55, strength=1.1, steep_mode='log', s0=0.02),
    'steel_terrace': dict(pal='steel', density=0.025, terrace=0.45, strength=1.1, steep_mode='log', s0=0.02),
    'zebra_strong': dict(pal='zebra', density=0.08, strength=1.1, steep_mode='log', s0=0.02),
    'smoke_terrace': dict(pal='smoke', density=0.03, terrace=0.5, strength=1.1, steep_mode='log', s0=0.02, interior='#0a2a8a'),
}

if __name__ == '__main__':
    path, nx, ny, ss, out = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4]), sys.argv[5]
    names = sys.argv[6].split(',') if len(sys.argv) > 6 else list(VARIANTS)
    for name in names:
        Image.fromarray(render(path, nx, ny, ss, **VARIANTS[name])).save(f'{out}{name}.png')
        print('wrote', f'{out}{name}.png')
