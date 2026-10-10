// Standalone proof-of-concept, not a certified floating-point renderer.
// Build: g++ -O3 -std=c++17 -ffp-contract=off mandelbrot_field_benchmark.cpp -o fieldbench
#include <complex>
#include <cmath>
#include <chrono>
#include <iostream>
#include <vector>
#include <iomanip>
#include <algorithm>
#include <limits>
#include <string>
using C = std::complex<double>;
using Clock = std::chrono::steady_clock;
using namespace std;
static volatile double sink=0.;
#ifndef NUM_SAMPLES
#define NUM_SAMPLES 36
#endif
#ifndef DEGREE
#define DEGREE 15
#endif
#ifndef CERT_SCALE
#define CERT_SCALE 2.45
#endif
constexpr int MAXIT=400, N=NUM_SAMPLES, P=DEGREE;
struct Sample {double g; C fp; bool escaped; int n;};
Sample orbit(C c,int maxit=MAXIT) {
 C z=c,d=1.; int n=1;
 while (std::norm(z)<1e16 && n<maxit) {
  C zn=z; z=zn*zn+c; d=2.*zn*d+1.; n++;
 }
 if(std::norm(z)<1e16 || !std::isfinite(std::norm(z))) return {0.,0.,false,n};
 double scl=std::ldexp(1.,-(n-1));
 return {scl*std::log(std::abs(z)),scl*d/z,true,n};
}
// Bounds all critical orbits in a parameter disk |c-center|<=R by a simple ball recurrence.
// Sufficient (not necessary) disk escape certificate.
bool disk_escape(C center,double R) {
 C z=0; double dr=0;
 double crit=std::max(2.0,std::abs(center)+R+2.0);
 for(int k=0;k<MAXIT;k++) {
  double az=std::abs(z);
  double drnext=(2*az+dr)*dr+R;
  z=z*z+center; dr=drnext;
  if(std::abs(z)-dr>crit) return true;
  if(!std::isfinite(dr) || dr>1.e60) return false;
 }
 return false;
}
struct Patch { C c; double s; double radius; double g0; C b[P+1]; int degree=0; bool ok=false;};
struct Rect { double xmin,xmax,ymin,ymax; int w,h,tile; C point(int x,int y) const {return {xmin+(x+0.5)*(xmax-xmin)/w,ymin+(y+0.5)*(ymax-ymin)/h};}};
struct Run { double setupms,queryms,baselineacceptms,baselineallms,hybridms; double coverage,maxg,relg,maxfprel,rmsg,rmsfp; size_t nodes,failednodes,accepted,accepted_samples,bad;};
vector<C> roots;
C weights[P+1][N];
void init() { for(int j=0;j<N;j++) {roots.emplace_back(std::polar(1.0,-2.*M_PI*j/N));weights[0][j]=1.;for(int k=1;k<=P;k++) weights[k][j]=weights[k-1][j]*roots[j];} }
bool construct(Patch &p) {
 double rr=p.radius;
 double R=rr*CERT_SCALE;
 if(!disk_escape(p.c,R)) return false;
 // Enlarge the guaranteed analytic disk; this is what makes low order rigorously useful.
 double last=R;
 for(int j=0;j<7;j++) {
  double trial=R*2;
  if(!disk_escape(p.c,trial)) break;
  R=trial;last=R;
 }
 // The last successful disk is conservative but guaranteed by ball escape arithmetic.
 p.radius=R;
 p.s=.65*R;
 double q=rr/R, t=p.s/R;
 auto center=orbit(p.c);
 if(!center.escaped || !(center.g>0)) return false;
 p.g0=center.g;
 const double goal_g=1.e-4;
 const double goal_fp=1.e-3;
 // Harnack gives |a_k| <= 2*G(c0)/R^k for a positive harmonic Green function.
 double gmin=p.g0*(1-q)/(1+q);
 if(gmin<=0) return false;
 // Compile all Fourier coefficients once, select only as many as the error bounds need.
 double vals[N];
 for(int j=0;j<N;j++) {
   auto v=orbit(p.c+p.s*std::conj(roots[j]));
   if(!v.escaped) return false;
   vals[j]=v.g;
 }
 p.g0=0;
 for(double x:vals) p.g0+=x/N;
 int degree=0;
 for(int k=1;k<=P;k++) {
  C acc=0.;
  for(int j=0;j<N;j++)acc+=vals[j]*weights[k][j];
  p.b[k]=(2.0/N)*acc;
  if(k<2) continue;
   double Eg=2*p.g0*pow(q,k+1)/(1-q);
   double Efp=(2*p.g0/R)*pow(q,k)*((k+1)/(1-q)+q/((1-q)*(1-q)));
   // Aliasing estimate for N-point DFT of harmonic circle samples.
   double v=q/t, aliasg=2*p.g0*pow(t,N)/(1-pow(t,N));
   double aliasfp=0;
   for(int l=1;l<=k;l++){
      double eb=2*p.g0*(pow(t,N-l)+pow(t,N+l))/(1-pow(t,N));
      aliasg+=eb*pow(v,l);
      aliasfp+=l*eb*pow(v,l-1)/p.s;
   }
   // Assumed node-value absolute error allowance; a production renderer
   // must replace this with outward-rounded validated node enclosures.
   double Esample=2.e-13;
   double sampleg=Esample*(1+2*v/(1-v));
   double samplefp=2*Esample/(p.s*(1-v)*(1-v));
   double gerr=Eg+aliasg+sampleg;
   double fperr=Efp+aliasfp+samplefp;
   double fpmin=std::abs(p.b[1])/p.s - fperr;
   for(int l=2;l<=k;l++) fpmin-=l*std::abs(p.b[l])*pow(v,l-1)/p.s;
   if(fpmin>0 && gerr/gmin<=goal_g && fperr/fpmin<=goal_fp) {degree=k;break;}
 }
 if(!degree) return false;
 p.degree=degree;
 p.ok=true; return true;
}

