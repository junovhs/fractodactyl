#!/usr/bin/env python3
"""Blind AUTO-02 corpus gate for the frozen AUTO-01 research compiler.

Runs one cold compiler decision and one cold fd BLA reference per viewport.
A failed jump stops the corpus; a decline runs the untouched fd baseline.
"""
import argparse
from decimal import Decimal, InvalidOperation
import json
import math
from pathlib import Path
import subprocess
import sys

import mpmath as mp
import numpy as np

from auto_discover import load_fd, read_native

HERE = Path(__file__).resolve().parent
FROZEN = {
    "tools/research/misiurewicz/auto_discover.py": "5986085edc4d4b787b53324e1d456ac41ae1f847",
    "tools/research/misiurewicz/auto_cycle_bench/src/main.rs": "bf386403e2250dbc9cf997f96121bccfca28d9ba",
}


def corpus_rows(path, *, subset=False):
    """Validate a fixed set of exact decimal cameras, never via f64."""
    rows = []
    names = set()
    for lineno, line in enumerate(Path(path).read_text().splitlines(), 1):
        line = line.split("#", 1)[0].strip()
        if not line:
            continue
        fields = line.split("|")
        if len(fields) not in (5, 6):
            raise ValueError(f"{path}:{lineno}: expected name|re|im|width|origin[|iter]")
        name, re, im, width, source = fields[:5]
        iterations = int(fields[5]) if len(fields) == 6 else None
        if iterations is not None and not 1 <= iterations <= 1_000_000:
            raise ValueError(f"{path}:{lineno}: iteration cap outside 1..1,000,000")
        if not name or name in names or "/" in name or not source:
            raise ValueError(f"{path}:{lineno}: invalid or duplicate name")
        try:
            coords = [Decimal(value) for value in (re, im, width)]
        except InvalidOperation as error:
            raise ValueError(f"{path}:{lineno}: invalid decimal") from error
        if not all(v.is_finite() for v in coords) or coords[2] <= 0:
            raise ValueError(f"{path}:{lineno}: nonfinite camera or nonpositive width")
        names.add(name)
        rows.append(dict(name=name, re=re, im=im, width=width, origin=source,
                         iterations=iterations))
    if not rows or (not subset and not 10 <= len(rows) <= 20):
        raise ValueError("blind corpus must contain 10..20 distinct views")
    widths = [Decimal(row["width"]) for row in rows]
    if not subset and (max(widths) < Decimal("1e-10") or
                       min(widths) >= Decimal("1e-300")):
        raise ValueError("corpus must span ~1e-10 to below 1e-300")
    return rows


def check_frozen(root):
    for path, sha in FROZEN.items():
        found = subprocess.check_output(
            ["git", "hash-object", path], cwd=root, text=True).strip()
        if found != sha:
            raise RuntimeError(f"AUTO-01 freeze violation: {path} {found} != {sha}")


def frame_counts(reference, n):
    classes = load_fd(reference, n)[0]
    counts = np.bincount(classes, minlength=3)
    return [int(counts[i]) for i in (0, 1, 2)]


def worst_indices(reference, native, n):
    """Worst class, smooth, DE and normal discrepancies over scored pixels."""
    cls, nu, de, normal = load_fd(reference, n)
    ours = read_native(native, n)
    worst = []
    wrong = np.flatnonzero(cls != ours["class"])
    if wrong.size:
        worst.append(int(wrong[0]))
    scored = np.flatnonzero((cls == 0) & (ours["class"] == 0) &
                            (de >= 1e-3))
    if scored.size:
        smooth = np.abs(nu[scored] - ours["nu"][scored]) * math.log(2) * de[scored] / 2
        relative = np.abs(ours["de"][scored] - de[scored]) / de[scored]
        angle = np.abs(np.angle(np.exp(1j * (
            ours["normal"][scored] - normal[scored] * math.tau / 65536))))
        for error in (smooth, relative, angle):
            finite = np.where(np.isfinite(error), error, np.inf)
            worst.append(int(scored[np.argmax(finite)]))
    return sorted(set(worst))


