#!/usr/bin/env python3
"""Per-frame finite-cycle discovery with pre-render authorization and abstention.

No location-dependent accelerator constants are embedded here.  This is a research
renderer, NOT a replacement for fd's production pixel kernel.  It can measure an
entire frame against 'fd control --bla per-frame' including discovery/compilation.
A rejected candidate always abstains, and a failed full-frame score is a failure,
never an auto-approved speedup.

Requires mpmath, numpy.  Example:
 python tools/research/misiurewicz/auto_discover.py \
    --fd target/release/fd --re 0 --im 1 --width 1e-60 \
    --size 160x90 --iter 20000 --out out/auto-mis-i
"""
import argparse
import json
import math
from pathlib import Path
import subprocess
import time

import mpmath as mp
import numpy as np


def coefficients_return(centre, point, period, dp, order):
    """G(u)=f_c^p(point+u)-point and dG/ dc in a moving cycle chart."""
    x = point
    dpoint = dp
    a = [mp.mpc(0)] * (order + 1)
    da = [mp.mpc(0)] * (order + 1)
    a[1] = mp.mpc(1)
    for _ in range(period):
        new = [mp.mpc(0)] * (order + 1)
        dnew = [mp.mpc(0)] * (order + 1)
        for n in range(1, order + 1):
            product = sum(a[j] * a[n-j] for j in range(1, n))
            dproduct = sum(da[j] * a[n-j] + a[j] * da[n-j] for j in range(1, n))
            new[n] = 2*x*a[n] + product
            dnew[n] = 2*dpoint*a[n] + 2*x*da[n] + dproduct
        dpoint = 2*x*dpoint + 1
        x = x*x + centre
        a, da = new, dnew
    return a, da


def poly_mul(a, b, n):
    r = [mp.mpc(0)] * (n + 1)
    for i in range(1, min(n, len(a)-1)+1):
        if not a[i]:
            continue
        for j in range(1, min(n-i, len(b)-1)+1):
            r[i+j] += a[i]*b[j]
    return r


def linearizer(a, da, lam, dlam, order):
    """Koenigs inverse K: G(K(w))=K(lambda*w), with parameter derivative."""
    k = [mp.mpc(0)] * (order + 1)
    dk = [mp.mpc(0)] * (order + 1)
    k[1] = mp.mpc(1)
    for n in range(2, order + 1):
        power = [mp.mpc(0)] * (n+1)
        dpower = [mp.mpc(0)] * (n+1)
        power[1] = mp.mpc(1)  # replaces the first factor only
        p = mp.mpc(0)
        dp = mp.mpc(0)
        # Compute coefficients of K^m with k_n absent at this stage.
        power = k[:n+1]
        dpower = dk[:n+1]
        for m in range(2, n+1):
            old = power
            power = poly_mul(old, k, n)
            dpower = [u+v for u,v in zip(poly_mul(dpower,k,n), poly_mul(old,dk,n))]
            p += a[m]*power[n]
            dp += da[m]*power[n] + a[m]*dpower[n]
        den = lam**n - lam
        if abs(den) < mp.mpf("1e-50"):
            raise ValueError("small divisor in Koenigs series")
        k[n] = p/den
        dk[n] = (dp - (n*lam**(n-1)-1)*dlam*k[n])/den
    return k[1:], dk[1:]


def cycle_jet(c, s, p):
    """F, F_z, F_c, F_zz and F_zc for the period-p return."""
    x, a, b, d, e = s, mp.mpc(1), mp.mpc(0), mp.mpc(0), mp.mpc(0)
    for _ in range(p):
        e = 2*a*b + 2*x*e
        d = 2*a*a + 2*x*d
        b = 2*x*b + 1
        a = 2*x*a
        x = x*x + c
    return x, a, b, d, e


