"""Drive real film/render/shade commands; keep deep and baseline evidence outside checkout."""
import argparse
from decimal import Decimal
import json
import math
from pathlib import Path
import struct
import subprocess

p = argparse.ArgumentParser()
p.add_argument('--fd', required=True)
p.add_argument('--out', type=Path, required=True)
p.add_argument('--before', type=Path, required=True)
a = p.parse_args()
a.out.mkdir(parents=True, exist_ok=True)
before = a.before
rung = next(line.split() for line in Path('tools/research/misiurewicz/ladder_rungs.txt').read_text().splitlines() if line.startswith('7676 '))
base = ['--seconds', '2', '--fps', '2', '--size', '8x8', '--iter', str(int(rung[1])*40), '--threads', '1', '--look', 'umber', '--x264', 'ultrafast']

def run(name, cmd, success=True):
    r = subprocess.run([str(v) for v in cmd], capture_output=True)
    (a.out/f'{name}.stdout').write_bytes(r.stdout)
    (a.out/f'{name}.stderr').write_bytes(r.stderr)
    if success:
        assert r.returncode == 0, f'{name} exit{r.returncode}: {r.stderr.decode()[-2000:]}'
    else:
        assert r.returncode != 0, f'{name} unexpectedly accepted'
    return r

def film(name, camera, flags, binary=a.fd):
    raw = a.out/name
    raw.mkdir(exist_ok=True)
    r = run(name, [binary, 'film', *camera, *base, *flags, '--raw-dir', raw, '--mp4', a.out/f'{name}.mp4'])
    report = json.loads(r.stdout)
    widths = [row['width'] for row in report['kernel_frame_process_cpu']]
    data = run(name+'-decode', ['ffmpeg', '-v', 'error', '-i', a.out/f'{name}.mp4', '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-']).stdout
    assert len(data) == report['frames'] * 8 * 8 * 3
    return report, widths

camera = [rung[2], rung[3], '--from', '1e-999', '--to', '1e-1000']
report, widths = film('deep', camera, ['--ss', '1', '--frames', '2..4'])
assert len(widths) == 2 and Decimal(widths[-1]) == Decimal('1e-1000'), widths
assert all(Decimal('1e-1000') <= Decimal(w) < Decimal('1e-999') for w in widths)
# A film's last deep frame must be the real samples at its exact decimal camera.
field = a.out/'deep.fds'
run('samples', [a.fd, 'render', '--re', rung[2], '--im', rung[3], '--width', widths[-1], '--size', '8x8', '--ss', '1', '--iter', str(int(rung[1])*40), '--threads', '1', '--columns', 'nu,de,normal', '-o', field])
b = field.read_bytes()
pos = 56
strings = []
for _ in range(4):
    n, = struct.unpack_from('<H', b, pos)
    pos += 2
    strings.append(b[pos:pos+n].decode())
    pos += n
