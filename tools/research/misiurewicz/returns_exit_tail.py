import mpmath as mp, math, sys, random
import argparse, json, time
from pathlib import Path
from multiprocessing import Pool
P=764; L=24; THR=mp.mpf('1.7e-25')*mp.mpf(10)**10; DPS=110; MAXIT=400000; R2=mp.mpf(10)**20
mp.mp.dps=DPS
C=mp.mpc('-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502','0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922')
def consts():
    mp.mp.dps=DPS
    c0=mp.mpc('-0.74329189085243020293162432597251075717634855812077658183233','0.131240552308797604770845906581478143077114151158584006731334')
    for it in range(80):
        z=mp.mpc(0); dz=mp.mpc(0); zs=[]; dzs=[]
        for n in range(L+3): zs.append(z); dzs.append(dz); dz=2*z*dz+1; z=z*z+c0
        st=(zs[L+2]-zs[L])/(dzs[L+2]-dzs[L]); c0-=st
        if abs(st)<mp.mpf(10)**-(DPS-5): break
    z=mp.mpc(0); dz=mp.mpc(0); zs=[]; dzs=[]
    for n in range(L+3): zs.append(z); dzs.append(dz); dz=2*z*dz+1; z=z*z+c0
    alpha, beta = zs[L], zs[L+1]
    rho=4*alpha*beta
    dalpha=(2*beta+1)/(1-rho)
    Dp = dzs[L]-dalpha                  # d/dc (z_L(c) - alpha(c)) at c0
    D = mp.mpc(1)
    for i in range(1,L): D*=2*zs[i]     # (f^{L-1})'(z_1=c0): z_1 -> z_L
    return c0, D/Dp, rho
def nu(c, z0=None, n0=0):
    mp.mp.dps=DPS
    z=mp.mpc(0) if z0 is None else z0
    dz=mp.mpc(0)
    for n in range(n0, MAXIT):
        dz=2*z*dz+1; z=z*z+c
        a=z.real*z.real+z.imag*z.imag
        if a>R2:
            az=mp.sqrt(a)
            return float(n+1-mp.log(mp.log(az))/mp.log(2)), 2*az*mp.log(az)/abs(dz), n+1
    return None, None, MAXIT
def work(args):
    mp.mp.dps=DPS
    d, w, c0, K = args
    c=C+d
    a=nu(c)
    # exact orbit to step P+1
    z=mp.mpc(0)
    for i in range(P+1): z=z*z+c
    k=1
    while abs(z-C) < THR and k < 60:      # still hugging the minibrot: one more return
        for i in range(P):
            z=z*z+c
            if abs(z) > 1e5: return ('escaped-in-return',)
        k+=1
    off_C=float(abs(z-C)); off_c0=float(abs(z-c0))
    b=nu(c0, z0=z, n0=k*P+1)
    if a[0] is None or b[0] is None: return (a[0] is None, b[0] is None, None, None, a[2], b[2], 0, k, float(abs(z-C)), float(abs(z-c0)))
    dnu=a[0]-b[0]+764+(k-1)*P
    disp=abs(dnu - 0)*math.log(2)*float(a[1]/(w/480))
    return (False, False, dnu - (k-1)*P, disp, a[2], b[2], float(a[1]/(w/480)), k, off_C, off_c0)
