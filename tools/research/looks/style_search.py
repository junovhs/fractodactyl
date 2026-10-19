#!/usr/bin/env python3
"""Research sandbox: novel field-derived appearance hypotheses, not a renderer backend.

Uses installed numpy/scipy/Pillow only. No fractal iterations, services or downloads.
Freeze/delete this prototype if its operators are ported to the shared WGSL graph.
"""
import argparse
import hashlib
import html
import json
import math
import struct
import time
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from scipy.ndimage import gaussian_filter, map_coordinates, maximum_filter

FAMILIES = ['log-relief', 'scale-relief', 'tangent-ridges', 'curved-mist',
            'directional-mist', 'backlit-mist', 'crevice-light', 'curl-billows',
            'ridge-metal', 'layered-combo']


def resize(a, size):
    return np.array(Image.fromarray(a.astype('float32')).resize(size, Image.Resampling.BILINEAR))


def read_fields(path, width):
    """Read documented v1 columns; average valid supersamples before resizing.

    DE IS ALREADY in output pixels. Never divide it by fractal camera pitch.
    Non-escaped values are undefined and excluded before any arithmetic.
    """
    raw = path.read_bytes()
    assert raw[:8] == b'FDSAMPLE' and struct.unpack_from('<H', raw, 8)[0] == 1
    columns, nx, ny, ss = struct.unpack_from('<IIII', raw, 12)
    assert ss > 0 and nx % ss == 0 and ny % ss == 0 and columns & 15 == 15
    pos = 56
    for _ in range(4):
        n = struct.unpack_from('<H', raw, pos)[0]
        pos += 2 + n
    pos = (pos + 7) & ~7
    fields = {}
    for bit, dtype, name in [(0, 'u1', 'class'), (1, '<f8', 'nu'),
                              (2, '<f4', 'de'), (3, '<u2', 'angle')]:
        nbytes = nx * ny * np.dtype(dtype).itemsize
        assert pos + nbytes <= len(raw)
        fields[name] = np.frombuffer(raw, dtype, nx * ny, pos).reshape(ny, nx)
        pos += (nbytes + 7) & ~7
    valid = ((fields['class'] & 3) == 0).astype('float32')
    ow, oh = nx // ss, ny // ss
    size = (width, round(oh * width / ow))

    def avg(a):
        return a.reshape(oh, ss, ow, ss).mean(axis=(1, 3))

    mask = resize(avg(valid), size)

    def field(a):
        a = np.where(valid > 0, a, 0)
        a = np.nan_to_num(a, nan=0, posinf=0, neginf=0)
        return resize(avg(a), size) / np.maximum(mask, 1e-8)

    angle = fields['angle'].astype('float32') * (2 * np.pi / 65536)
    de = field(fields['de'])
    nu = field(fields['nu'])
    # A constant origin removal preserves double precision before f32 resize.
    origin = float(np.min(fields['nu'][valid > 0]))
    nu = field(fields['nu'] - origin)
    return dict(mask=mask, de=np.maximum(de, 0), nu=nu,
                nx=field(np.cos(angle)), ny=field(np.sin(angle)),
                scale=width / 192, size=size,
                sha256=hashlib.sha256(raw).hexdigest(), origin=origin)


def sample(a, y, x):
    return map_coordinates(a, [y, x], order=1, mode='nearest', prefilter=False)


def streamline(seed, vx, vy, radius, mask):
    """Bounded curved convolution, with spatially varying seeds (not f(psi))."""
    y0, x0 = np.indices(seed.shape, dtype='float32')
    total, weight = seed.copy(), np.ones_like(seed)
    for sign in [-1, 1]:
        x, y = x0.copy(), y0.copy()
        for k in range(6):
            dx, dy = sample(vx, y, x), sample(vy, y, x)
            midx, midy = x + sign * radius * dx / 12, y + sign * radius * dy / 12
            x += sign * radius * sample(vx, midy, midx) / 6
            y += sign * radius * sample(vy, midy, midx) / 6
            w = math.exp(-((k + 1) / 4) ** 2) * sample(mask, y, x)
            total += sample(seed, y, x) * w
            weight += w
    return total / weight