assert strings[:3] == [rung[2], rung[3], widths[-1]], strings[:3]
body = (pos + 7) & ~7
classes = [v & 3 for v in b[body:body+64]]
assert len(classes) == 64 and 0 in classes and 2 in classes, classes
nu = struct.unpack_from('<64d', b, body+64)
de = struct.unpack_from('<64f', b, body+64+64*8)
escaped = [i for i,c in enumerate(classes) if c == 0]
assert all(math.isfinite(nu[i]) and math.isfinite(de[i]) and de[i] >= 0 for i in escaped)
assert len({nu[i] for i in escaped}) > 1, 'deep samples must contain nonconstant escape values' 
run('shade', [a.fd, 'shade', field, '--look', 'umber', '--aa', 'on', '--unresolved', 'interior', '--dither', 'on', '-o', a.out/'deep.png'])
rgb = run('shade-decode', ['ffmpeg', '-v', 'error', '-i', a.out/'deep.png/deep.umber.png', '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-']).stdout
assert rgb == (a.out/'deep/frame-000003.rgb').read_bytes(), 'film differs from actual deep sample shading'
# --place uses the same exact center and deep width; logs and pixels agree.
place = a.out/'deep.place'
place.write_text(f're {rung[2]}\nim {rung[3]}\nwidth 1e-1000\niter {int(rung[1])*40}\n')
_, placed = film('place', ['--place', str(place), '--from', '1e-999'], ['--ss', '1', '--frames', '2..4'])
assert widths == placed
assert (a.out/'deep/frame-000003.rgb').read_bytes() == (a.out/'place/frame-000003.rgb').read_bytes()
# Default keyframes must also operate below the old magnitude ceiling. Decimal
# offsets are local frame ratios; a deep film and a shallow scale-equivalent film
# must produce the same rounded width ratios and nonempty output.
key_report, key_widths = film('keys', camera, ['--offset', '1e-1000,-5e-1001', '--twist', '0.2', '--frames', '2..4'])
assert key_report['keyframes'] == 2 and key_widths
assert all(math.isfinite(float(Decimal(w).log10())) for w in key_widths)
# Reuse existing speed-curve path; no second width reconstruction.
speed = a.out/'speed.txt'
speed.write_text('-999 2\n-1000 2\n')
saved = base[:]
base[:] = base[2:]
_, curve_widths = film('curve', camera, ['--ss', '1', '--speed', str(speed), '--frames', '2..4'])
base[:] = saved
assert Decimal(curve_widths[-1]) == Decimal('1e-1000')
# Existing shallow camera path and output must agree with the unchanged binary.
shallow = ['-0.7453', '0.1127', '--from', '0.0065', '--to', '0.003']
saved = base[:]
base[:] = ['--seconds', '2', '--fps', '4', '--size', '8x8', '--iter', '32', '--threads', '1', '--look', 'umber', '--x264', 'ultrafast']
shallow_rows = {}
for name, ease in [('ease', ['--ease', 'on']), ('in', ['--ease-in', '1']), ('out', ['--ease-out', '1'])]:
    args = ['--ss', '1', *ease]
    old, old_widths = film('old-'+name, shallow, args, before)
    new, new_widths = film('new-'+name, shallow, args)
    assert len(old_widths) == len(new_widths)
    assert all(math.isclose(float(Decimal(x).log10()), float(Decimal(y).log10()), rel_tol=5e-13, abs_tol=5e-13) for x,y in zip(old_widths,new_widths)), (name,old_widths,new_widths)
    for f in range(new['frames']):
        assert (a.out/f'old-{name}/frame-{f:06}.rgb').read_bytes() == (a.out/f'new-{name}/frame-{f:06}.rgb').read_bytes()
    shallow_rows[name] = new_widths
for name, flags in [('key-spin', ['--offset', '0.0002,-0.0001', '--twist', '0.2'])]:
    old, _ = film('old-'+name, shallow, flags, before)
    new, _ = film('new-'+name, shallow, flags)
    assert old['keyframes'] == new['keyframes'] == 2
    for f in range(new['frames']):
        old_rgb = (a.out/f'old-{name}/frame-{f:06}.rgb').read_bytes()
        new_rgb = (a.out/f'new-{name}/frame-{f:06}.rgb').read_bytes()
        assert old_rgb == new_rgb, 'shallow rotated/offset keyframe output changed'
base[:] = saved
for name, flags in [('cap', ['--depth-cap', '999']), ('extreme', ['--depth-cap', '1001']), ('beyond', [])]:
    rejected_camera = camera[:-1] + ['1e-1001'] if name == 'beyond' else camera
    r = run(name, [a.fd, 'film', *rejected_camera, *base, *flags, '--mp4', a.out/f'rejected-{name}.mp4'], False)
    assert b'depth-cap' in r.stderr
    assert not (a.out/f'rejected-{name}.mp4').exists()
summary = dict(widths=widths, keyframe_widths=key_widths, classes={str(c):classes.count(c) for c in set(classes)}, samples=64, escaped_nu_range=[min(nu[i] for i in escaped),max(nu[i] for i in escaped)], exact_camera=True, shaded_rgb_identity=True, shallow_widths=shallow_rows)
(a.out/'summary.json').write_text(json.dumps(summary, indent=2)+'\n')
print(json.dumps(summary))
