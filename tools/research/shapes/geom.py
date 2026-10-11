"""PROB-21 numerics: external rays, Misiurewicz points and the geometric Hubbard tree.

Established methods only; every function declines (ValueError) rather than guess:
- parameter rays traced inward (Newton on z_n(c) = R e^{2 pi i 2^(n-1) theta}) and outward
  (the same equation with the potential raised; the angle's binary digits are read off as n
  drops), as in Douady-Hubbard and Heiland-Allen's mandelbrot-numerics;
- the Hubbard tree's regulated arcs by pulling back: when the critical point 0 is not on
  [x, y], f maps [x, y] homeomorphically onto [f x, f y], so [x, y] is the continuous square-root
  lift of [f x, f y] that starts at x. Whether 0 lies on [x, y] is read from the kneading
  partition (the critical diameter of the angle), which is exact for postcritical points;
- Tan Lei's similarity M ~ c0 + S (J_c0 - c0) near a Misiurewicz point c0, with
  S = lim (f^(n-1))'(c0) / (d z_n / dc) (z_1 = c).
Centres stay mpmath values written as decimal strings (DEC-21). Dynamical-plane geometry (the
tree, at scale O(1)) uses complex doubles, which is enough at that scale.
"""
from fractions import Fraction

import mpmath as mp
import numpy as np

import bks

ER = 1e4  # escape radius for ray tracing


def angle_type(t):
    """(preperiod, period) of t under doubling."""
    t = Fraction(t) % 1
    seen = {}
    for n in range(4096):
        if t in seen:
            return seen[t], n - seen[t]
        seen[t] = n
        t = 2 * t % 1
    raise ValueError('angle period over 4096: decline')


def angle_from_digits(pre, cyc):
    """0.pre(cyc) in binary as an exact Fraction."""
    a = int(pre, 2) if pre else 0
    b = int(cyc, 2)
    return (Fraction(a, 2 ** len(pre)) + Fraction(b, 2 ** len(pre) * (2 ** len(cyc) - 1))) % 1


# ---------------------------------------------------------------- parameter rays (inward)

def rays_in(thetas, depth=64, sharp=8, newton=3):
    """Trace many parameter rays inward at once (complex doubles); returns the end points.

    Fine for landing points whose local scale is far above 1e-13; deeper points use ray_out.
    """
    th = [Fraction(t) % 1 for t in thetas]
    m = len(th)
    phase = np.empty((depth + 1, m))
    for j, t in enumerate(th):
        a = t
        for n in range(1, depth + 1):  # z_n has angle 2^(n-1) theta
            phase[n, j] = float(a)
            a = 2 * a % 1
    c = ER * np.exp(2j * np.pi * phase[1])
    live = np.ones(m, bool)  # a ray stops once a full level moves it less than double precision
    with np.errstate(all='ignore'):
        for n in range(1, depth + 1):
            before = c.copy()
            for k in range(1, sharp + 1):
                target = ER ** (2.0 ** (-k / sharp)) * np.exp(2j * np.pi * phase[n])
                for _ in range(newton):
                    z = np.zeros(m, complex)
                    dz = np.zeros(m, complex)
                    for _ in range(n):
                        dz = 2 * z * dz + 1
                        z = z * z + c
                    step = (z - target) / dz
                    c = np.where(live & np.isfinite(step), c - step, c)
            moved = np.abs(c - before)
            live &= np.isfinite(c) & (moved > 1e-14 * (1 + np.abs(c)))
            c = np.where(np.isfinite(c), c, before)
            if not live.any():
                break
    return c


# ---------------------------------------------------------------- Misiurewicz points

def orbit(c, n):
    """z_0..z_n (z_0 = 0, z_1 = c) and dz/dc, in the current mp precision."""
    z = [mp.mpc(0)]
    d = [mp.mpc(0)]
    for _ in range(n):
        d.append(2 * z[-1] * d[-1] + 1)
        z.append(z[-1] ** 2 + c)
    return z, d


