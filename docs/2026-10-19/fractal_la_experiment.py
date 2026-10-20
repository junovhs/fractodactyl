#!/usr/bin/env python3
"""Reproducible deep-zoom perturbation / BLA / staged quadratic LA / series test.
Python prototype, NOT a formally rounded interval-arithmetic proof or a native benchmark.
Requires: mpmath, matplotlib (plot optional).
Run: python fractal_la_experiment.py [--quick]
"""
import argparse, bisect, collections, csv, math, statistics, time
from dataclasses import dataclass
from pathlib import Path
import mpmath as mp

N=640
CASES={
 'attracting_-1': ('-1','0'),
 'parabolic_cusp': ('-0.75','0'),
 'period3_center': ('-0.1225611668766536','0.7448617666197442'),
 'seahorse_edge': ('-0.743643887037151','0.131825904205330'),
 'misiurewicz_-2': ('-2','0'),
 'misiurewicz_i': ('0','1'),
}
WIDTHS=[14,17,20,23,26,29]
OFFSETS=[(0.4,0.2),(-0.6,0.45),(0.15,-0.8)]

# polynomial jets P=A*u+B*dc+C*u^2+D*u*dc+E*dc^2
@dataclass
class Jet:
    a: complex=1+0j
    b: complex=0j
    c: complex=0j
    d: complex=0j
    e: complex=0j
    def step(self, z):
        t=2*z; a,b,c,d,e=self.a,self.b,self.c,self.d,self.e
        return Jet(t*a,t*b+1,t*c+a*a,t*d+2*a*b,t*e+b*b)
    def then(self,y):
        x=self
        return Jet(y.a*x.a, y.a*x.b+y.b,
                   y.a*x.c+y.c*x.a*x.a,
                   y.a*x.d+2*y.c*x.a*x.b+y.d*x.a,
                   y.a*x.e+y.c*x.b*x.b+y.d*x.b+y.e)
    def apply(self,u,dc):
        return (self.a*u+self.b*dc+self.c*u*u+self.d*u*dc+self.e*dc*dc)
    def deriv(self,u,dc,dp):
        return (self.a+2*self.c*u+self.d*dc)*dp + (self.b+self.d*u+2*self.e*dc)

@dataclass
class Block:
    start: int
    end: int
    jet: Jet
    prefixes: list # tuples (magnitudes abs(A)..abs(E), ref abs at AFTER step)
    coeff_ops: int=0


def block_make(ref,start,end):
    p=Jet(); pre=[]
    for j in range(start,end):
        p=p.step(ref[j])
        pre.append((abs(p.a),abs(p.b),abs(p.c),abs(p.d),abs(p.e),abs(ref[j+1])))
    return Block(start,end,p,pre,end-start)


def choose_period(ref):
    # heuristic candidate; short period 2/3 gets priority if strong return.
    n=min(64,len(ref)-1)
    min_p=min(range(2,n+1),key=lambda j:abs(ref[j]))
    return min(32,min_p) if abs(ref[min_p])<0.1 else 4


