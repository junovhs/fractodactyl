"""BENC-04: markdown report, fraktaler-3 vs fd per-frame BLA vs our pipeline (same runner).
Usage: python report_f3.py OUT NXxNY WIDTH...
"""
import json, platform, sys


def jsonl(path):
    return [json.loads(l) for l in open(path) if l.lstrip().startswith('{')]


def main():
    out, size, widths = sys.argv[1], sys.argv[2], sys.argv[3:]
    f3 = dict(l.split() for l in open(f'{out}/f3-times.txt') if len(l.split()) == 2)
    fd = jsonl(f'{out}/ours/fd-time.jsonl')
    ko = {r['width']: r for r in jsonl(f'{out}/ours/koenigs.jsonl')}
    sc = {r['width']: r for r in jsonl(f'{out}/f3-score.txt')}
    print(f"# BENC-04: fraktaler-3 3.1 vs fd vs our pipeline, {size}, {platform.platform()}\n")
    print("| width | fraktaler-3 s | fd per-frame BLA s | ours s | ours vs fraktaler-3 | vs f3: >1e-3 px | class mismatches | max px |")
    print("|---|---|---|---|---|---|---|---|")
    for i, w in enumerate(widths):
        a, c = float(f3[w]), ko[w]['best_seconds']
        b = min(r['seconds'] for r in fd[i]['runs']) if i < len(fd) else float('nan')
        s = sc.get(w, {})
        print(f"| {w} | {a:.3f} | {b:.3f} | {c:.4f} | {a/c:.1f}x | {s.get('wrong_px_gt_1e3')} / {s.get('compared')} "
              f"| {s.get('class_mismatches')} | {s.get('max_px', float('nan')):.1e} |")
    print()
    print(open(f'{out}/f3-score.txt').read().strip().splitlines()[-1])
    print("\nfraktaler-3 seconds: wall time of one `fraktaler-3 --batch` process per frame (startup, reference, BLA,"
          " render, EXR write), best of runs, all cores, benchmarked wisdom.")
    print("Ours: pixels only (per-zone constants and the patch atlas are built once per zone).")
    print("Our pipeline vs fd (every pixel) is in ours/report.md.")


if __name__ == '__main__':
    main()