def legacy():
    c0,K,rho=consts()
    print("K = D/Delta' =", mp.nstr(K,10), "|K|", mp.nstr(abs(K),6), flush=True)
    random.seed(1)
    for w in [mp.mpf(s) for s in sys.argv[1:]]:
        pts=[(mp.mpc(random.uniform(-.5,.5)*w, random.uniform(-.28,.28)*w), w, c0, K) for k in range(160)]
        with Pool(18) as pool: res=pool.map(work, pts)
        nesc=sum(1 for r in res if r[0]=='escaped-in-return'); res=[r for r in res if r[0]!='escaped-in-return']
        print(f'   escaped during a return (exact, no mapping): {nesc}')
        ok=[r for r in res if r[2] is not None]; m=len(ok)
        mism=sum(1 for r in res if r[0]!=r[1])
        dn=sorted(r[2] for r in ok); off=dn[m//2]
        disp=sorted(abs(r[2]-off)*math.log(2)*r[6] for r in ok)
        import collections
        bins=collections.defaultdict(list)
        for r in res:
            e=math.floor(math.log10(r[9]))
            if r[2] is None: bins[e].append(('MISMATCH' if r[0]!=r[1] else 'bothin', 0)); continue
            bins[e].append(('ok', abs(r[2]-764)*math.log(2)*r[6]))
        for e in sorted(bins):
            v=bins[e]; errs=sorted(x for t,x in v if t=='ok'); mm=sum(1 for t,_ in v if t=='MISMATCH')
            print(f"   exit |zeta-c0| ~1e{e:<4}: n {len(v):3}  class-mismatch {mm:2}  err max {errs[-1] if errs else float('nan'):.1e} px")
        print(f"w={mp.nstr(w,3):8} class-mismatch {mism}/{len(res)} | nu offset median {off:.6f} | |nu-off| as px p50 {disp[m//2]:.1e} p95 {disp[int(.95*m)]:.1e} max {disp[-1]:.1e} | returns k {min(r[7] for r in ok)}-{max(r[7] for r in ok)} | iters deep {sum(r[4] for r in ok)/m:.0f} shallow {sum(r[5] for r in ok)/m:.0f}", flush=True)


# Independently derived NanoMB-style total-degree biseries. See mathr's
# 2021-05-14_deep_zoom_theory_and_practice.html. No upstream code is ported.
SCALE = mp.mpf('1e-25')
TRUTH = Path(__file__).with_name('return_map_truth.json')
DEPTHS = ['1e-35', '1e-38', '1e-40', '1e-43', '1e-46', '2e-48']
PROBE_MAXIT = 20000
MAP_RADIUS = THR
VALID_RADIUS = mp.mpf('1e-26')  # Empirical degree >= 4 guard, NOT a certified disk.


def encode(z):
    return [mp.nstr(z.real, DPS), mp.nstr(z.imag, DPS)]


def decode(z):
    return mp.mpc(*z)


def freeze_point(args):
    w, x, y = args
    mp.mp.dps = DPS
    w = mp.mpf(w)
    d = mp.mpc(x, y)*w
    c = C+d
    z = c
    dz = mp.mpc(1)
    k = 0
    escaped_in_return = False
    while abs(z-C) < MAP_RADIUS and k < 60:
        for _ in range(P):
            dz = 2*z*dz+1
            z = z*z+c
            if abs(z) > 1e5:
                escaped_in_return = True
                break
        k += 1
        if escaped_in_return:
            break
    global MAXIT
    MAXIT = PROBE_MAXIT
    truth = nu(c)
    return dict(w=str(w), d=encode(d), z=encode(z), dz=encode(dz), k=k,
                nu=truth[0], de=None if truth[1] is None else str(truth[1]),
                iterations=truth[2], escaped_in_return=escaped_in_return)


def freeze(jobs):
    # Fixed centre, corners, axes and off-axis points: no random resampling.
    points = [('.49','.27'), ('-.49','.27'), ('.49','-.27'), ('-.49','-.27'),
              ('.1','.05'), ('-.2','-.1'), ('0','.2'), ('.3','0')]
    args = [(w,x,y) for w in DEPTHS for x,y in points]
    with Pool(jobs) as pool:
        rows = pool.map(freeze_point, args)
    TRUTH.write_text(json.dumps(dict(dps=DPS, period=P, maxit=PROBE_MAXIT,
                                    centre=encode(C), radius=str(MAP_RADIUS), rows=rows), indent=2)+'\n')
    print(f'frozen {len(rows)} direct-iteration points at {DPS} dps: {TRUTH}')


def build_map(degree):
    import numpy as np
    start = time.perf_counter()
    # z=Z+s*t, c=C+s*v => t_next=2*Z*t+s*t*t+v.
    # Truncate only total degree > degree, retaining every mixed term.
    a = np.zeros((degree+1,degree+1), dtype=complex)
    a[1,0] = 1
    terms = [(i,j) for i in range(degree+1) for j in range(degree+1-i) if i+j]
    Z = C
    s = float(SCALE)
    for _ in range(P):
        b = 2*complex(Z)*a
        for i,j in terms:
            for h,l in terms:
                if i+h+j+l <= degree:
                    b[i+h,j+l] += s*a[i,j]*a[h,l]
        b[0,1] += 1
        a = b
        Z = Z*Z+C
    # Preserve the tiny nucleus residual at high precision, never float(C).
    return [(i,j,encode(mp.mpc(a[i,j]))) for i,j in terms], encode((Z-C)/SCALE), time.perf_counter()-start


def map_point(args):
    row, terms, bias, degree = args
    mp.mp.dps = DPS
    global MAXIT
    MAXIT = PROBE_MAXIT
    d = decode(row['d']); v = d/SCALE; u = v
    coeff = [(i,j,decode(a)) for i,j,a in terms]
    bias = decode(bias)
    max_input = mp.mpf(0)
    rejected = False
    try:
        for _ in range(row['k']):
            max_input = max(max_input, abs(u)*SCALE)
            rejected |= abs(u)*SCALE > VALID_RADIUS
            up = [u**i for i in range(degree+1)]
            vp = [v**i for i in range(degree+1)]
            u = bias+sum(a*up[i]*vp[j] for i,j,a in coeff)
            if not mp.isfinite(abs(u)) or abs(u)*SCALE > mp.mpf('1e5'):
                return dict(w=row['w'], domain_fail=True, rejected=rejected, mismatch=False, px=None,
                            state_px=None, radius=float(max_input), k=row['k'])
        z = C+SCALE*u
        if row['escaped_in_return']:
            # A whole-period map cannot expose the within-period escape step.
            # Keep these points in the denominator, requiring raw fallback.
            return dict(w=row['w'], domain_fail=True, rejected=rejected, mismatch=False, px=None,
                        state_px=None, radius=float(max_input), k=row['k'])
        # Local equivalent input displacement, independent of tail error.
        state_px = abs(z-decode(row['z']))/abs(decode(row['dz']))/(mp.mpf(row['w'])/480)
        result = nu(C+d, z, row['k']*P+1)
        mismatch = (result[0] is None) != (row['nu'] is None)
        px = None if mismatch or row['nu'] is None else abs(result[0]-row['nu'])*math.log(2)*float(mp.mpf(row['de'])/(mp.mpf(row['w'])/480))
        return dict(w=row['w'], domain_fail=False, rejected=rejected, mismatch=mismatch, px=px,
                    state_px=float(state_px), radius=float(max_input), k=row['k'])
    except (OverflowError, ZeroDivisionError):
        return dict(w=row['w'], domain_fail=True, rejected=rejected, mismatch=False, px=None,
                    state_px=None, radius=float(max_input), k=row['k'])


def load_truth():
    """Validate and load the unchanged PROB-03 fixture."""
    truth = json.loads(TRUTH.read_text())
    if (truth['centre'] != encode(C) or truth['period'] != P or
            truth['dps'] != DPS or truth['maxit'] != PROBE_MAXIT or
            truth['radius'] != str(MAP_RADIUS)):
        raise ValueError('incompatible frozen truth')
    return truth


def probe(jobs):
    start = time.perf_counter()
    truth = load_truth()
    print(f'candidate radius={mp.nstr(VALID_RADIUS)} (empirical); original return radius={mp.nstr(MAP_RADIUS)}; outside-guard evaluations are diagnostics only')
    print('degree width build_s max_px max_state_px mismatch evaluated fallback max_input_radius ops_per_return map_return_ops_per_px raw_return_ops_per_px score', flush=True)
    for degree in [1,2,4,6,8,10,12]:
        terms, bias, build_s = build_map(degree)
        with Pool(jobs) as pool:
            results = pool.map(map_point, [(row,terms,bias,degree) for row in truth['rows']])
        # Dense monomial evaluation: powers + two multiplies per term, plus
        # output scale. Raw step has one complex square; scalar cost differs.
        cmuls = 2*(degree-1)+2*len(terms)+1
        # Real arithmetic model: complex multiply=6, square=4, addition=2.
        ops = 6*cmuls+2*len(terms)+2
        raw_ops = 6*P
        scores = []
        for w in DEPTHS:
            rows = [r for r in results if mp.mpf(r['w']) == mp.mpf(w)]
            px = max((r['px'] for r in rows if r['px'] is not None), default=math.inf)
            state = max((r['state_px'] for r in rows if r['state_px'] is not None), default=math.inf)
            mm = sum(r['mismatch'] for r in rows)
            fails = sum(r['domain_fail'] or r['rejected'] for r in rows)
            evaluated = sum(not r['domain_fail'] for r in rows)
            mean_k = sum(r['k'] for r in rows)/len(rows)
            score = raw_ops/ops if not mm and not fails and max(px,state) <= 1e-3 else 0
            scores.append(score)
            print(f'{degree} {w} {build_s:.3f} {px:.6g} {state:.6g} {mm} {evaluated} {fails} {max(r["radius"] for r in rows):.4g} {ops} {mean_k*ops:.1f} {mean_k*raw_ops:.1f} {score:.3f}', flush=True)
        print(f'candidate degree={degree} all_depth_score={min(scores):.3f}', flush=True)
    print(f'elapsed_s={time.perf_counter()-start:.3f}; class horizon={PROBE_MAXIT}; fallback=legacy raw iteration; costs cover returns only, excluding tail/derivatives/build; no hierarchical-BLA timing or atlas-win claim', flush=True)


TIGHT_GUARDS = {2: VALID_RADIUS, 4: VALID_RADIUS,
                6: VALID_RADIUS, 8: VALID_RADIUS}


def tight_point(args):
    """Stop before an out-of-domain return, then evaluate E_C directly."""
    row, terms, bias, degree, guard_text = args
    mp.mp.dps = DPS
    global MAXIT
    MAXIT = PROBE_MAXIT
    d = decode(row['d']); v = d/SCALE; u = v
    coeff = [(i,j,decode(a)) for i,j,a in terms]
    bias = decode(bias)
    vp = [v**j for j in range(degree+1)]
    exact = C+d; dz = mp.mpc(1); k = 0
    guard = mp.mpf(guard_text)
    # Candidate exit scheduling uses only its own state, never truth's k.
    while abs(u)*SCALE <= guard and k < 60:
        up = [u**i for i in range(degree+1)]
        u = bias+sum(a*up[i]*vp[j] for i,j,a in coeff)
        k += 1
        # Auxiliary local-state diagnostic only. Frozen smooth/class truth
        # is never rebuilt; this prefix supplies the state at the new exit.
        for _ in range(P):
            dz = 2*exact*dz+1
            exact = exact*exact+C+d
        if not mp.isfinite(abs(u)):
            return dict(w=row['w'], failed=True, mismatch=False, px=None,
                        local_px=None, control_px=None, k=k, radius=None, tail=0)
    z = C+SCALE*u; n0 = k*P+1
    local_px = abs(z-exact)/abs(dz)/(mp.mpf(row['w'])/480)
    # E_C is one fixed function, with a fixed 20,000-step tail budget.
    # Apply the prefix offset once; no pixel parameter or c0 reaches E_C.
    tail = nu(C, z0=z)
    control = nu(C, z0=exact)
    predicted = None if tail[0] is None or n0+tail[2] > PROBE_MAXIT else n0+tail[0]
    control_nu = None if control[0] is None or n0+control[2] > PROBE_MAXIT else n0+control[0]
    mismatch = (predicted is None) != (row['nu'] is None)
    factor = None if row['de'] is None else math.log(2)*float(mp.mpf(row['de'])/(mp.mpf(row['w'])/480))
    px = None if mismatch or row['nu'] is None else abs(predicted-row['nu'])*factor
    control_px = None if control_nu is None or row['nu'] is None else abs(control_nu-row['nu'])*factor
    return dict(w=row['w'], failed=k == 60, mismatch=mismatch, px=px,
                local_px=float(local_px), control_px=control_px, k=k,
                radius=float(abs(z-C)), tail=tail[2], raw=row['iterations'])


def tight_probe(jobs, report=None, bla_exe=None, quadratic_guard=None):
    """DEC-14: seconds-scale frozen-truth scoring; no table speedup claim."""
    import collections
    start = time.perf_counter(); truth = load_truth(); summaries = []
    guards = dict(TIGHT_GUARDS)
    if quadratic_guard is not None:
        guards[2] = mp.mpf(quadratic_guard)
        if not 0 < guards[2] <= VALID_RADIUS:
            raise ValueError('quadratic guard must be positive and <= 1e-26')
    print('degree width guard max_px local_px fixed_C_control_px mismatches failures returns tail_raw raw_total projected_ops verdict', flush=True)
    with Pool(jobs) as pool:
        for degree in [2,4,6,8]:
            terms,bias,build_s = build_map(degree)
            results = pool.map(tight_point, [(row,terms,bias,degree,str(guards[degree])) for row in truth['rows']])
            cmuls = 2*(degree-1)+2*len(terms)+1
            ops = 6*cmuls+2*len(terms)+2
            scores = []
            for w in DEPTHS:
                rows = [r for r in results if mp.mpf(r['w']) == mp.mpf(w)]
                max_px = max((r['px'] for r in rows if r['px'] is not None), default=math.inf)
                local = max((r['local_px'] for r in rows if r['local_px'] is not None), default=math.inf)
                control = max((r['control_px'] for r in rows if r['control_px'] is not None), default=math.inf)
                mm = sum(r['mismatch'] for r in rows); failures = sum(r['failed'] for r in rows)
                keep = not mm and not failures and max(max_px,local) <= 1e-3
                mean_k = sum(r['k'] for r in rows)/len(rows)
                projected = mean_k*ops  # Plus ONE lookup: arithmetic cost not measured.
                raw = sum(r.get('raw',PROBE_MAXIT) for r in rows)/len(rows)
                tail = sum(r['tail'] for r in rows)/len(rows)
                bins = collections.Counter(math.floor(math.log10(r['radius'])) for r in rows if r['radius'])
                score = 6*raw/projected if keep and projected else 0
                scores.append(score)
                summary = dict(degree=degree, w=w, guard=str(guards[degree]),
                               build_s=build_s, max_px=max_px, local_px=local,
                               control_px=control, mismatches=mm, failures=failures,
                               returns=[min(r['k'] for r in rows), max(r['k'] for r in rows)],
                               mean_returns=mean_k, mean_tail=tail, mean_raw=raw,
                               map_ops=projected, raw_ops=6*raw,
                               histogram=dict(sorted(bins.items())), verdict='KEEP' if keep else 'KILL',
                               radius_min=min(r['radius'] for r in rows if r['radius']),
                               radius_max=max(r['radius'] for r in rows if r['radius']))
                summaries.append(summary)
                print(f'{degree} {w} {mp.nstr(guards[degree],2)} {max_px:.6g} {local:.6g} {control:.6g} {mm} {failures} {summary["returns"]} {tail:.1f} {raw:.1f} {projected:.1f}+lookup {summary["verdict"]}', flush=True)
                print(f'  exit_histogram floor(log10(|zeta-C|))={dict(sorted(bins.items()))}', flush=True)
            print(f'candidate degree={degree} all_depth_score={min(scores):.3f} (projected raw/map-ops; lookup unpriced)', flush=True)
    bla = []
    if bla_exe:
        if not report:
            raise ValueError('--bla-exe requires an external --report path')
        import subprocess
        base = Path(report).resolve().with_suffix('')
        path = base.with_suffix('.path')
        path.write_text(''.join(' '.join(encode(C)+[w])+'\n' for w in DEPTHS))
        command = [bla_exe, 'control', str(path), '--size', '8x4',
                   '--iter', str(PROBE_MAXIT), '--columns', 'nu,de',
                   '--threads', str(jobs), '--bla', 'per-frame', '--runs', '1',
                   '-o', str(base)+'-bla']
        result = subprocess.run(command, capture_output=True, text=True, check=True)
        Path(str(base)+'-bla.jsonl').write_text(result.stdout)
        for line in result.stdout.splitlines():
            frame = json.loads(line)
            if frame['record'] != 'frame': continue
            fallback = frame['fallback']['iterations_per_pixel']
            blocks = frame['atlas_work']['macro_operators_per_pixel']
            # Perturbation step: 2Z*dz + dz^2 + dc; BLA: A*dz+B*dc.
            # Both model as 14 scalar ops, excluding derivatives/guards.
            modeled_ops = 14*(fallback+blocks)
            bla.append(dict(w=frame['view']['width'], fallback=fallback,
                            blocks=blocks, modeled_ops=modeled_ops,
                            render_s=frame['bla']['render_seconds'],
                            cold_s=frame['timing']['cold_seconds']))
            print(f'BLA 8x4 width={frame["view"]["width"]} fallback={fallback:.3f} blocks={blocks:.3f} modeled_ops={modeled_ops:.3f}', flush=True)
    elapsed = time.perf_counter()-start
    print(f'elapsed_s={elapsed:.3f}; table not built; E_C evaluated directly; class horizon={PROBE_MAXIT}; BLA cohort is separate 8x4 grid, not frozen points; no timed speedup claim', flush=True)
    if report:
        Path(report).write_text(json.dumps(dict(elapsed_s=elapsed, rows=summaries, bla=bla), indent=2)+'\n')


def exit_state(v, coeff, bias):
    """Degree-4 prefix in doubles; retain C-relative coordinates throughout."""
    u = v; k = 0
    vp = [v**j for j in range(5)]
    while abs(u)*float(SCALE) <= float(VALID_RADIUS) and k < 60:
        up = [u**i for i in range(5)]
        u = bias+sum(a*up[i]*vp[j] for i,j,a in coeff)
        k += 1
    return u*float(SCALE), 1+k*P, k == 60


def tail_reference():
    """High-precision orbit, then double coefficients for delta recurrence.

    Start at C (not 0). Never form float(C+delta)-float(C), which would
    destroy every small exit. This is a probe, not a certified error bound.
    """
    import numpy as np
    z = C; ref = []
    for _ in range(PROBE_MAXIT+1):
        ref.append(complex(z)); z = z*z+C
    return np.asarray(ref)


def sampled_tails(offsets, ref):
    """Vectorized fixed-C direct perturbation; no parameter delta is added."""
    import numpy as np
    delta = np.asarray(offsets, dtype=complex).copy()
    active = np.arange(len(delta))
    smooth = np.full(len(delta), np.nan)
    steps = np.full(len(delta), PROBE_MAXIT, dtype=np.int32)
    for n in range(PROBE_MAXIT):
        if not len(active): break
        delta = (2*ref[n]+delta)*delta
        z = ref[n+1]+delta
        az = np.abs(z)
        escaped = az > 1e10
        ix = active[escaped]
        smooth[ix] = n+1-np.log2(np.log(az[escaped]))
        steps[ix] = n+1
        active = active[~escaped]; delta = delta[~escaped]
    return smooth, steps


def build_exit_table(radial, angular, ref):
    """Explicit rings around C; no unvalidated M(24,2) wrap across C-c0.

    Payload: float64 smooth escape + int32 escape horizon per node.
    Interpolation stores nu only, not de/normal. Mixed/unresolved corners,
    outside-domain inputs and horizon ambiguity require direct fallback.
    Uniform grids carry no certified interpolation bound: validation gates
    the entire candidate, not individual convenient pixels.
    """
    import numpy as np
    start = time.perf_counter()
    logs = np.linspace(-26, -4, 22*radial+1)
    angles = np.arange(angular)*2*math.pi/angular
    offsets = (10.0**logs[:,None]*np.exp(1j*angles[None,:])).ravel()
    smooth, steps = sampled_tails(offsets, ref)
    return dict(nu=smooth.reshape(len(logs),angular),
                steps=steps.reshape(len(logs),angular), radial=radial,
                angular=angular, samples=len(offsets),
                bytes=smooth.nbytes+steps.nbytes,
                build_s=time.perf_counter()-start,
                unresolved=int(np.isnan(smooth).sum()))


def lookup_exit(table, offset, n0):
    if not offset or not math.isfinite(abs(offset)): return None
    r = (math.log10(abs(offset))+26)*table['radial']
    if not 0 <= r < table['nu'].shape[0]-1: return None
    a = (math.atan2(offset.imag,offset.real) % (2*math.pi))*table['angular']/(2*math.pi)
    i = int(r); j = int(a); t = r-i; s = a-j
    jj = (j+1) % table['angular']
    corners = [table['nu'][i,j],table['nu'][i,jj],
               table['nu'][i+1,j],table['nu'][i+1,jj]]
    horizons = [table['steps'][i,j],table['steps'][i,jj],
                table['steps'][i+1,j],table['steps'][i+1,jj]]
    if not all(math.isfinite(x) for x in corners): return None
    if n0+max(horizons) > PROBE_MAXIT: return None
    return n0+(1-t)*((1-s)*corners[0]+s*corners[1])+t*((1-s)*corners[2]+s*corners[3])


def check_table_node(args):
    """Independent 110-dps spot check of a node used by an actual lookup."""
    radius, angle, sampled_nu, sampled_steps = args
    global MAXIT
    MAXIT = PROBE_MAXIT
    offset = mp.mpf(10)**mp.mpf(radius)*mp.exp(mp.j*mp.mpf(angle))
    exact = nu(C, C+offset)
    mismatch = (exact[0] is None) != (not math.isfinite(sampled_nu))
    return dict(mismatch=mismatch,
                nu=exact[0], steps=exact[2],
                delta_nu=None if exact[0] is None or mismatch else abs(exact[0]-sampled_nu),
                step_difference=int(sampled_steps)-exact[2])


def table_probe(jobs, report, bla_exe):
    """DEC-14 kill gate on frozen points before the 6,000-point promotion.

    Score = matched BLA render / warm candidate time, only when every point
    passes <=1e-3 px with no fallback or class mismatch. Rejection here is
    sufficient to kill these configurations, not all possible exit tables.
    """
    import hashlib, subprocess
    start = time.perf_counter(); truth = load_truth(); rows = truth['rows']
    terms,bias,map_build_s = build_map(4)
    coeff = [(i,j,complex(decode(a))) for i,j,a in terms]
    cbias = complex(decode(bias))
    inputs = [complex(decode(row['d'])/SCALE) for row in rows]
    states = [exit_state(v,coeff,cbias) for v in inputs]
    ref = tail_reference()
    # Separate double-tail control from interpolation and return truncation.
    direct, steps = sampled_tails([s[0] for s in states],ref)
    exact_args = [(row,terms,bias,4,str(VALID_RADIUS)) for row in rows]
    with Pool(jobs) as pool:
        controls = pool.map(tight_point, exact_args)
    control_rows = []
    for row,state,dnu,n,control in zip(rows,states,direct,steps,controls):
        factor = None if row['de'] is None else math.log(2)*float(mp.mpf(row['de'])/(mp.mpf(row['w'])/480))
        pred = None if not math.isfinite(dnu) or state[1]+n > PROBE_MAXIT else state[1]+dnu
        mismatch = (pred is None) != (row['nu'] is None)
        px = None if factor is None or pred is None else abs(pred-row['nu'])*factor
        control_rows.append(dict(w=row['w'], radius=abs(state[0]),
                                 prefix_steps=state[1], double_direct_px=px,
                                 double_direct_mismatch=mismatch,
                                 mp_direct_px=control['px'],
                                 mp_local_px=control['local_px'],
                                 mp_fixed_C_control_px=control['control_px']))
    summaries = []
    for radial,angular in [(8,64),(32,256)]:
        table = build_exit_table(radial,angular,ref)
        nodes = set()
        for offset,_,_ in states:
            i = int((math.log10(abs(offset))+26)*radial)
            j = int((math.atan2(offset.imag,offset.real) % (2*math.pi))*angular/(2*math.pi))
            if 0 <= i < table['nu'].shape[0]-1:
                nodes.update((ii,jj) for ii in (i,i+1) for jj in (j,(j+1)%angular))
        with Pool(jobs) as pool:
            node_checks = pool.map(check_table_node,
                [(-26+i/radial,j*2*math.pi/angular,float(table['nu'][i,j]),
                  int(table['steps'][i,j])) for i,j in sorted(nodes)])
        # Re-interpolate with 110-dps truth at every used corner. This
        # isolates interpolation failure even if sampled node values drift.
        exact_table = dict(table, nu=table['nu'].copy(), steps=table['steps'].copy())
        for (i,j),check in zip(sorted(nodes),node_checks):
            exact_table['nu'][i,j] = math.nan if check['nu'] is None else check['nu']
            exact_table['steps'][i,j] = check['steps']
        results = []
        for row,state in zip(rows,states):
            pred = lookup_exit(table,state[0],state[1])
            fallback = pred is None or state[2]
            # Do not hide rejected lookups by substituting exact truth.
            mismatch = not fallback and ((pred is None) != (row['nu'] is None))
            px = None if fallback or row['de'] is None else abs(pred-row['nu'])*math.log(2)*float(mp.mpf(row['de'])/(mp.mpf(row['w'])/480))
            exact_pred = lookup_exit(exact_table,state[0],state[1])
            exact_node_px = None if exact_pred is None or row['de'] is None else abs(exact_pred-row['nu'])*math.log(2)*float(mp.mpf(row['de'])/(mp.mpf(row['w'])/480))
            results.append(dict(w=row['w'], fallback=fallback, mismatch=mismatch, px=px,
                                exact_node_px=exact_node_px))
        # Warm scalar map + lookup, no truth, no diagnostic prefix/tail.
        repetitions = 20; t0 = time.perf_counter()
        for _ in range(repetitions):
            for v in inputs:
                offset,n0,_ = exit_state(v,coeff,cbias)
                lookup_exit(table,offset,n0)
        warm_s = (time.perf_counter()-t0)/repetitions
        depth_rows = []
        for w in DEPTHS:
            batch = [r for r in results if mp.mpf(r['w']) == mp.mpf(w)]
            depth_rows.append(dict(w=w, points=len(batch),
                                  max_px=max((r['px'] for r in batch if r['px'] is not None),default=None),
                                  exact_node_max_px=max((r['exact_node_px'] for r in batch if r['exact_node_px'] is not None),default=None),
                                  mismatches=sum(r['mismatch'] for r in batch),
                                  fallback=sum(r['fallback'] for r in batch)))
        keep = all(not r['fallback'] and not r['mismatches'] and r['max_px'] is not None and r['max_px'] <= 1e-3 for r in depth_rows)
        summary = {k:v for k,v in table.items() if k not in ('nu','steps')}
        summary.update(rows=depth_rows, warm_batch_s=warm_s,
                       node_checks=len(node_checks),
                       node_max_delta_nu=max((r['delta_nu'] for r in node_checks if r['delta_nu'] is not None),default=None),
                       node_mismatches=sum(r['mismatch'] for r in node_checks),
                       node_max_step_difference=max((abs(r['step_difference']) for r in node_checks),default=0),
                       warm_us_per_pixel=1e6*warm_s/len(rows),
                       score=0, verdict='CORRECTNESS_KEEP' if keep else 'KILL')
        summaries.append(summary)
        print(json.dumps(summary),flush=True)
    bla = []; bla_render_s = None
    if bla_exe:
        base = Path(report).resolve().with_suffix('')
        path = Path(str(base)+'-matched.path')
        # A 1x1 view samples its centre exactly. Each line is one frozen C+d.
        path.write_text(''.join(' '.join(encode(C+decode(row['d']))+[row['w']])+'\n' for row in rows))
        command = [bla_exe,'control',str(path),'--size','1x1',
                   '--iter',str(PROBE_MAXIT),'--columns','nu,de',
                   '--threads','1','--bla','per-frame','--runs','1',
                   '-o',str(base)+'-bla']
        result = subprocess.run(command,capture_output=True,text=True,check=True)
        Path(str(base)+'-bla.jsonl').write_text(result.stdout)
        for line in result.stdout.splitlines():
            frame = json.loads(line)
            if frame['record'] == 'frame':
                bla.append(dict(render_s=frame['bla']['render_seconds'],
                                cold_s=frame['timing']['cold_seconds'],
                                view=frame['view']))
        if len(bla) != len(rows): raise ValueError('BLA did not return the matched cohort')
        bla_render_s = sum(r['render_s'] for r in bla)
        for summary in summaries:
            summary['bla_us_per_pixel'] = 1e6*bla_render_s/len(rows)
            if summary['verdict'] == 'CORRECTNESS_KEEP':
                summary['score'] = bla_render_s/summary['warm_batch_s']
    elapsed = time.perf_counter()-start
    output = dict(elapsed_s=elapsed, truth_sha256=hashlib.sha256(TRUTH.read_bytes()).hexdigest(),
                  points=len(rows), map_build_s=map_build_s, controls=control_rows,
                  configurations=summaries, bla=bla,
                  sharing=dict(measured_depth_frames=len(DEPTHS), tables_per_configuration=1,
                               frames_per_table=len(DEPTHS), v0_all_frames='not measured'),
                  contract='nu-only, fixed C, log10 radius [-26,-4), periodic angular seam; no certified bound; rejected configurations require direct fallback',
                  timing_note='Python warm predecoded map+lookup vs Rust BLA 1x1 render; per-point reference builds and tiny-frame overhead are not an atlas speedup',
                  wide_validation='Not promoted on a failed frozen-pack kill gate; no >=1000 points/depth claim')
    Path(report).write_text(json.dumps(output,indent=2,allow_nan=False)+'\n')
    print(f'elapsed_s={elapsed:.3f}; matched BLA us/pixel={None if bla_render_s is None else 1e6*bla_render_s/len(rows)}; no atlas-win claim',flush=True)


if __name__ == '__main__':
    if any(flag in sys.argv for flag in ('--probe','--freeze','--tight-probe','--table-probe')):
        parser = argparse.ArgumentParser(description='Frozen-truth period-764 biseries probe')
        mode = parser.add_mutually_exclusive_group(required=True)
        mode.add_argument('--freeze', action='store_true')
        mode.add_argument('--probe', action='store_true')
        mode.add_argument('--tight-probe', action='store_true')
        mode.add_argument('--table-probe', action='store_true')
        parser.add_argument('--jobs', type=int, default=18)
        parser.add_argument('--report', help='Optional JSON report path (keep outside the worktree)')
        parser.add_argument('--bla-exe', help='Optional existing fd binary for a separate 8x4 BLA control')
        parser.add_argument('--quadratic-guard', help='Optional smaller guard for the degree-2 variant')
        args = parser.parse_args()
        if args.jobs < 1: parser.error('--jobs must be positive')
        if args.table_probe:
            if not args.report: parser.error('--table-probe requires an external --report path')
            table_probe(args.jobs,args.report,args.bla_exe)
        elif args.tight_probe:
            tight_probe(args.jobs, args.report, args.bla_exe, args.quadratic_guard)
        elif args.freeze:
            freeze(args.jobs)
        else:
            probe(args.jobs)
    else:
        legacy()
