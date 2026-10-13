"""Standard-library tests for the lab's execution provenance guard."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("research_run.py")

class EvidenceGuardTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.program = self.root / "toy.py"
        self.program.write_text("import json, os\nfrom pathlib import Path\nPath(os.environ['DTL_RUN_DIR'], 'result.json').write_text(json.dumps({'value': 7}))\n")
        self.config = self.root / "config.json"
        self.config.write_text('{"seed":1}')

    def cmd(self, *args):
        return subprocess.run([sys.executable, str(SCRIPT), *map(str, args)], capture_output=True, text=True)

    def perform(self):
        r = self.cmd("run", "--source", self.program, "--config", self.config, "--out", self.root / "evidence", "--", sys.executable, self.program)
        self.assertEqual(r.returncode, 0, r.stderr)
        return Path(r.stdout.strip())

    def test_complete_and_tamper(self):
        manifest = self.perform()
        self.assertEqual(self.cmd("verify", manifest).returncode, 0)
        (manifest.parent / "result.json").write_text("tamper")
        self.assertNotEqual(self.cmd("verify", manifest).returncode, 0)

    def test_checkpoint_is_run_bound(self):
        manifest = self.perform()
        doc = json.loads(manifest.read_text())
        cp = self.root / "cp.json"
        cp.write_text(json.dumps({"run_id": doc["run_id"], "fingerprint": doc["fingerprint"]}))
        self.assertEqual(self.cmd("check-resume", manifest, cp).returncode, 0)
        cp.write_text(json.dumps({"run_id": doc["run_id"], "fingerprint": "incorrect"}))
        self.assertNotEqual(self.cmd("check-resume", manifest, cp).returncode, 0)

    def test_change_config_never_resumes(self):
        first = self.perform()
        self.config.write_text('{"seed":2}')
        second = self.perform()
        a, b = json.loads(first.read_text()), json.loads(second.read_text())
        self.assertNotEqual(a["run_id"], b["run_id"])
        self.assertNotEqual(a["fingerprint"], b["fingerprint"])
        self.assertEqual(self.cmd("verify", first).returncode, 0)
        self.assertEqual(self.cmd("verify", second).returncode, 0)

    def test_detect_mutated_source(self):
        self.program.write_text("from pathlib import Path\nimport os\np=Path(__file__);p.write_text('changed')\n")
        r = self.cmd("run", "--source", self.program, "--config", self.config, "--out", self.root / "evidence", "--", sys.executable, self.program)
        self.assertNotEqual(r.returncode, 0)
        self.assertIn('MUTATED_INPUT', Path(r.stdout.strip()).read_text())

if __name__ == "__main__":
    unittest.main()
