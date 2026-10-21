#!/usr/bin/env python3
"""PROB-28: measure the frozen PROB-22 film with the current production route.

Run as a named Ishoo check; it builds fd-cli from the exact lane snapshot first.
Outputs (JSON, encoder and progress log, MP4, invocation) stay outside the checkout.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('run', choices=['target', 'slice'])
parser.add_argument('--fd', type=Path, required=True)
parser.add_argument('--out', type=Path, required=True)
parser.add_argument('--workers', type=int, required=True)
a = parser.parse_args()
if a.workers < 1:
    parser.error('--workers must be positive')
# Bound fd, its renderer workers and the encoder to the same CPU set. This is
# intentionally a measured machine setting, not a change to the production path.
cpus = sorted(os.sched_getaffinity(0))[:a.workers]
if len(cpus) != a.workers:
    parser.error('not enough CPUs in the caller affinity')
os.sched_setaffinity(0, cpus)
root = Path(__file__).resolve().parents[2]
# A lane's persistent binary could otherwise belong to an older snapshot. Build
# this checkout before timing, with output and compiler workers explicitly bound.
fd = a.fd.resolve()
build_env = dict(os.environ, CARGO_TARGET_DIR=str(fd.parents[1]),
                 CARGO_BUILD_JOBS=str(a.workers), RUST_TEST_THREADS=str(a.workers))
subprocess.run(['cargo', 'build', '--jobs', str(a.workers), '-p', 'fd-cli', '--release'],
               cwd=root, env=build_env, check=True)
plan = json.loads((root / 'docs/2026-10-20/series-frames/results/plan.json').read_text())['best']
a.out.mkdir(parents=True, exist_ok=True)
if a.run == 'target':
    # Original PROB-22 defaults: 0.5 wide, rounded planner endpoint, 0.5 decades/s.
    flags = ['--from', '0.5', '--to', '4.542e-28', '--rate', '0.5', '--size', '1280x720']
else:
    # 10 s in the expensive non-zone part of the same target, 300 frames at 1080p.
    flags = ['--from', '1e-20', '--to', '1e-22', '--seconds', '10', '--size', '1920x1080']
cmd = [str(fd), 'film', plan['re'], plan['im'], *flags,
       '--fps', '30', '--keyframes', '2', '--preset', 'master-ice',
       '--threads', str(a.workers), '--json', '--mp4', str(a.out / f'{a.run}.mp4')]
metadata = {'command': cmd, 'cpu_affinity': cpus, 'workers': a.workers,
            'source_commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(),
            'environment': {k: os.environ[k] for k in ['FD_CPU_SERIES', 'FD_ZONE_READBACK',
                              'WGPU_BACKEND', 'VK_ICD_FILENAMES'] if k in os.environ},
            'started_unix': time.time()}
(a.out / f'{a.run}-invocation.json').write_text(json.dumps(metadata, indent=2) + '\n')
print(json.dumps(metadata), flush=True)
with (a.out / f'{a.run}.json').open('w') as report, (a.out / f'{a.run}.stderr').open('w') as log:
    result = subprocess.run(cmd, stdout=report, stderr=log)
metadata['elapsed_seconds'] = time.time() - metadata['started_unix']
metadata['exit_code'] = result.returncode
(a.out / f'{a.run}-invocation.json').write_text(json.dumps(metadata, indent=2) + '\n')
print(json.dumps(metadata), flush=True)
if result.returncode:
    raise SystemExit(result.returncode)
r = json.loads((a.out / f'{a.run}.json').read_text())
assert r['schema'] == 'fd-film/1'
assert sum(v['frames'] for v in r['routes'].values()) == r['frames']
assert abs(sum(v['seconds'] for v in r['routes'].values()) - r['render']['total_seconds']) <= r['render']['total_seconds'] * .05
assert r['frames'] == (1623 if a.run == 'target' else 300), r['frames']
print(json.dumps(r), flush=True)
