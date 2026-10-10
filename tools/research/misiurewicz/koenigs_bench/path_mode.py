"""PROB-10/PROB-20: full-path, every-pixel Koenigs-zone versus per-frame BLA.

fd decides each frame before rendering it (DEC-19): `fd control --zone` uses the zone
only where `zone_covers` admits the frame (every sample within the zone's max_dc, and
a bounded first-return truncation shift, PROB-20). Two whole-film executions are timed:
fd alone and the mixed film. Every admitted frame is then rendered by both and scored
with fd compare (class, nu, de, normal, non-finite); disputes beyond the FIX-04 gap are
adjudicated against mpmath with diagnose_pixels.py. Camera centres stay exact decimals.
"""
import json
from pathlib import Path
import subprocess
import sys
import time

import mpmath as mp

from diagnose_pixels import adjudicate, load


def read_path(path):
    frames = []
    for line in Path(path).read_text().splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        fields = line.split()
        if len(fields) not in (3, 4):
            raise ValueError(f"bad path line: {line}")
        re, im, width = fields[:3]
        rot = fields[3] if len(fields) == 4 else "0"
        if mp.mpf(width) <= 0 or not mp.isfinite(mp.mpf(rot)):
            raise ValueError(f"invalid width/rotation: {line}")
        frames.append((re, im, width, rot))
    if not frames:
        raise ValueError("empty path")
    return frames


def lines(frames):
    return "".join(" ".join(f[:3] if f[3] == "0" else f) + "\n" for f in frames)


def run(cmd, dest):
    with Path(dest).open("w") as log:
        p = subprocess.run(cmd, stdout=log, stderr=subprocess.STDOUT, check=False)
    if p.returncode:
        raise RuntimeError(f"{' '.join(map(str, cmd))} exited {p.returncode}; see {dest}")
    records = [json.loads(s) for s in Path(dest).read_text().splitlines() if s.startswith("{")]
    if not records or not records[-1].get("ok", False):
        raise RuntimeError(f"failed or incomplete run: {dest}")
    return [r for r in records if r.get("record") == "frame"]


def valid(score, zone_used):
    if not zone_used or "error" in score:
        return False
    kinds = score["kinds"]
    # FIX-04: a zone may prove Interior where fd times out Unresolved.
    other = score["class_mismatches"] - kinds["unresolved_to_interior"]
    return (
        other == 0 and score["nu"]["over_px"] == 0
        and score["non_finite"] == 0
        and score["de"]["over_tol"] == 0
        and score["normal"]["over_tol"] == 0
    )


def ranges(ids):
    """[1, 2, 3, 7] -> "1-3, 7"."""
    out, start = [], None
    for i, k in enumerate(ids):
        if start is None:
            start = k
        if i + 1 == len(ids) or ids[i + 1] != k + 1:
            out.append(f"{start}" if start == k else f"{start}-{k}")
            start = None
    return ", ".join(out) or "none"


def film(cmd, dest):
    """Run one whole-film fd control execution; its frames and wall seconds."""
    start = time.perf_counter()
    frames = run(cmd, dest)
    return frames, time.perf_counter() - start


def judge(score, zone_used, rows, total):
    """Admitted frame verdict: valid as scored, or valid once every dispute beyond
    FIX-04 is adjudicated as fd's fault (fd max_iter-boundary errors, PROB-18)."""
    if valid(score, zone_used):
        return "pass"
    other = score["class_mismatches"] - score["kinds"]["unresolved_to_interior"]
    values_ok = (score["nu"]["over_px"] == 0 and score["non_finite"] == 0
                 and score["de"]["over_tol"] == 0 and score["normal"]["over_tol"] == 0)
    if (zone_used and values_ok and other > 0 and total == len(rows)
            and all(r["fault"] == "fd" for r in rows)):
        return "pass (fd faults)"
    return "FAIL"


