"""Centre-orbit dwell census for PROB-16 (not a per-pixel shortcut).

Usage: python tools/research/misiurewicz/census.py [locations file]
Input: name|decimal-real|decimal-imag|steps|digits (last two optional).
Bands are orbit iteration ranges, not zoom widths. Requires numpy, mpmath.
"""
import argparse
from pathlib import Path
import time

import mpmath as mp
import numpy as np

HERE = Path(__file__).resolve().parent
BANDS = ((1, 100), (101, 1000), (1001, 10000), (10001, 100000), (100001, 1000000))
TOL = 1e-3
MIN_LAPS = 4
GATE_LOG = 0.1


def locations(path):
    for no, line in enumerate(Path(path).read_text().splitlines(), 1):
        line = line.split('#', 1)[0].strip()
        if not line:
            continue
        fields = [s.strip() for s in line.split('|')]
        if len(fields) not in (3, 4, 5) or not fields[0]:
            raise ValueError(f'{path}:{no}: expected name|real|imag|steps|digits')
        name, re, im = fields[:3]
        steps = int(fields[3]) if len(fields) > 3 else 100000
        digits = int(fields[4]) if len(fields) > 4 else max(80, len(re), len(im)) + 20
        if steps < 1 or digits < max(len(re), len(im)) + 5:
            raise ValueError(f'{path}:{no}: insufficient steps or precision')
        yield name, re, im, steps, digits


def orbit(re, im, steps, digits):
    # Decimal parameters never pass through f64; double is for observations only.
    with mp.workdps(digits):
        c = mp.mpc(re, im)
        z = mp.mpc(0)
        values = np.empty(steps + 1, dtype=complex)
        values[0] = 0
        for n in range(1, steps + 1):
            z = z*z + c
            if abs(z) > 1e6:
                return values[:n], n - 1, True
            values[n] = complex(z)
    return values, steps, False


