#!/usr/bin/env python3
"""PERF-03 process-cold CPU census; invoke through a leased Ishoo check.

Builds the exact snapshot before measuring. CPU time is child user+system time,
including startup and serialization; wall time covers the same interval. These
process intervals deliberately do not claim device or individual worker timings.
"""
import argparse
import json
import os
from pathlib import Path
import resource
import shutil
import subprocess
import time

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--fd', type=Path, required=True)
p.add_argument('--out', type=Path, required=True)
p.add_argument('--workers', type=int, required=True)
p.add_argument('--label', required=True)
p.add_argument('--runs', type=int, default=3)
a = p.parse_args()
root = Path(__file__).resolve().parents[2]
cpus = sorted(os.sched_getaffinity(0))[:a.workers]
assert a.workers > 0 and len(cpus) == a.workers
os.sched_setaffinity(0, cpus)
fd = a.fd.resolve()
env = dict(os.environ, CARGO_TARGET_DIR=str(fd.parents[1]),
           CARGO_BUILD_JOBS=str(a.workers), RUST_TEST_THREADS=str(a.workers))
subprocess.run(['cargo', 'build', '--jobs', str(a.workers), '-p', 'fd-cli', '--release'],
               cwd=root, env=env, check=True)
out = a.out / a.label
out.mkdir(parents=True, exist_ok=True)
shutil.copy2(fd, out / 'fd')
frozen = [l for l in (root / 'docs/2026-10-20/series-frames/results/frames.txt').read_text().splitlines()
          if l and not l.startswith('#')]
# Exact real-film frames.
frames = [(f'prob22-{i}', frozen[i]) for i in (0, 10, 17, 23)]
meta = {'commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(),
        'cpu_affinity': cpus, 'workers': a.workers, 'frames': frames,
        'cpu_scope': 'child process user+system including startup and serialization'}
(out / 'invocation.json').write_text(json.dumps(meta, indent=2) + '\n')
rows = []
for name, line in frames:
    path = out / f'{name}.path'
    path.write_text(line + '\n')
    for mode in ('none', 'per-frame'):
        for run in range(a.runs):
            prefix = out / f'{name}-{mode}-{run}'
            cmd = [str(fd), 'control', str(path), '--size', '480x270', '--ss', '1',
                   '--iter', '100000', '--columns', 'nu,de,normal', '--threads', str(a.workers),
                   '--bla', mode, '-o', str(prefix)]
            start_cpu = resource.getrusage(resource.RUSAGE_CHILDREN)
            start = time.perf_counter()
            with prefix.with_suffix('.jsonl').open('w') as report, prefix.with_suffix('.stderr').open('w') as log:
                result = subprocess.run(cmd, cwd=root, stdout=report, stderr=log)
            wall = time.perf_counter() - start
            end_cpu = resource.getrusage(resource.RUSAGE_CHILDREN)
            cpu = end_cpu.ru_utime + end_cpu.ru_stime - start_cpu.ru_utime - start_cpu.ru_stime
            assert result.returncode == 0, (cmd, result.returncode)
            report = json.loads(prefix.with_suffix('.jsonl').read_text().splitlines()[0])
            row = dict(frame=name, mode=mode, run=run, wall_seconds=wall,
                       cpu_seconds=cpu, busy_threads=cpu / wall,
                       utilization=cpu / wall / a.workers, report=report)
            rows.append(row)
            print(json.dumps({k: v for k, v in row.items() if k != 'report'}), flush=True)
(out / 'results.json').write_text(json.dumps(rows, indent=2) + '\n')