def main(fd, out, source, size, threads, runs, maxit):
    mp.mp.dps = 180
    out = Path(out).resolve()
    out.mkdir(parents=True, exist_ok=True)
    fd = str(Path(fd).resolve())
    nx, ny = map(int, size.split("x"))
    if not (nx > 0 and ny > 0 and threads > 0 and runs > 0 and maxit > 0):
        raise ValueError("invalid size/threads/runs/max_iter")
    path = read_path(source)
    zone = out / "v0.zone"
    maker = Path(__file__).resolve().parent.parent / "make_zone.sh"
    subprocess.run(["bash", str(maker), str(zone)], check=True)
    (out / "path.txt").write_text(lines(path))
    common = ["--size", size, "--iter", str(maxit), "--columns", "nu,de,normal",
              "--threads", str(threads), "--bla", "per-frame"]

    print(f"== fd-only film: {len(path)} frames", flush=True)
    timed = [*common, "--runs", str(runs)]
    base, base_wall = film([fd, "control", str(out / "path.txt"), *timed], out / "fd-film.jsonl")
    print("== mixed film: fd --zone decides each frame before rendering it", flush=True)
    mixed, mixed_wall = film([fd, "control", str(out / "path.txt"), *timed, "--zone", str(zone)],
                             out / "mixed-film.jsonl")
    if len(base) != len(path) or len(mixed) != len(path):
        raise RuntimeError("film frame count differs from the path")
    admitted = [i for i, r in enumerate(mixed) if r.get("zone", {}).get("used")]

    rows_out = []
    # Score admitted frames in chunks; two 1280x720 FDS frames are ~28 MiB.
    for start in range(0, len(admitted), 8):
        batch = admitted[start:start + 8]
        subset = out / "chunk-path.txt"
        subset.write_text(lines([path[i] for i in batch]))
        a, b = out / "fd-ref", out / "zone"
        for directory in (a, b):
            directory.mkdir(exist_ok=True)
            for file in directory.glob("frame-*.fds"):
                file.unlink()
        print(f"== score frames {batch[0]}..{batch[-1]}", flush=True)
        ref = run([fd, "control", str(subset), *common, "--runs", "1", "-o", str(a)],
                  out / "chunk-fd.jsonl")
        fast = run([fd, "control", str(subset), *common, "--runs", "1", "--zone", str(zone),
                    "-o", str(b)], out / "chunk-zone.jsonl")
        cmp = subprocess.run([fd, "compare", str(a), str(b)],
                             text=True, capture_output=True, check=False)
        compared = [json.loads(x) for x in cmp.stdout.splitlines() if x.startswith("{")]
        compared = [x for x in compared if x.get("record") == "frame"]
        if len(ref) != len(batch) or len(fast) != len(batch) or len(compared) != len(batch):
            raise RuntimeError("partial scoring: frame count differs")
        for j, index in enumerate(batch):
            score = compared[j]
            used = fast[j].get("zone", {}).get("used", False)
            rows, total = [], 0
            if not valid(score, used):
                name = f"frame-{j:05d}.fds"
                rows, total = adjudicate(load(a / name), load(b / name), 200)
            verdict = judge(score, used, rows, total)
            rows_out.append(dict(frame=index, width=path[index][2], score=score,
                                 disputes=total, adjudicated=rows, verdict=verdict,
                                 fd_seconds=base[index]["timing"]["cold_seconds"],
                                 mixed_seconds=mixed[index]["timing"]["cold_seconds"]))
    for directory in (out / "fd-ref", out / "zone"):
        for file in directory.glob("frame-*.fds"):
            file.unlink()
    (out / "scores.jsonl").write_text("".join(json.dumps(r) + "\n" for r in rows_out))

    failed = [r for r in rows_out if r["verdict"] == "FAIL"]
    faults = [r for r in rows_out if r["verdict"] == "pass (fd faults)"]
    gap = sum(r["score"]["kinds"]["unresolved_to_interior"] for r in rows_out)
    base_frames = sum(r["timing"]["cold_seconds"] for r in base)
    mixed_frames = sum(r["timing"]["cold_seconds"] for r in mixed)
    report = [
        "# PROB-20: v0 path, frames decided before rendering",
        "",
        f"- Path: `{source}`; {len(path)} frames; {size}; {threads} threads; "
        f"{runs} run(s)/frame; max_iter {maxit}.",
        f"- Admitted by fd's pre-render guard: {len(admitted)} frames ({ranges(admitted)}).",
        f"- Admitted frames failing after adjudication: {len(failed)} "
        f"({ranges([r['frame'] for r in failed])}).",
        f"- Admitted frames whose only disputes are fd faults (mpmath): {len(faults)} "
        f"({ranges([r['frame'] for r in faults])}).",
        f"- fd-only film: {base_wall:.3f} s wall ({base_frames:.3f} s in frames).",
        f"- Mixed film: {mixed_wall:.3f} s wall ({mixed_frames:.3f} s in frames).",
        f"- Measured end-to-end speed-up: {base_wall / mixed_wall:.2f}x wall "
        f"({base_frames / mixed_frames:.2f}x in frames).",
        f"- FIX-04 fd Unresolved -> zone Interior on admitted frames: {gap} samples "
        "(not counted as zone errors).",
        "",
        "| Frame | Width | fd s | Mixed s | Other class | FIX-04 | Nonfinite | "
        "nu over | de over | normal over | Disputes (fd/zone fault) | Verdict |",
        "|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|:---|",
    ]
    for r in rows_out:
        s = r["score"]
        fix = s["kinds"]["unresolved_to_interior"]
        fd_fault = sum(x["fault"] == "fd" for x in r["adjudicated"])
        report.append(
            f"| {r['frame']} | {r['width']} | {r['fd_seconds']:.3f} | {r['mixed_seconds']:.3f} | "
            f"{s['class_mismatches'] - fix} | {fix} | {s['non_finite']} | {s['nu']['over_px']} | "
            f"{s['de']['over_tol']} | {s['normal']['over_tol']} | "
            f"{r['disputes']} ({fd_fault}/{len(r['adjudicated']) - fd_fault}) | {r['verdict']} |")
    for r in rows_out:
        for x in r["adjudicated"]:
            report.append(f"- frame {r['frame']} ({x['x']},{x['y']}): fd class {x['fd_class']}, "
                          f"zone class {x['zone_class']}, mpmath {x['truth']} at n={x['n']}: "
                          f"{x['fault']} fault {json.dumps(x['err'])}")
    (out / "report.md").write_text("\n".join(report) + "\n")
    print("\n".join(report[:11]), flush=True)
    return 1 if failed else 0


if __name__ == "__main__":
    if len(sys.argv) != 8:
        sys.exit("usage: path_mode.py FD OUT PATH NXxNY THREADS RUNS MAX_ITER")
    sys.exit(main(sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4],
                  int(sys.argv[5]), int(sys.argv[6]), int(sys.argv[7])))
