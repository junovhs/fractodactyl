"""AUTO-02 corpus format and cold-gate regressions."""
from decimal import Decimal
from pathlib import Path
import tempfile
import unittest

from auto_corpus import HERE, corpus_rows, row_markdown, save_summary
from auto02_precheck import BUDGETS, PIXELS, admission


class CorpusTests(unittest.TestCase):
    def test_blind_span_and_offcentre_controls(self):
        rows = corpus_rows(HERE / "auto_corpus.txt")
        self.assertEqual(len(rows), 18)
        by_name = {row["name"]: row for row in rows}
        self.assertLess(Decimal(by_name["eye-ultradeep"]["width"]), Decimal("1e-300"))
        self.assertEqual(Decimal(by_name["v0-near"]["width"]), Decimal("1e-10"))
        self.assertNotEqual(Decimal(by_name["i-medium"]["re"]), 0)
        self.assertNotEqual(Decimal(by_name["q32-deep"]["re"]),
                            Decimal("0.4196433776070805662759262823266433002120893730487961233893793197021016110409832128692177097141535071"))

    def test_controls_and_fixed_budgets(self):
        controls = corpus_rows(HERE / "auto02_controls.txt", subset=True)
        self.assertEqual(len(controls), 3)
        self.assertTrue(all(row["iterations"] == 20_000 for row in controls))
        rescue = corpus_rows(HERE / "auto02_rescue.txt", subset=True)
        self.assertEqual({row["name"] for row in rescue},
                         {"v0-deep", "eye-ultradeep", "famous-deep"})
        self.assertEqual(BUDGETS[0], 20_000)
        self.assertEqual(BUDGETS[-1], 1_000_000)

    def test_admission_requires_cost_and_both_classes(self):
        valid = dict(counts=[PIXELS // 2, PIXELS // 2, 0], bla_seconds=5.0)
        self.assertTrue(admission(valid))
        self.assertFalse(admission(dict(valid, counts=[PIXELS, 0, 0])))
        self.assertFalse(admission(dict(valid, counts=[0, PIXELS, 0])))
        self.assertFalse(admission(dict(valid, counts=[
            PIXELS - 5_185, 1, 5_184])))
        self.assertFalse(admission(dict(valid, bla_seconds=4.999)))
        self.assertTrue(admission(dict(valid, counts=[
            PIXELS - 5_184, 2, 5_182])))

    def test_duplicate_or_lossy_input_fails_closed(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "corpus.txt"
            valid = (HERE / "auto_corpus.txt").read_text()
            path.write_text(valid + valid.splitlines()[-1] + "\n")
            with self.assertRaisesRegex(ValueError, "duplicate"):
                corpus_rows(path)
            path.write_text(valid.replace("1e-360", "nan", 1))
            with self.assertRaisesRegex(ValueError, "nonfinite"):
                corpus_rows(path)

    def test_pending_rows_and_failure_are_explicit(self):
        rows = corpus_rows(HERE / "auto_corpus.txt")
        with tempfile.TemporaryDirectory() as tmp:
            folder = Path(tmp)
            save_summary(folder, rows, [], "view-name: mismatch")
            report = (folder / "summary.md").read_text()
            self.assertIn("DEC-14 STOP", report)
            for row in rows:
                self.assertIn("| " + row["name"] + " | not run |", report)
        self.assertIn("| STOP |", row_markdown(dict(
            name="failing-view", width="1e-40", failed="wrong class")))


if __name__ == "__main__":
    unittest.main()
