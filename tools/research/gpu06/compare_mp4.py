#!/usr/bin/env python3
"""Compare selected decoded RGB frames; report per-pixel maximum-channel error."""
import argparse
import subprocess


def frames(path, size, selected):
    expression = "+".join(f"eq(n\\,{i})" for i in selected)
    process = subprocess.run(
        [
            "ffmpeg", "-v", "error", "-i", path,
            "-vf", f"select={expression}", "-vsync", "0",
            "-pix_fmt", "rgb24", "-f", "rawvideo", "-",
        ],
        capture_output=True,
        check=True,
    )
    n = size[0] * size[1] * 3
    if len(process.stdout) != n * len(selected):
        raise ValueError(f"{path}: expected {len(selected)} frames, got {len(process.stdout) // n}")
    return [process.stdout[i * n:(i + 1) * n] for i in range(len(selected))]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("cpu")
    parser.add_argument("gpu")
    parser.add_argument("--size", default="1920x1080")
    parser.add_argument("--frames", default="0,600,1199")
    args = parser.parse_args()
    size = tuple(map(int, args.size.split("x")))
    selected = [int(s) for s in args.frames.split(",")]
    cpu = frames(args.cpu, size, selected)
    gpu = frames(args.gpu, size, selected)
    for index, a, b in zip(selected, cpu, gpu):
        good = sum(
            max(abs(a[i + k] - b[i + k]) for k in range(3)) <= 1
            for i in range(0, len(a), 3)
        )
        ratio = good / (len(a) // 3)
        print(f"frame {index}: <=1 RGB level {ratio:.6%} ({good}/{len(a) // 3} pixels)")
        if ratio < 0.999:
            raise SystemExit(f"FAIL: frame {index} below 99.9%")
    print("PASS: all selected frames >=99.9%")


if __name__ == "__main__":
    main()
