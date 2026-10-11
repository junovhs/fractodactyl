"""PROB-23 directed planner numerical and CLI contracts (no fd renders)."""
import json
import math
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import mpmath as mp
import numpy as np

import decades


def sample(i, n):
    t = i / (n - 1)
    hero = math.exp(-0.5 * ((t - 0.70) / 0.1) ** 2)
    shift = 1 if i >= n // 2 else 0
    return dict(edge=0.02 + 0.2 * hero, mid=0.20 + 0.48 * hero,
                interior=0.06 + 0.2 * hero, fine=0.09 + shift * 0.18,
                rough=-3 + shift * 2.2, aniso=0.08 + shift * 0.22,
                dust=0.02, symmetry=0.12 + 0.75 * hero,
                focus=0.4 + 0.4 * hero, occupied=0.3 + 0.5 * hero,
                unresolved=0, solid=0.02)


class DirectedArcTests(unittest.TestCase):
    def test_haze_empty_and_off_centre_are_not_heroes(self):
        shape = sample(7, 10)
        haze = dict(shape, dust=0.65, fine=0.7)
        empty = dict(shape, occupied=0.01, mid=0.01, interior=0)
        corner = dict(shape, focus=0.01)
        self.assertGreater(decades.hero_score(shape), decades.hero_score(haze))
        self.assertGreater(decades.hero_score(shape), decades.hero_score(empty))
        self.assertGreater(decades.hero_score(shape), decades.hero_score(corner))

    def test_600_second_speed_integral_hero_linger_and_calm_end(self):
        sigs = [sample(i, 58) for i in range(58)]
        pace, elapsed, ramp = decades.design_speeds(sigs, [1.0] * 58, 10)
        self.assertAlmostEqual(sum(elapsed), 600, delta=1e-7)
        self.assertAlmostEqual(2 * ramp * pace[0], 15, delta=0.2)
        self.assertLess(abs(float(np.mean(pace)) - 10.3), 0.7)
        self.assertGreater(np.mean(pace[36:45]), np.mean(pace[:8]))
        self.assertTrue(all(pace[i] + 1e-8 >= pace[i - 1] for i in range(53, 58)))

    def test_coherent_section_length_and_actual_shift(self):
        sigs = [sample(i, 60) for i in range(60)]
        result = decades.assess(sigs, [1.0] * 60, 10)
        sections = result['sections']
        self.assertTrue(all(3 <= s['last_decade'] - s['first_decade'] + 1 <= 6
                            for s in sections))
        self.assertTrue(all(30 <= s['duration_seconds'] <= 60 for s in sections))
        self.assertTrue(any(s['transition'] == 'sudden shift' for s in sections))
        self.assertEqual(sum(s['last_decade'] - s['first_decade'] + 1
                             for s in sections), 60)
        self.assertGreater(result['target'][42], result['target'][0])
        self.assertGreater(result['target'][42], result['target'][-1])

    def test_bad_climax_and_dust_rank_below_good_curve(self):
        good = [sample(i, 58) for i in range(58)]
        bad = [dict(s, dust=0.55, occupied=0.1, mid=0.12)
               if 32 <= i <= 48 else s for i, s in enumerate(good)]
        self.assertGreater(decades.assess(good, [1] * 58, 10)['score'],
                           decades.assess(bad, [1] * 58, 10)['score'])

    def test_known_deep_nucleus_stays_exact(self):
        with mp.workdps(150):
            re = '-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502'
            im = '0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922'
            result = decades.nucleus(mp.mpc(mp.mpf(re), mp.mpf(im)), mp.mpf('1e-46'), 2000)
            self.assertIsNotNone(result)
            centre, period, size = result
            self.assertEqual(period, 764)
            self.assertTrue(mp.mpf('3e-50') < size < mp.mpf('5e-50'))
            self.assertLess(abs(centre - mp.mpc(mp.mpf(re), mp.mpf(im))), mp.mpf('1e-46'))
            self.assertGreater(mp.log10(4 / (size * 6)), 45)

    def test_deep_speed_format_uses_log_width_only(self):
        with tempfile.TemporaryDirectory() as td:
            start = '3.456789123456789e-200'
            end = '1.234567891234567e-260'
            file = decades.write_directed_speed(start, end, [10.0] * 61,
                                                 0.70, Path(td))
            rows = [x.split() for x in file.read_text().splitlines()
                    if x and not x.startswith('#')]
            self.assertEqual(rows[0][1], 'rest')
            self.assertEqual(len(rows), 62)
            self.assertAlmostEqual(float(rows[0][0]), float(mp.log10(mp.mpf(start))),
                                   delta=1e-7)
            self.assertTrue(all(float(a[0]) > float(b[0])
                                for a, b in zip(rows, rows[1:])))
            self.assertTrue(all(float(x[1]) > 0 for x in rows[1:]))

    def test_ranked_region_seeds_include_elephant_and_spirals(self):
        roots = decades.starting_regions('-0.75', '0.1', '0.5')
        self.assertGreaterEqual(len(roots), 6)
        self.assertIn('elephant', [x[0] for x in roots])
        self.assertIn('triple-spiral', [x[0] for x in roots])
        self.assertEqual(len(decades.starting_regions('-0.75', '0.1', '0.5', False)), 1)

    def test_preview_reuses_exact_ten_minute_speed_curve(self):
        with tempfile.TemporaryDirectory() as td, patch.object(decades.subprocess, 'run') as run:
            run.return_value.stdout = '{"frames":3600,"wall_seconds":120}'
            final = dict(re='-0.123456789123456789', im='0.1', from_width='4',
                         to='1.23e-60', minutes=10)
            decades.run_film(final, td, 60, '3840x2160', 'master-ice', 60, 0.65)
            cmd = run.call_args.args[0]
            self.assertEqual(cmd[1], 'film')
            self.assertEqual(cmd[2], final['re'])
            self.assertEqual(cmd[cmd.index('--speed') + 1], str(Path(td) / 'speed.txt'))
            self.assertEqual(cmd[cmd.index('--gpu') + 1], 'on')
            self.assertEqual(cmd[cmd.index('--keyframes') + 1], '2')
            self.assertEqual(cmd[cmd.index('--fps') + 1], '60')
            self.assertEqual(cmd[cmd.index('--frames') + 1], '23400..27000')
            self.assertEqual(cmd[cmd.index('--mp4') + 1], str(Path(td) / 'preview.mp4'))
            self.assertNotIn('--rate', cmd)

    def test_film_pick_never_replans(self):
        with tempfile.TemporaryDirectory() as td:
            folder = Path(td) / 'cand-3'
            folder.mkdir()
            settings = dict(minutes=10, fps=60, size='3840x2160', preset='master-ice')
            (folder / 'plan.json').write_text(json.dumps(dict(inputs=settings, from_width='4')))
            (folder / 'speed.txt').write_text('dummy')
            args = ['decades.py', '--out', td, '--minutes', '10', '--size',
                    '3840x2160', '--fps', '60', '--film', '--pick', '3']
            with patch.object(sys, 'argv', args), \
                 patch.object(decades, 'plan_directed') as plan, \
                 patch.object(decades, 'run_film') as film:
                decades.main()
                plan.assert_not_called()
                self.assertEqual(film.call_args.args[1], folder)


if __name__ == '__main__':
    unittest.main()
