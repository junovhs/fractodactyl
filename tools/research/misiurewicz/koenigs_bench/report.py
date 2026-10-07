"""PROB-08: summarise a run.sh output directory as a Markdown table."""
import json, platform, sys
from pathlib import Path

out = Path(sys.argv[1])
fd = [json.loads(l) for l in (out/'fd-time.jsonl').read_text().splitlines() if l.startswith('{')]
fd = [f for f in fd if f.get('record') == 'frame']
mode = {m: [json.loads(l) for l in (out/f'{m}.jsonl').read_text().splitlines()] for m in ('koenigs', 'perturb')}
score = {}
for m in mode:
    lines = (out/f'{m}-score.txt').read_text().splitlines()
    score[m] = dict(rows=[json.loads(l) for l in lines if l.startswith('{')], verdict=lines[-1])
k0 = mode['koenigs'][0]
print(f"# PROB-08 timing: {k0['nx']}x{k0['ny']}, {k0['threads']} threads, best of {k0['runs']} runs, "
      f"{platform.platform()} ({platform.processor() or platform.machine()})\n")
print(f"{(out/'consts.txt').read_text().splitlines()[0][2:]}\n")
print('| width | fd per-frame BLA s | lean perturbation s | Koenigs pipeline s | speed-up vs fd BLA | vs lean perturbation | Koenigs wrong px (>1e-3) | class mismatches | max px |')
print('|---|---|---|---|---|---|---|---|---|')
for f, k, p, s in zip(fd, mode['koenigs'], mode['perturb'], score['koenigs']['rows']):
    assert f['view']['width'] == k['width'] == p['width'] == s['width']
    t = min(r['seconds'] for r in f['runs'])
    print(f"| {k['width']} | {t:.4f} | {p['best_seconds']:.4f} | {k['best_seconds']:.4f} | "
          f"{t/k['best_seconds']:.1f}x | {p['best_seconds']/k['best_seconds']:.1f}x | "
          f"{s['wrong_px_gt_1e3']} / {s['compared']} | {s['class_mismatches']} | {s['max_px']:.1e} |")
print(f"\nKoenigs pipeline vs fd: {score['koenigs']['verdict']}; lean perturbation vs fd: {score['perturb']['verdict']}")
print('fd seconds: whole frame from `fd control` runs (includes its per-frame reference and BLA build).')
print('Koenigs seconds: pixels only; per-zone constants are built once in Python (see consts.txt), shared by every frame.')