def direct_oracle(row, size, max_iter, indices, fd, native):
    """Direct mpmath orbit and parameter derivative at worst whole-frame pixels."""
    nx, ny = size
    n = nx * ny
    ref_class, ref_nu, ref_de, ref_norm = load_fd(fd, n)
    ours = read_native(native, n)
    width = mp.mpf(row["width"])
    digits = min(1200, max(90, int(-mp.log10(width)) + 65))
    checks = []
    with mp.workdps(digits):
        centre = mp.mpc(row["re"], row["im"])
        h = mp.mpf(row["width"]) / nx
        for idx in indices:
            x, y = idx % nx, idx // nx
            offset = mp.mpc(mp.mpf(x) + mp.mpf("0.5") - nx / 2,
                            ny / 2 - mp.mpf(y) - mp.mpf("0.5"))
            c = centre + h * offset
            z = d = mp.mpc(0)
            truth = None
            for step in range(1, max_iter + 1):
                d = 2 * z * d + 1
                z = z * z + c
                if abs(z) > mp.mpf("1e10"):
                    smooth = step + 1 - mp.log(mp.log(abs(z), 2), 2)
                    distance = 2 * abs(z) * mp.log(abs(z)) / (abs(d) * h)
                    normal = (-mp.arg(z / d)) % (2 * mp.pi)
                    truth = (smooth, distance, normal)
                    break
            record = dict(index=idx, x=x, y=y,
                          oracle_class="escaped" if truth else "unresolved",
                          fd_class=int(ref_class[idx]), candidate_class=int(ours["class"][idx]))
            if truth:
                smooth, distance, normal = truth
                record.update(
                    oracle_nu=float(smooth),
                    fd_nu_error_px=float(abs(mp.mpf(float(ref_nu[idx])) - smooth) *
                                         mp.log(2) * distance / 2),
                    candidate_nu_error_px=float(abs(mp.mpf(float(ours["nu"][idx])) - smooth) *
                                                mp.log(2) * distance / 2),
                    fd_de_relative_error=float(abs(mp.mpf(float(ref_de[idx])) / distance - 1)),
                    candidate_de_relative_error=float(abs(mp.mpf(float(ours["de"][idx])) / distance - 1)),
                    fd_normal_error_rad=float(abs(mp.arg(
                        mp.exp(1j * (mp.mpf(int(ref_norm[idx])) * 2 * mp.pi / 65536 - normal))))),
                    candidate_normal_error_rad=float(abs(mp.arg(
                        mp.exp(1j * (mp.mpf(float(ours["normal"][idx])) - normal))))))
            checks.append(record)
    return checks


def baseline_cold(fd, path, folder, size, threads, iterations):
    """Run fd even on declined cameras; the compiler did not render them."""
    command = [str(fd), "control", str(path), "--size", size,
               "--iter", str(iterations), "--columns", "nu,de,normal",
               "--threads", str(threads), "--bla", "per-frame", "--runs", "1",
               "-o", str(folder / "baseline")]
    result = subprocess.run(command, capture_output=True, text=True)
    (folder / "baseline.log").write_text(result.stdout + "\n" + result.stderr)
    result.check_returncode()
    frames = [json.loads(line) for line in result.stdout.splitlines()
              if '"record":"frame"' in line]
    if len(frames) != 1:
        raise RuntimeError("fd control did not return exactly one frame")
    return float(frames[0]["timing"]["cold_seconds"])


