"""Synthetic PROB-23 planner contracts; no fd renders required."""
import math
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import numpy as np

import decades


def sample(i, n):
    t = i / (n - 1)
    hero = math.exp(-0.5 * ((t - 0.7) / 0.1) ** 2)
    shift = 1 if i >= n // 2 else 0
    return dict(edge=0.022 + 0.20 * hero, mid=0.20 + 0.48 * hero,
                interior=0.06 + 0.20 * hero, fine=0.09 + shift * 0.18,
                rough=-3.0 + shift * 2.2, aniso=0.08 + shift * 0.22,
                dust=0.02, symmetry=0.12 + 0.75 * hero)


class DirectedArcTests(unittest.TestCase):
    def test_haze_and_empty_are_not_heroes(self):
        shape = sample(7, 10)
        haze = dict(shape, dust=0.65, fine=0.7)
        empty = dict(shape, edge=0.001, mid=0.01, interior=0)
        self.assertGreater(decades.hero_score(shape), decades.hero_score(haze))
        self.assertGreater(decades.hero_score(shape), decades.hero_score(empty))

    def test_smooth_speed_exact_duration_and_calm_landing(self):
        sigs = [sample(i, 60) for i in range(60)]
        pace, elapsed, ramp = decades.design_speeds(sigs, [1] * 60, 10)
        self.assertAlmostEqual(sum(elapsed), 600, delta=1e-7)
        self.assertGreater(ramp * 2 * pace[0], 12)
        self.assertLess(ramp * 2 * pace[0], 17)
        self.assertTrue(all(pace[i] + 1e-8 >= pace[i - 1] for i in range(56, 60)))
        self.assertGreater(np.mean(pace[38:46]), np.mean(pace[:8]))

    def test_sections_and_arc(self):
        sigs = [sample(i, 60) for i in range(60)]
        result = decades.assess(sigs, [1] * 60, 10)
        self.assertTrue(all(29.5 <= s['duration_seconds'] <= 60.5
                            for s in result['sections']))
        self.assertTrue(any(s['transition'] == 'sudden shift'
                            for s in result['sections']))
        self.assertGreater(result['target'][42], result['target'][0])
        self.assertGreater(result['target'][42], result['target'][-1])
        self.assertEqual(sum(s['last_decade'] - s['first_decade'] + 1
                             for s in result['sections']), 60)

    def test_deep_speed_knots_preserve_decimal_width_and_fd_syntax(self):
        with tempfile.TemporaryDirectory() as td:
            from_width = '3.456789123456789e-200'
            end = '1.234567891234567e-260'
            path = decades.write_directed_speed(from_width, end, [10.0] * 61,
                                                    0.70, Path(td))
            knots = [line.split() for line in path.read_text().splitlines()
                     if line and not line.startswith('#')]
            self.assertEqual(knots[0][1], 'rest')
            self.assertEqual(len(knots), 62)
            self.assertAlmostEqual(float(knots[0][0]),
                                   float(decades.mp.log10(decades.mp.mpf(from_width))), places=7)
            self.assertTrue(all(float(a[0]) > float(b[0])
                                for a, b in zip(knots, knots[1:])))
            self.assertTrue(all(float(x[1]) > 0 for x in knots[1:]))

    def test_single_film_command_uses_gpu_and_speed(self):
        with tempfile.TemporaryDirectory() as td, patch.object(decades.subprocess, 'run') as run:
            run.return_value.stdout = '{"frames":3600,"wall_seconds":120}'
            run.return_value.stderr = ''
            final = dict(re='-0.1234567891234567891234', im='0.1',
                         to='1.23e-50', minutes=1)
            decades.run_film(final, '0.5', td, 60, '3840x2160', 'master-ice')
            cmd = run.call_args.args[0]
            self.assertEqual(cmd[1], 'film')
            self.assertEqual(cmd[2], final['re'])
            self.assertEqual(cmd[3], final['im'])
            self.assertEqual(cmd[cmd.index('--gpu') + 1], 'on')
            self.assertEqual(cmd[cmd.index('--speed') + 1], str(Path(td) / 'speed.txt'))
            self.assertEqual(cmd[cmd.index('--fps') + 1], '60')
            self.assertEqual(cmd[cmd.index('--keyframes') + 1], '2')
            self.assertEqual(cmd[cmd.index('--size') + 1], '3840x2160')
            self.assertEqual(cmd[cmd.index('--to') + 1], final['to'])
            self.assertNotIn('--rate', cmd)
            self.assertNotIn('segments', ' '.join(cmd))


if __name__ == '__main__':
    unittest.main()