def anatomy(f, family):
    t = time.perf_counter()
    m, d, s = f['mask'], f['de'], f['scale']
    z = np.log1p(d) / 5
    levels = [gaussian_filter(z, k * s) for k in [.6, 2, 6, 14]]
    fine = levels[0] - levels[1]
    mid = levels[1] - levels[2]
    broad = levels[2] - levels[3]
    density = np.exp(-d / 26) * m
    normal_mag = np.hypot(f['nx'], f['ny']) + 1e-6
    nx, ny = f['nx'] / normal_mag, f['ny'] / normal_mag
    vx, vy = -ny, nx
    # Orientation and different scale bands vary ALONG the transport contours.
    seed = density * (.12 + .88 * np.maximum(0, nx * .8 - ny * .6) ** 3)
    seed *= np.clip(.4 + 6 * np.abs(mid) + 3 * np.abs(broad), 0, 1)
    flow = streamline(seed, vx, vy, (8 + family * 2) * s, m)
    if family == 7:
        # A multiscale potential distinct from both density and its emission seed.
        gy, gx = np.gradient(levels[1] + .7 * levels[2])
        mag = np.hypot(gx, gy) + 1e-6
        flow = streamline(seed, -gy / mag, gx / mag, 24 * s, m)
    h = (levels[0] * .15 + mid * (1 + family % 3) + broad)
    if family in [0, 2, 8, 9]:
        # Continuous escape terraces are a comparison arm, never integer bands.
        phase = f['nu'] * (.6 if family == 0 else 1.3)
        h += (.035 * np.sin(phase) + .012 * np.sin(phase * 2.7)) * density
    if family in [3, 5, 7, 9]:
        h += flow * .25
    hy, hx = np.gradient(h)
    hx, hy = hx * 20 * s, hy * 20 * s
    # Fixed horizon probes: synthetic height visibility, no real-depth claim.
    yy, xx = np.indices(d.shape, dtype='float32')
    horizons = []
    for lx, ly in [(.8, -.6), (-.8, -.6), (.6, .8), (-.6, .8)]:
        occ = np.zeros_like(d)
        for r in [2, 5, 11, 20]:
            rr = r * s
            other = sample(h, yy + ly * rr, xx + lx * rr)
            occ = np.maximum(occ, (other - h) * 20 / r - .08)
        horizons.append(np.exp(-4 * occ))
    # Finite-distance ridges: do not paint every subpixel boundary fragment gold.
    ridge = np.exp(-np.abs(np.log(np.maximum(d, 1e-6) / .8)) * 1.2)
    ridge *= (.2 + .8 * np.maximum(0, nx * .8 - ny * .6) ** 2)
    return dict(mask=m, density=density, flow=flow, ridge=ridge,
                fine=fine, mid=mid, broad=broad, hx=hx, hy=hy,
                nx=nx, ny=ny, visibility=horizons, de=d,
                seconds=time.perf_counter() - t)


def render(a, p):
    fam = p['family']
    theta = p['light']
    lx, ly = np.cos(theta), np.sin(theta)
    x = a['hx'] * p['relief'] + a['nx'] * 2.4
    y = a['hy'] * p['relief'] + a['ny'] * 2.4
    norm = np.sqrt(x*x + y*y + 1)
    diffuse = np.clip((-x * lx - y * ly + .45) / norm, 0, 1)
    back = np.clip((x * lx + y * ly + .25) / norm, 0, 1)
    rim = np.clip(1 - 1 / norm, 0, 1)
    visibility = a['visibility'][p['direction']]
    ao = visibility ** p['shadow']
    cyan = (a['flow'] * .7 + a['density'] * back ** 3 * .3) * p['mist']
    violet = np.clip(np.abs(a['mid']) * 2, 0, 1) * p['violet'] * back ** 2 * .18
    gold = (np.clip(a['ridge'] - p['threshold'], 0, 1) ** p['gold_power'])
    gold *= diffuse ** 2 * visibility * p['gold']
    if fam == 1:
        cyan *= .3
    if fam == 2:
        cyan = a['density'] * back * .5 * p['mist']
    if fam == 4:
        cyan *= diffuse + .2
    if fam == 5:
        gold *= .3
        cyan *= 1.6
    if fam == 6:
        cyan = a['density'] * (1 - visibility) * p['mist']
        gold *= 1.8
    if fam == 8:
        cyan *= .2
        gold += a['ridge'] * diffuse ** 6 * .3
    # Competing hue weights prevent additive RGB whitening; strength is separate.
    signals = np.stack([cyan, violet, gold], axis=-1)
    energy = signals.max(axis=-1)
    weights = (signals + 1e-8) ** p['competition']
    weights /= weights.sum(axis=-1, keepdims=True)
    colors = np.array([[.012, .43, 1.0], [.24, .045, .44], [1.0, .40, .065]])
    emission = (weights @ colors) * energy[..., None]
    # Dark, colored material receives directional illumination independently.
    tint = (.5 + .5 * a['nx'])[..., None]
    albedo = np.array([.006, .018, .044]) * (1-tint) + np.array([.024, .010, .027]) * tint
    surface = albedo * (p['ambient'] + diffuse[..., None] * p['surface']) * ao[..., None]
    hdr = (surface + emission * p['exposure']) * a['mask'][..., None]
    assert np.isfinite(hdr).all() and (hdr >= 0).all()
    mapped = hdr / (1 + hdr)
    rgb = np.where(mapped <= .0031308, mapped * 12.92, 1.055 * mapped ** (1/2.4) - .055)
    return np.clip(rgb, 0, 1).astype('float32')


