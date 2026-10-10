//! Native finite-cycle jump research executor (DEC-10, DEC-19, DEC-21).
//! The compiler supplies a reference at the exact decimal camera centre.
//! Parameter and state offsets remain mantissa times 2^exponent, never c0 + dc.
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

const LN2: f64 = std::f64::consts::LN_2;
const ONE: Cx = Cx { re: 1.0, im: 0.0 };

#[derive(Clone, Copy, Debug, Default)]
struct Cx { re: f64, im: f64 }
impl Cx {
    fn new(re: f64, im: f64) -> Self { Self { re, im } }
    fn add(self, b: Self) -> Self { Self::new(self.re+b.re, self.im+b.im) }
    fn sub(self, b: Self) -> Self { Self::new(self.re-b.re, self.im-b.im) }
    fn mul(self, b: Self) -> Self {
        Self::new(self.re*b.re-self.im*b.im, self.re*b.im+self.im*b.re)
    }
    fn scale(self, x: f64) -> Self { Self::new(self.re*x, self.im*x) }
    fn norm(self) -> f64 { self.re.hypot(self.im) }
    fn div(self, b: Self) -> Self {
        let n=b.re*b.re+b.im*b.im;
        Self::new((self.re*b.re+self.im*b.im)/n, (self.im*b.re-self.re*b.im)/n)
    }
    fn finite(self) -> bool { self.re.is_finite() && self.im.is_finite() }
}

// A complex number as mantissa * 2^exp; add/multiply do not round the
// underlying decimal centre, even if the pixel offset is below f64 range.
#[derive(Clone, Copy, Debug, Default)]
struct Scaled { a: Cx, e: i64 }
impl Scaled {
    fn new(a: Cx, e: i64) -> Self {
        let mag=a.re.abs().max(a.im.abs());
        if mag==0.0 { return Self { a, e: 0 }; }
        if !mag.is_finite() { return Self { a, e }; }
        let shift=mag.log2().floor() as i64;
        let factor=2f64.powi((-shift) as i32);
        Self { a: a.scale(factor), e: e+shift }
    }
    fn add(self, b: Self) -> Self {
        if self.a.norm()==0.0 { return b; }
        if b.a.norm()==0.0 { return self; }
        let (top,low)=if self.e>=b.e {(self,b)} else {(b,self)};
        let gap=low.e-top.e;
        if gap < -1022 { return top; }
        Self::new(top.a.add(low.a.scale(2f64.powi(gap as i32))), top.e)
    }
    fn sub(self, b: Self) -> Self { self.add(Self { a: b.a.scale(-1.0), e: b.e }) }
    fn mul_c(self, b: Cx) -> Self { Self::new(self.a.mul(b), self.e) }
    fn mul(self, b: Self) -> Self { Self::new(self.a.mul(b.a), self.e+b.e) }
    fn as_cx(self) -> Cx {
        if !(-1074..=1023).contains(&self.e) { return Cx::default(); }
        self.a.scale(2f64.powi(self.e as i32))
    }
    fn ln_norm(self) -> f64 {
        if self.a.norm()==0.0 { f64::NEG_INFINITY }
        else { self.a.norm().ln()+self.e as f64*LN2 }
    }
    fn from_polar(ln_r: f64, theta: f64) -> Self {
        let e=(ln_r/LN2).floor() as i64;
        let (s,c)=theta.sin_cos();
        Self::new(Cx::new(c,s).scale((ln_r-e as f64*LN2).exp()),e)
    }
}

