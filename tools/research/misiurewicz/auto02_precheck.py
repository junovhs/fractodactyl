#!/usr/bin/env python3
"""AUTO-02 baseline-only preflight: fixed iteration rescue and blind admission.

Imports shared corpus/FDS parsing helpers, but does not call discovery,
compile an operator, render a native candidate or inspect compiler decisions.
"""
import argparse
import json
from pathlib import Path
import subprocess
import sys

import numpy as np
from auto_corpus import corpus_rows, load_fd

HERE = Path(__file__).resolve().parent
BUDGETS = (20_000, 40_000, 80_000, 160_000, 320_000, 640_000, 1_000_000)
SIZE = "960x540"
PIXELS = 960 * 540


def classify(path):
    classes = load_fd(path, PIXELS)[0]
    counts = np.bincount(classes, minlength=3)
    if int(sum(counts)) != PIXELS or int(sum(counts[3:])):
        raise RuntimeError("unknown fd class")
    return [int(counts[i]) for i in range(3)]


def render(fd, view, budget, work, threads):
    path = work / "path.txt"
    path.write_text(f'{view["re"]} {view["im"]} {view["width"]} 0\n')
    out = work / f"baseline-{budget}"
    cmd = [str(fd), "control", str(path), "--size", SIZE,
           "--iter", str(budget), "--columns", "nu,de,normal",
           "--bla", "per-frame", "--runs", "1", "--threads", str(threads),
           "-o", str(out)]
    result = subprocess.run(cmd, text=True, capture_output=True)
    (work / f"baseline-{budget}.log").write_text(result.stdout + "\n" + result.stderr)
    result.check_returncode()
    frames = [json.loads(line) for line in result.stdout.splitlines()
              if '"record":"frame"' in line]
    if len(frames) != 1:
        raise RuntimeError("fd control did not return one complete frame")
    counts = classify(out / "frame-00000.fds")
    return dict(iter=budget, counts=counts,
                bla_seconds=float(frames[0]["timing"]["cold_seconds"]))


def admission(measure, min_cost=5.0):
    escaped, interior, unresolved = measure["counts"]
    return (escaped > 0 and interior > 0 and unresolved * 100 < PIXELS and
            measure["bla_seconds"] >= min_cost)


def write_results(out, source, attempts, admitted, failure=None):
    (out / "precheck.json").write_text(json.dumps(
        dict(source=str(source), budget_rule=list(BUDGETS),
             attempts=attempts, admitted=admitted, failure=failure), indent=2) + "\n")
    (out / "admitted.txt").write_text(
        "# Baseline-only precheck; no compiler was consulted.\n" +
        "# name|re|im|width|source|fixed_iterations\n" +
        "".join(f'{row["name"]}|{row["re"]}|{row["im"]}|{row["width"]}|'
                f'{row["origin"]}|{row["iter"]}\n' for row in admitted))
    lines = ["| View | Budget | E/I/U | BLA s | Admission |",
             "|---|---:|---:|---:|---|"]
    for result in attempts:
        for measurement in result["budgets"]:
            cls = "/".join(map(str, measurement["counts"]))
            verdict = ("admit" if result.get("admitted") and
                       measurement == result["budgets"][-1] else "no")
            lines.append(f'| {result["name"]} | {measurement["iter"]} | {cls} | '
                         f'{measurement["bla_seconds"]:.3f} | {verdict} |')
    if failure:
        lines.append("\nSTOP: " + failure)
    (out / "precheck.md").write_text("\n".join(lines) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", choices=("admit", "rescue"), required=True)
    parser.add_argument("--views", type=Path, required=True)
    parser.add_argument("--fd", type=Path, required=True)
    parser.add_argument("--threads", type=int, default=4)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--limit", type=int, default=10,
                        help="maximum admitted fresh views (input order)")
    args = parser.parse_args()
    if args.threads < 1 or not 6 <= args.limit <= 10:
        parser.error("threads >= 1, admission limit 6..10")
    rows = corpus_rows(args.views, subset=True)
    if args.mode == "rescue" and len(rows) != 3:
        parser.error("rescue needs exactly the three frozen blank views")
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    if (out / "precheck.json").exists():
        parser.error("--out already contains a run; use a fresh directory")
    attempts, admitted = [], []
    fd = args.fd.resolve()
    for view in rows:
        work = out / view["name"]
        work.mkdir()
        log = dict(name=view["name"], budgets=[], admitted=False)
        attempts.append(log)
        try:
            budgets = BUDGETS if args.mode == "rescue" else (
                (view["iterations"],) if view["iterations"] is not None else (100_000,))
            for budget in budgets:
                result = render(fd, view, budget, work, args.threads)
                log["budgets"].append(result)
                print(f'{view["name"]} {budget}: E/I/U={result["counts"]} '
                      f'BLA={result["bla_seconds"]:.3f}s', flush=True)
                if args.mode == "rescue":
                    if result["counts"][2] * 100 < PIXELS:
                        log["resolved"] = True
                        log["admitted"] = True
                        admitted.append(dict(view, iter=budget))
                        break
                else:
                    accepted = admission(result) and len(admitted) < args.limit
                    if accepted:
                        selected = dict(view, iter=budget)
                        admitted.append(selected)
                        log["admitted"] = True
            write_results(out, args.views, attempts, admitted)
        except Exception as exc:
            write_results(out, args.views, attempts, admitted,
                          f'{view["name"]}: {exc}')
            raise
    if args.mode == "rescue":
        unresolved = [r["name"] for r in attempts if not r.get("resolved")]
        if unresolved:
            print("CAPPED UNRESOLVED: " + ", ".join(unresolved), file=sys.stderr)
            return 2
    elif len(admitted) < 6:
        print(f"INSUFFICIENT MIXED, EXPENSIVE VIEWS: {len(admitted)}/6",
              file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
