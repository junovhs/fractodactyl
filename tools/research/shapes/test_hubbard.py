"""PROB-21 known cases: rays, Hubbard trees (combinatorial and geometric), arms, views.

Run: python3 -m unittest discover -s tools/research/shapes -p 'test_*.py'
"""
from fractions import Fraction as F
import unittest

import mpmath as mp
import numpy as np

import bks
import geom
import hubbard


def landed(theta):
    qa, n = geom.angle_type(theta)
    with mp.workdps(40):
        return geom.misiurewicz(geom.rays_in([theta])[0], qa + 1, n, 40, mp.mpf('0.05'))


class Rays(unittest.TestCase):
    def test_rays_land_on_known_points(self):
        for theta, c, q, r in ((F(1, 6), 1j, 2, 2), (F(1, 2), -2, 2, 1)):
            got, gq, gr = landed(theta)
            self.assertLess(abs(complex(got) - c), 1e-30)
            self.assertEqual((gq, gr), (q, r))

    def test_ray_out_reads_the_angle(self):
        start = geom.rays_in([F(1, 6)], depth=16)[0]  # a point on the 1/6 ray, outside M
        self.assertEqual(geom.ray_out(start, 30), '0010101010101010')  # 1/6 = 0.0(01)

    def test_partner_rays(self):
        # 9/56 lands where z_4 is the alpha fixed point (3 rays, rotation 1/3): three rays at c.
        self.assertEqual(hubbard.partners(F(9, 56), 4, 1), {F(9, 56), F(11, 56), F(15, 56)})
        self.assertEqual(hubbard.rotation([F(9, 56), F(11, 56), F(15, 56)], 4, 1), (1, 3))
        self.assertEqual(hubbard.partners(F(1, 6), 2, 2), {F(1, 6)})


class Trees(unittest.TestCase):
    def test_combinatorial_trees(self):
        nodes, edges, _ = bks.abstract_tree(F(1, 6), 2, 2)  # c = i: a tripod
        self.assertEqual(sorted(t['arms'] for t in nodes if t['arms'] >= 3), [3])
        nodes, edges, _ = bks.abstract_tree(F(1, 2), 2, 1)  # c = -2: a segment
        self.assertEqual(max(t['arms'] for t in nodes), 2)
        self.assertEqual(len(edges), len(nodes) - 1)

    def test_i_tree_branches_at_alpha(self):
        T = geom.HubbardTree(1j, F(1, 6), 2, 2)
        st = T.structure()
        alpha = (1 - np.sqrt(1 - 4j)) / 2  # the alpha fixed point is the tripod's centre
        self.assertEqual([b['arms'] for b in st['branch']], [3])
        self.assertLess(abs(st['branch'][0]['point'] - alpha), 3e-3)
        self.assertLess(st['spread'], 3e-3)

    def test_minus_two_tree_is_the_real_segment(self):
        T = geom.HubbardTree(-2, F(1, 2), 2, 1)
        self.assertEqual(T.structure()['branch'], [])
        path = T.arcs[(1, 2)]  # [-2, 2] through the critical point
        self.assertLess(np.abs(path.imag).max(), 1e-12)
        self.assertLess(abs(path[0] + 2) + abs(path[-1] - 2), 1e-12)

    def test_geometric_tree_agrees_with_combinatorics(self):
        for theta in (F(1, 4), F(1, 12), F(9, 56), F(13, 40), F(5, 72)):
            c, q, r = landed(theta)
            st = geom.HubbardTree(complex(c), theta, q, r).structure()
            self.assertLess(st['spread'], 5e-3, theta)


class V0(unittest.TestCase):
    def test_v0_is_one_armed_and_twisting(self):
        theta = F(8388607, 25165824)  # recovered by ray_out (hubbard.py --point on v0)
        self.assertEqual(geom.angle_type(theta), (23, 2))
        with mp.workdps(120):
            c, q, r = geom.misiurewicz(mp.mpc(*hubbard.V0), 24, 2, 120, mp.mpf('1e-20'))
            m = hubbard.measure(c, [theta], q, r, tree=False)
        self.assertEqual((m['q'], m['r'], m['arms']), (24, 2, 1))
        self.assertAlmostEqual(m['abs_multiplier'], 1.1532, places=4)
        self.assertAlmostEqual(m['twist_deg'], 27.08, places=2)
        self.assertGreater(m['turns_in_view'], 2)  # a tightly wound one-armed spiral


if __name__ == '__main__':
    unittest.main()
