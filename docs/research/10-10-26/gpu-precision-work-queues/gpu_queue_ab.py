#!/usr/bin/env python3
"""BENC-08 GPU scheduling ablation (unmeasured prototype).

Place this next to the repository's gpu_bench.py and run e.g.
  python gpu_queue_ab.py CONSTS outputs 1920x1080 7 1e-35 1e-38 1e-40 1e-43 1e-46 2e-48 --chunks 4,16,32,64

Requires existing gpu_bench.py, single_precision.py, numpy and pyopencl.
This program imports the exact established reference OpenCL source and splits only
its final `while (1)` escape loop into a bounded inline tail and compacted spill
queue. It DOES NOT use FP32 or change the underlying mathematical operations.
It can isolate the benefit/cost of queue compaction, but alone cannot certify
nu/de/normal under the full Fractodactyl contract (BENC-08 only outputs class/nu).

Timer: wall time from enqueue through queue.finish (includes enqueue/host sync;
excludes one-time zone build and CPU readback). Outputs are copied and checked
outside the timings. Report all six frames and compare to independent oracle.
"""

import argparse
import csv
import math
import os
import sys
import time
from pathlib import Path

import numpy as np
import pyopencl as cl
import pyopencl.cltypes

import gpu_bench as upstream


def create_split_source(src):
    """Preserve original kernel prefix/escape source verbatim.

    Fail closed if the upstream contract is changed (rather than silently
    benchmarking a different algorithm).
    """
    signature_end = '__global uchar *cls, __global double *nu)'
    loop_start = '    while (1) {\n'
    if src.count(signature_end) != 1 or src.count(loop_start) != 1:
        raise RuntimeError('Upstream GPU kernel layout changed; review split manually.')
    head, tail = src.split(loop_start)
    if not tail.endswith('\n}\n'):
        raise RuntimeError('Upstream GPU finish loop has unexpected end.')
    # Keep all constant data and all return-map/Koenigs/tail-chart operations exact.
    modified_head = head.replace(
        signature_end,
        '__global uchar *cls, __global double *nu,\n'
        '                     volatile __global unsigned int *queue_count,\n'
        '                     __global int *queue_index, __global cx *queue_z,\n'
        '                     __global long *queue_n, int chunk)'
    )
    # Use unchanged Mandelbrot escape arithmetic for the inline chunk.
    # Extract escape body's C expression from known source with fail-closed checks.
    escape_start = loop_start + tail
    for term in [
        'if (n >= maxit)', 'z = cmul(z, z) + C;',
        'double a = dot(z, z);', 'if (a > 1e20)',
    ]:
        if term not in escape_start:
            raise RuntimeError(f'Unexpected upstream final loop: missing {term}')
    inline_loop = '''    for (int iter = 0; iter < chunk; ++iter) {
        if (n >= maxit) { cls[gid] = 1; nu[gid] = 0; return; }
        z = cmul(z, z) + C; n++;
        double a = dot(z, z);
        if (a > 1e20) { cls[gid] = 0; nu[gid] = n + 1 - log2(0.5*log2(a)); return; }
    }
    uint offset = atomic_inc(queue_count);
    queue_index[offset] = gid;
    queue_z[offset] = z;
    queue_n[offset] = n;
}

__kernel void finish(__global const int *queue_index,
                     __global const cx *queue_z, __global const long *queue_n,
                     int count, cx C, int maxit,
                     __global uchar *cls, __global double *nu)
{
    int lane = get_global_id(0);
    if (lane >= count) return;
    int gid = queue_index[lane];
    cx z = queue_z[lane];
    long n = queue_n[lane];
'''
    # The finish loop is text-identical to the upstream baseline (one exception:
    # gid now comes from the compacted queue, not get_global_id(0)).
    return modified_head + inline_loop + escape_start


def as_complex_matrix(values):
    return np.ascontiguousarray(
        np.array([[complex(x).real, complex(x).imag] for x in values], np.float64)
    )