def discover(re, im, width, maxiter, qmax=64, pmax=128, order=14):
    """Detect and compile a finite repelling critical-orbit landing; or abstain."""
    t0 = time.perf_counter()
    digits = min(1200, max(90, int(-mp.log10(mp.mpf(width)))+55))
    mp.mp.dps = digits
    c = mp.mpc(re, im)
    z = [mp.mpc(0)]
    cap = min(maxiter, qmax+pmax+1)
    for _ in range(cap):
        zn = z[-1]*z[-1]+c
        z.append(zn)
        if abs(zn)>mp.mpf("1e10"):
            break
    tol = mp.mpf(width)*8
    hits = []
    for q in range(1, min(qmax,len(z)-2)+1):
        for p in range(1, min(pmax,len(z)-q-1)+1):
            if abs(z[q+p]-z[q]) <= tol:
                hits.append((q+p,q,p))
    if not hits:
        return None, {"decision":"decline", "reason":"no critical-orbit recurrence at camera scale",
                      "discovery_seconds":time.perf_counter()-t0}
    for _,q,p in sorted(hits):
        # A repelling cycle outside this local disk cannot be jumped without
        # crossing the intermediate escape/near-escape region. Reject cheaply.
        if abs(z[q]) >= mp.mpf('1.9'):
            continue
        try:
            s = z[q]
            for _ in range(32):
                f, a, b, d, e = cycle_jet(c,s,p)
                if abs(a-1) < mp.mpf("1e-20"):
                    break
                step = (f-s)/(a-1)
                s -= step
                if abs(step)<mp.power(10,-digits+20):
                    break
            f, lam, b, d, e = cycle_jet(c,s,p)
            if abs(f-s)>mp.power(10,-digits+15) or not (abs(lam)>1.01):
                continue
            if abs(z[q]-s) > tol:
                continue
            # Distinguish primitive period from its multiples.
            phases = [s]
            for _ in range(p-1):
                phases.append(phases[-1]**2+c)
            if any(abs(phases[m]-s)<mp.mpf("1e-20") for m in range(1,p)):
                continue
            if any(abs(x)>=mp.mpf("1.9") for x in phases):
                continue
            if abs(lam) < mp.mpf("1.5"):
                return None, {"decision":"decline",
                    "reason":"weakly repelling multiplier outside validated derivative-jump domain",
                    "q":q,"p":p,"multiplier":[float(mp.re(lam)),float(mp.im(lam))],
                    "discovery_seconds":time.perf_counter()-t0}
            dp = b/(1-lam)
            # Cheap optimistic profitability bound before calculating any Koenigs jet.
            # The final jump radius is <= 0.002, so failing this bound guarantees
            # the current operator-cost heuristic cannot approve the candidate.
            zz=mp.mpc(0); deriv=mp.mpc(0)
            for _ in range(q):
                deriv=2*zz*deriv+1
                zz=zz*zz+c
            seed=abs(z[q]-s)+abs(deriv-dp)*mp.mpf(width)/2
            if seed<=0:
                seed=mp.mpf(width)
            optimistic=max(0,int(mp.floor(mp.log(mp.mpf("0.002")/seed)/mp.log(abs(lam)))))
            cost=90+4*order+q
            if p*optimistic<cost:
                return None, {"decision":"decline",
                    "reason":"even optimistic operator savings cannot amortize the jump",
                    "q":q,"p":p,"predicted_skipped_upper":p*optimistic,
                    "operator_equivalent_steps":cost,
                    "discovery_seconds":time.perf_counter()-t0}
            build_start=time.perf_counter()
            dlam = d*dp+e
            a, da = coefficients_return(c,s,p,dp,order)
            if abs(a[1]-lam)>mp.mpf("1e-20")*max(1,abs(lam)):
                continue
            k,dk=linearizer(a,da,lam,dlam,order)
            # A small conservative phase disk prevents skipping a bailout.
            prefix=mp.mpf(1)
            gain=mp.mpf(1)
            for x in phases:
                prefix *= 2*abs(x)
                gain=max(gain,prefix)
            nonlin=max([abs(k[j-1])**(mp.mpf(1)/(j-1)) for j in range(2,order+1)] or [mp.mpf(1)])
            radius=float(min(mp.mpf("0.002"),mp.mpf("0.005")/gain, mp.mpf("0.02")/max(1,nonlin)))
            if radius<1e-70:
                continue
            # Check the conjugacy residual at a nontrivial complex test point.
            kw=mp.mpc(radius/3,radius/7)
            def eval_poly(cs,w):
                v=mp.mpc(0)
                for coef in reversed(cs):
                    v=(v+coef)*w
                return v
            h=eval_poly(k,kw)
            g=s+h
            for _ in range(p):
                g=g*g+c
            err=abs((g-s)-eval_poly(k,lam*kw))
            if err>mp.mpf("1e-13")*max(abs(h),mp.mpf("1e-30")):
                continue
            # q-step derivative, used to estimate potential jump length.
            zz=mp.mpc(0); deriv=mp.mpc(0)
            for _ in range(q):
                deriv=2*zz*deriv+1
                zz=zz*zz+c
            seed=abs(z[q]-s)+abs(deriv-dp)*mp.mpf(width)/2
            if seed<=0:
                seed=mp.mpf(width)
            predicted=int(max(0,mp.floor(mp.log(mp.mpf(radius)/seed)/mp.log(abs(lam)))))
            expected=p*predicted
            cost=90+4*order+q
            data={"decision":"accelerate" if expected>=cost else "decline",
                  "reason":"operator cost exceeds predicted skipped iterations" if expected<cost else "profitable candidate predicted",
                  "q":q,"p":p,"multiplier":[float(mp.re(lam)),float(mp.im(lam))],
                  "radius":radius,"predicted_skipped_steps":expected,"operator_equivalent_steps":cost,
                  "zref":[complex(float(mp.re(v)),float(mp.im(v))) for v in z[:q+1]],
                  "bias":complex(float(mp.re(z[q]-s)),float(mp.im(z[q]-s))),
                  "cycle":complex(float(mp.re(s)),float(mp.im(s))),
                  "dp":complex(float(mp.re(dp)),float(mp.im(dp))),
                  "dlam":complex(float(mp.re(dlam)),float(mp.im(dlam))),
                  "k":np.array([complex(float(mp.re(v)),float(mp.im(v))) for v in k]),
                  "dk":np.array([complex(float(mp.re(v)),float(mp.im(v))) for v in dk]),
                  "rho":complex(float(mp.re(lam)),float(mp.im(lam)))}
            if not all(np.isfinite(v).all() for v in [data["k"],data["dk"]]):
                continue
            data["build_seconds"]=time.perf_counter()-build_start
            data["discovery_seconds"]=build_start-t0
            data["bias_hp"]=z[q]-s
            data["phases_hp"]=phases
            info={key:value for key,value in data.items() if key not in
                  ("zref","bias","cycle","dp","dlam","k","dk","rho","bias_hp","phases_hp")}
            return data, info
        except (OverflowError,ValueError,ZeroDivisionError):
            continue
    return None, {"decision":"decline","reason":"no numerically usable primitive repelling cycle",
                  "discovery_seconds":time.perf_counter()-t0}


