#!/usr/bin/env python3
"""Record and verify a research execution without external dependencies.

Captures command, declared source/input/config bytes, output, exit code,
and provenance fingerprints. Not a proof of mathematical correctness.
"""
import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import uuid

def digest(data):
    return hashlib.sha256(data).hexdigest()

def stable(obj):
    return json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode()

def iso_now():
    return dt.datetime.now(dt.timezone.utc).isoformat()

def declare(files, kind):
    out = []
    for path in files:
        p = Path(path).resolve(strict=True)
        if not p.is_file():
            raise ValueError(f"not a regular file: {p}")
        data = p.read_bytes()
        out.append({"kind": kind, "path": str(p), "sha256": digest(data), "data": data})
    return out

def declared(args):
    if not args.source or not args.config:
        raise ValueError("at least one --source and exactly one --config are required")
    cfg = declare([args.config], "config")
    try:
        json.loads(cfg[0]["data"].decode("utf-8"))
    except (UnicodeError, ValueError) as exc:
        raise ValueError("config must be valid UTF-8 JSON") from exc
    return declare(args.source, "source") + declare(args.input or [], "input") + cfg

def fingerprint(items, command):
    return digest(stable({"files": [{"kind": x["kind"], "path": x["path"], "sha256": x["sha256"]} for x in items], "command": command}))

def safe_file_names(root):
    for p in sorted(root.rglob("*")):
        if p.is_symlink():
            raise ValueError(f"symlink prohibited in run archive: {p}")
        if p.is_file():
            yield p.relative_to(root).as_posix(), p

def record_outputs(folder):
    return {name: digest(path.read_bytes()) for name, path in safe_file_names(folder) if name != "manifest.json"}

def write_manifest(path, doc):
    temp = path.with_suffix(".tmp")
    temp.write_bytes(json.dumps(doc, indent=2, sort_keys=True).encode() + b"\n")
    temp.replace(path)

def do_run(args):
    command = list(args.command)
    if command and command[0] == "--":
        command.pop(0)
    if not command:
        raise ValueError("execution command missing; pass after --")
    items = declared(args)
    fp = fingerprint(items, command)
    run_id = iso_now().replace(":", "-").replace("+00:00", "Z") + "-" + fp[:12] + "-" + uuid.uuid4().hex[:8]
    root = Path(args.out).resolve()
    root.mkdir(parents=True, exist_ok=True)
    folder = root / run_id
    folder.mkdir(mode=0o700, exist_ok=False)
    snapshot = folder / "declared"
    snapshot.mkdir()
    entries = []
    for i, item in enumerate(items):
        name = f"{i:03d}-{item['kind']}-{Path(item['path']).name}"
        rel = f"declared/{name}"
        (snapshot / name).write_bytes(item["data"])
        entries.append({"kind": item["kind"], "path": item["path"], "snapshot": rel, "sha256": item["sha256"]})
    manifest = {"schema": 1, "run_id": run_id, "fingerprint": fp, "started_utc": iso_now(),
                "command": command, "cwd": str(Path.cwd().resolve()), "declared": entries,
                "environment": {"python": sys.version.split()[0], "platform": platform.platform()},
                "status": "RUNNING", "exit_code": None}
    write_manifest(folder / "manifest.json", manifest)
    env = dict(os.environ, DTL_RUN_DIR=str(folder), DTL_RUN_ID=run_id, DTL_FINGERPRINT=fp)
    try:
        with (folder / "stdout.bin").open("wb") as stdout, (folder / "stderr.bin").open("wb") as stderr:
            code = subprocess.run(command, stdout=stdout, stderr=stderr, env=env, check=False).returncode
        unchanged = all(Path(e["path"]).is_file() and digest(Path(e["path"]).read_bytes()) == e["sha256"] for e in entries)
        manifest["exit_code"] = code
        manifest["status"] = "COMPLETE" if code == 0 and unchanged else ("MUTATED_INPUT" if not unchanged else "FAILED")
    except OSError as exc:
        (folder / "stderr.bin").write_text(f"Launcher failure: {exc}\n")
        (folder / "stdout.bin").touch()
        manifest["exit_code"] = -1
        manifest["status"] = "FAILED"
    manifest["finished_utc"] = iso_now()
    manifest["files"] = record_outputs(folder)
    write_manifest(folder / "manifest.json", manifest)
    print(folder / "manifest.json")
    return 0 if manifest["status"] == "COMPLETE" else 1

def verify(manifest_path):
    path = Path(manifest_path).resolve(strict=True)
    doc = json.loads(path.read_text())
    root = path.parent
    if doc.get("schema") != 1 or doc.get("run_id") != root.name:
        raise ValueError("invalid schema or run ID")
    if doc.get("status") != "COMPLETE" or doc.get("exit_code") != 0:
        raise ValueError("run did not complete successfully")
    expected = doc.get("files", {})
    if record_outputs(root) != expected:
        raise ValueError("archive file set or SHA-256 mismatch")
    entries = doc["declared"]
    for e in entries:
        if not e["snapshot"].startswith("declared/") or ".." in Path(e["snapshot"]).parts:
            raise ValueError("bad snapshot path")
        if digest((root / e["snapshot"]).read_bytes()) != e["sha256"]:
            raise ValueError("snapshot integrity failure")
    items = [{"kind": e["kind"], "path": e["path"], "sha256": e["sha256"]} for e in entries]
    if digest(stable({"files": items, "command": doc["command"]})) != doc["fingerprint"]:
        raise ValueError("fingerprint mismatch")
    return doc

def do_check_resume(args):
    doc = verify(args.manifest)
    cp = json.loads(Path(args.checkpoint).read_text())
    if cp.get("run_id") != doc["run_id"] or cp.get("fingerprint") != doc["fingerprint"]:
        raise ValueError("checkpoint from another source/configuration/run; refuse resume")
    print("CHECKPOINT MATCH")
    return 0

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    subs = parser.add_subparsers(dest="action", required=True)
    run = subs.add_parser("run", help="execute one immutable run; output must go to DTL_RUN_DIR")
    run.add_argument("--source", action="append", required=True, help="source file, repeat for all sources")
    run.add_argument("--input", action="append", help="data/input file, repeat as needed")
    run.add_argument("--config", required=True, help="UTF-8 JSON experiment config")
    run.add_argument("--out", required=True, help="run archive root")
    run.add_argument("command", nargs=argparse.REMAINDER)
    check = subs.add_parser("verify", help="verify immutable run archive and status")
    check.add_argument("manifest")
    resume = subs.add_parser("check-resume", help="reject checkpoint not matching a verified run")
    resume.add_argument("manifest")
    resume.add_argument("checkpoint")
    args = parser.parse_args()
    try:
        if args.action == "run":
            return do_run(args)
        if args.action == "verify":
            doc = verify(args.manifest)
            print(f"VERIFIED {doc['run_id']} {doc['fingerprint']}")
            return 0
        if args.action == "check-resume":
            return do_check_resume(args)
    except (ValueError, OSError, KeyError, json.JSONDecodeError) as exc:
        print(f"EVIDENCE GUARD FAILED: {exc}", file=sys.stderr)
        return 1
    return 2

if __name__ == "__main__":
    sys.exit(main())