def alloc_args(ctx, k, nx, ny):
    mf = cl.mem_flags
    def buf(a):
        return cl.Buffer(ctx, mf.READ_ONLY | mf.COPY_HOST_PTR,
                         hostbuf=np.ascontiguousarray(a))
    d2 = lambda z: cl.cltypes.make_double2(z.real, z.imag)
    deg = int(k['deg'])
    bis = np.zeros((deg + 1) ** 2, np.complex128)
    for a, b, cf in k['bis']:
        bis[a * (deg + 1) + b] = cf
    lcf = np.stack([k['lcf'].real, k['lcf'].imag], -1).astype(np.float64).reshape(-1, 2)
    return [
        buf(as_complex_matrix(bis)), np.int32(deg),
        buf(as_complex_matrix(k['orbit'])),
        buf(as_complex_matrix(k['phi'])), np.int32(len(k['phi'])),
        np.float64(k['scale']), np.float64(k['guard']),
        np.float64(k['r0']), np.int32(int(k['period'])),
        d2(k['c']), d2(k['alpha']), d2(k['z24_minus_alpha']),
        np.float64(math.log(abs(k['rho']))),
        np.float64(math.atan2(k['rho'].imag, k['rho'].real)),
        np.int32(k['nx']), buf(as_complex_matrix(k['ncen'])),
        buf(k['nkid'].astype(np.int32)),
        buf(k['nleaf'].astype(np.int32)),
        buf(as_complex_matrix(k['lcen'])),
        buf(k['lr'].astype(np.float64)), buf(k['ln'].astype(np.int32)),
        buf(lcf),
    ]