#[derive(Default)]
struct Model {
    center_re: String, center_im: String, width: String,
    h_m: f64, h_e: i64, q: usize, p: usize, radius: f64, parameter_radius: f64,
    bias: Scaled, dp: Cx, dlam: Cx, rho: Cx,
    reference: Vec<Cx>, phases: Vec<Cx>, k: Vec<Cx>, dk: Vec<Cx>,
}
fn parse_model(s: &str) -> Model {
    let mut m=Model::default();
    for l in s.lines() {
        if l.trim().is_empty() || l.starts_with('#') { continue; }
        let a:Vec<&str>=l.split_whitespace().collect();
        let v=|i:usize| a[i].parse::<f64>().expect("numeric model field");
        let cx=|i:usize| Cx::new(v(i),v(i+1));
        match a[0] {
            "center_re"=>m.center_re=a[1].to_owned(),
            "center_im"=>m.center_im=a[1].to_owned(),
            "width"=>m.width=a[1].to_owned(),
            "scale"=>{m.h_m=v(1);m.h_e=a[2].parse().unwrap();},
            "q"=>m.q=a[1].parse().unwrap(),
            "p"=>m.p=a[1].parse().unwrap(),
            "radius"=>m.radius=v(1),
            "parameter_radius"=>m.parameter_radius=v(1),
            "bias"=>m.bias=Scaled::new(cx(1),a[3].parse().unwrap()),
            "dp"=>m.dp=cx(1), "dlam"=>m.dlam=cx(1), "rho"=>m.rho=cx(1),
            "ref"=>m.reference.push(cx(2)),
            "phase"=>m.phases.push(cx(2)),
            "k"=>m.k.push(cx(2)), "dk"=>m.dk.push(cx(2)),
            field=>panic!("unknown model field {field}"),
        }
    }
    assert!(!m.center_re.is_empty() && !m.center_im.is_empty() && !m.width.is_empty());
    assert!(m.q>0 && m.p>0 && m.reference.len()==m.q+1);
    assert!(m.phases.len()==m.p && !m.k.is_empty() && m.k.len()==m.dk.len());
    assert!(m.h_m>0.0 && m.h_m.is_finite() && m.radius>0.0 &&
            m.parameter_radius>0.0 && m.rho.norm()>1.5);
    m
}
fn read_model(path: &str) -> Model {
    parse_model(&fs::read_to_string(path).expect("read compiled model"))
}
fn k_value(m:&Model,w:Cx,dc:Cx)->(Cx,Cx,Cx) {
    let (mut v,mut d,mut c)=(Cx::default(),Cx::default(),Cx::default());
    for i in (0..m.k.len()).rev() {
        d=d.mul(w).add(v);
        v=v.mul(w).add(m.k[i]).add(dc.mul(m.dk[i]));
        c=c.mul(w).add(m.dk[i]);
    }
    (v.mul(w),d.mul(w).add(v),c.mul(w))
}

// Check both the phase return and its parameter derivative at the exit.
// This is the operator's own local consistency check, not a truth render.
fn derivative_guard(m:&Model, exit:Cx, lambda:Cx, dc:Cx)->bool {
    let input=exit.div(lambda);
    let (start,_,partial)=k_value(m,input,dc);
    let mut delta=m.dp.mul(dc).add(start);
    let mut d=m.dp.add(partial);
    for phase in &m.phases {
        d=phase.add(delta).mul(d).scale(2.0).add(ONE);
        delta=phase.mul(delta).scale(2.0).add(delta.mul(delta)).add(dc);
    }
    let (end,dend,pend)=k_value(m,exit,dc);
    let target=m.dp.mul(dc).add(end);
    let derivative=m.dp.add(pend).add(m.dlam.div(lambda).mul(exit).mul(dend));
    delta.finite() && d.finite() && derivative.finite()
        && delta.sub(target).norm() <= 1e-7*target.norm().max(1e-12)
        && d.sub(derivative).norm() <= 2e-4*derivative.norm().max(1.0)
}

