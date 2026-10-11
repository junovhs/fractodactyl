#!/usr/bin/env python3
"""Compare unencoded RGB24 frames (before x264). Exit on any >1-channel error."""
import argparse
from pathlib import Path


def main():
    p = argparse.ArgumentParser()
    p.add_argument("cpu_dir")
    p.add_argument("gpu_dir")
    p.add_argument("--size", default="320x180")
    p.add_argument("--max-channel-error", type=int, default=1)
    args = p.parse_args()
    width, height = map(int, args.size.split("x"))
    cpu = sorted(Path(args.cpu_dir).glob("frame-*.rgb"))
    gpu = sorted(Path(args.gpu_dir).glob("frame-*.rgb"))
    if not cpu or [x.name for x in cpu] != [x.name for x in gpu]:
        raise SystemExit(f"FAIL raw parity: mismatched/empty frames: CPU {[x.name for x in cpu]}, GPU {[x.name for x in gpu]}")
    n = width * height * 3
    overall_max = 0
    overall_sum = 0
    overall_samples = 0
    overall_bad = 0
    for a, b in zip(cpu, gpu):
        x, y = a.read_bytes(), b.read_bytes()
        if len(x) != n or len(y) != n:
            raise SystemExit(f"FAIL raw parity {a.name}: expected {n} bytes, CPU={len(x)}, GPU={len(y)}")
        errors = [abs(u - v) for u, v in zip(x, y)]
        maximum = max(errors)
        bad = sum(e > args.max_channel_error for e in errors)
        mean = sum(errors) / n
        overall_max = max(overall_max, maximum)
        overall_sum += sum(errors)
        overall_samples += n
        overall_bad += bad
        print(f"raw {a.name}: max_channel_error={maximum} mean_channel_error={mean:.6f} "
              f"channels_over_{args.max_channel_error}={bad}/{n}", flush=True)
    print(f"raw total: max_channel_error={overall_max} mean_channel_error={overall_sum/overall_samples:.6f} "
          f"channels_over_{args.max_channel_error}={overall_bad}/{overall_samples}", flush=True)
    if overall_bad:
        raise SystemExit(f"FAIL raw parity: {overall_bad} channels differ by more than "
                         f"{args.max_channel_error}; inspect source RGB before x264")
    print("PASS raw parity: every compared RGB channel within tolerance", flush=True)


if __name__ == "__main__":
    main()
