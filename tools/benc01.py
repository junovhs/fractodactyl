"""BENC-01 report: summarise the three benchmark arms into one machine-readable JSON.

Reads, from one run directory (docs/spec/BENCH.md "Reproduce"):
  A.jsonl  fd control (independent, plain perturbation)          fd-control/1
  B.jsonl  fd control --bla per-frame (independent, own BLA)     fd-control/1
  C.jsonl  fd play from the compiled atlas                       fd-play/1
  C.compile.log                                                  fd compile stdout
  A.time B.time C.time C.compile.time                            GNU time -v
  cmpAB.jsonl cmpAC.jsonl cmpBC.jsonl                            fd compare (fd-compare/1)
  load.log phases.log                                            1-min load samples
and prints one JSON object. Analysis only (no rendering): Python is allowed here.

usage: python3 tools/benc01.py RUN_DIR > bench/benc-01-results.json
"""
import json
import os
import re
import sys


def jsonl(path):
    with open(path) as f:
        return [json.loads(line) for line in f if line.strip()]


def gnu_time(path):
    text = open(path).read()
    out = {}
    m = re.search(r"Elapsed \(wall clock\) time \(h:mm:ss or m:ss\): (\S+)", text)
    if m:
        parts = [float(x) for x in m.group(1).split(":")]
        secs = 0.0
        for x in parts:
            secs = secs * 60 + x
        out["wall_seconds"] = secs
    m = re.search(r"Maximum resident set size \(kbytes\): (\d+)", text)
    if m:
        out["process_peak_rss_bytes"] = int(m.group(1)) * 1024
    m = re.search(r"User time \(seconds\): (\S+)", text)
    if m:
        out["user_seconds"] = float(m.group(1))
    m = re.search(r"System time \(seconds\): (\S+)", text)
    if m:
        out["system_seconds"] = float(m.group(1))
    m = re.search(r"^exit (\d+)$", text, re.M)
    if m:
        out["exit"] = int(m.group(1))
    return out


def compile_log(path):
    vals = {}
    for line in open(path):
        w = line.split()
        if len(w) == 2:
            vals[w[0]] = w[1]
    keep = [
        "frames", "atlas.bytes", "atlas.chunks", "atlas.bla_share", "orbits", "bla_tables",
        "reuse.frames_per_orbit.mean", "reuse.frames_per_orbit.max", "reuse.frames_per_bla.mean",
        "reuse.frames_per_bla.max", "reuse.frames_without_bla", "tiles", "tile_demand",
        "reuse.frames_per_tile.mean", "reuse.frames_per_tile.max", "reuse.tiles_shared",
        "dedup.ratio.chunks", "dedup.ratio.bytes", "dedup.logical_bytes", "budget.target", "budget.cap",
        "budget.headroom", "budget.within_cap", "seconds.plan", "seconds.estimate", "seconds.build",
        "seconds.reference", "seconds.operator", "seconds.certification", "seconds.manifests",
        "seconds.verify", "seconds.wall", "peak_rss_bytes", "makespan_seconds", "workers",
    ]
    out = {}
    for k in keep:
        if k in vals:
            v = vals[k]
            try:
                out[k] = int(v)
            except ValueError:
                try:
                    out[k] = float(v)
                except ValueError:
                    out[k] = v
    return out


def frame_seconds(rec):
    return rec["runs"][0]["seconds"]


def arm(records, kind):
    frames = [r for r in records if r.get("record") == "frame"]
    totals = [r for r in records if r.get("record") == "totals"][-1]
    secs = [frame_seconds(r) for r in frames]
    oracle = [r for r in frames if r.get("oracle")]
    out = {
        "frames": len(frames),
        "seconds_total": sum(secs),
        "seconds_per_frame": sum(secs) / len(secs),
        "iterations_executed": totals["iterations"]["total"],
        "iterations_per_pixel": totals["iterations"]["per_pixel"],
        "fallback": totals["fallback"],
        "macro_operators_per_pixel": totals["atlas_work"]["macro_operators_per_pixel"],
        "peak_rss_bytes_per_frame_max": totals["memory"]["peak_rss_bytes"],
        "classes": totals["classes"],
        "oracle": {
            "frames": [r["frame"] for r in oracle],
            "passed": sum(1 for r in oracle if r["oracle"]["ok"]),
            "failed_frames": [r["frame"] for r in oracle if not r["oracle"]["ok"]],
            "class_mismatches": sum(r["oracle"]["class_mismatch"] for r in oracle),
            "failures": {str(r["frame"]): r["oracle"]["failures"][:3] for r in oracle if not r["oracle"]["ok"]},
        },
        "ok": totals["ok"],
        "error": totals["error"],
    }
    if kind == "control":
        out["cold_seconds_per_frame"] = out["seconds_per_frame"]
        out["warm_seconds_per_frame"] = None
        out["reference_seconds_total"] = totals["reference_seconds"]["total"]
        out["bytes_read"] = 0
        if "bla" in totals:
            b = totals["bla"]
            out["bla"] = b
            out["operator_seconds_total"] = b["operator_seconds"]["total"]
            out["iterations_equivalent"] = b["iterations_equivalent"]
        else:
            out["operator_seconds_total"] = 0
            out["iterations_equivalent"] = totals["iterations"]["total"]
    else:
        tm = totals["timing"]
        out["cold_seconds_per_frame"] = tm["cold_seconds_per_frame"]
        out["cold_frames"] = tm["cold_frames"]
        out["warm_seconds_per_frame"] = tm["warm_seconds_per_frame"]
        out["warm_frames"] = tm["warm_frames"]
        out["load_seconds"] = totals["load_seconds"]
        out["reference_seconds_total"] = 0
        out["operator_seconds_total"] = 0
        out["bytes_read"] = totals["bytes"]["atlas_bytes_read"]
        out["bytes"] = totals["bytes"]
        out["tiles_touched_per_frame"] = totals["atlas_work"]["tiles_touched_per_frame"]
        out["bla"] = totals["bla"]
        out["iterations_equivalent"] = totals["iterations"]["equivalent"]
    return out, frames


