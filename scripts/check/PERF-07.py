#!/usr/bin/env python3
"""Actual fd film CPU-report contract and unchanged-output proof (Ishoo lane)."""
import argparse
from decimal import Decimal
import json
import os
from pathlib import Path
import resource
import subprocess
import time

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--fd', type=Path, required=True)
p.add_argument('--before', type=Path, required=True)
p.add_argument('--out', type=Path, required=True)
p.add_argument('--workers', type=int, required=True)
a = p.parse_args()
root = Path(__file__).resolve().parents[2]
cpus = sorted(os.sched_getaffinity(0))[:a.workers]
assert len(cpus) == a.workers and a.workers > 0
os.sched_setaffinity(0, cpus)
fd = a.fd.resolve()
env = dict(os.environ, CARGO_TARGET_DIR=str(fd.parents[1]), CARGO_BUILD_JOBS=str(a.workers))
subprocess.run(['cargo', 'build', '--jobs', str(a.workers), '-p', 'fd-cli', '--release'],
               cwd=root, env=env, check=True)
a.out.mkdir(parents=True, exist_ok=True)

def run(cmd, stem):
    start_cpu = resource.getrusage(resource.RUSAGE_CHILDREN)
    start = time.perf_counter()
    result = subprocess.run([str(x) for x in cmd], cwd=root, text=True, capture_output=True)
    wall = time.perf_counter() - start
    end_cpu = resource.getrusage(resource.RUSAGE_CHILDREN)
    cpu = end_cpu.ru_utime + end_cpu.ru_stime - start_cpu.ru_utime - start_cpu.ru_stime
    (a.out / f'{stem}.stdout').write_text(result.stdout)
    (a.out / f'{stem}.stderr').write_text(result.stderr)
    assert result.returncode == 0, (cmd, result.stderr)
    return result, dict(wall_seconds=wall, process_tree_cpu_seconds=cpu)

films = []
for mode in ('off', '2'):
    common = ['film', '-0.75', '0.1', '--from', '4', '--to', '1', '--seconds', '1',
              '--fps', '3', '--size', '64x36', '--iter', '256', '--threads', str(a.workers),
              '--keyframes', mode, '--gpu', 'off', '--x264', 'ultrafast', '--json']
    result, timing = run([fd, *common, '--mp4', a.out / f'new-{mode}.mp4'], f'new-{mode}')
    report = json.loads(result.stdout)
    records = report['kernel_frame_process_cpu']  # red: field absent before PERF-07
    assert records and len(records) == sum(r['kernel_frames'] for r in report['routes'].values())
    assert len(records) == (3 if mode == 'off' else report['render']['perturbation']['frames'] + report['render']['bla']['frames'])
    for record in records:
        assert record['wall_seconds'] > 0 and record['requested_threads'] == a.workers
        assert record['process_cpu_seconds'] >= 0
        assert abs(record['average_process_busy_threads'] - record['process_cpu_seconds'] / record['wall_seconds']) < 1e-9
        assert record['width'] and record['route']
    assert result.stderr.count('kernel process CPU:') == len(records)
    assert 'overlapping' in report['kernel_frame_process_cpu_scope']
    assert 'setup' in report['kernel_frame_process_cpu_scope']
    assert sum(r['process_cpu_seconds'] for r in records) <= timing['process_tree_cpu_seconds'] + .001
    assert sum(r['wall_seconds'] for r in records) <= timing['wall_seconds']
    _, before_timing = run([a.before, *common, '--mp4', a.out / f'old-{mode}.mp4'], f'old-{mode}')
    def decode(path):
        return subprocess.check_output(['ffmpeg', '-loglevel', 'error', '-i', str(path),
                                        '-vsync', '0', '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-'])
    assert decode(a.out / f'new-{mode}.mp4') == decode(a.out / f'old-{mode}.mp4')
    films.append(dict(mode=mode, records=records, before=before_timing, after=timing))

# No renderer or serialization changes: compare actual .fds bytes on the same
# four exact real-film views selected before any candidate measurement.
frozen = [l for l in (root / 'docs/2026-10-20/series-frames/results/frames.txt').read_text().splitlines()
          if l and not l.startswith('#')]
path = a.out / 'frozen.path'
path.write_text('\n'.join(frozen[i] for i in (0, 10, 17, 23)) + '\n')
for name, binary in [('old', a.before), ('new', fd)]:
    run([binary, 'control', path, '--size', '480x270', '--ss', '1', '--iter', '100000',
         '--columns', 'nu,de,normal', '--threads', str(a.workers), '--bla', 'per-frame',
         '-o', a.out / name], f'control-{name}')
for frame in range(4):
    file = f'frame-{frame:05}.fds'
    assert (a.out / 'old' / file).read_bytes() == (a.out / 'new' / file).read_bytes()
run([fd, 'compare', a.out / 'old', a.out / 'new'], 'compare')

# Fixed real-film views also exercise the actual automatic production route.
# This is a census, not a speedup gate: one cold process per arm/view, including
# encoder startup/shutdown outside the kernel interval, so no multiplier is claimed.
census = []
for index in (0, 10, 17, 23):
    re, im, width = frozen[index].split()
    common = ['film', re, im, '--from', width, '--to', str(Decimal(width) / 2),
              '--seconds', '1', '--fps', '2', '--frames', '0', '--size', '480x270',
              '--ss', '1', '--iter', '100000', '--threads', str(a.workers),
              '--keyframes', 'off', '--gpu', 'off', '--x264', 'ultrafast', '--json']
    arms = {}
    for name, binary in [('old', a.before), ('new', fd)]:
        result, measured = run([binary, *common, '--mp4', a.out / f'{name}-deep-{index}.mp4'],
                               f'{name}-deep-{index}')
        measured['report'] = json.loads(result.stdout)
        arms[name] = measured
    records = arms['new']['report']['kernel_frame_process_cpu']
    assert len(records) == 1
    assert records[0]['process_cpu_seconds'] <= arms['new']['process_tree_cpu_seconds'] + .001
    census.append(dict(frozen_index=index, **arms))
(a.out / 'summary.json').write_text(json.dumps(dict(cpu_affinity=cpus, films=films,
    fixed_actual_film_census=census,
    sample_frames_byte_identical=4, decoded_films_byte_identical=2,
    end_to_end_cpu_scope='process tree including encoder, startup and shutdown'), indent=2) + '\n')
print('PERF-07: direct/keyframe CPU records valid; 4 sample frames and 2 decoded films byte-identical')