def features(rgb):
    lum = rgb @ np.array([.2126, .7152, .0722])
    hi, lo = rgb.max(axis=-1), rgb.min(axis=-1)
    sat = (hi-lo) / np.maximum(hi, 1e-6)
    grad = np.hypot(*np.gradient(lum))
    return np.array([np.mean(lum < .04), np.mean(lum > .75),
                     np.mean((hi > .65) & (sat < .22)),
                     np.mean(sat), np.mean(lum), np.mean(grad),
                     np.mean((rgb[..., 2] > rgb[..., 0]*1.4) & (lum > .08)),
                     np.mean((rgb[..., 0] > rgb[..., 2]*1.5) & (lum > .15))])


def metrics(rgb, a, target):
    feat = features(rgb)
    # Distribution-only heuristic, NOT visual similarity or physical correctness.
    weights = np.array([.4, .5, 4, .25, .7, 1, .4, .4])
    score = -float(np.sum(weights * np.abs(feat - target)))
    # Avoid constant masks and pale carpet; reward retained middle-scale contrast.
    lum = rgb @ np.array([.2126, .7152, .0722])
    local = np.abs(lum - gaussian_filter(lum, 2))
    score += .25 * min(float(local.mean()), .08)
    interior = a['mask'] == 0
    leakage = float(rgb[interior].max()) if interior.any() else 0
    return score, feat.tolist(), leakage


def parameters(rng, family, trial):
    light = float(rng.uniform(-math.pi, math.pi))
    directions = np.array([[.8, -.6], [-.8, -.6], [.6, .8], [-.6, .8]])
    direction = int(np.argmax(directions @ np.array([np.cos(light), np.sin(light)])))
    return dict(id=trial, family=family, light=light,
                relief=float(rng.uniform(.25, 3)), direction=direction,
                shadow=float(rng.uniform(.1, 2.5)), mist=float(rng.uniform(.15, 3.5)),
                violet=float(rng.uniform(.1, 1.4)), gold=float(rng.uniform(.2, 8)),
                gold_power=float(rng.uniform(1, 3)), threshold=float(rng.uniform(.02, .4)),
                competition=float(rng.uniform(1.5, 5)), ambient=float(rng.uniform(.1, .5)),
                surface=float(rng.uniform(.8, 4)), exposure=float(rng.uniform(.5, 3)))


def save(path, rgb):
    Image.fromarray(np.uint8(np.clip(rgb * 255 + .5, 0, 255))).save(path)