// For representable, sufficiently small pixel deltas, all per-pixel
// multipliers are rho^j to within the parameter guard. Check the local
// differentiated identity at eight chart exits before enabling this path.
fn fast_table(m:&Model,nx:usize,ny:usize,max_iter:u64)->Option<(f64,Vec<Cx>)> {
    if !(-900..=-60).contains(&m.h_e) || m.bias.ln_norm()>=-30.0*LN2 {
        return None;
    }
    let h=m.h_m*2f64.powi(m.h_e as i32);
    let bound=h*(nx as f64).hypot(ny as f64)/2.0;
    if bound > m.parameter_radius ||
        (max_iter as f64)*bound*m.dlam.norm()/m.rho.norm()>1e-13 {
        return None;
    }
    for i in 0..8 {
        let angle=i as f64*std::f64::consts::TAU/8.0;
        let (sn,cs)=angle.sin_cos();
        let w=Cx::new(cs,sn).scale(m.radius/2.0);
        if !derivative_guard(m,w,m.rho,Cx::default()) { return None; }
    }
    let limit=(max_iter as usize/m.p).min(1024);
    let mut powers=Vec::with_capacity(limit+1);
    powers.push(ONE);
    for _ in 0..limit {
        let next=powers.last().unwrap().mul(m.rho);
        if !next.finite() || next.norm()>1e300 { break; }
        powers.push(next);
    }
    Some((h,powers))
}

// A cheap native path with the same exact-centre phase perturbation, but
// without exponent bookkeeping after the offset and derivative fit in f64.
fn fast_pixel(m:&Model,dc:Cx,h:f64,powers:&[Cx],max_iter:u64,rot:f64)->Option<Record> {
    let (mut delta,mut der)=(Cx::default(),Cx::default());
    for zref in &m.reference[..m.q] {
        der=zref.add(delta).mul(der).scale(2.0).add(ONE);
        delta=zref.mul(delta).scale(2.0).add(delta.mul(delta)).add(dc);
    }
    let w=m.bias.as_cx().add(delta).sub(m.dp.mul(dc));
    if !w.finite() || w.norm()==0.0 || w.norm()>=(m.radius/4.0)
        || dc.norm()>m.parameter_radius { return None; }
    let lr=m.rho.norm().ln();
    let possible=((m.radius/2.0).ln()-w.norm().ln())/lr;
    if !possible.is_finite() || possible<2.0 {return None;}
    let j=(possible.floor() as u64)
        .min(max_iter.saturating_sub(m.q as u64+1)/m.p as u64) as usize;
    let mul=*powers.get(j)?;
    if j<2 {return None;}
    let exit=w.mul(mul);
    if !exit.finite() || exit.norm()>m.radius/2.0 {return None;}
    // K(w) = w + O(w²), K_w(w) = 1 + O(w) at these guarded tiny inputs.
    let (value,dk_dw,dk_dc)=k_value(m,exit,dc);
    let mut delta=m.dp.mul(dc).add(value);
    let mut dz=m.dp.add(dk_dc)
        .add(mul.mul(dk_dw).mul(der.sub(m.dp)))
        .add(m.dlam.div(m.rho).mul(exit).mul(dk_dw).scale(j as f64));
    let linear=m.dp.add(mul.mul(der.sub(m.dp)))
        .add(m.dlam.div(m.rho).mul(exit).scale(j as f64));
    if !delta.finite() || !dz.finite() || !linear.finite()
        || dz.sub(linear).norm()>0.01*dz.norm().max(1.0) {
        return None;
    }
    let (mut n,mut plain,mut phase_index)=(m.q as u64+j as u64*m.p as u64,0,0);
    while n<max_iter {
        let phase=m.phases[phase_index];
        let z=phase.add(delta);
        dz=z.mul(dz).scale(2.0).add(ONE);
        delta=phase.mul(delta).scale(2.0).add(delta.mul(delta)).add(dc);
        phase_index=(phase_index+1)%m.p;
        n+=1;
        plain+=1;
        let next=m.phases[phase_index].add(delta);
        let az=next.norm();
        if az>1e10 {
            let nu=n as f64+1.0-az.log2().log2();
            let de=2.0*az*az.ln()/(dz.norm()*h);
            let v=next.div(dz);
            let normal=(rot-v.im.atan2(v.re)).rem_euclid(std::f64::consts::TAU);
            return Some(Record {class:0,nu,de,normal,jump:1,skipped:j as u64*m.p as u64,
                plain,..Record::default()});
        }
    }
    Some(Record {class:2,jump:1,skipped:j as u64*m.p as u64,
        plain,..Record::default()})
}

