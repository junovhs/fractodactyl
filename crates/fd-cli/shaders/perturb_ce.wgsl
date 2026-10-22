// Reuse the production float-float library: no second arithmetic implementation.
alias PN = CE;
fn pn_zero() -> PN {return CE(ff(0.0),ff(0.0),0);}
fn pn_one() -> PN {return CE(ff(0.5),ff(0.0),1);}
fn pn_add(a:PN,b:PN) -> PN {return ce_add(a,b);}
fn pn_mul(a:PN,b:PN) -> PN {return ce_mul(a,b);}
fn pn_scale(a:PN,b:f32) -> PN {return ce_norm(ff_mul_f(a.re,b),ff_mul_f(a.im,b),a.e);}
fn pn_bad(a:PN) -> bool {return cf_bad(CF(vec2<f32>(a.re.hi,a.im.hi),a.e))||cf_bad(CF(vec2<f32>(a.re.lo,a.im.lo),a.e));}
fn pn_input_ok(x:PN) -> bool {let a=max(abs(x.re.hi),abs(x.im.hi));return !pn_bad(x)&&(a==0.0||(a>=0.5&&a<1.0));}
fn pn_in_range(x:PN) -> bool {return x.e>=-16000&&x.e<=16000;}
// Nominal branch choices need not certify output: the host's unchanged lower-radius
// guard and pre-loss CPU restart decide admission. Retain low components here.
fn pn_norm_le(a:PN,b:PN) -> bool {
 let aa=ff_add(ff_mul(a.re,a.re),ff_mul(a.im,a.im));let bb=ff_add(ff_mul(b.re,b.re),ff_mul(b.im,b.im));
 if(aa.hi==0.0){return true;}if(bb.hi==0.0){return false;}
 let e=a.e-b.e;if(e>1){return false;}if(e< -1){return true;}
 let d=ff_sub(ff_scale(aa,2*e),bb);return d.hi<0.0||(d.hi==0.0&&d.lo<=0.0);
}
fn pn_magnitude_upper(a:PN) -> CF {
 // hi+lo input is nonoverlapping; 1e-4 covers conversion and f32 sqrt, used only
 // as an upper bound. It is NOT charged as an additive CE state error.
 return cf_magnitude_upper(CF(vec2<f32>(a.re.hi,a.im.hi),a.e));
}

// Empirical capability gate, consistent with GPU27's device-validated FF contract.
// Not a universal rounding proof; a failed capability check must restart on CPU.
fn pn_capability_ok() -> bool {
 let a=opq(1.0);let b=opq(5.960464477539063e-8);
 let sum=two_sum(a,b);let cancel=two_sum(a,-opq(0.9999999403953552));
 let v=opq(1.000244140625);let prod=two_prod(v,v);
 let z=ce_norm(ff(opq(0.75)),ff(opq(0.5)),0);let square=ce_mul(z,z);
 return sum.hi==a&&sum.lo==b&&cancel.hi==b&&cancel.lo==0.0&&prod.hi==opq(1.00048828125)&&prod.lo==b&&square.e==0&&square.re.hi==opq(0.3125)&&square.re.lo==0.0&&square.im.hi==opq(0.75)&&square.im.lo==0.0;
}
