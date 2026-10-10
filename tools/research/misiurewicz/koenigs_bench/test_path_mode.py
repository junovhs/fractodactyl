"""Fast, dependency-free checks for PROB-10 path contracts."""
import tempfile
from pathlib import Path
import unittest

import mpmath as mp
from path_mode import accepted_times, covered, read_path, valid


class PathModeTests(unittest.TestCase):
    def setUp(self):
        mp.mp.dps = 180

    def test_path_rotation_optional(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "path"
            p.write_text("# comment\n1 2 3\n1 2 1e-30 0.4\n")
            self.assertEqual(read_path(p),
                             [("1", "2", "3", "0"), ("1", "2", "1e-30", "0.4")])

    def test_centre_never_rounds_to_f64(self):
        c = mp.mpf("0.1234567890123456789012345678901234567890123456789012345")
        self.assertTrue(covered((str(c), "0", "1e-40", "0"), (c, mp.mpf(0)),
                                (1280, 720), mp.mpf("1e-28")))
        self.assertFalse(covered((str(c + mp.mpf("2e-28")), "0", "1e-40", "0"),
                                 (c, mp.mpf(0)), (1280, 720), mp.mpf("1e-28")))

    def test_rotation_uses_fd_y_down_convention(self):
        # One pixel: rotation maps screen y-down to -imaginary at zero angle.
        self.assertTrue(covered(("0", "0", "0.5", "0.3"),
                                (mp.mpf(0), mp.mpf(0)), (2, 2), mp.mpf("0.3")))
        self.assertFalse(covered(("0", "0", "3", "0.3"),
                                 (mp.mpf(0), mp.mpf(0)), (2, 2), mp.mpf("0.3")))

    def test_noncontiguous_passes_are_accepted(self):
        times = [{"frame": i, "fd_seconds": 3.0, "koenigs_seconds": 1.0}
                 for i in (431, 432, 433, 742, 743, 748, 749)]
        scores = [{"valid_shortcut": ok} for ok in
                  (True, False, True, False, True, False, True)]
        accepted = accepted_times(times, scores)
        self.assertEqual([t["frame"] for t in accepted], [431, 433, 743, 749])
        fd = sum(t["fd_seconds"] for t in times)
        projected = fd - sum(t["fd_seconds"] for t in accepted) + sum(
            t["koenigs_seconds"] for t in accepted)
        self.assertEqual(projected, 13.0)
        with self.assertRaises(ValueError):
            accepted_times(times, scores[:-1])

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
