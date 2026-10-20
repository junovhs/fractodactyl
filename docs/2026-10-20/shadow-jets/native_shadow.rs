//! PROB-40 independent research prototype: real-reference shadow-jet preprocessing.
//! No production integration, no outward-rounded error certificate.
use std::{env,fs,hint::black_box,time::Instant};
#[derive(Clone,Copy,Default)]
struct C{r:f64,i:f64}
impl C{
 fn add(self,b:C)->C{C{r:self.r+b.r,i:self.i+b.i}}
 fn sub(self,b:C)->C{C{r:self.r-b.r,i:self.i-b.i}}
 fn mul(self,b:C)->C{C{r:self.r*b.r-self.i*b.i,i:self.r*b.i+self.i*b.r}}
 fn scale(self,t:f64)->C{C{r:self.r*t,i:self.i*t}}
 fn div(self,b:C)->C{let d=b.norm2();C{r:(self.r*b.r+self.i*b.i)/d,i:(self.i*b.r-self.r*b.i)/d}}
 fn norm2(self)->f64{self.r*self.r+self.i*self.i}
 fn abs(self)->f64{self.norm2().sqrt()}
}
struct Point{eps:C,n:usize,z:C,q:C}
#[derive(Clone)]
struct Chart{n:usize,w:f64,p:Vec<f64>,q1:f64,e:f64,f:f64,work:usize,grows:usize,trace:[usize;3]}
fn build(k:usize,p:usize,rational:bool,rule:&str,radius:f64,phase:f64,maxn:usize)->Chart{
 let rho=radius*1e-8;let ec=0.5e-8;
 let mut active=if rule.starts_with("adapt"){8.min(k)}else{k};
 let growth=match rule{"adapt0"=>1.0,"adapt1"=>1e-1,"adapt2"=>1e-2,"adapt4"=>1e-4,"adapt8"=>1e-8,"adapt12"=>1e-12,_=>0.};
 let mut b=vec![0.0_f64;k+1];let mut w=-0.5;let mut e=0.;let mut f=0.;
 let mut best=Chart{n:0,w,p:vec![0.],q1:0.,e:0.,f:0.,work:0,grows:0,trace:[0;3]};
 let mut work=0usize;let mut grows=0usize;let mut trace=[0;3];
 let mut sq=vec![0.0_f64;2*k+1];let mut v=vec![0.0_f64;k+1];
 for n in 1..=maxn{
  sq.fill(0.);work+=active*(active+1)/2;
  for i in 1..=active{for j in i..=active{sq[i+j]+=b[i]*b[j]*if i==j{1.}else{2.};}}
  let m:f64=b.iter().map(|a|a.abs()).sum();
  let d:f64=b.iter().enumerate().map(|(j,a)|j as f64*a.abs()).sum();
  let tail:f64=sq[active+1..].iter().map(|a|a.abs()).sum();
  let dtail:f64=sq.iter().enumerate().skip(active+1).map(|(j,a)|j as f64*a.abs()).sum();
  let en=2.*((0.5+w).abs()+m)*e+e*e+tail;
  let fn_=2.*((0.5+w).abs()+m+e)*f+2.*e*d+dtail;
  let a=1.+2.*w;
  for j in 0..=k{v[j]=if j<=active{a*b[j]+sq[j]}else{0.};}v[1]+=rho;
  std::mem::swap(&mut b,&mut v);w=w+w*w+ec;e=en;f=fn_;
  let mn:f64=b.iter().map(|a|a.abs()).sum();
  let dn:f64=b.iter().enumerate().map(|(j,a)|j as f64*a.abs()).sum();
  if !e.is_finite() || !f.is_finite() || (0.5+w).abs()+mn+e>=2.{break;}
  if growth>0. && active<k && e>1e-15*growth{active=(2*active).min(k);trace[grows]=n;grows+=1;}
  if n%32!=0{continue;}
  let q1=if rational && b[p-1].abs()>1e-300{-b[p]/b[p-1]}else{0.};
  let den=1.-q1.abs();if den<=0.{continue;}
  let degree=if rational && q1!=0.{p-1}else{p};
  let mut pp=vec![0.0_f64;degree+1];
  for j in 0..=degree{pp[j]=b[j]+if j>0{q1*b[j-1]}else{0.};}
  let mut h=0.;let mut dh=0.;
  for j in 0..=k+1{
   let x=if j<=k{b[j]}else{0.}+if j>0{q1*b[j-1]}else{0.}-if j<=degree{pp[j]}else{0.};
   h+=x.abs();dh+=j as f64*x.abs();
  }
  let err=h/den+e;let der=dh/den+h*q1.abs()/(den*den)+f;
  let mut g0=(w*w+ec).abs();
  if rule=="phase-disk"{
   // Coefficient norm of (w+S)^2+ec+rho*x away from its constant.
   let mut spread=0.;
   for j in 1..=2*k{
    let mut x=0.;
    for i in 1..=k{if j>i && j-i<=k{x+=b[i]*b[j-i];}}
    if j<=k{x+=2.*w*b[j];}if j==1{x+=rho;}spread+=x.abs();
   }
   g0=(g0-spread-2.*(w.abs()+mn)*e-e*e).max(0.);
  }
  let budget=if rule=="fixed" || rule.starts_with("adapt"){1e-15}else{phase*g0};
  if err<=budget && der<=1e-8*rho.max(dn+f){best=Chart{n,w,p:pp,q1,e:err,f:der,work,grows,trace};}
 }
 best.work=work;best.grows=grows;best.trace=trace;best
}
fn horner(p:&[f64],x:C)->(C,C){let(mut v,mut d)=(C::default(),C::default());for &a in p.iter().rev(){d=d.mul(x).add(v);v=v.mul(x).add(C{r:a,i:0.});}(v,d)}
fn evaluate(c:&Chart,eps:C,radius:f64)->(usize,C,C){
 let x=eps.scale(1e8).sub(C{r:0.5,i:0.}).scale(1./radius);
 let(p,dp)=horner(&c.p,x);let den=C{r:1.,i:0.}.add(x.scale(c.q1));
 let mut w=C{r:c.w,i:0.}.add(p.div(den));
 let mut q=dp.mul(den).sub(p.scale(c.q1)).div(den.mul(den)).scale(1./radius);
 let mut n=c.n;
 while n<100000 && w.add(C{r:0.5,i:0.}).norm2()<=4.{q=C{r:1.,i:0.}.add(w.scale(2.)).mul(q).add(C{r:1e-8,i:0.});w=w.add(w.mul(w)).add(eps);n+=1;}
 (n,w.add(C{r:0.5,i:0.}),q)
}
fn num<T:std::str::FromStr>(it:&mut std::str::SplitWhitespace<'_>)->T{it.next().unwrap().parse().ok().unwrap()}
fn cx(it:&mut std::str::SplitWhitespace<'_>)->C{C{r:num(it),i:num(it)}}
fn cold(pts:&[Point],radius:f64,maxn:usize,maxbatch:usize){
 let started=Instant::now();
 let configs=[("plain",0,0,false,"fixed"),("K32-R16",32,16,true,"fixed"),("K32-P32",32,32,false,"fixed"),("K64-P64",64,64,false,"fixed"),("A64-g4",64,64,false,"adapt4")];
 println!("batch,method,cold_seconds,speedup,errors,score");
 let mut checksum=0.;
 for count in [9,128,512,2048,4096]{
  if count>maxbatch{continue;}
  let mut samples=vec![Vec::new();configs.len()];let mut errors=vec![0;configs.len()];
  for round in 0..3{for off in 0..configs.len(){let j=(round+off)%configs.len();let(name,k,p,r,rule)=configs[j];let t=Instant::now();
   let c=if k==0{Chart{n:0,w:-0.5,p:vec![0.],q1:0.,e:0.,f:0.,work:0,grows:0,trace:[0;3]}}else{build(k,p,r,rule,radius,1e-6,maxn)};
   for i in 0..count{let pt=&pts[i%pts.len()];let(n,z,q)=evaluate(black_box(&c),black_box(pt.eps),radius);checksum+=black_box(n as f64+z.r+q.r*1e-12);}
   samples[j].push(t.elapsed().as_secs_f64());
   if round==0{for pt in pts{let(n,z,q)=evaluate(&c,pt.eps,radius);let de=q.sub(pt.q).abs()/pt.q.abs();let escaped=z.norm2()>4.;let nu=if escaped{(z.abs().ln().log2()-pt.z.abs().ln().log2()).abs()}else{0.};
    if n!=pt.n || escaped!=(pt.z.norm2()>4.) || z.sub(pt.z).abs()>1e-6*pt.z.abs().max(1.) || de>1e-6 || nu>1e-6{errors[j]+=1;}
   }}
   black_box(name);
  }}
  assert!(errors.iter().all(|&e|e==0),"cold batch accuracy failed");
  let med:Vec<_>=samples.iter_mut().map(|v|{v.sort_by(f64::total_cmp);v[1]}).collect();
  for(j,(name,_,_,_,_))in configs.iter().enumerate(){let speed=med[0]/med[j];let score=if errors[j]==0{speed}else{0.};println!("{count},{name},{:.9},{speed:.6},{},{score:.6}",med[j],errors[j]);}
 }
 eprintln!("Actual cold batches: fresh reference+coefficients+guard per run, median 3 rotated rounds; repeated frozen points, NOT frames; checksum={checksum}; elapsed_s={:.6}",started.elapsed().as_secs_f64());
}
fn main(){
 let args:Vec<_>=env::args().collect();let path=args.get(1).expect("frozen chart export with points");let maxn=args.get(2).map(|s|s.parse().unwrap()).unwrap_or(36000);
 let text=fs::read_to_string(path).unwrap();let mut it=text.split_whitespace();let np:usize=num(&mut it);let mut pts=Vec::new();
 for _ in 0..np{pts.push(Point{eps:cx(&mut it),n:num(&mut it),z:cx(&mut it),q:cx(&mut it)});}
 // Trailing old chart data intentionally unused: all candidates are freshly built.
 let radius=0.21;let phase=1e-6;
 if args.get(3).map(String::as_str)==Some("cold"){let maxbatch=args.get(4).map(|s|s.parse().unwrap()).unwrap_or(4096);cold(&pts,radius,maxn,maxbatch);return;}
 let configs=[("plain",0,0,false,"fixed"),("K32-R16",32,16,true,"fixed"),("K32-P32",32,32,false,"fixed"),("K64-R16",64,16,true,"fixed"),("K64-R24",64,24,true,"fixed"),("K64-P32",64,32,false,"fixed"),("K64-P64",64,64,false,"fixed"),("K64-R16-phase",64,16,true,"phase-ref"),("K64-R16-disk",64,16,true,"phase-disk"),("A64-g0",64,64,false,"adapt0"),("A64-g1",64,64,false,"adapt1"),("A64-g2",64,64,false,"adapt2"),("A64-g4",64,64,false,"adapt4"),("A64-g8",64,64,false,"adapt8"),("A64-g12",64,64,false,"adapt12")];
 println!("method,prefix,errors,pixels,build_s,pixel_s,warm_speedup,break_even_pixels,cold9_speedup,cold512_speedup,cold4096_speedup,max_derivative_relative_error,max_smooth_error,state_bound,derivative_bound,coefficient_pairs,order_doublings,growth_8_16,growth_16_32,growth_32_64");
 let mut baseline=0.;let mut failed=0;
 for(name,k,p,r,rule)in configs{
  let t=Instant::now();let chart=if k==0{Chart{n:0,w:-0.5,p:vec![0.],q1:0.,e:0.,f:0.,work:0,grows:0,trace:[0;3]}}else{build(k,p,r,rule,radius,phase,maxn)};let setup=if k==0{0.}else{t.elapsed().as_secs_f64()};
  let mut bad=0;let mut mq:f64=0.;let mut ms:f64=0.;
  for pt in &pts{let(n,z,q)=evaluate(&chart,pt.eps,radius);let de=q.sub(pt.q).abs()/pt.q.abs();let escaped=z.norm2()>4.;let nu=if escaped{(z.abs().ln().log2()-pt.z.abs().ln().log2()).abs()}else{0.};
   if n!=pt.n || escaped!=(pt.z.norm2()>4.) || z.sub(pt.z).abs()>1e-6*pt.z.abs().max(1.) || de>1e-6 || nu>1e-6{bad+=1;}mq=mq.max(de);ms=ms.max(nu);
  }
  let mut timings=Vec::new();let mut checksum=0.;
  for _ in 0..5{let t=Instant::now();for _ in 0..8{for pt in &pts{let(n,z,q)=evaluate(black_box(&chart),black_box(pt.eps),radius);checksum+=black_box(n as f64+z.r+q.r*1e-12);}}timings.push(t.elapsed().as_secs_f64()/(8*np)as f64);}
  black_box(checksum);timings.sort_by(f64::total_cmp);let per=timings[2];if k==0{baseline=per;}
  let be=if per<baseline{(setup/(baseline-per)).ceil()}else{f64::INFINITY};
  let cold=|n:f64|n*baseline/(setup+n*per);
  println!("{name},{},{bad},{np},{setup:.9},{per:.12},{:.6},{be},{:.6},{:.6},{:.6},{mq:.12e},{ms:.12e},{:.12e},{:.12e},{},{},{},{},{}",chart.n,baseline/per,cold(9.),cold(512.),cold(4096.),chart.e,chart.f,chart.work,chart.grows,chart.trace[0],chart.trace[1],chart.trace[2]);
  failed+=bad;
 }
 eprintln!("all builds include real centered reference, coefficients, shadow remainder and admission; cold ratios are setup+N*measured per-point cost extrapolations; failures={failed}");
 assert_eq!(failed,0,"native sparse accuracy failed");
}
