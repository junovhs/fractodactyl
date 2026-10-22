// Number adapter for the shared perturbation recurrence's original f32 tier.
alias PN = CF;
fn pn_zero() -> PN {return cf_zero();}
fn pn_one() -> PN {return cf_one();}
fn pn_add(a:PN,b:PN) -> PN {return cf_add(a,b);}
fn pn_mul(a:PN,b:PN) -> PN {return cf_mul(a,b);}
fn pn_scale(a:PN,b:f32) -> PN {return cf_scale(a,b);}
fn pn_bad(a:PN) -> bool {return cf_bad(a);}
fn pn_input_ok(x:PN) -> bool {let a=max(abs(x.m.x),abs(x.m.y));return !cf_bad(x)&&(a==0.0||(a>=0.5&&a<1.0));}
fn pn_in_range(x:PN) -> bool {return x.e>=-16000&&x.e<=16000;}
fn pn_norm_le(a:PN,b:PN) -> bool {let aa=dot(a.m,a.m);let bb=dot(b.m,b.m);if(aa==0.0){return true;}if(bb==0.0){return false;}let e=a.e-b.e;if(e>1){return false;}if(e< -1){return true;}return aa*pow2(2*e)<=bb;}