def eval_k(k, w, dc):
    """K_c(w), dK_c/dw, and partial dK_c/dc, first-order in camera delta."""
    a=k["k"]; da=k["dk"]
    val=np.zeros_like(w); der=np.zeros_like(w); par=np.zeros_like(w)
    for i in range(len(a)-1,-1,-1):
        der=der*w+val
        val=val*w+(a[i]+dc*da[i])
        par=par*w+da[i]
    return val*w, der*w+val, par*w



def load_fd(path,n):
    # The columns are exactly class,nu,de,normal; 8-byte alignment per column.
    b=Path(path).read_bytes()
    pad=lambda x:(x+7)&~7
    at=len(b)-(pad(n)+pad(n*8)+pad(n*4)+pad(n*2))
    cls=np.frombuffer(b,dtype="u1",count=n,offset=at)&3
    at+=pad(n)
    nu=np.frombuffer(b,dtype="<f8",count=n,offset=at)
    at+=pad(8*n)
    de=np.frombuffer(b,dtype="<f4",count=n,offset=at).astype("f8")
    at+=pad(4*n)
    norm=np.frombuffer(b,dtype="<u2",count=n,offset=at)
    return cls,nu,de,norm


def score(ref,ours,px=1e-3):
    cls,nu,de,norm=ref
    mismatch=int(np.count_nonzero(cls!=ours["class"]))
    mask=(cls==0)&ours["escaped"]
    if not np.any(mask):
        return {"class_mismatches":mismatch,"escaped_compared":0,"ok":bool(mismatch==0)}
    disp=np.abs(nu[mask]-ours["nu"][mask])*math.log(2)*de[mask]/2
    de_err=np.abs(ours["de"][mask]-de[mask])/np.maximum(1e-30,de[mask])
    # fd stores screen normal as a quantized angle. Candidate stores a continuous
    # angle; account for one u16 quantization step.
    nm0=norm[mask].astype("f8")*2*math.pi/65536
    ang=np.abs(np.angle(np.exp(1j*(ours["normal"][mask]-nm0))))
    return {"class_mismatches":mismatch,"escaped_compared":int(np.sum(mask)),
            "over_1e_3_px":int(np.count_nonzero(disp>px)),
            "max_displacement_px":float(np.max(disp)),
            "max_de_relative_error":float(np.max(de_err)),
            "max_normal_error_rad":float(np.max(ang)),
            "ok":bool(mismatch==0 and not np.any(disp>px) and np.max(de_err)<1e-3 and np.max(ang)<1e-3)}