#[derive(Clone, Copy, Default)]
struct Record {
    class:u8,nu:f64,de:f64,normal:f64,jump:u64,skipped:u64,plain:u64,
    state_guard:u64, parameter_guard:u64, derivative_guard:u64,
}
fn pixel(m:&Model,dc:Scaled,max_iter:u64,rot:f64)->Record {
    let mut delta=Scaled::default();
    let mut der=Cx::default();
    for zref in &m.reference[..m.q] {
        let z=zref.add(delta.as_cx());
        der=z.mul(der).scale(2.0).add(ONE);
        delta=delta.mul_c(zref.scale(2.0)).add(delta.mul(delta)).add(dc);
    }
    // At q the exact reference is z_q = phase_0 + bias.
    let u=m.bias.add(delta).sub(dc.mul_c(m.dp));
    delta=m.bias.add(delta);
    let (mut n,mut dz)=(m.q as u64,Scaled::new(der,0));
    let (mut jump,mut skipped)=(0,0);
    let (mut state_guard,mut parameter_guard,mut derivative_reject)=(0,0,0);
    let parameter_ok=dc.ln_norm() <= m.parameter_radius.ln();
    let state_ok=u.ln_norm() < (m.radius/4.0).ln() && u.a.norm()>0.0;
    if !parameter_ok { parameter_guard=1; }
    if !state_ok { state_guard=1; }
    if parameter_ok && state_ok && n+1<max_iter {
        let dcf=dc.as_cx();
        let lambda=m.rho.add(m.dlam.mul(dcf));
        let lr=lambda.norm().ln();
        if lambda.finite() && lr>0.0 && m.dlam.mul(dcf).norm()<0.01*m.rho.norm() {
            let mut w=u;
            let mut newton_ok=true;
            if w.ln_norm() > (1e-12f64).ln() {
                let target=u.as_cx();
                let mut x=target;
                for _ in 0..8 {
                    let (val,prime,_)=k_value(m,x,dcf);
                    if !prime.finite() || prime.norm()<0.5 {newton_ok=false;break;}
                    let step=val.sub(target).div(prime);
                    x=x.sub(step);
                    if !x.finite() {newton_ok=false;break;}
                    if step.norm()<1e-15*x.norm().max(1e-200) {break;}
                }
                w=Scaled::new(x,0);
                let (check,_,_)=k_value(m,x,dcf);
                newton_ok &= check.sub(target).norm() < 1e-9*m.radius;
            }
            if newton_ok && w.a.norm()>0.0 {
                let possible=((m.radius/2.0).ln()-w.ln_norm())/lr;
                if possible.is_finite() && possible>=2.0 {
                    let j=(possible.floor() as u64)
                        .min(max_iter.saturating_sub(n+1)/(m.p as u64));
                    if j>=2 {
                        // Never form lambda^j as an f64; it may exceed 1e308.
                        let mul=Scaled::from_polar(j as f64*lr,
                            j as f64*lambda.im.atan2(lambda.re));
                        let exit=w.mul(mul).as_cx();
                        if exit.finite() && exit.norm()>0.0 && exit.norm()<=m.radius/2.0 {
                            if derivative_guard(m,exit,lambda,dcf) {
                                let (v,dout,pout)=k_value(m,exit,dcf);
                                let (_,din,pin)=k_value(m,w.as_cx(),dcf);
                                let ratio=dout.div(din);
                                let new_d=Scaled::new(m.dp.add(pout)
                                    .add(m.dlam.div(lambda).mul(exit).mul(dout).scale(j as f64)),0)
                                    .add(mul.mul_c(ratio.mul(der.sub(m.dp).sub(pin))));
                                let new_delta=dc.mul_c(m.dp).add(Scaled::new(v,0));
                                if new_delta.a.finite() && new_d.a.finite() {
                                    delta=new_delta;dz=new_d;n+=j*m.p as u64;
                                    jump=1;skipped=j*m.p as u64;
                                } else { derivative_reject=1; }
                            } else { derivative_reject=1; }
                        } else { state_guard=1; }
                    } else { state_guard=1; }
                } else { state_guard=1; }
            } else { state_guard=1; }
        } else { parameter_guard=1; }
    }
    // Once the jump exits the chart, its state is ordinary f64 sized. Keep
    // the expensive scaled representation only for the derivative exponent.
    // This is the common fast path; out-of-domain pixels use scaled fallback.
    if jump!=0 {
        let mut fast=delta.as_cx();
        let dcf=dc.as_cx();
        let mut phase_index=0;
        let mut plain=0;
        while n<max_iter {
            let phase=m.phases[phase_index];
            let z=phase.add(fast);
            let inv=if dz.e>1022 {0.0} else if dz.e< -1022 {
                f64::INFINITY
            } else {2f64.powi((-dz.e) as i32)};
            dz.a=z.mul(dz.a).scale(2.0).add(Cx::new(inv,0.0));
            let dm=dz.a.norm();
            if dm>1e24 || (dm>0.0 && dm<1e-24) {
                dz=Scaled::new(dz.a,dz.e);
            }
            fast=phase.mul(fast).scale(2.0).add(fast.mul(fast)).add(dcf);
            phase_index=(phase_index+1)%m.p;
            n+=1;
            plain+=1;
            let next=m.phases[phase_index].add(fast);
            let az=next.norm();
            if az>1e10 {
                let nu=n as f64+1.0-az.log2().log2();
                let h_ln=m.h_m.ln()+m.h_e as f64*LN2;
                let de=((2.0*az*az.ln()).ln()-dz.ln_norm()-h_ln).exp();
                let v=next.div(dz.a);
                let normal=(rot-v.im.atan2(v.re)).rem_euclid(std::f64::consts::TAU);
                return Record {class:0,nu,de,normal,jump,skipped,plain,
                    state_guard,parameter_guard,derivative_guard:derivative_reject};
            }
        }
        return Record {class:2,jump,skipped,plain,state_guard,parameter_guard,
            derivative_guard:derivative_reject,..Record::default()};
    }
    let mut plain=0;
    let mut phase_index=0;
    while n<max_iter {
        let z=m.phases[phase_index].add(delta.as_cx());
        dz=dz.mul_c(z.scale(2.0)).add(Scaled::new(ONE,0));
        delta=delta.mul_c(m.phases[phase_index].scale(2.0))
            .add(delta.mul(delta)).add(dc);
        phase_index=(phase_index+1)%m.p;
        n+=1;
        plain+=1;
        let next=m.phases[phase_index].add(delta.as_cx());
        let az=next.norm();
        if az>1e10 {
            let nu=n as f64+1.0-az.log2().log2();
            let h_ln=m.h_m.ln()+m.h_e as f64*LN2;
            let log_de=(2.0*az*az.ln()).ln()-dz.ln_norm()-h_ln;
            let de=log_de.exp();
            let v=next.div(dz.a);
            let normal=(rot-v.im.atan2(v.re)).rem_euclid(std::f64::consts::TAU);
            return Record {class:0,nu,de,normal,jump,skipped,plain,
                state_guard,parameter_guard,derivative_guard:derivative_reject};
        }
    }
    Record {class:2,jump,skipped,plain,state_guard,parameter_guard,
        derivative_guard:derivative_reject,..Record::default()}
}
fn main() {
    let args:Vec<String>=std::env::args().collect();
    assert!(args.len()==8, "usage: native-cycles CONSTS OUT NXxNY WIDTH MAXITER THREADS ROT");
    let m=read_model(&args[1]);
    assert_eq!(args[4],m.width,"native width must match exact model width");
    let (nx,ny)=args[3].split_once('x').map(|(x,y)|
        (x.parse::<usize>().unwrap(),y.parse::<usize>().unwrap())).unwrap();
    let max_iter:u64=args[5].parse().unwrap();
    let threads:usize=args[6].parse().unwrap();
    let rotation:f64=args[7].parse().unwrap();
    let (sin,cos)=rotation.sin_cos();
    let fast=fast_table(&m,nx,ny,max_iter);
    let t=Instant::now();
    let next=AtomicUsize::new(0);
    let rows:Vec<(usize,Vec<Record>)>=std::thread::scope(|scope|{
        let handles:Vec<_>=(0..threads.max(1)).map(|_|scope.spawn(||{
            let mut work=Vec::new();
            loop {
                let j=next.fetch_add(1,Ordering::Relaxed);
                if j>=ny {break;}
                let mut row=Vec::with_capacity(nx);
                for i in 0..nx {
                    let x=(i as f64+0.5-nx as f64/2.0)*m.h_m;
                    let y=-(j as f64+0.5-ny as f64/2.0)*m.h_m;
                    let units=Cx::new(x*cos-y*sin,x*sin+y*cos);
                    let candidate=fast.as_ref().and_then(|(h,powers)|
                        fast_pixel(&m,units.scale(2f64.powi(m.h_e as i32)),
                            *h,powers,max_iter,rotation));
                    row.push(candidate.unwrap_or_else(||
                        pixel(&m,Scaled::new(units,m.h_e),max_iter,rotation)));
                }
                work.push((j,row));
            }
            work
        })).collect();
        handles.into_iter().flat_map(|x|x.join().unwrap()).collect()
    });
    let mut data=vec![Record::default();nx*ny];
    for (j,row) in rows {data[j*nx..(j+1)*nx].copy_from_slice(&row);}
    let elapsed=t.elapsed().as_secs_f64();
    let mut bytes=Vec::with_capacity(data.len()*25);
    for a in &data { bytes.push(a.class); }
    for f in [
        (|x:&Record|x.nu) as fn(&Record)->f64,
        (|x:&Record|x.de) as fn(&Record)->f64,
        (|x:&Record|x.normal) as fn(&Record)->f64,
    ] { for a in &data { bytes.extend_from_slice(&f(a).to_le_bytes()); } }
    fs::write(&args[2],bytes).unwrap();
    let totals=data.iter().fold([0u64;7],|mut s,x|{
        for (dst,value) in s.iter_mut().zip([x.jump,x.skipped,x.plain,
                x.state_guard,x.parameter_guard,x.derivative_guard,1]) {*dst+=value;}
        s
    });
    println!(r#"{{"native_seconds":{},"jumps":{},"skipped":{},"finish_steps":{},"state_fallbacks":{},"parameter_fallbacks":{},"derivative_fallbacks":{},"pixels":{},"fast_table":{}}}"#,
        elapsed,totals[0],totals[1],totals[2],totals[3],totals[4],totals[5],totals[6],fast.is_some());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_centre_and_sub_f64_offset_are_not_rounded() {
        let m=parse_model("center_re 1.000000000000000000000000000000000000000001e-400\n\
            center_im 1\nwidth 1e-400\nscale 0.75 -1332\nq 1\np 1\n\
            radius 0.002\nparameter_radius 0.00001\nbias 0.5 0 -1332\n\
            dp 0 0\ndlam 0 0\nrho 2 0\nphase 0 0 1\n\
            ref 0 0 0\nref 1 0 1\nk 1 1 0\ndk 1 0 0\n");
        assert_eq!(m.center_re,"1.000000000000000000000000000000000000000001e-400");
        assert_eq!(m.width,"1e-400");
        let dc=Scaled::new(Cx::new(0.75,0.25),m.h_e);
        assert!(dc.ln_norm().is_finite());
        assert!(dc.as_cx().norm()==0.0);
        assert!(m.bias.add(dc).a.norm()>0.0);
    }
    #[test]
    fn scaled_jump_does_not_overflow() {
        let tiny=Scaled::new(ONE,-3500);
        let mul=Scaled::from_polar(3500.0*LN2,0.0);
        let output=tiny.mul(mul).as_cx();
        assert!((output.re-1.0).abs()<1e-10);
    }
}
