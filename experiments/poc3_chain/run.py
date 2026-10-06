import sys, pathlib; sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2]))  # repo root
"""PoC 3 (POC-02): chained hidden swaps vs the real ever-deeper path.

    python experiments/poc3_chain/run.py [--swaps 5] [--seed 0] [--frames-dir DIR] [--control-budget 2400]

Writes tests/blind/round3/: chain.mp4, chain_frames.mp4, control.mp4 (as far as its budget
allowed), timing.csv, timing.png, summary.txt, and answer.txt (swap frames, for afterwards).
"""
import argparse, csv, pickle, subprocess, tempfile, time
import numpy as np
from PIL import Image, ImageDraw
from fractadactyl import chain, library

REPO = pathlib.Path(__file__).resolve().parents[2]
OUT = REPO / "tests" / "blind" / "round3"
FONT = r"drawtext=fontfile='C\:/Windows/Fonts/arial.ttf':text='frame %{n}':x=20:y=20:fontsize=36:fontcolor=white:box=1:boxcolor=black@0.6:boxborderw=8"


def render(name, segs, frames_dir, budget_s=None):
    d = frames_dir / name; d.mkdir(parents=True, exist_ok=True)
    for f in d.glob("f*.png"): f.unlink()
    save = lambda k, rgb: Image.fromarray(rgb).save(d / f"f{k:04d}.png")
    t0 = time.time()
    recs, finished = chain.run(segs, save, budget_s=budget_s, log=lambda m: print(f"[{name}] {m}", flush=True))
    print(f"[{name}] {len(recs)} frames in {(time.time()-t0)/60:.1f} min, finished={finished}", flush=True)
    return recs, finished, d


def cached_plan(path, make):
    """Plans take minutes (the control's grow with depth); cache them with their timing."""
    if path.exists():
        return pickle.loads(path.read_bytes())
    t = time.time(); segs = make(); out = (segs, time.time() - t)
    path.parent.mkdir(parents=True, exist_ok=True); path.write_bytes(pickle.dumps(out))
    return out


def encode(frames, mp4, with_numbers=False):
    vf = "scale=1280:720:flags=lanczos" + ("," + FONT if with_numbers else "")
    subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-framerate", "30", "-i", str(frames / "f%04d.png"),
                    "-vf", vf, "-c:v", "libx264", "-crf", "16", "-pix_fmt", "yuv420p", str(mp4)], check=True)


def seg_means(recs):
    segs = sorted({r["seg"] for r in recs})
    return [(s, float(np.mean([r["secs"] for r in recs if r["seg"] == s])),
             sum(r["seg"] == s for r in recs)) for s in segs]