def row_markdown(row):
    if row.get("failed"):
        return f'| {row["name"]} | STOP | {row["width"]} | {row["failed"]} |'
    fmt = lambda v: f"{v:.3f}"
    comparison = row["fd_compare"]
    counts = row["classes"]
    classes = "/".join(str(counts[k]) for k in ("escaped", "interior", "unresolved"))
    errors = (str(comparison.get("class_mismatches", "-")) + "/" +
              str(comparison.get("nu", {}).get("over_px", "-")) + "/" +
              str(comparison.get("de", {}).get("over_tol", "-")) + "/" +
              str(comparison.get("normal", {}).get("over_tol", "-")))
    oracle = [check for check in row["oracle"] if check["oracle_class"] == "escaped"]
    maximum = lambda key: max((check[key] for check in oracle), default=0.0)
    return (f'| {row["name"]} | {row["decision"]} | {row["width"]} | '
            f'{row["q"]}/{row["p"]} | {row["iterations"]} | {fmt(row["discover"])} | {fmt(row["build"])} | '
            f'{fmt(row["render"])} | {fmt(row["total"])} | {fmt(row["bla"])} | '
            f'{row["speedup"]:.2f}x | {row["coverage"]:.1%} | {row["fallback"]:.1%} | '
            f'{classes} | {errors} | {len(row["oracle"])} | '
            f'{maximum("candidate_nu_error_px"):.2g}/'
            f'{maximum("candidate_de_relative_error"):.2g}/'
            f'{maximum("candidate_normal_error_rad"):.2g} | {row["correctness"]} |')


def save_summary(folder, rows, results, failure=None):
    header = [
        "# AUTO-02 blind finite-cycle corpus",
        "",
        "Frozen AUTO-01 code: e42ca0baae4bd2434330f96ca267d2274cd255ef;",
        "base main: da3f9cb95df2c577119b15bbc3c4a763cd37223e.",
        "Every row is one cold invocation at 960x540 unless CLI size was overridden.",
        "Seconds: discover, build, render (jump native or declined BLA),",
        "total (decision + rendered path), same-machine cold fd BLA.",
        "Coverage is jumping pixels / frame; fallback is the rest.",
        "Correctness uses fd compare on all pixels of jumped frames;",
        "a decline is an abstention and renders only through fd BLA.",
        "Worst class/nu/de/normal pixels are checked with direct high-precision",
        "orbit and derivative; their measurements are in report.json.",
        "",
        "| View | Decision | Width | q/p | Iter | Discover s | Build s | Render s | Total s | BLA s | BLA/total | Coverage | Fallback | E/I/U | Bad class/nu/de/normal | Oracle px | Oracle max nu px / DE rel / normal rad | Correctness |",
        "|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---|---:|---|---|",
    ]
    found = {row["name"]: row for row in results}
    for entry in rows:
        if entry["name"] in found:
            header.append(row_markdown(found[entry["name"]]))
        else:
            header.append(f'| {entry["name"]} | not run | {entry["width"]} | - | - | - | - | - | - | - | - | - | not checked |')
    if failure:
        header += ["", "**DEC-14 STOP:** " + failure]
    (folder / "summary.md").write_text("\n".join(header) + "\n")
    (folder / "report.json").write_text(json.dumps(
        dict(frozen=FROZEN, corpus=rows, results=results, failure=failure), indent=2) + "\n")


