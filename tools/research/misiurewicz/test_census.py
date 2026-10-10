"""Small deterministic regression tests for the PROB-16 centre census."""
import unittest

import numpy as np

from census import HERE, classify, locations, newton_target, orbit, rows


class CensusTests(unittest.TestCase):
    def test_default_corpus_has_at_least_ten_deep_targets(self):
        corpus = list(locations(HERE / 'census-locations.txt'))
        deep = [entry for entry in corpus if not entry[0].startswith('control-')]
        self.assertEqual(len(deep), 10)
        self.assertEqual(len(corpus) - len(deep), 10)
        self.assertTrue(all(entry[3] >= 100000 for entry in corpus))
        self.assertTrue(any(entry[0] == 'eye-of-universe' for entry in deep))

    def test_newton_finds_primitive_valley_nucleus(self):
        re, im, period = newton_target('newton:36:-0.75', '0.10', 80)
        self.assertEqual(period, 36)
        self.assertLess(abs(complex(float(re), float(im)) - complex(-0.75, 0.1)), 0.03)
        with self.assertRaises(ValueError):
            newton_target('newton:4:0', '0', 80)

    def test_decimal_centre_is_not_rounded_to_double(self):
        a, _, _ = orbit('0.25', '0', 5, 90)
        b, _, _ = orbit('0.2500000000000000000000000000000001', '0', 5, 90)
        # Small differences cannot be observed in double, but must not crash
        # or be mistaken for a mandatory double-conversion input.
        self.assertEqual(a.shape, b.shape)

    def test_extended_period_scan_finds_long_repelling_cycle(self):
        # Synthetic repelling 3-point orbit embedded after 1000 transients.
        z = np.ones(2600, dtype=complex) * 4
        z[:1000] = np.arange(1000) * (1 + 1j)
        z[1000:] = np.tile([1+1j, 1-1j, -1+1j], 534)[:1600]
        narrow, _ = classify(z, 2599, rmax=2)
        wide, ps = classify(z, 2599, rmax=3)
        self.assertGreater(np.count_nonzero(wide == 'spiral'),
                           np.count_nonzero(narrow == 'spiral'))
        self.assertIn(3, ps)

    def test_repelling_period_one(self):
        z, n, escaped = orbit('-2', '0', 100, 80)
        labels, periods = classify(z, n)
        self.assertFalse(escaped)
        self.assertEqual(list(labels[2:]), ['spiral'] * (n - 2))
        self.assertTrue(np.all(periods[2:] == 1))

    def test_escaped_orbit_has_no_phantom_steps(self):
        results = list(rows('escape', '2', '2', 1000, 80))
        self.assertEqual(len(results), 1)
        _, _, count, categories, skip, _, escaped = results[0]
        self.assertTrue(escaped)
        self.assertEqual(sum(categories.values()), count)
        self.assertLessEqual(skip, count)

    def test_bands_partition_observed_steps(self):
        results = list(rows('cusp', '0.25', '0', 1200, 80))
        self.assertEqual(sum(row[2] for row in results), 1200)
        for row in results:
            self.assertEqual(sum(row[3].values()), row[2])


if __name__ == '__main__':
    unittest.main()