def high_precision_spotcheck(re, im, width, nx, ny, rot, maxiter, fd, ours, indices):
    """Independent orbit-and-derivative oracle for worst disagreement pixels."""
    mp.mp.dps = min(1200,max(90,2*int(-math.log10(float(width)))+60))
    c0=mp.mpc(re,im)
    h=mp.mpf(width)/nx
    phase=mp.exp(mp.j*mp.mpf(rot))
    cls,nu0,de0,n0=fd
    checks=[]
    for idx in sorted(set(map(int,indices))):
        i,j=idx%nx,idx//nx
        dx=mp.mpf(i)+mp.mpf("0.5")-nx/2
        dy=ny/2-mp.mpf(j)-mp.mpf("0.5")
        c=c0+phase*h*mp.mpc(dx,dy)
        z,d=mp.mpc(0),mp.mpc(0)
        truth=None
        for n in range(1,maxiter+1):
            d=2*z*d+1
            z=z*z+c
            if abs(z)>mp.mpf("1e10"):
                smooth=mp.mpf(n)+1-mp.log(mp.log(abs(z),2),2)
                distance=2*abs(z)*mp.log(abs(z))/(abs(d)*h)
                angle=(mp.mpf(rot)-mp.arg(z/d))%(2*mp.pi)
                truth=float(smooth),float(distance),float(angle)
                break
        if truth is None:
            checks.append({"index":idx,"oracle_class":"unresolved"})
            continue
        nu,de,ang=truth
        circ=lambda a,b:abs(math.atan2(math.sin(a-b),math.cos(a-b)))
        checks.append({"index":idx,"x":i,"y":j,
            "oracle_class":"escaped","oracle_nu":nu,"oracle_de_px":de,
            "fd_nu_error":abs(float(nu0[idx])-nu),
            "candidate_nu_error":abs(float(ours["nu"][idx])-nu),
            "fd_de_relative_error":abs(float(de0[idx])/de-1),
            "candidate_de_relative_error":abs(float(ours["de"][idx])/de-1),
            "fd_normal_error_rad":circ(float(n0[idx])*math.tau/65536,ang),
            "candidate_normal_error_rad":circ(float(ours["normal"][idx]),ang)})
    return checks