def sheet(paths, labels, out, cols=5, tile=(288, 180)):
    rows = math.ceil(len(paths) / cols)
    canvas = Image.new('RGB', (cols * tile[0], rows * (tile[1] + 28)), '#11151d')
    draw = ImageDraw.Draw(canvas)
    for i, (path, label) in enumerate(zip(paths, labels)):
        im = Image.open(path).convert('RGB')
        im = im.resize((tile[0], round(im.height * tile[0] / im.width)), Image.Resampling.LANCZOS)
        x, y = (i % cols) * tile[0], (i // cols) * (tile[1] + 28)
        canvas.paste(im, (x, y))
        draw.text((x+5, y+tile[1]+5), label, fill='white')
    canvas.save(out)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--samples', type=Path, required=True)
    ap.add_argument('--reference', type=Path, required=True)
    ap.add_argument('--output', type=Path, required=True)
    ap.add_argument('--trials', type=int, default=1280)
    ap.add_argument('--seed', type=int, default=20261019)
    ap.add_argument('--width', type=int, default=192)
    ap.add_argument('--final-width', type=int, default=1024)
    ap.add_argument('--finalists', type=int, default=10)
    ap.add_argument('--baseline', type=Path)
    args = ap.parse_args()
    assert args.trials >= 50 and args.width >= 64 and 1 <= args.finalists <= 50
    out = args.output
    out.mkdir(parents=True, exist_ok=True)
    start = time.perf_counter()
    f = read_fields(args.samples, args.width)
    reference = Image.open(args.reference).convert('RGB')
    reference.thumbnail((384, 240))
    target = features(np.array(reference, dtype='float32') / 255)
    reference.save(out / 'reference.png')
    if args.baseline:
        Image.open(args.baseline).convert('RGB').save(out/'baseline.png')
    rng = np.random.default_rng(args.seed)
    rows, short = [], []
    operators = []
    for family, name in enumerate(FAMILIES):
        a = anatomy(f, family)
        assert a['seconds'] < 30, 'operator probe exceeded 30-second research gate'
        operators.append(dict(family=name, preparation_seconds=a['seconds']))
        group = []
        for trial in range(family, args.trials, len(FAMILIES)):
            p = parameters(rng, family, trial)
            t = time.perf_counter()
            rgb = render(a, p)
            score, feat, leakage = metrics(rgb, a, target)
            row = dict(parameters=p, score=score, features=feat, interior_leakage=leakage,
                       seconds=time.perf_counter()-t)
            assert leakage == 0 and row['seconds'] < 30
            rows.append(row)
            group.append(row)
        # A quality pool plus farthest-point distribution diversity per family.
        pool = sorted(group, key=lambda r: r['score'], reverse=True)[:max(5, len(group)//2)]
        selected = [pool.pop(0)]
        while len(selected) < 5:
            best = max(pool, key=lambda r: min(np.linalg.norm(np.array(r['features'])-
                        np.array(s['features'])) for s in selected))
            selected.append(best)
            pool.remove(best)
        for row in selected:
            p = row['parameters']
            path = out / f"candidate-{p['id']:04d}.png"
            save(path, render(a, p))
            row['image'] = path.name
            short.append(row)
        print(json.dumps(dict(family=name, trials=len(group), best_score=selected[0]['score'])), flush=True)
    assert len(short) == 50 and len({r['parameters']['id'] for r in short}) == 50
    paths = [out/r['image'] for r in short]
    labels = [f"#{r['parameters']['id']:04d} {FAMILIES[r['parameters']['family']]}" for r in short]
    sheet(paths[:25], labels[:25], out/'contact-1.png')
    sheet(paths[25:], labels[25:], out/'contact-2.png')
    # Best per family first; then globally ranked remainder if >10 requested.
    leaders = sorted([max(short[i:i+5], key=lambda r: r['score']) for i in range(0, 50, 5)],
                     key=lambda r: r['score'], reverse=True)
    rest = sorted([r for r in short if r not in leaders], key=lambda r: r['score'], reverse=True)
    full = read_fields(args.samples, args.final_width)
    finalists = []
    for row in (leaders+rest)[:args.finalists]:
        p = row['parameters']
        a = anatomy(full, p['family'])
        path = out/f"final-{p['id']:04d}.png"
        rgb = render(a, p)
        assert np.array_equal(rgb, render(a, p)), 'stateless replay changed pixels'
        save(path, rgb)
        finalists.append(dict(id=p['id'], family=FAMILIES[p['family']], image=path.name,
                              sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
    # One same-input controlled comparison, with final controls held fixed.
    p = leaders[0]['parameters']
    a = anatomy(full, p['family'])
    no_transport = dict(a, flow=np.zeros_like(a['flow']))
    isotropic = dict(a, flow=gaussian_filter(a['flow'], 5 * full['scale']))
    # Equalize source totals; this is a control, not a physical energy guarantee.
    isotropic['flow'] *= float(a['flow'].sum()) / max(float(isotropic['flow'].sum()), 1e-9)
    save(out/'control-no-transport.png', render(no_transport, p))
    save(out/'control-isotropic.png', render(isotropic, p))
    ablation = dict(candidate=p['id'], flow_rms_change_from_zero=float(np.sqrt(np.mean(a['flow']**2))),
                    directional_vs_isotropic_rms=float(np.sqrt(np.mean((a['flow']-isotropic['flow'])**2))))
    report = dict(seed=args.seed, trials=len(rows), scored_pixel_evaluations=len(rows)*int(np.prod(f['size'])),
                  samples=str(args.samples), input_sha256=f['sha256'],
                  reference=str(args.reference), reference_sha256=hashlib.sha256(args.reference.read_bytes()).hexdigest(),
                  target_features=target.tolist(), feature_names=['dark_fraction','bright_fraction',
                  'pale_bright_fraction','mean_saturation','mean_luminance','mean_gradient','cool_fraction','warm_fraction'],
                  operators=operators, shortlist=short, finalists=finalists, trials_manifest=rows, ablation=ablation,
                  total_seconds=time.perf_counter()-start,
                  notes=['Offline novel-operator research, not production engine output or GPU timing.',
                         'Scores compare global distributions, NOT reference similarity.',
                         'DE is stored in original output pixels. Filter radii scale with preview resolution.',
                         'No synthetic noise, no fractal iterations, no history; camera stability unverified.',
                         'Interior AND unresolved samples excluded; mask-zero pixels exactly black.',
                         'Fifty variants across ten operator families, not fifty independent inventions.',
                         'Reference supplied by owner; no claim about how it was made.',
                         'No reference-match or quality threshold claimed. Human aesthetic selection owed.',
                         'Visibility uses the nearest of four horizon directions to the selected light.',
                         'Control isotropic blur smooths the curved light field; it does not isolate all orientation effects.'])
    assert hashlib.sha256(args.samples.read_bytes()).hexdigest() == f['sha256']
    (out/'report.json').write_text(json.dumps(report, indent=2))
    final_by_id = {r['id']: r['image'] for r in finalists}
    cards = []
    for row in short:
        p = row['parameters']
        large = final_by_id.get(p['id'], row['image'])
        cards.append(f'<a href="{large}" target="_blank"><img loading="lazy" src="{row["image"]}">'
                     f'<b>#{p["id"]:04d} {FAMILIES[p["family"]]}</b><small>Heuristic score {row["score"]:.3f}</small></a>')
    baseline_html = '<a href="baseline.png"><img src="baseline.png"><b>Previous engine render (same view)</b></a>' if args.baseline else ''
    page = '''<!doctype html><meta charset="utf-8"><title>Fractal styling search</title>
<style>body{background:#0b1018;color:#dce6f2;font:16px system-ui;margin:32px}h1{font-size:26px}
.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(250px,1fr));gap:16px}a{color:inherit;text-decoration:none;background:#151e2b;padding:8px;border-radius:8px}img{width:100%;display:block}b,small{display:block;margin:6px}small,p{color:#a6b4c8}.refs{display:grid;grid-template-columns:1fr 1fr;gap:16px;max-width:1000px;margin-bottom:28px}</style>
<h1>50 candidates · 10 approaches</h1>
<p>Offline field experiments. Click candidates for larger images where available. Scores screen global image statistics; they do not measure resemblance. These are ten related construction families, not fifty independent inventions.</p>
'''+f'<p>{args.trials:,} trials · seed {args.seed} · {html.escape(args.samples.name)} · no new fractal iterations</p>'
    page += '<div class="refs"><a href="reference.png"><img src="reference.png"><b>Owner reference (different view)</b></a>'+baseline_html+'</div>'
    page += '<div class="grid">'+''.join(cards)+'</div><p><a href="report.json">Complete parameters, timings, hashes and scores</a></p>'
    (out/'index.html').write_text(page)
    print(json.dumps({k: report[k] for k in ['trials','scored_pixel_evaluations','total_seconds','finalists']}), flush=True)


if __name__ == '__main__':
    main()