def misiurewicz(c, q, n, dps, max_step):
    """Newton on z_{q+n} = z_q (z_1 = c) from c; returns (c, q, r) with q, r minimal.

    q counts from z_1 = c: z_q is the first periodic point (so a ray of preperiod q_a gives
    q = q_a + 1); r is the point's period, which divides the ray period n.
    """
    with mp.workdps(dps):
        c = mp.mpc(c)
        tol = mp.mpf(10) ** (8 - dps)
        start = c
        for _ in range(80):
            z, d = orbit(c, q + n)
            f, df = z[q + n] - z[q], d[q + n] - d[q]
            if df == 0:
                raise ValueError('singular Newton step: decline')
            step = f / df
            c -= step
            if abs(step) < tol * (1 + abs(c)):
                break
        else:
            raise ValueError('Newton did not converge: decline')
        if abs(c - start) > max_step:
            raise ValueError('Newton left the landing neighbourhood: decline')
        z, _ = orbit(c, q + n)
        scale = max(abs(z[q + n] - z[q + n - 1]), mp.mpf(10) ** (-dps // 2))
        eps = tol * 1e6 * (1 + scale)
        r = next(rr for rr in range(1, n + 1) if n % rr == 0 and abs(z[q + rr] - z[q]) < eps)
        qq = next(k for k in range(1, q + 1) if abs(z[k + r] - z[k]) < eps)
        if qq < 2:
            raise ValueError('periodic, not strictly preperiodic: decline')
        return c, qq, r


def local_data(c, q, r, dps, levels=60):
    """Multiplier lambda = (f^r)'(z_q) and Tan Lei scale S (complex, so it also rotates)."""
    with mp.workdps(dps):
        n = q + r * levels
        z, d = orbit(c, n)
        lam = mp.mpc(1)
        for j in range(q, q + r):
            lam *= 2 * z[j]
        if abs(lam) <= 1 + mp.mpf('1e-9'):
            raise ValueError('cycle not repelling: decline')
        dyn = mp.mpc(1)
        for j in range(1, n):
            dyn *= 2 * z[j]
        return z[:q + r], lam, dyn / d[n]


# ---------------------------------------------------------------- parameter rays (outward)

def ray_out(c, dps, sharp=8, newton=3, max_iter=100000):
    """Follow the parameter ray through an escaping c outward; returns its angle's digits d_1...

    Let n be the first iterate with |z_n| > sqrt(ER). The potential is raised on a fixed schedule
    (|z_n| from sqrt(ER) up to ER, keeping arg z_n); then |z_{n-1}| is about sqrt(ER), n drops by
    one, and the half-plane of z_n is digit n of the angle (z_n has angle 2^(n-1) theta).
    """
    digits = {}
    with mp.workdps(dps):
        c = mp.mpc(c)
        z, n = mp.mpc(0), 0
        while abs(z) <= mp.sqrt(ER):
            z, n = z * z + c, n + 1
            if n > max_iter:
                raise ValueError('start point does not escape: decline')
        while n >= 1:
            digits[n] = '1' if mp.im(z) < 0 else '0'
            if n == 1:
                break
            phase = z / abs(z)
            for k in range(1, sharp + 1):
                radius = mp.mpf(ER) ** (mp.mpf(2) ** (mp.mpf(k) / sharp - 1))
                if radius <= abs(z):
                    continue
                for _ in range(newton):
                    zz, d = orbit(c, n)
                    c -= (zz[n] - radius * phase) / d[n]
            n -= 1
            z = orbit(c, n)[0][n]
        return ''.join(digits[k] for k in range(1, max(digits) + 1))


def angles_at(c0, q, r, S, lam, starts=32, cycles=6, max_mult=8):
    """External angles of the Misiurewicz point c0, from rays traced out of nearby points.

    Start points sit on a small circle around c0 (dynamical radius |lambda|^-(cycles*max_mult)/50,
    mapped by S), close enough that each ray shadows c0's for about `cycles` repeats of the longest
    allowed cycle. A digit string is accepted only as exactly 0.pre(cyc), |pre| = q-1, |cyc| a
    multiple of r, with the cycle repeated at least 3 times in the reliable prefix (the last 10
    digits are dropped: there the start point's own orbit departs from c0's).
    Returns {angle: votes}.
    """
    found = {}
    reps = cycles * max_mult
    rho = abs(lam) ** -mp.mpf(reps) / 50
    dps = int(30 + max(0, -mp.log10(abs(S) * rho)))
    with mp.workdps(dps):
        for j in range(starts):
            p = c0 + S * rho * mp.expjpi(mp.mpf(2 * j + 1) / starts)
            try:
                bits = ray_out(p, dps)
            except ValueError:
                continue
            reliable = bits[:max(0, len(bits) - 10)]
            qa, body = q - 1, reliable[q - 1:]
            for mult in range(1, max_mult + 1):
                period = r * mult
                if len(body) < max(3 * period, 12):
                    break
                if all(body[i] == body[i % period] for i in range(len(body))):
                    t = angle_from_digits(reliable[:qa], body[:period])
                    if angle_type(t) == (qa, period):
                        found[t] = found.get(t, 0) + 1
                    break
    return found


# ---------------------------------------------------------------- the geometric Hubbard tree

def _lift(base, start, cpt, spacing):
    """Continuous square-root lift of polyline `base` (z -> z^2 + c inverse) starting at `start`."""
    for _ in range(12):
        w = np.sqrt(base - cpt)
        flip = np.real(w[1:] * np.conj(w[:-1])) < 0
        sign = np.concatenate(([1.0], np.where(np.cumsum(flip) % 2, -1.0, 1.0)))
        w = w * sign
        if abs(w[0] + start) < abs(w[0] - start):
            w = -w
        gaps = np.abs(np.diff(w))
        if gaps.max() <= spacing:
            return w
        # Refine the base where the lift stretches (near the critical value) and lift again.
        pieces = [base[:1]]
        for a, b, g in zip(base[:-1], base[1:], gaps):
            k = int(min(64, np.ceil(g / spacing)))
            pieces.append(a + (b - a) * np.arange(1, k + 1) / k)
        base = np.concatenate(pieces)
    return w


def _resample(path, count):
    seg = np.abs(np.diff(path))
    s = np.concatenate(([0.0], np.cumsum(seg)))
    if s[-1] == 0:
        return np.full(count, path[0])
    u = np.linspace(0, s[-1], count)
    return np.interp(u, s, path.real) + 1j * np.interp(u, s, path.imag)


def _hausdorff(a, b, pieces=160):
    a = a[:: max(1, len(a) // pieces)]
    b = b[:: max(1, len(b) // pieces)]
    d = np.abs(a[:, None] - b[None, :])
    return max(d.min(axis=1).max(), d.min(axis=0).max())


class HubbardTree:
    """Regulated arcs between postcritical points of z^2 + c, c Misiurewicz with angle theta.

    Point j is z_j (z_0 = 0, z_1 = c); its angle is 2^(j-1) theta and its kneading side is
    which open half of the circle, cut at theta/2 and (theta+1)/2, holds that angle.
    """

    def __init__(self, c, theta, q, r, count=600, max_depth=400, tol=1e-6):
        self.c = complex(c)
        self.theta = Fraction(theta) % 1
        self.n = q + r
        self.r = r
        z = [0j]
        for _ in range(self.n):
            z.append(z[-1] ** 2 + self.c)
        self.z = z[:self.n]
        ang = [None, self.theta]
        for _ in range(2, self.n):
            ang.append(2 * ang[-1] % 1)
        self.angle = ang
        lo, hi = self.theta / 2, (self.theta + 1) / 2
        # Every ray at a point other than 0 lies in the same open half, so any angle gives its side.
        self.side = [None] + [1 if lo < a < hi else 0 for a in ang[1:]]
        if any(a in (lo, hi) for a in ang[1:]):
            raise ValueError('postcritical angle on the critical diameter: decline')
        self.image = [1] + [j + 1 if j + 1 < self.n else q for j in range(1, self.n)]
        self.count = count
        self.depth, self.residual = self._solve(max_depth, tol)
        if self.residual > 1e-4:
            raise ValueError(f'regulated arcs did not converge ({self.residual:.2e}): decline')

    def _solve(self, max_depth, tol):
        n, cnt = self.n, self.count
        pairs = [(i, j) for i in range(n) for j in range(n) if i != j]
        arcs = {(i, j): np.linspace(self.z[i], self.z[j], cnt) for i, j in pairs}
        span = max(abs(a - b) for a in self.z for b in self.z) or 1.0
        spacing = 4 * span / cnt
        moved = np.inf
        for depth in range(1, max_depth + 1):
            new = {}
            for m in range(1, n):  # arcs from z_m to the critical point first
                base = arcs[(self.image[m], 1)]
                new[(m, 0)] = _resample(_lift(base, self.z[m], self.c, spacing), cnt)
                new[(0, m)] = new[(m, 0)][::-1]
            for i, j in pairs:
                if 0 in (i, j):
                    continue
                if self.side[i] != self.side[j]:
                    path = np.concatenate((new[(i, 0)], new[(0, j)][1:]))
                else:
                    path = _lift(arcs[(self.image[i], self.image[j])], self.z[i], self.c, spacing)
                new[(i, j)] = _resample(path, cnt)
            if depth % 8 == 0:  # Hausdorff distance: arcs are fractal, so compare as sets
                moved = max(_hausdorff(new[k], arcs[k]) for k in pairs if k[0] < k[1]) / span
            arcs = new
            if moved < tol:
                break
        self.arcs = arcs
        return depth, moved

    def _centre(self, x, y, w):
        """Triod centre of z_x, z_y, z_w, found on [z_x, z_y].

        The centre is the only point common to all three arcs. Along [z_x, z_y] the summed
        distance to the other two arcs is zero there and grows on both sides, so its minimum
        finds the centre even though separately resampled fractal arcs differ in fine wiggles.
        """
        p = self.arcs[(x, y)]
        d = sum(np.abs(p[:, None] - self.arcs[k][None, :]).min(axis=1) for k in ((y, w), (x, w)))
        return p[int(np.argmin(d))]

    def structure(self, rel=2e-3):
        """Place the combinatorial (BKS) tree in the plane.

        Topology and arm counts come from bks.abstract_tree (exact). Each branch vertex is placed
        as the centre of a triod of postcritical points, one in each of three of its arms, found
        from each of the three corners; `spread` is how far those three placements disagree, a
        check of the arcs against the combinatorics. Also returns c's neighbour v1 and the arc
        [c, v1] (the tree edge at c).
        """
        nodes, edges, _ = bks.abstract_tree(self.theta, self.n - self.r, self.r)
        span = max(abs(a - b) for a in self.z for b in self.z)
        tol = rel * span
        adj = {t['id']: [] for t in nodes}
        for a, b in edges:
            adj[a].append(b)
            adj[b].append(a)
        orbit = {t['id']: t['orbit'] for t in nodes}

        def leaf_beyond(start, via):  # a postcritical point in the arm of `start` through `via`
            seen, todo = {start}, [via]
            while todo:
                v = todo.pop()
                if orbit[v] is not None:
                    return orbit[v]
                seen.add(v)
                todo += [w for w in adj[v] if w not in seen]
            raise ValueError('arm without a postcritical point: decline')

        place, spread = {}, 0.0
        for t in nodes:
            if t['orbit'] is not None:
                place[t['id']] = self.z[t['orbit']]
                continue
            x, y, w = (leaf_beyond(t['id'], nb) for nb in adj[t['id']][:3])
            pts = [self._centre(x, y, w), self._centre(y, w, x), self._centre(w, x, y)]
            place[t['id']] = sum(pts) / 3
            spread = max(spread, max(abs(p - place[t['id']]) for p in pts) / span)
        cid = next(t['id'] for t in nodes if t['orbit'] == 1)
        v1 = adj[cid][0]
        far = leaf_beyond(cid, v1)
        path = self.arcs[(1, far)]
        cut = int(np.argmin(np.abs(path - place[v1])))
        edge = path[:cut + 1] if cut > 2 else np.linspace(self.c, place[v1], 32)
        return dict(branch=[dict(point=place[t['id']], arms=t['arms']) for t in nodes if t['arms'] >= 3],
                    first_vertex=place[v1], first_is_branch=orbit[v1] is None, edge=edge,
                    spread=spread, nodes=len(nodes))


def turning(path, pieces=40):
    """Net signed turning (radians) of a polyline at a coarse resolution.

    Regulated arcs in dendrite Julia sets wiggle at every scale, so total absolute turning grows
    without bound with resolution; the net turn of the tangent at a fixed number of pieces (about
    one per 8 pixels of a 320-pixel view) is the stable, view-level quantity.
    """
    p = _resample(np.asarray(path), pieces + 1)
    d = np.diff(p)
    d = d[np.abs(d) > 0]
    if len(d) < 2:
        return 0.0
    return float(np.angle(d[1:] / d[:-1]).sum())