def write_native_model(k, path, width, nx, re, im):
    """Keep the exact decimal centre and binary-scaled sample offset (DEC-21)."""
    def pair(z):
        return f"{z.real:.17g} {z.imag:.17g}"

    def scaled(z):
        size = max(abs(mp.re(z)), abs(mp.im(z)))
        if not size:
            return "0 0 0"
        _, exponent = mp.frexp(size)
        unit = mp.power(2, exponent)
        return f"{float(mp.re(z)/unit):.17g} {float(mp.im(z)/unit):.17g} {int(exponent)}"

    h = mp.mpf(width) / nx
    mantissa, exponent = mp.frexp(h)
    radius = mp.mpf(k["radius"])
    sensitivity = 1 + abs(k["dp"]) + radius*abs(k["dlam"])/abs(k["rho"])
    sensitivity += sum(abs(v)*float(radius)**(i+1) for i,v in enumerate(k["dk"]))
    parameter_radius = float(radius / (64*sensitivity))
    lines = [
        "center_re "+re, "center_im "+im, "width "+width,
        f"scale {float(mantissa):.17g} {int(exponent)}",
        "q "+str(k["q"]), "p "+str(k["p"]),
        "radius "+repr(k["radius"]), "parameter_radius "+repr(parameter_radius),
        "bias "+scaled(k["bias_hp"]),
        "dp "+pair(k["dp"]), "dlam "+pair(k["dlam"]),
        "rho "+pair(k["rho"])]
    lines += [f"phase {j} {pair(z)}" for j,z in enumerate(k["phases_hp"])]
    lines += [f"ref {j} {pair(z)}" for j,z in enumerate(k["zref"])]
    lines += [f"k {j+1} {pair(z)}" for j,z in enumerate(k["k"])]
    lines += [f"dk {j+1} {pair(z)}" for j,z in enumerate(k["dk"])]
    path.write_text("\n".join(lines)+"\n")


def estimate_cost(info, nx, ny):
    """Empirical cold-cost screen, independent of the BLA truth renderer."""
    # Conservative coefficients calibrated on the 960x540 guarded native runs.
    # Recalibrate on each machine; estimates are not a mathematical cost bound.
    pixels = nx*ny
    discover = info.get("discovery_seconds", 0.0)
    build = info.get("build_seconds", 0.0)
    auto = 0.26e-6*pixels
    bla = 0.50e-6*pixels
    total = discover+build+auto
    return {"predicted_discover_seconds": discover,
            "predicted_build_seconds": build,
            "predicted_render_auto_seconds": auto,
            "predicted_bla_cold_seconds": bla,
            "predicted_auto_total_seconds": total,
            "predicted_profitable": bool(total*1.05 < bla)}


def write_candidate_fds(baseline, out, ours, count):
    """Retain fd's matching header, write all four candidate columns for fd compare."""
    raw = Path(baseline).read_bytes()
    pad = lambda n: (n+7)&~7
    offset = len(raw)-(pad(count)+pad(8*count)+pad(4*count)+pad(2*count))
    def aligned(data):
        return data+b"\0"*(pad(len(data))-len(data))
    cls = aligned(ours["class"].astype("u1").tobytes())
    nu = aligned(ours["nu"].astype("<f8").tobytes())
    de = aligned(ours["de"].astype("<f4").tobytes())
    angle = np.mod(ours["normal"], math.tau)
    norm = np.floor(angle*(65536/math.tau)+0.5).astype("<u2")
    normal = aligned(norm.tobytes())
    Path(out).parent.mkdir(parents=True, exist_ok=True)
    Path(out).write_bytes(raw[:offset]+cls+nu+de+normal)

def read_native(path,n):
    b=path.read_bytes()
    if len(b)!=n*(1+8*3):
        raise ValueError("native result buffer wrong length")
    o=n
    cls=np.frombuffer(b,dtype="u1",count=n,offset=0).copy()
    nu=np.frombuffer(b,dtype="<f8",count=n,offset=o).copy();o+=n*8
    de=np.frombuffer(b,dtype="<f8",count=n,offset=o).copy();o+=n*8
    normal=np.frombuffer(b,dtype="<f8",count=n,offset=o).copy()
    return {"escaped":cls==0,"class":cls,"nu":nu,"de":de,"normal":normal}

