//! Native, camera-compiled Koenigs jump research kernel.
//! The Python discoverer generates all cycle and linearizer constants; no zone-specific
//! values or periods are baked into this binary. This deliberately does not write .fds:
//! the driver scores all columns against fd's independent per-frame BLA .fds output.
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

#[derive(Clone, Copy, Debug, Default)]
struct Cx { re: f64, im: f64 }
impl Cx {
    fn new(re: f64, im: f64) -> Self { Self { re, im } }
    fn add(self, b: Self) -> Self { Self::new(self.re+b.re, self.im+b.im) }
    fn sub(self, b: Self) -> Self { Self::new(self.re-b.re, self.im-b.im) }
    fn mul(self, b: Self) -> Self {
        Self::new(self.re*b.re-self.im*b.im, self.re*b.im+self.im*b.re)
    }
    fn scale(self, v: f64) -> Self { Self::new(self.re*v, self.im*v) }
    fn norm(self) -> f64 { self.re.hypot(self.im) }
    fn div(self,b:Self) -> Self {
        let n=b.re*b.re+b.im*b.im;
        Self::new((self.re*b.re+self.im*b.im)/n,(self.im*b.re-self.re*b.im)/n)
    }
    fn pow(self,mut k:u64)->Self {
        let (mut a,mut r)=(self,Self::new(1.0,0.0));
        while k>0 { if k&1==1 {r=r.mul(a);} k>>=1; if k>0 {a=a.mul(a);} }
        r
    }
    fn finite(self)->bool { self.re.is_finite() && self.im.is_finite() }
}
#[derive(Default)]
struct Model {
    c: Cx, q:usize, p:u64, radius:f64, bias:Cx, point:Cx, dp:Cx,
    dlam:Cx, rho:Cx, reference:Vec<Cx>, k:Vec<Cx>, dk:Vec<Cx>
}
fn read_model(path:&str)->Model {
    let s=fs::read_to_string(path).expect("read compiled model");
    let mut m=Model::default();
    for l in s.lines() {
        if l.trim().is_empty() || l.starts_with('#') {continue}
        let a:Vec<&str>=l.split_whitespace().collect();
        let v=|idx:usize| a[idx].parse::<f64>().expect("numeric field");
        let cx=|idx:usize| Cx::new(v(idx),v(idx+1));
        match a[0] {
            "c"=>m.c=cx(1), "q"=>m.q=v(1) as usize, "p"=>m.p=v(1) as u64,
            "radius"=>m.radius=v(1), "bias"=>m.bias=cx(1),
            "point"=>m.point=cx(1), "dp"=>m.dp=cx(1),
            "dlam"=>m.dlam=cx(1),"rho"=>m.rho=cx(1),
            "ref"=>m.reference.push(cx(2)),
            "k"=>m.k.push(cx(2)), "dk"=>m.dk.push(cx(2)),
            s=>panic!("unknown constant {s}")
        }
    }
    assert!(m.reference.len()==m.q+1 && !m.k.is_empty() && m.k.len()==m.dk.len());
    assert!(m.p>0 && m.radius>0.0);
    m
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
#[derive(Clone,Copy,Default)]
struct Record {class:u8,nu:f64,de:f64,normal:f64,jump:u64,skipped:u64,plain:u64}
fn pixel(m:&Model,dc:Cx,h:f64, max_iter:u64,rot:f64)->Record {
    let mut d=Cx::default();
    let mut der=Cx::default();
    for zref in &m.reference[..m.q] {
        der=zref.add(d).mul(der).scale(2.0).add(Cx::new(1.0,0.0));
        d=zref.scale(2.0).mul(d).add(d.mul(d)).add(dc);
    }
    let u=m.bias.add(d).sub(m.dp.mul(dc));
    let mut w=u;
    let (mut n,mut z,mut dz)=(m.q as u64,m.reference[m.q].add(d),der);
    let mut jump=0u64;
    let mut skipped=0u64;
    if u.norm()<m.radius/2.0 && u.norm()>0.0 {
        let lambda=m.rho.add(m.dlam.mul(dc));
        let lr=lambda.norm().ln();
        if lr>0.0 && lambda.finite() {
            for _ in 0..8 {
                let (val,derivative,_)=k_value(m,w,dc);
                let step=val.sub(u).div(derivative);
                w=w.sub(step);
                if !w.finite() { break; }
                if step.norm() <= 1e-15*w.norm().max(1e-200) {break}
            }
            if w.finite() && w.norm()>0.0 {
                let possible=((m.radius/w.norm()).ln()/lr).floor();
                let j=(possible.max(0.0) as u64).min((max_iter-n.saturating_add(1))/m.p);
                if j>=2 {
                    let mul=lambda.pow(j);
                    let wo=w.mul(mul);
                    if wo.finite() && wo.norm()<=m.radius {
                        let (f,df,dcf)=k_value(m,wo,dc);
                        let (_,d0,c0)=k_value(m,w,dc);
                        let ratio=mul.mul(df).div(d0);
                        let new_d=m.dp.add(dcf).add(ratio.mul(der.sub(m.dp).sub(c0)))
                            .add(m.dlam.div(lambda).mul(wo).mul(df).scale(j as f64));
                        let new_z=m.point.add(m.dp.mul(dc)).add(f);
                        if new_d.finite() && new_z.finite() {
                            z=new_z;dz=new_d;n+=j*m.p;jump=1;skipped=j*m.p;
                        }
                    }
                }
            }
        }
    }
    let c=m.c.add(dc);
    let mut plain=0u64;
    while n<max_iter {
        dz=z.mul(dz).scale(2.0).add(Cx::new(1.0,0.0));
        z=z.mul(z).add(c);
        n+=1;
        plain+=1;
        let az=z.norm();
        if az>1e10 {
            let nu=n as f64+1.0-(az.log2()).log2();
            let de=2.0*az*az.ln()/(dz.norm()*h);
            let ang=(rot-z.div(dz).im.atan2(z.div(dz).re)).rem_euclid(std::f64::consts::TAU);
            return Record {class:0,nu,de,normal:ang,jump,skipped,plain};
        }
    }
    Record {class:1,jump,skipped,plain,..Record::default()}
}
fn main() {
    let args:Vec<String>=std::env::args().collect();
    assert!(args.len()==9, "usage: native-cycles CONSTS OUT NXxNY WIDTH MAXITER THREADS ROT RESERVED");
    let m=read_model(&args[1]);
    let (nx,ny)=args[3].split_once('x').map(|(x,y)|(x.parse::<usize>().unwrap(),y.parse::<usize>().unwrap())).unwrap();
    let width:f64=args[4].parse().unwrap();
    let max_iter:u64=args[5].parse().unwrap();
    let threads:usize=args[6].parse().unwrap();
    let rotation:f64=args[7].parse().unwrap();
    let h=width/nx as f64;
    let (sin,cos)=rotation.sin_cos();
    let t=Instant::now();
    let next=AtomicUsize::new(0);
    let rows:Vec<(usize,Vec<Record>)>=std::thread::scope(|scope|{
        let handles:Vec<_>=(0..threads.max(1)).map(|_|scope.spawn(||{
            let mut work=Vec::new();
            loop {
                let j=next.fetch_add(1,Ordering::Relaxed);
                if j>=ny {break}
                let mut row=Vec::with_capacity(nx);
                for i in 0..nx {
                    let x=(i as f64+0.5-nx as f64/2.0)*h;
                    let y=-(j as f64+0.5-ny as f64/2.0)*h;
                    let dc=Cx::new(x*cos-y*sin,x*sin+y*cos);
                    row.push(pixel(&m,dc,h,max_iter,rotation));
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
    for a in &data {bytes.push(a.class);}
    for f in [
        (|x:&Record|x.nu) as fn(&Record)->f64,
        (|x:&Record|x.de) as fn(&Record)->f64,
        (|x:&Record|x.normal) as fn(&Record)->f64,
    ] {for a in &data {bytes.extend_from_slice(&f(a).to_le_bytes());}}
    fs::write(&args[2],bytes).unwrap();
    let (jumps,skipped,plain)=data.iter().fold((0u64,0u64,0u64),|(j,s,p),x|(j+x.jump,s+x.skipped,p+x.plain));
    println!(r#"{{"native_seconds":{},"jumps":{},"skipped":{},"finish_steps":{},"pixels":{}}}"#,
        elapsed,jumps,skipped,plain,data.len());
}
