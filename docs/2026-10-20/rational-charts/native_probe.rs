//! PROB-39 research evaluator only. No renderer, guard builder or native setup benchmark.
//! Reads frozen chart coefficients and truth exported by local_probe.py.
use std::{env, fs, hint::black_box, time::Instant};

#[derive(Clone, Copy, Default)]
struct C { r: f64, i: f64 }
impl C {
    fn add(self, b: C) -> C { C { r: self.r+b.r, i: self.i+b.i } }
    fn sub(self, b: C) -> C { C { r: self.r-b.r, i: self.i-b.i } }
    fn mul(self, b: C) -> C { C { r: self.r*b.r-self.i*b.i, i: self.r*b.i+self.i*b.r } }
    fn scale(self, b: f64) -> C { C { r: self.r*b, i: self.i*b } }
    fn div(self, b: C) -> C { let d=b.norm2(); C { r: (self.r*b.r+self.i*b.i)/d, i: (self.i*b.r-self.r*b.i)/d } }
    fn norm2(self) -> f64 { self.r*self.r+self.i*self.i }
    fn abs(self) -> f64 { self.norm2().sqrt() }
}
struct Point { eps: C, n: usize, z: C, q: C }
struct Chart { name: String, n: usize, w: C, center: f64, radius: f64, p: Vec<C>, q: Vec<C> }
fn num<T: std::str::FromStr>(it: &mut std::str::SplitWhitespace<'_>) -> T { it.next().unwrap().parse().ok().unwrap() }
fn cx(it: &mut std::str::SplitWhitespace<'_>) -> C { C { r: num(it), i: num(it) } }
fn horner(p: &[C], x: C) -> (C,C) {
    let (mut v, mut d)=(C::default(),C::default());
    for &a in p.iter().rev() { d=d.mul(x).add(v); v=v.mul(x).add(a); }
    (v,d)
}
fn evaluate(c: &Chart, eps: C) -> (usize,C,C) {
    let x=eps.scale(1e8).sub(C { r:c.center,i:0. }).scale(1./c.radius);
    let (p,dp)=horner(&c.p,x); let (den,dd)=horner(&c.q,x);
    let mut w=c.w.add(p.div(den));
    let mut q=dp.mul(den).sub(p.mul(dd)).div(den.mul(den)).scale(1./c.radius);
    let mut n=c.n;
    while n<100000 && w.add(C{r:0.5,i:0.}).norm2()<=4. {
        q=C{r:1.,i:0.}.add(w.scale(2.)).mul(q).add(C{r:1e-8,i:0.});
        w=w.add(w.mul(w)).add(eps); n+=1;
    }
    (n,w.add(C{r:0.5,i:0.}),q)
}
fn main() {
    let path=env::args().nth(1).expect("chart export path");
    let input=fs::read_to_string(path).unwrap();let mut it=input.split_whitespace();
    let np:usize=num(&mut it);let mut points=Vec::new();
    for _ in 0..np { points.push(Point {eps:cx(&mut it),n:num(&mut it),z:cx(&mut it),q:cx(&mut it)}); }
    let nc:usize=num(&mut it);let mut charts=Vec::new();
    for _ in 0..nc {
        let name=it.next().unwrap().to_owned();let n=num(&mut it);let w=cx(&mut it);
        let center=num(&mut it);let radius=num(&mut it);let lp:usize=num(&mut it);let lq:usize=num(&mut it);
        let p=(0..lp).map(|_|cx(&mut it)).collect();let q=(0..lq).map(|_|cx(&mut it)).collect();
        charts.push(Chart{name,n,w,center,radius,p,q});
    }
    assert!(it.next().is_none());
    let mut failures=Vec::new();let mut worsts=Vec::new();
    for c in &charts {
        let mut bad=0;let mut worst_q:f64=0.;let mut worst_nu:f64=0.;
        for p in &points {
            let (n,z,q)=evaluate(c,p.eps);
            let qe=q.sub(p.q).abs()/p.q.abs();
            let nu=(z.abs().ln().log2()-p.z.abs().ln().log2()).abs();
            if n!=p.n || (z.norm2()>4.)!=(p.z.norm2()>4.) || z.sub(p.z).abs()>1e-6*p.z.abs().max(1.) || qe>1e-6 || nu>1e-6 {bad+=1;}
            worst_q=worst_q.max(qe);worst_nu=worst_nu.max(nu);
        }
        failures.push(bad);worsts.push((worst_q,worst_nu));
    }
    let mut times=vec![Vec::new();nc]; let mut checksum=0.;
    // Rotate execution order across rounds; same 9 points, 32 repetitions per arm.
    for round in 0..7 {
        for k in 0..nc {
            let j=(k+round)%nc;let t=Instant::now();
            for _ in 0..32 { for p in &points { let (n,z,q)=evaluate(black_box(&charts[j]),black_box(p.eps));checksum+=black_box(n as f64+z.r+q.r*1e-12); } }
            times[j].push(t.elapsed().as_secs_f64()/(32*np) as f64);
        }
    }
    let medians:Vec<_>=times.iter_mut().map(|v|{v.sort_by(f64::total_cmp);v[3]}).collect();
    println!("method,prefix,pixels,failures,pixel_s,warm_speedup,max_derivative_relative_error,max_smooth_error");
    for (j,c) in charts.iter().enumerate() {println!("{},{},{},{},{:.12},{:.6},{:.12e},{:.12e}",c.name,c.n,np,failures[j],medians[j],medians[0]/medians[j],worsts[j].0,worsts[j].1);}
    eprintln!("checksum={checksum:.9}; seven rotated rounds; 32 repeats; evaluator only; setup excluded");
    assert!(failures.iter().all(|&n|n==0),"candidate fails sparse frozen truth");
}
