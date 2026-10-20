#!/usr/bin/env python3
"""PROB-41: freeze the 24 FIX-09 views from the PROB-22 planner path.

Reruns tools/research/variety/decades.py's PROB-22 planner (MODE=target, plan2) with its
default Seahorse start (--re -0.75 --im 0.1 --width 0.5, 24 candidates, seed 1, at least
20 decades), then picks the FIX-09 frames on the straight zoom into the chosen nucleus:
five 3-decade bins over widths 1e-14..1e-29, 5+5+5+5+4 frames at log-even depths inside
each bin (the same rule as the FIX-09 branch's scripts/check/FIX-09.sh).

    FD=target/release/fd python freeze_frames.py --out results

Writes plan.json (planner output) and frames.txt (fd path format: re im width).
The contact-strip montage of the original planner is skipped (needs ImageMagick).
"""
import argparse
import os
import sys
from pathlib import Path

import mpmath as mp

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[2] / 'tools' / 'research' / 'variety'))
os.environ.setdefault('MODE', 'target')
import decades  # noqa: E402

BINS = (5, 5, 5, 5, 4)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--out', required=True)
    a = ap.parse_args()
    out = Path(a.out)
    decades.strip = lambda steps, out: None
    final = decades.plan2('-0.75', '0.1', '0.5', str(out), n=24, seed=1, min_decades=20)
    lines = ['# PROB-41 frozen FIX-09 views on the PROB-22 path (decades.py MODE=target defaults)',
             '# nucleus period %d, size %.3e, planner end width %.3e' % (final['period'], final['size'], final['to']),
             '# bins (depth = -log10 width): ' + ' '.join('%d..%d:%d' % (14 + 3 * b, 17 + 3 * b, n)
                                                          for b, n in enumerate(BINS)),
             '# re im width']
    for b, count in enumerate(BINS):
        for j in range(count):
            depth = 14 + 3 * b + 3 * (j + 0.5) / count
            lines.append('%s %s %s' % (final['re'], final['im'], mp.nstr(mp.mpf(10) ** -mp.mpf(depth), 8)))
    (out / 'frames.txt').write_text('\n'.join(lines) + '\n', encoding='utf-8')
    print('froze %d frames to %s' % (sum(BINS), out / 'frames.txt'))


if __name__ == '__main__':
    main()
