"""Bruin-Kaffl-Schleicher combinatorial Hubbard trees from an external angle (PROB-21).

Kneading sequence, formal triod (majority vote) map and median closure, after Bruin, Kaffl and
Schleicher, "Existence of quadratic Hubbard trees" (Fund. Math. 2009), Proposition 3.5.
Written by ChatGPT (web mode, gpt/PROB-21 round 2); exact, rational-angle combinatorics only.
"""
from fractions import Fraction
from itertools import combinations


def angle_period(a):
    seen = {}
    t = Fraction(a) % 1
    for n in range(1025):
        if t in seen:
            return seen[t], n - seen[t]
        seen[t] = n
        t = 2 * t % 1
    raise ValueError('angle has period over 1024')


def kneading(angle):
    """Critical value itinerary under the critical-ray semicircle partition."""
    a=Fraction(angle)%1
    pre,period=angle_period(a)
    low,high=a/2,(a+1)/2
    chars=[]
    t=a
    for _ in range(pre+period):
        chars.append('*' if t in (low,high) else ('1' if low<t<high else '0'))
        t=2*t%1
    if '*' in chars:
        raise ValueError('critical value ray hits partition boundary: decline')
    return Word(''.join(chars[:pre]), ''.join(chars[pre:]))


class Word:
    """Eventually periodic symbolic address, with exact finite description."""
    def __init__(self, before, period):
        if not period:
            raise ValueError('empty cycle')
        self.before,self.period=before,period

    def at(self, i):
        if i<len(self.before):
            return self.before[i]
        return self.period[(i-len(self.before))%len(self.period)]

    def signature(self, length):
        return ''.join(self.at(k) for k in range(length))

    def shift(self,n):
        if n<len(self.before):
            return Word(self.before[n:],self.period)
        off=(n-len(self.before))%len(self.period)
        return Word('',self.period[off:]+self.period[:off])

    def __repr__(self):
        return self.before+'('+self.period+')'


def triod(a,b,c,nu,cap=4096):
    """Formal triod/majority-vote map (BKS 2009, Proposition 3.5).

    Returns a median itinerary. A repeated formal state gives an eventual
    periodic branch point; a 0,* ,1 stop gives a precritical median.
    """
    current=[a,b,c]
    history={}
    voted=[]
    for k in range(cap):
        sig=tuple(repr(s) for s in current)
        if sig in history:
            start=history[sig]
            return Word(''.join(voted[:start]), ''.join(voted[start:]))
        history[sig]=k
        symbols=[w.at(0) for w in current]
        if set(symbols)=={'0','1','*'}:
            return Word(''.join(voted)+'*'+nu.before,nu.period)
        major='1' if symbols.count('1')>=2 else '0' if symbols.count('0')>=2 else None
        if major is None:
            if symbols.count('*')>=2:
                raise ValueError('ambiguous triod with two critical points: decline')
            major=next(v for v in symbols if v!='*')
        voted.append(major)
        current=[nu if symbol!=major and symbol!='*' else word.shift(1)
                 for word,symbol in zip(current,symbols)]
    raise ValueError('triod iteration cap reached: decline')


def abstract_tree(angle,q,p):
    """Incidence graph from triod medians, not geometric chord minimization."""
    nu=kneading(angle)
    seq=[Word('*'+nu.before,nu.period)]+[nu.shift(j) for j in range(q+p-1)]
    siglen=max(96,8*(q+p+len(nu.before)+len(nu.period)))
    def key(w):return w.signature(siglen)
    nodes={key(w):dict(symbol=repr(w), orbit=i) for i,w in enumerate(seq)}
    words={key(w):w for w in seq}
    if len(nodes)!=len(seq):
        raise ValueError('angle itinerary collapses distinct critical orbit points: decline')
    # Add all triod medians of postcritical points. Closure may require medians of
    # newly discovered branch points; expand until no new marked points occur.
    for _ in range(8):
        before=len(words)
        if before>96:
            raise ValueError('more than 96 abstract tree nodes: decline')
        for a,b,c in combinations(list(words),3):
            middle=triod(words[a],words[b],words[c],nu)
            k=key(middle)
            if k not in words:
                words[k]=middle
                nodes[k]=dict(symbol=repr(middle),orbit=None)
        if len(words)==before:
            break
    else:
        raise ValueError('median closure not reached: decline')
    # Two vertices are adjacent iff no third marked vertex is on their arc.
    # Evaluate median relation directly; avoids embedding-dependent triangulation.
    names=list(words)
    between={}
    for a,b,c in combinations(names,3):
        med=key(triod(words[a],words[b],words[c],nu))
        for x,y,z in ((a,b,c),(a,c,b),(b,c,a)):
            between[frozenset((x,y,z))]=med
    edges=[]
    for a,b in combinations(names,2):
        if not any(between[frozenset((a,b,t))]==t for t in names if t not in (a,b)):
            edges.append((a,b))
    if len(edges)!=len(names)-1:
        raise ValueError('inconsistent abstract tree incidence: decline')
    neighbors={k:[] for k in names}
    for a,b in edges:
        neighbors[a].append(b)
        neighbors[b].append(a)
    if not names or len(set(names))!=len(names):
        raise ValueError('bad tree: decline')
    visited=set(); pending=[names[0]]
    while pending:
        k=pending.pop()
        if k in visited:continue
        visited.add(k);pending.extend(neighbors[k])
    if len(visited)!=len(names):
        raise ValueError('disconnected tree: decline')
    ids={k:i for i,k in enumerate(names)}
    return ([dict(id=ids[k], arms=len(neighbors[k]), **nodes[k]) for k in names],
            [(ids[a],ids[b]) for a,b in edges],repr(nu))