def main():
    p=argparse.ArgumentParser(description=__doc__)
    for name in ("re", "im", "width"):
        p.add_argument("--"+name, required=True)
    p.add_argument("--fd", required=True)
    p.add_argument("--size", default="160x90")
    p.add_argument("--iter", type=int, default=20000)
    p.add_argument("--threads", type=int, default=4)
    p.add_argument("--out", required=True)
    p.add_argument("--rotation", type=float, default=0.0)
    p.add_argument("--force", action="store_true",
                   help="research-only override of pre-render profitability decline")
    p.add_argument("--native", required=True, help="compiled native-cycles executor")
    p.add_argument("--no-compare", action="store_true",
                   help="skip retrospective fd BLA render and acceptance comparison")
    a=p.parse_args()
    nx,ny=map(int,a.size.split("x"))
    out=Path(a.out);out.mkdir(parents=True,exist_ok=True)
    path=out/"path.txt"
    path.write_text(f"{a.re} {a.im} {a.width} {a.rotation}\n")
    t=time.perf_counter()
    k,info=discover(a.re,a.im,a.width,a.iter)
    result=dict(info,frame_pixels=nx*ny)
    if k is not None:
        started=time.perf_counter()
        write_native_model(k,out/"model.txt",a.width,nx,a.re,a.im)
        info["build_seconds"]+=time.perf_counter()-started
        result["build_seconds"]=info["build_seconds"]
        result.update(estimate_cost(info,nx,ny))
    result["pre_render_decision"]="decline"
    if k is None:
        pass
    elif k["decision"]!="accelerate" and not a.force:
        result["reason"]=k["reason"]
    elif not result["predicted_profitable"] and not a.force:
        result["reason"]="predicted discovery + build + auto render does not beat cold BLA"
    else:
        result["pre_render_decision"]="accelerate"
        result["decision"]="accelerate"
        result["reason"]="pre-render operator and profitability checks pass"
        t=time.perf_counter()
        cmd=[a.native,str(out/"model.txt"),str(out/"native.bin"),a.size,
             a.width,str(a.iter),str(a.threads),str(a.rotation)]
        native=subprocess.run(cmd,capture_output=True,text=True)
        if native.returncode:
            raise RuntimeError(native.stderr+"\n"+native.stdout)
        ours=read_native(out/"native.bin",nx*ny)
        ours.update(json.loads(native.stdout.strip().splitlines()[-1]))
        ours["fallback_samples"]=nx*ny-ours["jumps"]
        render_time=time.perf_counter()-t
        result.update({x:ours[x] for x in ("jumps","skipped","fallback_samples","finish_steps")})
        result["fast_table"]=ours.get("fast_table",False)
        result["render_seconds"]=render_time
        result["total_candidate_seconds"]=time.perf_counter()-t+info["discovery_seconds"]+info.get("build_seconds",0.0)
        if not a.no_compare:
            cmd=[a.fd,"control",str(path),"--size",a.size,"--iter",str(a.iter),
                 "--columns","nu,de,normal","--threads",str(a.threads),
                 "--bla","per-frame","--runs","1","-o",str(out/"baseline")]
            baseline_run=subprocess.run(cmd,capture_output=True,text=True)
            if baseline_run.returncode:
                raise RuntimeError(baseline_run.stdout+"\n"+baseline_run.stderr)
            baseline=json.loads([x for x in baseline_run.stdout.splitlines()
                                  if '"record":"frame"' in x][0])
            cold=baseline["timing"]["cold_seconds"]
            result["baseline_seconds"]=cold
            result["speedup_vs_fd_cold"]=cold/result["total_candidate_seconds"]
            reference=out/"baseline"/"frame-00000.fds"
            candidate=out/"candidate"/"frame-00000.fds"
            write_candidate_fds(reference,candidate,ours,nx*ny)
            comparison=subprocess.run([a.fd,"compare",str(out/"baseline"),
                                       str(out/"candidate")],capture_output=True,text=True)
            result["fd_compare"]=json.loads(comparison.stdout.strip().splitlines()[-1])
            result["full_frame"]=score(load_fd(reference,nx*ny),ours)
            result["retrospective_pass"]=comparison.returncode==0
            result["acceptance"]="pass" if comparison.returncode==0 else "fail"
            # The check never feeds back into the pre-render acceleration decision.
    if result["pre_render_decision"]=="decline":
        result["decision"]="decline"
    result["decision_seconds"]=time.perf_counter()-t if k is None else (
        info["discovery_seconds"]+info.get("build_seconds",0.0))
    (out/"report.json").write_text(json.dumps(result,indent=2)+"\n")
    print(json.dumps(result))
    if result.get("acceptance")=="fail":
        raise SystemExit(2)


if __name__=="__main__":
    main()