def read_outputs(q, cls_g, nu_g, n):
    kinds = np.empty(n, dtype=np.uint8)
    nus = np.empty(n, dtype=np.float64)
    cl.enqueue_copy(q, kinds, cls_g)
    cl.enqueue_copy(q, nus, nu_g)
    q.finish()
    return kinds, nus


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('constants')
    p.add_argument('out_dir')
    p.add_argument('size', help='e.g. 1920x1080')
    p.add_argument('runs', type=int)
    p.add_argument('widths', nargs='+')
    p.add_argument('--chunks', default='4,16,32,64', help='Comma-separated inline step budgets')
    args = p.parse_args()
    nx, ny = map(int, args.size.split('x'))
    N = nx * ny
    chunks = [int(s) for s in args.chunks.split(',')]
    if any(c < 0 for c in chunks) or args.runs < 2:
        p.error('Nonnegative chunks and at least two timing rounds are required.')

    k = upstream.sp.load(args.constants)
    want = os.environ.get('DEVICE', 'NVIDIA').lower()
    available = [(platform, device) for platform in cl.get_platforms()
                 for device in platform.get_devices()]
    selected = [(pl, d) for pl, d in available if want in (pl.name + d.name).lower()]
    if not selected:
        raise RuntimeError(f'No OpenCL device containing {want!r}. Available: '
                           + repr([(pl.name, d.name) for pl, d in available]))
    _, dev = selected[0]
    ctx = cl.Context([dev])
    q = cl.CommandQueue(ctx, dev)
    original = upstream.SRC
    split = create_split_source(original)
    baseline = cl.Program(ctx, original).build(options=['-DPATCH32=0'])
    queue = cl.Program(ctx, split).build(options=['-DPATCH32=0'])
    mf = cl.mem_flags
    constants = alloc_args(ctx, k, nx, ny)
    cls_g = cl.Buffer(ctx, mf.WRITE_ONLY, N)
    nu_g = cl.Buffer(ctx, mf.WRITE_ONLY, 8 * N)
    cnt_g = cl.Buffer(ctx, mf.READ_WRITE, 4)
    ix_g = cl.Buffer(ctx, mf.READ_WRITE, 4 * N)
    z_g = cl.Buffer(ctx, mf.READ_WRITE, 16 * N)
    n_g = cl.Buffer(ctx, mf.READ_WRITE, 8 * N)
    zero = np.zeros(1, dtype=np.uint32)
    count_out = np.zeros(1, dtype=np.uint32)
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    def base_run(width):
        h = np.float64(float(width) / nx)
        t = time.perf_counter()
        baseline.render(q, ((N + 255)//256*256,), (256,),
                        *constants, np.int32(nx), np.int32(ny), h,
                        np.int32(20000), cls_g, nu_g)
        q.finish()
        return (time.perf_counter() - t), 0

    def split_run(width, chunk):
        h = np.float64(float(width) / nx)
        t = time.perf_counter()
        cl.enqueue_fill_buffer(q, cnt_g, zero, 0, 4)
        queue.render(q, ((N + 255)//256*256,), (256,),
                     *constants, np.int32(nx), np.int32(ny), h,
                     np.int32(20000), cls_g, nu_g,
                     cnt_g, ix_g, z_g, n_g, np.int32(chunk))
        cl.enqueue_copy(q, count_out, cnt_g)
        q.finish()
        left = int(count_out[0])
        if left > 0:
            queue.finish(q, ((left + 255)//256*256,), (256,),
                         ix_g, z_g, n_g, np.int32(left),
                         cl.cltypes.make_double2(k['c'].real, k['c'].imag),
                         np.int32(20000), cls_g, nu_g)
            q.finish()
        return (time.perf_counter() - t), left

    print('device:', dev.name)
    print('source: original BENC-08 OpenCL, untouched arithmetic; compact final tail only')
    rows = []
    for width in args.widths:
        # Warm both arms and verify all samples match prior to collecting timings.
        base_run(width)
        gold = read_outputs(q, cls_g, nu_g, N)
        gold[0].tofile(out_dir / f'baseline_width_{width}_class.u8')
        gold[1].tofile(out_dir / f'baseline_width_{width}_nu.f64')
        for chunk in chunks:
            split_run(width, chunk)
            got = read_outputs(q, cls_g, nu_g, N)
            class_bad = int(np.count_nonzero(gold[0] != got[0]))
            finite = np.isfinite(gold[1]) & np.isfinite(got[1])
            finite_diff = abs(gold[1][finite] - got[1][finite])
            maxdiff = float(np.max(finite_diff, initial=0.0))
            nonfinite_bad = int(np.count_nonzero(~finite & (gold[1].view(np.uint64) != got[1].view(np.uint64))))
            # There is no 'acceptable approximate' result here: mathematically
            # unchanged scheduling should produce identical results on this GPU.
            if class_bad or maxdiff or nonfinite_bad:
                raise AssertionError(f'queue({chunk}) differs from GPU FP64 baseline: '
                                     f'class={class_bad} max_nu_diff={maxdiff} nonfinite={nonfinite_bad}')
        timings = {'baseline': []}
        for chunk in chunks:
            timings[f'queue_{chunk}'] = []
        pending = {}
        # Rotating order prevents all the early measurements being at a different
        # temperature/clocks. Include host queue synchronization in frame time.
        modes = list(timings)
        for r in range(args.runs):
            ordered = modes[r % len(modes):] + modes[:r % len(modes)]
            for mode in ordered:
                if mode == 'baseline':
                    t, left = base_run(width)
                else:
                    t, left = split_run(width, int(mode.split('_')[1]))
                timings[mode].append(t * 1e3)
                pending[mode] = left
        med_base = float(np.median(timings['baseline']))
        for mode, samples in timings.items():
            median = float(np.median(samples))
            speed = med_base / median
            row = dict(device=dev.name, width=width, resolution=args.size,
                       mode=mode, chunk=(0 if mode == 'baseline' else int(mode.split('_')[1])),
                       tail_queue_pixels=pending[mode], queue_fraction=pending[mode] / N,
                       median_ms=median, baseline_ms=med_base, speedup=speed,
                       class_mismatches=0, max_baseline_nu_diff=0.0,
                       timings_ms=' '.join(f'{x:.5f}' for x in samples))
            rows.append(row)
            print(f'  {width:>9} {mode:>11} median={median:9.3f} ms '
                  f'speedup={speed:.3f}x overflow={pending[mode]:>9}/{N}')
    filename = out_dir / 'gpu_queue_ab.csv'
    with filename.open('w', newline='') as f:
        writer = csv.DictWriter(f, rows[0].keys())
        writer.writeheader()
        writer.writerows(rows)
    print('wrote', filename)
    print('Next: compare original GPU baseline against high-precision fd samples, '
          'then add DE/normal columns and a certified precision arm.')


if __name__ == '__main__':
    main()