def run(args):
    root = HERE.parents[2]
    check_frozen(root)
    rows = corpus_rows(args.corpus, subset=args.subset)
    width, height = map(int, args.size.split("x"))
    if width < 1 or height < 1 or args.iter < 1 or args.threads < 1:
        raise ValueError("positive size, iterations and thread count required")
    folder = args.out.resolve()
    folder.mkdir(parents=True, exist_ok=True)
    if (folder / "report.json").exists():
        raise RuntimeError("choose a fresh --out directory for cold runs")
    results = []
    save_summary(folder, rows, results)
    for view in rows:
        work = folder / view["name"]
        work.mkdir()
        budget = view["iterations"] or args.iter
        command = [sys.executable, str(HERE / "auto_discover.py"),
                   "--fd", str(args.fd.resolve()), "--native", str(args.native.resolve()),
                   "--re", view["re"], "--im", view["im"],
                   "--width", view["width"], "--size", args.size,
                   "--iter", str(budget), "--threads", str(args.threads),
                   "--out", str(work)]
        try:
            completed = subprocess.run(command, capture_output=True, text=True)
            (work / "compiler.log").write_text(completed.stdout + "\n" + completed.stderr)
            if not (work / "report.json").exists():
                raise RuntimeError(f"compiler exited {completed.returncode} without report")
            report = json.loads((work / "report.json").read_text())
            decision = report["pre_render_decision"]
            reference = work / "baseline" / "frame-00000.fds"
            if decision == "decline":
                cold = baseline_cold(args.fd.resolve(), work / "path.txt",
                                     work, args.size, args.threads, budget)
            elif decision == "accelerate":
                cold = float(report["baseline_seconds"])
            else:
                raise RuntimeError("unknown pre-render decision")
            counts = frame_counts(reference, width * height)
            jumped = int(report.get("jumps", 0))
            fallback = int(report.get("fallback_samples", width * height))
            if jumped + fallback != width * height:
                raise RuntimeError("pixel accounting mismatch")
            checks = []
            if decision == "accelerate":
                indices = worst_indices(reference, work / "native.bin", width * height)
                checks = direct_oracle(view, (width, height), budget, indices,
                                       reference, work / "native.bin")
            build = float(report.get("build_seconds", 0))
            discover = float(report["discovery_seconds"])
            render = float(report.get("render_seconds", 0)) if decision == "accelerate" else cold
            total = discover + build + render
            comparison = report.get("fd_compare", {})
            correct = "abstained (fd BLA)"
            if decision == "accelerate":
                correct = ("pass" if completed.returncode == 0
                           and report.get("acceptance") == "pass"
                           and comparison.get("ok") is True
                           and comparison.get("class_mismatches") == 0
                           and comparison.get("nu", {}).get("over_px") == 0
                           and comparison.get("de", {}).get("over_tol") == 0
                           and comparison.get("normal", {}).get("over_tol") == 0
                           else "FAIL")
            record = dict(name=view["name"], origin=view["origin"], iterations=budget,
                          width=view["width"], decision=decision,
                          q=report.get("q", "-"), p=report.get("p", "-"),
                          discover=discover, build=build, render=render,
                          total=total, bla=cold, speedup=cold / total,
                          jumps=jumped, coverage=jumped / (width * height),
                          fallback=fallback / (width * height),
                          classes=dict(zip(("escaped", "interior", "unresolved"), counts)),
                          correctness=correct, fd_compare=comparison, oracle=checks,
                          compiler=report)
            results.append(record)
            print(row_markdown(record), flush=True)
            save_summary(folder, rows, results)
            if correct == "FAIL":
                raise RuntimeError("whole-frame fd compare failed")
            for check in checks:
                oracle_escaped = check["oracle_class"] == "escaped"
                candidate_escaped = check["candidate_class"] == 0
                if oracle_escaped != candidate_escaped:
                    raise RuntimeError(f'oracle class mismatch at pixel {check["index"]}')
                if oracle_escaped and (check["candidate_nu_error_px"] > 1e-3 or
                                       check["candidate_de_relative_error"] > 1e-3 or
                                       check["candidate_normal_error_rad"] > 1e-3):
                    raise RuntimeError(f'oracle numeric mismatch at pixel {check["index"]}')
            if completed.returncode:
                raise RuntimeError(f"compiler exit {completed.returncode}")
        except Exception as exc:
            failure = f'{view["name"]}: {exc}'
            if not results or results[-1]["name"] != view["name"]:
                results.append(dict(name=view["name"], width=view["width"],
                                    failed=str(exc)))
            save_summary(folder, rows, results, failure)
            print(f"DEC-14 STOP: {failure}", file=sys.stderr)
            return 2
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", type=Path, default=HERE / "auto_corpus.txt")
    parser.add_argument("--subset", action="store_true",
                        help="accept a smaller committed controls or admitted subset")
    parser.add_argument("--fd", type=Path, required=True)
    parser.add_argument("--native", type=Path, required=True)
    parser.add_argument("--size", default="960x540")
    parser.add_argument("--iter", type=int, default=20000)
    parser.add_argument("--threads", type=int, default=4)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    sys.exit(run(args))


if __name__ == "__main__":
    main()