def classify(values, steps, rmax=256, tol=TOL, min_laps=MIN_LAPS):
    """Find candidate cycle dwells; no jump validity or error bound is implied."""
    labels = np.full(steps, 'other', dtype='<U9')
    periods = np.zeros(steps, dtype=int)
    score = np.zeros(steps, dtype=int)
    radii = np.abs(values[:steps + 1])
    records = []
    best = np.inf
    for n in range(1, steps + 1):
        if radii[n] < best:
            best = radii[n]
            if n > 2:
                records.append(n)
    candidate = set(range(1, min(rmax, steps // 2) + 1))
    candidate.update(n for n in records[-32:] if n <= min(4096, steps // 2))
    for r in sorted(candidate):
        close = np.abs(values[r:steps + 1] - values[:steps + 1 - r]) < tol
        changes = np.diff(np.r_[False, close, False].astype(np.int8))
        starts = np.flatnonzero(changes == 1)
        ends = np.flatnonzero(changes == -1)
        for a, b in zip(starts, ends):
            run = int(b - a)
            if run < 2*r or a + r > steps:
                continue
            cycle = values[a:a + r]
            log_lambda = float(np.sum(np.log(np.maximum(np.abs(2*cycle), 1e-300))))
            if abs(log_lambda) <= GATE_LOG and run >= min_laps*r:
                kind = 'gate'
            elif r in records and radii[r] < tol and run >= min_laps*r:
                kind = 'minibrot'
            elif log_lambda > 0 and run >= min_laps*r:
                kind = 'spiral'
            else:
                continue
            # Disjoint cover: strongest lap count wins; tied scores stay first.
            hi = min(b, steps)
            quality = run*10000 // r
            sl = slice(a, hi)
            take = quality > score[sl]
            labels[sl] = np.where(take, kind, labels[sl])
            periods[sl] = np.where(take, r, periods[sl])
            score[sl] = np.where(take, quality, score[sl])
    seen = np.flatnonzero(labels != 'other')
    if len(seen):
        labels[:seen[0]] = 'approach'
    else:
        labels[:] = 'approach'
    return labels, periods


def rows(name, re, im, steps, digits, diagnose=False):
    values, n, escaped = orbit(re, im, steps, digits)
    labels, periods = classify(values, n)
    if diagnose and n >= 10000:
        lo = 10000
        baseline = int(np.count_nonzero(labels[lo:] != 'other'))
        print(f'# diagnostic {name} tail={n-lo} baseline={baseline}', flush=True)
        for cap, threshold, laps in ((2048, TOL, MIN_LAPS),
                                     (256, 1e-2, MIN_LAPS),
                                     (256, TOL, 2),
                                     (2048, 1e-2, 2)):
            alt, _ = classify(values, n, rmax=cap, tol=threshold,
                              min_laps=laps)
            covered = int(np.count_nonzero(alt[lo:] != 'other'))
            print(f'# diagnostic rmax={cap} tol={threshold:g} '
                  f'min_laps={laps}: classified={covered} '
                  f'change={covered-baseline:+d}', flush=True)
    for lo, hi in BANDS:
        if lo > n:
            continue
        selection = labels[lo - 1:min(hi, n)]
        ps = periods[lo - 1:min(hi, n)]
        counts = {k: int(np.count_nonzero(selection == k)) for k in
                  ('approach', 'spiral', 'minibrot', 'gate', 'other')}
        count = len(selection)
        skip = counts['spiral'] + counts['minibrot'] + counts['gate']
        cycles = sorted(set(int(p) for p in ps[(ps > 0) & np.isin(selection, ('spiral', 'minibrot', 'gate'))]))
        yield name, f'{lo}-{min(hi, n)}', count, counts, skip, cycles, escaped



def newton_target(spec, im, digits):
    """Find a periodic nucleus from a nearby seed; reject nonconvergence."""
    # spec = newton:period:seed-re; all arithmetic is mpmath decimal.
    _, period, seed_re = spec.split(':', 2)
    period = int(period)
    with mp.workdps(digits):
        c = mp.mpc(seed_re, im)
        threshold = mp.mpf(10) ** (-digits + 12)
        for _ in range(60):
            z = u = mp.mpc(0)
            for _ in range(period):
                u = 2*z*u + 1
                z = z*z + c
            if u == 0:
                break
            step = z/u
            if abs(step) > mp.mpf('0.05'):
                break
            c -= step
            if abs(step) < threshold:
                z = mp.mpc(0)
                for _ in range(period):
                    z = z*z + c
                if abs(z) > threshold*100:
                    break
                # Verify primitive period, not a lower period divisor.
                z = mp.mpc(0)
                for n in range(1, period):
                    z = z*z + c
                    if period % n == 0 and abs(z) < threshold*100:
                        raise ValueError(f'Newton converged to period {n}, not {period}')
                return mp.nstr(c.real, digits - 10), mp.nstr(c.imag, digits - 10), period
    raise ValueError(f'no certified period-{period} nucleus from {seed_re} {im}i')

def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('locations', nargs='?', type=Path,
                        default=HERE / 'census-locations.txt')
    parser.add_argument('--only', help='Run one named location')
    parser.add_argument('--diagnose-eye', action='store_true',
                        help='Compare period caps, recurrence tolerances and lap minimums')
    args = parser.parse_args(argv)
    start = time.monotonic()
    print('location | band | steps | approach | spiral | minibrot | gate | other | candidate skip | periods')
    for location in locations(args.locations):
        name, re, im, cap, dps = location
        if args.only and name != args.only:
            continue
        if re.startswith('newton:'):
            re, im, period = newton_target(re, im, dps)
            print(f'# solved {name}: nucleus period={period}, dps={dps}, re={re}, im={im}', flush=True)
        for name, band, n, counts, skip, cycles, escaped in rows(name, re, im, cap, dps, args.diagnose_eye and name == 'eye-of-universe'):
            print(f'{name} | {band} | {n} | {counts["approach"]} | {counts["spiral"]} | '
                  f'{counts["minibrot"]} | {counts["gate"]} | {counts["other"]} | '
                  f'{skip/n:.1%} | {",".join(map(str, cycles)) or "-"}'
                  f'{" (escaped)" if escaped else ""}')
    print(f'Elapsed: {time.monotonic() - start:.1f} s')


if __name__ == '__main__':
    main()
