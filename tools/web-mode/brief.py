"""Web mode: build a ChatGPT brief for an Ishoo issue (docs/spec/WEB-MODE.md).

usage: brief.py ID [--note FILE] [--out DIR]  ->  DIR/brief-ID.txt (default DIR: $WEB_DIR or /tmp/web-mode)
The brief is head.txt (ISSUEID replaced) + the issue's title and Scope Contract, read
from the Ishoo store (read-only), + an optional note. Nothing is typed by hand.
"""
import argparse, os, re, subprocess
from pathlib import Path

here = Path(__file__).resolve().parent
root = here.parents[1]
ap = argparse.ArgumentParser()
ap.add_argument("id")
ap.add_argument("--note")
ap.add_argument("--out", default=os.environ.get("WEB_DIR", "/tmp/web-mode"))
a = ap.parse_args()
rec = root / ".ishoo/records/issues" / f"{a.id}.rec"
if not rec.exists():  # inside an Ishoo worktree the store lives in the main checkout
    rec = Path(subprocess.run(["git", "-C", str(root), "rev-parse", "--path-format=absolute", "--git-common-dir"],
                              capture_output=True, text=True, check=True).stdout.strip()).parent / ".ishoo/records/issues" / f"{a.id}.rec"
t = subprocess.run(["zstd", "-dc", str(rec)], capture_output=True, text=True, check=True).stdout
title = re.search(r'^title = "(.*)"$', t, re.M).group(1).encode().decode("unicode_escape")
desc = re.search(r'^description_md = """\n(.*?)"""', t, re.M | re.S).group(1)
out = (here / "head.txt").read_text().replace("ISSUEID", a.id) + f"{a.id}: {title}\n\n{desc.strip()}\n"
if a.note:
    out += "\n" + Path(a.note).read_text()
Path(a.out).mkdir(parents=True, exist_ok=True)
dest = Path(a.out) / f"brief-{a.id}.txt"
dest.write_text(out)
print(dest, len(out), "chars")
