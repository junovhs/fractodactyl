"""Fast, dependency-free checks for PROB-10/PROB-20 path contracts."""
import tempfile
from pathlib import Path
import unittest

import mpmath as mp
from path_mode import judge, ranges, read_path, valid


class PathModeTests(unittest.TestCase):
    def setUp(self):
        mp.mp.dps = 180

    def test_path_rotation_optional(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "path"
            p.write_text("# comment\n1 2 3\n1 2 1e-30 0.4\n")
            self.assertEqual(read_path(p),
                             [("1", "2", "3", "0"), ("1", "2", "1e-30", "0.4")])

    def test_admitted_frames_print_as_ranges(self):
        self.assertEqual(ranges([435, 436, 437, 742, 744, 745]), "435-437, 742, 744-745")
        self.assertEqual(ranges([]), "none")

    def test_only_adjudicated_fd_faults_pass_a_class_dispute(self):
        score = {"class_mismatches": 3,
                 "kinds": {"unresolved_to_interior": 1},
                 "nu": {"over_px": 0}, "non_finite": 0,
                 "de": {"over_tol": 0}, "normal": {"over_tol": 0}}
        fd = [{"fault": "fd"}, {"fault": "fd"}]
        self.assertEqual(judge(score, True, fd, 2), "pass (fd faults)")
        self.assertEqual(judge(score, True, fd[:1] + [{"fault": "zone"}], 2), "FAIL")
        self.assertEqual(judge(score, True, fd, 3), "FAIL")  # one pixel unchecked
        self.assertEqual(judge(score, False, fd, 2), "FAIL")
        score["de"]["over_tol"] = 1
        self.assertEqual(judge(score, True, fd, 2), "FAIL")
        score["de"]["over_tol"] = 0
        score["class_mismatches"] = 1
        self.assertEqual(judge(score, True, [], 0), "pass")

    def test_de_and_normal_failures_abstain(self):
        score = {"class_mismatches": 0,
                 "kinds": {"unresolved_to_interior": 0},
                 "nu": {"over_px": 0}, "non_finite": 0,
                 "de": {"over_tol": 0}, "normal": {"over_tol": 0}}
        self.assertTrue(valid(score, True))
        score["de"]["over_tol"] = 1
        self.assertFalse(valid(score, True))
        score["de"]["over_tol"] = 0
        score["normal"]["over_tol"] = 1
        self.assertFalse(valid(score, True))

    def test_fix04_not_a_shortcut_error(self):
        score = {"class_mismatches": 12,
                 "kinds": {"unresolved_to_interior": 12},
                 "nu": {"over_px": 0}, "non_finite": 0,
                 "de": {"over_tol": 0}, "normal": {"over_tol": 0}}
        self.assertTrue(valid(score, True))
        self.assertFalse(valid(score, False))
        score["class_mismatches"] += 1
        self.assertFalse(valid(score, True))


if __name__ == "__main__":
    unittest.main()
