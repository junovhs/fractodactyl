"""PROB-10: full-path, every-pixel Koenigs-zone versus per-frame BLA.

The original Rust koenigs-bench writes only class/nu. Path mode uses fd's
production implementation of the same Koenigs pipeline so fd compare also
scores de, normal and non-finite samples (GATE-01/02). It preserves exact
decimal camera coordinates and fd's rotated unit_offset geometry.
"""
import json
import math
from pathlib import Path
import subprocess
import sys

import mpmath as mp


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


def covered(frame, centre, size, guard):
    """Test the four extremal sample centres, not just the camera centre."""
    re, im, width, rotation = frame
    nx, ny = size
    step = mp.mpf(width) / nx
    co, si = mp.cos(mp.mpf(rotation)), mp.sin(mp.mpf(rotation))
    dx, dy = mp.mpf(re) - centre[0], mp.mpf(im) - centre[1]
    for x in (-mp.mpf(nx - 1) / 2, mp.mpf(nx - 1) / 2):
        for y in (-mp.mpf(ny - 1) / 2, mp.mpf(ny - 1) / 2):
            # fd Plane::unit_offset uses y increasing downward.
            a = dx + step * (co * x + si * y)
            b = dy + step * (si * x - co * y)
            if a * a + b * b > guard * guard:
                return False
    return True


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
    consts = {}
    for line in zone.read_text().splitlines():
        a = line.split()
        if a and a[0] in ("c_exact", "guard"):
            consts[a[0]] = a[1:]
    centre = tuple(mp.mpf(s) for s in consts["c_exact"])
    mant, *exponent = consts["guard"]
    guard = mp.mpf(mant) * (2 ** int(exponent[0]) if exponent else 1)
    indices = [i for i, f in enumerate(path) if covered(f, centre, (nx, ny), guard)]
    if not indices:
        raise RuntimeError("no full frames fit the Koenigs guard")
    if indices != list(range(indices[0], len(path))):
        raise RuntimeError("guard-covered frames are not a deep suffix of the path")

    (out / "outside-path.txt").write_text(lines(path[:indices[0]]))
    (out / "band-path.txt").write_text(lines([path[i] for i in indices]))
    common = ["--size", size, "--iter", str(maxit), "--columns", "nu,de,normal",
              "--threads", str(threads), "--bla", "per-frame", "--runs", str(runs)]
    outside = []
    if indices[0]:
        print(f"== outside guard: {indices[0]} fd frames", flush=True)
        outside = run([fd, "control", str(out / "outside-path.txt"), *common],
                      out / "outside.jsonl")

    times, scores = [], []
    # Compare in bounded chunks; two 1280x720 FDS frames are ~28 MiB.
    # Keeping the entire 300+ frame band in .fds would exhaust Actions disk.
    for start in range(0, len(indices), 8):
        batch = indices[start:start + 8]
        subset = out / "chunk-path.txt"
        subset.write_text(lines([path[i] for i in batch]))
        a, b = out / "fd-ref", out / "zone"
        a.mkdir(exist_ok=True)
        b.mkdir(exist_ok=True)
        print(f"== frames {batch[0]}..{batch[-1]}: fd and Koenigs", flush=True)
        ref = run([fd, "control", str(subset), *common, "-o", str(a)],
                  out / "chunk-fd.jsonl")
        fast = run([fd, "control", str(subset), *common, "--zone", str(zone), "-o", str(b)],
                   out / "chunk-zone.jsonl")
        cmp = subprocess.run([fd, "compare", str(a), str(b)],
                             text=True, capture_output=True, check=False)
        (out / "chunk-compare.jsonl").write_text(cmp.stdout + cmp.stderr)
        compared = [json.loads(s) for s in cmp.stdout.splitlines() if s.startswith("{")
                    and json.loads(s).get("record") == "frame"]
        if len(ref) != len(batch) or len(fast) != len(batch) or len(compared) != len(batch):
            raise RuntimeError("partial scoring: frame count differs")
        for j, index in enumerate(batch):
            t = {"frame": index, "width": path[index][2],
                 "fd_seconds": ref[j]["timing"]["cold_seconds"],
                 "koenigs_seconds": fast[j]["timing"]["cold_seconds"],
                 "zone_used": fast[j].get("zone", {}).get("used", False)}
            s = dict(compared[j], frame=index, width=path[index][2])
            s["valid_shortcut"] = valid(s, t["zone_used"])
            times.append(t)
            scores.append(s)
        for directory in (a, b):
            for file in directory.glob("frame-*.fds"):
                file.unlink()
    (out / "times.jsonl").write_text("".join(json.dumps(r) + "\n" for r in times))
    (out / "scores.jsonl").write_text("".join(json.dumps(r) + "\n" for r in scores))

    # A shortcut may be retained only across a contiguous passing deep suffix.
    accepted = len(scores)
    while accepted and scores[accepted - 1]["valid_shortcut"]:
        accepted -= 1
    accepted_rows = times[accepted:]
    fd_outside = sum(r["timing"]["cold_seconds"] for r in outside)
    fd_band = sum(r["fd_seconds"] for r in times)
    fast_band = sum(r["koenigs_seconds"] for r in times)
    film_fd = fd_outside + fd_band
    accepted_fd = sum(r["fd_seconds"] for r in accepted_rows)
    accepted_fast = sum(r["koenigs_seconds"] for r in accepted_rows)
    projected = film_fd - accepted_fd + accepted_fast
    failures = [s for s in scores if not s["valid_shortcut"]]
    gap = sum(s.get("kinds", {}).get("unresolved_to_interior", 0) for s in scores)
    report = [
        "# PROB-10: v0 path, full-resolution every-pixel comparison",
        "",
        f"- Path: `{source}`; {len(path)} frames; {size}; {threads} threads; "
        f"{runs} cold run(s)/frame; max_iter {maxit}.",
        f"- Guard: {guard} from zone; {len(indices)} full-frame-covered candidates "
        f"(frames {indices[0]}–{indices[-1]}).",
        f"- Passing deep suffix: {len(accepted_rows)} frames; shallowest passing width "
        f"`{path[indices[accepted]][2] if accepted_rows else 'none'}`"
        f" (frame {indices[accepted] if accepted_rows else 'none'}). "
        "Shallower frames remain on fd.",
        f"- Guard-band fd: {fd_band:.3f} s; Koenigs: {fast_band:.3f} s; "
        f"ratio: {fd_band / fast_band:.2f}x.",
        f"- Accepted suffix fd: {accepted_fd:.3f} s; Koenigs: {accepted_fast:.3f} s; "
        f"ratio: {accepted_fd / accepted_fast:.2f}x" if accepted_fast else
        "- No accepted suffix.",
        f"- Whole-film fd: {film_fd:.3f} s; accepted band fraction: "
        f"{accepted_fd / film_fd:.2%}; guard candidate fraction: {fd_band / film_fd:.2%}.",
        f"- Estimated film with accepted Koenigs suffix: {projected:.3f} s; "
        f"end-to-end speed-up: {film_fd / projected:.2f}x.",
        f"- FIX-04 known fd Unresolved → zone Interior: {gap} samples, "
        "reported separately (not counted as shortcut failures).",
        f"- Failing frames (beyond FIX-04): {len(failures)}; "
        f"max nu displacement: {max(s.get('nu', {}).get('px_max', 0) for s in scores):.6g} px.",
        "",
        "| Frame | Width | fd s | Koenigs s | Speed-up | Wrong px | Other class | "
        "FIX-04 | Nonfinite | de over | normal over | Valid |",
        "|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|:---:|",
    ]
    for t, s in zip(times, scores):
        k = s.get("kinds", {})
        fix = k.get("unresolved_to_interior", 0)
        other = s.get("class_mismatches", 0) - fix
        ratio = t["fd_seconds"] / t["koenigs_seconds"]
        report.append(f"| {t['frame']} | {t['width']} | {t['fd_seconds']:.3f} | "
                      f"{t['koenigs_seconds']:.3f} | {ratio:.2f}x | "
                      f"{s.get('nu', {}).get('over_px', '?')} | {other} | {fix} | "
                      f"{s.get('non_finite', '?')} | {s.get('de', {}).get('over_tol', '?')} | "
                      f"{s.get('normal', {}).get('over_tol', '?')} | "
                      f"{'yes' if s['valid_shortcut'] else 'NO'} |")
    (out / "report.md").write_text("\n".join(report) + "\n")
    print("\n".join(report[:14]), flush=True)


if __name__ == "__main__":
    if len(sys.argv) != 8:
        sys.exit("usage: path_mode.py FD OUT PATH NXxNY THREADS RUNS MAX_ITER")
    main(sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4],
         int(sys.argv[5]), int(sys.argv[6]), int(sys.argv[7]))