def chart(series, path, W=1200, H=500, pad=60):
    """Per-frame render seconds for each series; dashed lines mark the fake's swap starts."""
    img = Image.new("RGB", (W, H), "white"); g = ImageDraw.Draw(img)
    n = max(len(r) for _, r, _, _ in series)
    ymax = max(max(x["secs"] for x in r) for _, r, _, _ in series) * 1.05
    X = lambda i: pad + (W - 2 * pad) * i / max(1, n - 1)
    Y = lambda v: H - pad - (H - 2 * pad) * v / ymax
    g.line([(pad, H - pad), (W - pad, H - pad)], fill="black"); g.line([(pad, pad), (pad, H - pad)], fill="black")
    for v in np.linspace(0, ymax, 6):
        g.text((5, Y(v) - 6), f"{v:.1f}s", fill="black"); g.line([(pad, Y(v)), (W - pad, Y(v))], fill="#eeeeee")
    for label, recs, colour, marks in series:
        for i in marks:
            for y in range(pad, H - pad, 8): g.line([(X(i), y), (X(i), y + 4)], fill=colour)
        g.line([(X(r["frame"]), Y(r["secs"])) for r in recs], fill=colour, width=2)
    g.text((pad, 15), "render seconds per frame (dashed: swap starts)", fill="black")
    for j, (label, _, colour, _) in enumerate(series):
        g.text((W - 380, 15 + 16 * j), label, fill=colour)
    g.text((W // 2 - 20, H - pad + 20), "frame", fill="black")
    img.save(path)


def main():
    ap = argparse.ArgumentParser(description="PoC 3: render a chain of hidden swaps and an ever-deeper control, then compare timing.")
    ap.add_argument("--swaps", type=int, default=5, help="number of hidden swaps in the chain")
    ap.add_argument("--seed", type=int, default=0, help="random seed for chain planning")
    ap.add_argument("--frames-dir", default=str(pathlib.Path(tempfile.gettempdir()) / "fractadactyl-out" / "poc3"),
                    help="where to write rendered PNG frames")
    ap.add_argument("--control-budget", type=float, default=2400, help="seconds for control planning+render each")
    a = ap.parse_args()
    OUT.mkdir(parents=True, exist_ok=True); frames_dir = pathlib.Path(a.frames_dir)
    lib = library.load()

    print("== planning chain (twins)", flush=True)
    segs, plan_fake = cached_plan(frames_dir / f"plan_chain_s{a.swaps}_seed{a.seed}.pkl",
                                  lambda: chain.plan(a.swaps, lib, seed=a.seed))
    recs, _, fdir = render("chain", segs, frames_dir)
    swaps = [r["frame"] for r in recs if r["swap_start"]]
    encode(fdir, OUT / "chain.mp4"); encode(fdir, OUT / "chain_frames.mp4", with_numbers=True)
    (OUT / "answer.txt").write_text("swap fade-in starts at frames: " + ", ".join(map(str, swaps)) + "\n")

    print("== planning control (real nested minis, no twins)", flush=True)
    csegs, plan_ctl = cached_plan(frames_dir / f"plan_control_s{a.swaps}_seed{a.seed}.pkl",
                                  lambda: chain.plan(a.swaps, None, seed=a.seed, budget_s=a.control_budget))
    crecs, cdone, cdir = render("control", csegs, frames_dir, budget_s=a.control_budget)
    if crecs: encode(cdir, OUT / "control.mp4")

    with open(OUT / "timing.csv", "w", newline="") as fh:
        w = csv.writer(fh); w.writerow(["series", "frame", "segment", "seconds"])
        for name, rr in (("chain", recs), ("control", crecs)):
            for r in rr: w.writerow([name, r["frame"], r["seg"], f"{r['secs']:.4f}"])
    chart([("chain (hidden swaps)", recs, "#1f77b4", swaps),
           ("control (real depth, no swaps)", crecs, "#d62728", [])], OUT / "timing.png")

    lines = [f"chain: {len(recs)} frames ({len(recs)/30:.1f} s), {len(swaps)} swaps, planning {plan_fake:.0f}s"]
    lines += [f"  chain seg {s}: mean {m:.2f} s/frame over {n} frames" for s, m, n in seg_means(recs)]
    lines += [f"  twin mismatch per swap: " + ", ".join(f"{sg['err']:.2f}%" for sg in segs if sg['err'] is not None)]
    lines += [f"  world |s| per segment: " + ", ".join(f"{abs(sg['X']['s']):.1e}" for sg in segs)]
    lines += [f"control: {len(crecs)} frames, finished={cdone}, planning {plan_ctl:.0f}s for {len(csegs)-1} nested minis"]
    lines += [f"  control seg {s}: mean {m:.2f} s/frame over {n} frames" for s, m, n in seg_means(crecs)]
    lines += [f"  control world |s| per segment: " + ", ".join(f"{abs(sg['X']['s']):.1e}" for sg in csegs)]
    lines += [f"  control world period per segment: " + ", ".join(str(sg['X']['p']) for sg in csegs)]
    (OUT / "summary.txt").write_text("\n".join(lines) + "\n")
    print("\n".join(lines), flush=True)


if __name__ == "__main__":
    main()