def build_jets(ref,period):
    n=len(ref)-1; lookup=collections.defaultdict(list); num_steps=0
    # stage zero: macro period; subsequent stages: 4x macro, then 16x, etc.
    size=period
    prev={}
    while size<=n:
        this={}
        for s in range(0,n-size+1,size):
            block=block_make(ref,s,s+size)
            if prev:
                # Higher stages are constructed by composing four lower-stage maps.
                children=[prev.get(s+j*(size//4)) for j in range(4)]
                if all(child is not None for child in children):
                    jet=children[0].jet
                    for child in children[1:]:jet=jet.then(child.jet)
                    block.jet=jet
            lookup[s].append(block);this[s]=block;num_steps+=size
        prev=this
        size*=4
    return lookup,num_steps


def build_bla(ref,width,eta=0.05):
    n=len(ref)-1; levels=collections.defaultdict(list)
    # A, B, heuristic validity radius for state and max ref norm
    prev=[]
    for j in range(n):
        a=2*ref[j]; r=eta*abs(a)
        prev.append((j,j+1,a,1+0j,r,abs(ref[j+1])))
        levels[j].append(prev[-1])
    builds=n; length=1
    while 2*length<=n:
        cur=[]
        for i in range(0,len(prev)-1,2):
            x,y=prev[i:i+2]
            if x[1]!=y[0]: continue
            a=y[2]*x[2]; b=y[2]*x[3]+y[3]
            ry=(y[4]-abs(x[3])*width)
            if abs(x[2])>0: r=min(x[4],max(0.,ry/abs(x[2])))
            else: r=x[4] if ry>0 else 0.
            elem=(x[0],y[1],a,b,r,max(x[5],y[5]))
            cur.append(elem);levels[x[0]].append(elem)
            builds+=1
        prev=cur;length*=2
        if not prev: break
    return levels,builds


def cert_precise(block,r,rho,ref):
    """Precomputed scalar majorant, using L_j and Q_j BEFORE each step."""
    E=0.0; u_bound=r; lip=1.0
    safe=True
    # prefixes represent Taylor at end of each step
    La=r; Qa=0.
    for h,(A,B,C,D,F,rabs) in enumerate(block.prefixes):
        j=block.start+h
        # quadratic truncation residual at previous step
        prev_E=E
        E=(2*abs(ref[j])+2*(La+Qa))*E+E*E+2*La*Qa+Qa*Qa
        l=A*r+B*rho
        q=C*r*r+D*r*rho+F*rho*rho
        u_bound=l+q+E
        lip*=2*(abs(ref[j])+La+Qa+prev_E)
        if rabs+u_bound>=2.0: safe=False
        La,Qa=l,q
        if not math.isfinite(E) or not math.isfinite(lip):
            return float('inf'),float('inf'),False
    return E,lip,safe


class CertTable:
    """Scene-radius majorants precomputed once per reference and zoom width."""
    def __init__(self,stages,ref,rho):
        self.rho=rho; self.powers=[rho*4.**k for k in range(54)]
        self.table={};self.prep_steps=0;self.heads={}
        for blocks in stages.values():
            for block in blocks:
                head=1+0j
                for j in range(block.start+1,block.end): head*=2*ref[j]
                self.heads[block.start,block.end]=head
                vals=[]
                for r in self.powers:
                    E,lip,safe=cert_precise(block,r,rho,ref)
                    # head-only LA omits later quadratic and cross terms
                    J=block.jet
                    h_err=abs(J.c-head)*r*r+abs(J.d)*r*rho+abs(J.e)*rho*rho
                    vals.append((E,lip,safe,E+h_err))
                    self.prep_steps+=block.end-block.start
                self.table[block.start,block.end]=vals
    def query(self,block,r,head=False):
        if r>self.powers[-1]:return float('inf'),float('inf'),False
        index=bisect.bisect_left(self.powers,r)
        E,lip,safe,headE=self.table[block.start,block.end][min(index,len(self.powers)-1)]
        return (headE if head else E),lip,safe


class EscapeGuard:
    """Compensated near-bailout comparison; uses HP reference near |z|=2.
    Does not replace a full rounding/error interval and is not a proof.
    """
    def __init__(self,hpr,ref):
        self.hpr=hpr;self.ref=ref;self.guards=0
    def escaped(self,i,u):
        q=abs(self.ref[i]+u)
        if abs(q-2.0)>1e-12:return q>2
        self.guards+=1
        return abs(self.hpr[i]+mp.mpc(u.real,u.imag))>2


def plain(ref,dc,guard):
    u=0j; d=0j; esc=None
    for j in range(len(ref)-1):
        d=2*(ref[j]+u)*d+1
        u=u*(2*ref[j]+u)+dc
        if guard.escaped(j+1,u):
            esc=j+1;break
    return u,d,esc or len(ref)-1,{'ordinary':(esc or len(ref)-1),'skip':0,'blocks':0,'rejected':0,'candidates':0,'lens':[],'pre':0,'guards':guard.guards}


def bla(ref,dc,lookup,guard,min_skip=2):
    n=len(ref)-1; u=0j; d=0j;i=0
    st=dict(ordinary=0,skip=0,blocks=0,rejected=0,candidates=0,lens=[],pre=0)
    escaped=False
    while i<n:
        chosen=None
        for item in reversed(lookup.get(i,[])):
            s,e,A,B,r,m=item
            if e-s<min_skip:continue
            st['candidates']+=1
            if abs(u)<r and abs(ref[e]+A*u+B*dc)<2 and m+abs(A*u+B*dc)<2:
                # cheap heuristic, not a certificate for all intermediate states
                chosen=item;break
            st['rejected']+=1
        if chosen:
            s,e,A,B,r,m=chosen
            d=A*d+B
            u=A*u+B*dc
            st['skip']+=e-s;st['blocks']+=1;st['lens'].append(e-s);i=e
        else:
            d=2*(ref[i]+u)*d+1
            u=u*(2*ref[i]+u)+dc
            i+=1;st['ordinary']+=1
        if guard.escaped(i,u): escaped=True;break
    st['guards']=guard.guards
    return u,d,i,st


def staged(ref,dc,lookup,rho,cert_table,guard,min_skip=2,tolerance=1e-6,certify=True,head=False):
    n=len(ref)-1;u=0j;d=0j;i=0;error=0.; eps=rho*tolerance
    st=dict(ordinary=0,skip=0,blocks=0,rejected=0,candidates=0,lens=[],pre=0,
            bound=0.,unsafe=0)
    while i<n:
        chosen=None; chosen_e=None
        for block in reversed(lookup.get(i,[])):
            if block.end-block.start<min_skip: continue
            st['candidates']+=1
            # constant-time lookup into scene-wide precomputed radius certificates.
            bound,lip,safe=cert_table.query(block,1.01*(abs(u)+error)+1e-300,head=head)
            new_e=bound+lip*error
            if (not certify or (safe and new_e <= eps)) and math.isfinite(new_e):
                chosen=block;chosen_e=new_e;break
            st['rejected']+=1
        if chosen:
            if head:
                h=cert_table.heads[chosen.start,chosen.end]
                newd=h*2*(ref[i]+u)*d+chosen.jet.b
                u=h*u*(2*ref[i]+u)+chosen.jet.b*dc
                d=newd
            else:
                d=chosen.jet.deriv(u,dc,d)
                u=chosen.jet.apply(u,dc)
            error=chosen_e if certify else error
            step=chosen.end-chosen.start
            st['skip']+=step;st['blocks']+=1;st['lens'].append(step)
            i=chosen.end
        else:
            d=2*(ref[i]+u)*d+1
            error*=2*(abs(ref[i])+abs(u)+error)
            u=u*(2*ref[i]+u)+dc
            i+=1;st['ordinary']+=1
        if guard.escaped(i,u):
            if abs(abs(ref[i]+u)-2)<=error: st['unsafe']+=1
            break
    st['guards']=guard.guards
    st['bound']=error
    return u,d,i,st


def initial_series(ref,dc,rho,order=4,tolerance=1e-6):
    """Parameter-series initialization (AT-like alternative; NOT Imagina's AT)."""
    n=len(ref)-1
    coeff=[0j]*(order+1); error=0.;last=0; best=[0j]*(order+1);best_err=0.
    for j in range(n):
        v=[0j]*(order+1)
        v[1]=2*ref[j]*coeff[1]+1
        for k in range(2,order+1):
            v[k]=2*ref[j]*coeff[k]+sum(coeff[h]*coeff[k-h] for h in range(1,k))
        # residual of polynomial update at the current state, terms > order
        res=sum(abs(coeff[a]*coeff[b])*rho**(a+b)
                for a in range(1,order+1) for b in range(1,order+1) if a+b>order)
        pbound=sum(abs(coeff[k])*rho**k for k in range(1,order+1))
        error=2*(abs(ref[j])+pbound)*error+error*error+res
        coeff=v
        pb=sum(abs(coeff[k])*rho**k for k in range(1,order+1))
        if error>rho*tolerance or abs(ref[j+1])+pb+error>=2:
            break
        last=j+1;best=coeff[:];best_err=error
    u=sum(best[k]*dc**k for k in range(1,order+1))
    der=sum(k*best[k]*dc**(k-1) for k in range(1,order+1))
    return last,u,der,best_err


def hp_reference(c,N):
    out=[mp.mpc(0)]
    for i in range(N):out.append(out[-1]*out[-1]+c)
    return out


def hp_pixel(ref_hp,dc,N):
    u=mp.mpc(0);der=mp.mpc(0);e=None
    for j in range(N):
        der=2*(ref_hp[j]+u)*der+1
        u=2*ref_hp[j]*u+u*u+dc
        if abs(ref_hp[j+1]+u)>2:
            e=j+1;break
    end=e or N
    return complex(u),complex(der),end


def smooth(ref,end,u,hpr=None):
    q=abs(ref[end]+u)
    if q>2 and abs(q-2)>1e-10:
        return end+1-math.log(math.log(q))/math.log(2)
    if hpr is not None:
        qm=abs(hpr[end]+mp.mpc(u.real,u.imag))
        if qm>2:return float(end+1-mp.log(mp.log(qm))/mp.log(2))
    return float('nan')


def run(out,quick=False):
    mp.mp.dps=100
    widths=WIDTHS if not quick else [14,23,29]
    cases=CASES if not quick else dict(list(CASES.items())[:3])
    offsets=OFFSETS if not quick else OFFSETS[:2]
    rows=[];t0=time.perf_counter()
    for name,(re,im) in cases.items():
        c=mp.mpc(mp.mpf(re),mp.mpf(im))
        a=time.perf_counter();hpr=hp_reference(c,N)
        ref=[complex(z) for z in hpr]
        period=choose_period(ref)
        j0=time.perf_counter();stages,jet_build=build_jets(ref,period)
        st_build_s=time.perf_counter()-j0
        print(f'REFERENCE {name} p={period}, |Z_N|={float(abs(hpr[-1])):.4g}, jet_build={st_build_s:.3f}s',flush=True)
        for w in widths:
            rho=10.**(-w)
            t=time.perf_counter();blas,bla_build=build_bla(ref,rho,eta=0.05)
            bla_prep=time.perf_counter()-t
            tc=time.perf_counter(); cert_table=CertTable(stages,ref,rho)
            cert_prep_s=time.perf_counter()-tc
            for ox,oy in offsets:
                dc_hp=mp.mpc(str(ox),str(oy))*mp.power(10,-w)
                dc=complex(dc_hp)
                hp_u,hp_d,hp_n=hp_pixel(hpr,dc_hp,N)
                guard=EscapeGuard(hpr,ref)
                pp_u,pp_d,pp_n,pp_stat=plain(ref,dc,guard)
                methods={ 'plain':(pp_u,pp_d,pp_n,pp_stat)}
                for eta in [0.005,0.05]:
                    if eta !=0.05:
                        base,_=build_bla(ref,rho,eta=eta)
                    else: base=blas
                    methods[f'BLA_{eta}']=bla(ref,dc,base,EscapeGuard(hpr,ref))
                for tol in [1e-4,1e-6]:
                    methods[f'stage2_{tol}']=staged(ref,dc,stages,rho,cert_table,EscapeGuard(hpr,ref),tolerance=tol)
                methods['LA_head']=staged(ref,dc,stages,rho,cert_table,EscapeGuard(hpr,ref),tolerance=1e-6,head=True)
                for minskip in [8,32]:
                    methods[f'stage2_min{minskip}']=staged(ref,dc,stages,rho,cert_table,EscapeGuard(hpr,ref),min_skip=minskip,tolerance=1e-6)
                pref,u0,d0,pe=initial_series(ref,dc,rho,order=4,tolerance=1e-6)
                # AT-like initial series then fallback EXACT ordinary perturbation (not staged)
                u,d,n=u0,d0,pref; rem=0; guard=EscapeGuard(hpr,ref)
                while n<N and not guard.escaped(n,u):
                    d=2*(ref[n]+u)*d+1
                    u=u*(2*ref[n]+u)+dc;n+=1;rem+=1
                methods['initial_series4']=(u,d,n,dict(ordinary=rem,skip=pref,blocks=int(pref>0),rejected=0,candidates=1,lens=[pref] if pref else [],pre=0,bound=pe,guards=guard.guards))
                for method,(u,d,end,s) in methods.items():
                    z_err=abs((ref[end]+u)-(complex(hpr[end])+hp_u)) if end==hp_n else float('nan')
                    derr=abs(d-hp_d)/max(1,abs(hp_d)) if end==hp_n else float('nan')
                    nu_err=abs(smooth(ref,end,u,hpr)-smooth(ref,end,hp_u,hpr)) if end==hp_n and end<N else float('nan')
                    rows.append(dict(case=name,width=f'1e-{w}',offset=f'{ox:+g}{oy:+g}i',method=method,
                        reference_steps=N,escape_ref=hp_n,iteration=end,correct=int(hp_n==end),
                        ordinary=s['ordinary'],skipped=s['skip'],blocks=s['blocks'],
                        max_block=max(s['lens'],default=0),mean_block=statistics.mean(s['lens']) if s['lens'] else 0,
                        tested=s['candidates'],rejected=s['rejected'],reject_rate=s['rejected']/max(1,s['candidates']),
                        z_error=z_err,d_rel_error=derr,nu_error=nu_err,
                        bound=s.get('bound',0),guards=s.get('guards',0),stage_period=period,
                        preprocess_ops=(jet_build+cert_table.prep_steps if (method.startswith('stage2') or method=='LA_head') else bla_build if method.startswith('BLA') else N if method=='initial_series4' else 0),
                        preprocess_s=((st_build_s+cert_prep_s) if (method.startswith('stage2') or method=='LA_head') else bla_prep if method.startswith('BLA') else 0),
                        # illustrative arithmetic work units, NOT measured latency or cycles:
                        cost=s['ordinary']*1.0 + s['blocks']*(2.1 if (method.startswith('stage2') or method=='LA_head') else 1.25 if method.startswith('BLA') else 1.6)
                        +s['candidates']*(0.7 if (method.startswith('stage2') or method=='LA_head') else 0.28 if method.startswith('BLA') else 0)))
        print(' completed',name,'elapsed',round(time.perf_counter()-t0,2),'s',flush=True)
    p=out/'fractal_la_per_pixel.csv'
    with p.open('w',newline='') as f:
        writer=csv.DictWriter(f,fieldnames=list(rows[0]));writer.writeheader();writer.writerows(rows)
    from collections import defaultdict
    groups=defaultdict(list)
    for row in rows:groups[row['method'],row['case'],row['width']].append(row)
    summary=[]
    for (method,name,w),group in sorted(groups.items()):
        def avg(key):return statistics.mean(x[key] for x in group)
        summary.append(dict(method=method,case=name,width=w,n=len(group),ordinary=avg('ordinary'),
            skipped=avg('skipped'),blocks=avg('blocks'),max_block=max(x['max_block'] for x in group),
            rejected=avg('rejected'),reject_rate=avg('reject_rate'),guards=avg('guards'),correct=sum(x['correct'] for x in group),
            max_z_error=max((x['z_error'] for x in group if math.isfinite(x['z_error'])),default=float('nan')),
            max_deriv_rel=max((x['d_rel_error'] for x in group if math.isfinite(x['d_rel_error'])),default=float('nan')),
            max_smooth_error=max((x['nu_error'] for x in group if math.isfinite(x['nu_error'])),default=float('nan')),
            estimated_cost=avg('cost'),preprocess_s=avg('preprocess_s')))
    with (out/'fractal_la_summary.csv').open('w',newline='') as f:
        ww=csv.DictWriter(f,fieldnames=list(summary[0]));ww.writeheader();ww.writerows(summary)
    # compact stdout aggregate across all cases+offsets
    print('METHOD        ordinary  maxblock reject-rate error_count max|drel|  cost/pp speedup_work',flush=True)
    methods=sorted(set(x['method'] for x in rows));plain_cost=statistics.mean(x['cost'] for x in rows if x['method']=='plain')
    for method in methods:
        a=[x for x in rows if x['method']==method]
        print(f"{method:<16} {statistics.mean(x['ordinary'] for x in a):7.1f} {max(x['max_block'] for x in a):7d}  {statistics.mean(x['reject_rate'] for x in a):7.2%}  {len(a)-sum(x['correct'] for x in a):3d}  {max((x['d_rel_error'] for x in a if math.isfinite(x['d_rel_error'])),default=float('nan')):8.2g}  {statistics.mean(x['cost'] for x in a):8.1f} {plain_cost/statistics.mean(x['cost'] for x in a):8.2f}",flush=True)
    try:
        import matplotlib;matplotlib.use('Agg')
        import matplotlib.pyplot as plt
        fig,ax=plt.subplots(figsize=(10,5.5))
        for method in ['plain','BLA_0.05','LA_head','stage2_1e-06','initial_series4']:
            ys=[]
            for w in widths:
                sub=[x for x in rows if x['method']==method and x['width']==f'1e-{w}']
                ys.append(statistics.mean(x['cost'] for x in sub))
            ax.plot(widths,ys,marker='o',label=method)
        ax.set(xlabel='Negative log10(parameter width)',ylabel='Illustrative per-pixel work units',
               title='Algorithmic operation proxy; not a native wall-clock measurement')
        ax.grid(alpha=.3);ax.legend();fig.tight_layout()
        fig.savefig(out/'fractal_la_costs.png',dpi=160);plt.close(fig)
    except ImportError: pass
    print('ROWS',len(rows),'files',list(str(x) for x in out.glob('fractal_la*')),flush=True)

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--quick',action='store_true')
    parser.add_argument('--output',default='/mnt/data');args=parser.parse_args()
    run(Path(args.output),args.quick)