def depth_bins(fa, fb, fc, size=75):
    bins = []
    for s in range(0, len(fa), size):
        e = min(s + size, len(fa))
        sa = sum(frame_seconds(r) for r in fa[s:e])
        sb = sum(frame_seconds(r) for r in fb[s:e])
        sc = sum(frame_seconds(r) for r in fc[s:e])
        n = e - s
        bins.append({
            "frames": [s, e],
            "log10_width": [round(fa[s]["depth"]["log10_width"], 2), round(fa[e - 1]["depth"]["log10_width"], 2)],
            "kernel_A": sorted({r["kernel"].split(" ")[0] for r in fa[s:e]}),
            "A_seconds_per_frame": sa / n,
            "B_seconds_per_frame": sb / n,
            "C_seconds_per_frame": sc / n,
            "B_frames_bla_used": sum(1 for r in fb[s:e] if r["bla"]["use"] == "used"),
            "C_frames_bla_used": sum(1 for r in fc[s:e] if r["play"]["bla_use"] == "used"),
            "A_over_C": sa / sc,
            "B_over_C": sb / sc,
        })
    return bins


def break_even(compile_s, fx, fc):
    """Plays needed for compile + k plays < k independent paths, and, within the first
    play, the first frame count F at which compile + C[0..F) <= X[0..F)."""
    px = sum(frame_seconds(r) for r in fx)
    pc = sum(frame_seconds(r) for r in fc)
    gain = px - pc
    plays = None if gain <= 0 else compile_s / gain
    first, cx, cc = None, 0.0, compile_s
    for f, (a, c) in enumerate(zip(fx, fc)):
        cx += frame_seconds(a)
        cc += frame_seconds(c)
        if first is None and cc <= cx:
            first = f + 1
    return {
        "independent_path_seconds": px,
        "atlas_play_seconds": pc,
        "saving_per_play_seconds": gain,
        "compile_seconds": compile_s,
        "plays_to_break_even": plays,
        "first_play_break_even_frames": first,
        "first_play_break_even_fraction": None if first is None else first / len(fx),
    }


def loads(path):
    vals = []
    for line in open(path):
        w = line.split()
        if len(w) >= 2:
            vals.append(float(w[1]))
    return vals


def main():
    d = sys.argv[1]
    p = lambda n: os.path.join(d, n)
    a, fa = arm(jsonl(p("A.jsonl")), "control")
    b, fb = arm(jsonl(p("B.jsonl")), "control")
    c, fc = arm(jsonl(p("C.jsonl")), "play")
    for x, n in [(a, "A.time"), (b, "B.time"), (c, "C.time")]:
        x["process"] = gnu_time(p(n))
    comp = compile_log(p("C.compile.log"))
    comp["process"] = gnu_time(p("C.compile.time"))
    compile_s = comp["process"]["wall_seconds"]
    cmp = {}
    for n in ["AB", "AC", "BC"]:
        recs = jsonl(p(f"cmp{n}.jsonl"))
        t = recs[-1]
        worst = sorted((r for r in recs[:-1] if "nu" in r), key=lambda r: -r["nu"]["px_max"])[:3]
        cmp[n] = {k: t[k] for k in ["frames", "errors", "frames_class_identical", "frames_bytes_identical", "frames_width_off", "samples", "class_mismatches", "kinds", "nu", "px", "ok"]}
        cmp[n]["frames_with_class_mismatch"] = [r["frame"] for r in recs[:-1] if r.get("class_mismatches", 1) != 0]
        cmp[n]["worst_nu_px_frames"] = [[r["frame"], r["nu"]["px_max"]] for r in worst]
    lv = loads(p("load.log"))
    out = {
        "schema": "fd-benc01/1",
        "path": "bench/path-atlas-v0.txt",
        "settings": {"size": "960x540", "ss": 1, "iter": 100000, "columns": "nu,de,normal", "kernel": "auto", "threads": 12, "compile_workers": 12},
        "machine": {"cpu": "Ryzen 9 3900X, 12 cores / 24 threads", "shared": True,
                    "load_1min": {"min": min(lv), "max": max(lv), "mean": sum(lv) / len(lv), "samples": len(lv)},
                    "phases": open(p("phases.log")).read().strip().splitlines()},
        "arms": {"A_independent_plain": a, "B_independent_per_frame_bla": b, "C_atlas_play": c},
        "compile": comp,
        "speedup": {
            "A_over_C": a["seconds_total"] / c["seconds_total"],
            "B_over_C": b["seconds_total"] / c["seconds_total"],
            "A_over_B": a["seconds_total"] / b["seconds_total"],
        },
        "break_even": {"vs_A": break_even(compile_s, fa, fc), "vs_B": break_even(compile_s, fb, fc)},
        "depth_bins": depth_bins(fa, fb, fc),
        "correctness": cmp,
    }
    json.dump(out, sys.stdout, indent=1)
    print()


if __name__ == "__main__":
    main()