Sample field(const Patch &p,C point) {
 C t=(point-p.c)/p.s;
 C val=0,d=0;
 for(int k=p.degree;k>=1;k--) { d=d*t+val; val=val*t+p.b[k]; }
 // val=sum_{k=1}^P b[k]*t^(k-1); d=derivative of that, so f'= (val +t*d)/s
 C f=p.g0+t*val;
 C fp=(val+t*d)/p.s;
 return {f.real(),fp,true,0};
}
static double msec(Clock::time_point a,Clock::time_point b){return chrono::duration<double,milli>(b-a).count();}
Run bench(Rect rect,bool validate) {
 int tx=(rect.w+rect.tile-1)/rect.tile,ty=(rect.h+rect.tile-1)/rect.tile;
 vector<Patch> patches(tx*ty);
 auto t0=Clock::now(); size_t goodpatches=0,failed=0; int psum=0,pmax=0,pmin=999;
 for(int y=0;y<ty;y++)for(int x=0;x<tx;x++) {
  int x0=x*rect.tile,x1=min(rect.w,x0+rect.tile),y0=y*rect.tile,y1=min(rect.h,y0+rect.tile);
  auto &p=patches[y*tx+x];
  p.c={rect.xmin+(x0+x1)*.5*(rect.xmax-rect.xmin)/rect.w, rect.ymin+(y0+y1)*.5*(rect.ymax-rect.ymin)/rect.h};
  double rx=(x1-x0)*.5*(rect.xmax-rect.xmin)/rect.w,ry=(y1-y0)*.5*(rect.ymax-rect.ymin)/rect.h;
  p.radius=std::hypot(rx,ry);
  if(construct(p)) {goodpatches++; psum+=p.degree;pmax=std::max(pmax,p.degree);pmin=std::min(pmin,p.degree);}
  else failed++;
 }
 auto t1=Clock::now();
 vector<unsigned char> covered(rect.w*rect.h,0);
 size_t accepted=0; double sg=0;
 // Time field queries on all accepted pixels, with indexing and selection.
 for(int y=0;y<rect.h;y++) for(int x=0;x<rect.w;x++) {
  const auto &p=patches[(y/rect.tile)*tx+(x/rect.tile)];
  if(!p.ok) continue;
  auto v=field(p,rect.point(x,y));
  covered[y*rect.w+x]=1; accepted++; sg+=v.g + 0.01*std::abs(v.fp);
 }
 auto t2=Clock::now();sink=sg;
 double meang2=0,maxg=0,maxfprel=0,rmsg2=0,rmsfp2=0;
 size_t good=0,bad=0;
 if(validate) {
  // Validation on deterministic held-out pixel centers (not training samples).
  for(int y=0;y<rect.h;y+=3) for(int x=0;x<rect.w;x+=3) {
   if(!covered[y*rect.w+x]) continue;
   const Patch&p=patches[(y/rect.tile)*tx+(x/rect.tile)];
   auto a=field(p,rect.point(x,y));auto b=orbit(rect.point(x,y));
   if(!b.escaped){bad++;continue;}
   double er=std::abs(a.g-b.g)/std::max(b.g,1.e-300);
   double ef=std::abs(a.fp-b.fp)/std::max(std::abs(b.fp),1.e-300);
   maxg=std::max(maxg,er); maxfprel=std::max(maxfprel,ef);
   rmsg2+=er*er;rmsfp2+=ef*ef;good++;
  }
 }
 // Baseline orbital queries only on covered, not advantaged/disadvantaged by interiors.
 auto b0=Clock::now();double sb=0;
 for(int y=0;y<rect.h;y++)for(int x=0;x<rect.w;x++)if(covered[y*rect.w+x]) {
  auto v=orbit(rect.point(x,y));sb+=v.g+0.01*std::abs(v.fp);
 }
 auto b1=Clock::now();sink=sb;
 // Hybrid additional fallbacks, includes full maxit interior which common to both methods.
 auto h0=Clock::now();double sf=0;
 for(int y=0;y<rect.h;y++)for(int x=0;x<rect.w;x++)if(!covered[y*rect.w+x]) {
  auto v=orbit(rect.point(x,y));sf+=v.g+0.01*std::abs(v.fp);
 }
 auto h1=Clock::now();sink=sf;
 double ac=msec(b0,b1),fb=msec(h0,h1),setup=msec(t0,t1),q=msec(t1,t2);
 return {setup,q,ac,ac+fb,setup+q+fb,double(accepted)/(rect.w*rect.h),maxg,0,maxfprel,good?std::sqrt(rmsg2/good):0.,good?std::sqrt(rmsfp2/good):0.,goodpatches,failed,accepted,good,bad};
}
int main(int argc,char**argv){
 init();
 int tile=argc>1?atoi(argv[1]):24;
 int w=argc>2?atoi(argv[2]):960,h=argc>3?atoi(argv[3]):720;
 vector<pair<string,Rect>> test={
  {"wide", {-2.2,1.1,-1.35,1.35,w,h,tile}},
  {"boundary", {-.95,-.55,.05,.45,w,h,tile}},
  {"exterior", {-1.7,-.2,.45,1.25,w,h,tile}},
  {"righttip", {.255,.355,-.07,.07,w,h,tile}},
  {"leftspike", {-2.08,-1.78,.015,.10,w,h,tile}},
  {"faroutside", {.6,1.8,.55,1.45,w,h,tile}}
 };
 cout<<setprecision(5)<<fixed;
 for(auto &[name,rect]:test){
  if(argc>4 && name!=argv[4])continue;
  auto a=bench(rect,true);
  cout<<name<<",tile="<<tile<<",res="<<w<<"x"<<h<<",patch="<<a.nodes<<",failed="<<a.failednodes<<",covered="<<a.coverage
   <<",construction_ms="<<a.setupms<<",lookup_ms="<<a.queryms<<",orbit_covered_ms="<<a.baselineacceptms
   <<",orbit_all_ms="<<a.baselineallms<<",hybrid_all_ms="<<a.hybridms
   <<",speed_covered="<<(a.baselineacceptms/(a.setupms+a.queryms))
   <<",speed_full="<<(a.baselineallms/a.hybridms)
   <<",max_rel_G="<<std::scientific<<a.maxg<<",rms_rel_G="<<a.rmsg
   <<",max_rel_Fprime="<<a.maxfprel<<",rms_rel_Fprime="<<a.rmsfp<<std::fixed
   <<",sampled="<<a.accepted_samples<<",failcount="<<a.bad<<"\n";
 }
}
