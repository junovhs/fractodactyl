#!/usr/bin/env python3
"""PROB-41: growing-order series initializer on the frozen PROB-22 frames, cold, per bin.

    python docs/2026-10-20/series-frames/run.py --out OUT [--threads 8] [--runs 3]

Builds the probe (probe/, release, the fd workspace's profile), and fd unless one is
already built (--fd; this issue does not change fd compare), renders the 24
frames in results/frames.txt with every arm (series-probe; setup inside each frame's
clock), runs `fd compare plain ARM` for every other arm, and writes OUT/bins.csv (cold
seconds and speedups per 3-decade bin), OUT/compare.csv (per frame and arm: class
mismatches, nu px, de over tolerance) and OUT/compare-ARM.jsonl. Exit 0 when every step
ran; the go/no-go is read from the tables, not the exit code.
"""
import argparse
import csv
import json
import re
import os
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
BINS = (5, 5, 5, 5, 4)
EXE = '.exe' if os.name == 'nt' else ''


def target_dir():
    return Path(os.environ.get('CARGO_TARGET_DIR', ROOT / 'target'))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--out', required=True)
    ap.add_argument('--frames', default=str(HERE / 'results' / 'frames.txt'))
    ap.add_argument('--threads', default='8')
    ap.add_argument('--runs', default='3')
    ap.add_argument('--arms', default='plain,bla,series,series-bla,series-k64')
    ap.add_argument('--skip-render', action='store_true', help='reuse OUT/frames.csv and OUT/ARM/*.fds')
    ap.add_argument('--fd', default=str(target_dir() / 'release' / ('fd' + EXE)))
    a = ap.parse_args()
    out = Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    fd = Path(a.fd)
    if not fd.is_file():
        subprocess.run(['cargo', 'build', '--release', '--locked', '-p', 'fd-cli'], cwd=ROOT, check=True)
    if not a.skip_render:
        subprocess.run(['cargo', 'build', '--release'], cwd=HERE / 'probe', check=True)
        probe = target_dir() / 'release' / ('series-probe' + EXE)
        subprocess.run([str(probe), a.frames, str(out), '--threads', a.threads, '--runs', a.runs,
                        '--arms', a.arms], check=True)
    rows = list(csv.DictReader(open(out / 'frames.csv', newline='')))
    arms = a.arms.split(',')
    frame_bin = [b for b, n in enumerate(BINS) for _ in range(n)]

    # Correctness: every arm against plain, whole frames (DEC-17 contract of fd compare).
    cmp_rows = []
    for arm in arms[1:]:
        r = subprocess.run([str(fd), 'compare', str(out / 'plain'), str(out / arm)],
                           capture_output=True, text=True)
        (out / ('compare-%s.jsonl' % arm)).write_text(r.stdout)
        for line in r.stdout.splitlines():
            # fd compare prints a bare inf for ill-conditioned near-boundary maxima.
            j = json.loads(re.sub(r':(-?)inf\b', r':\1Infinity', line))
            if j.get('record') != 'frame':
                continue
            cmp_rows.append(dict(frame=j['frame'], arm=arm, ok=j['ok'],
                                 class_mismatches=j.get('class_mismatches'),
                                 nu_px_max=j.get('nu', {}).get('px_max'),
                                 nu_over_px=j.get('nu', {}).get('over_px'),
                                 non_finite=j.get('non_finite'),
                                 de_rel_max=j.get('de', {}).get('rel_max'),
                                 de_over_tol=j.get('de', {}).get('over_tol')))
    with open(out / 'compare.csv', 'w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=list(cmp_rows[0]))
        w.writeheader()
        w.writerows(cmp_rows)
    bad = defaultdict(int)
    for r in cmp_rows:
        bad[(frame_bin[r['frame']], r['arm'])] += 0 if r['ok'] else 1

    secs = defaultdict(float)
    for r in rows:
        secs[(frame_bin[int(r['frame'])], r['arm'])] += float(r['cold_s'])
    lines = []
    fields = ['bin', 'depths', 'frames'] + ['%s_s' % x for x in arms] + \
        ['%s_vs_plain' % x for x in arms[1:]] + ['%s_vs_bla' % x for x in arms if x.startswith('series')] + \
        ['%s_failed_frames' % x for x in arms[1:]]
    for b, n in enumerate(BINS):
        d = dict(bin=b, depths='1e-%d..1e-%d' % (14 + 3 * b, 17 + 3 * b), frames=n)
        for x in arms:
            d['%s_s' % x] = round(secs[(b, x)], 4)
        for x in arms[1:]:
            d['%s_vs_plain' % x] = round(secs[(b, 'plain')] / secs[(b, x)], 3)
            d['%s_failed_frames' % x] = bad[(b, x)]
        for x in arms:
            if x.startswith('series') and 'bla' in arms:
                d['%s_vs_bla' % x] = round(secs[(b, 'bla')] / secs[(b, x)], 3)
        lines.append(d)
    with open(out / 'bins.csv', 'w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=fields)
        w.writeheader()
        w.writerows(lines)
    for d in lines:
        print(d['depths'], ' '.join('%s %.3fs' % (x, d['%s_s' % x]) for x in arms), '|',
              ' '.join('%s %.2fx' % (x, d['%s_vs_plain' % x]) for x in arms[1:]), '| failed frames',
              ' '.join('%s %d' % (x, d['%s_failed_frames' % x]) for x in arms[1:]))
    return 0


if __name__ == '__main__':
    sys.exit(main())
