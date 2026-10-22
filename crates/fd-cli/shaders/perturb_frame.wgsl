// Experimental CE frame adapter; unchanged escaped-only output/fallback contract.
@group(0) @binding(0) var<storage, read> reference_words: array<vec4<u32>>;
@group(0) @binding(1) var<storage, read> offset_words: array<vec4<u32>>;
@group(0) @binding(2) var<storage, read> args: array<u32>;
@group(0) @binding(3) var<storage, read_write> result: array<vec4<u32>>;
fn unpack_cf(w: vec4<u32>) -> CF {return CF(vec2<f32>(bitcast<f32>(w.x),bitcast<f32>(w.y)),bitcast<i32>(w.z));}
fn packed_cf(x:CF) -> vec4<u32> {return vec4<u32>(bitcast<u32>(x.m.x),bitcast<u32>(x.m.y),bitcast<u32>(x.e),0u);}
fn unpack_ce(a:vec4<u32>,b:vec4<u32>) -> CE {return CE(FF(bitcast<f32>(a.x),bitcast<f32>(a.z)),FF(bitcast<f32>(a.y),bitcast<f32>(a.w)),bitcast<i32>(b.x));}
fn inflated(x:CF) -> CF {return cf_scale(x,1.0001);}
fn radd(a:CF,b:CF) -> CF {return inflated(cf_add(a,b));}
fn rmul(a:CF,b:CF) -> CF {return inflated(cf_mul(a,b));}
// Conditional RN model allowance256u^2; empirical Heuristic guard, NOT a
// portable rigorous arithmetic bound. See ce-error-bound.md.
fn rounding(x:CF) -> CF {return inflated(cf_scale(x,9.094947017729282e-13));}
fn reference_z(m:u32) -> CE {return unpack_ce(reference_words[3u*m],reference_words[3u*m+1u]);}
fn reference_q(m:u32) -> CF {return unpack_cf(reference_words[3u*m+2u]);}
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 let i=id.x+args[12];if(i>=args[0]){return;}
 result[8u*i+7u]=vec4<u32>(0x47505501u,i,0u,0u);
 if(args[3]==2u){return;} // Test-only incomplete attempt, visibly rejected by host.
 ff_init(args[3]);
 if(args[3]!=0u||!pn_capability_ok()){result[8u*i+6u]=vec4<u32>(0u,0u,4u,0u);result[8u*i+7u]=vec4<u32>(0x47505502u,i,0u,0u);return;}
 let dc=unpack_ce(offset_words[3u*i],offset_words[3u*i+1u]);let rc=unpack_cf(offset_words[3u*i+2u]);
 let radius=unpack_ce(vec4<u32>(args[4],args[5],args[6],args[7]),vec4<u32>(args[8],args[9],args[10],args[11]));
 var s=PerturbState(pn_zero(),0u,0u,0u,0u);var derivative=pn_zero();var e=cf_zero();var ed=cf_zero();var la_blocks=0u;var la_skipped=0u;
 for(var step=0u;step<args[2];step+=1u){
  if(s.n>=args[2]){break;}
  let jump=bla_advance(s,dc,rc,derivative,e,ed);
  if(jump.length>0u){s=jump.s;derivative=jump.derivative;e=jump.e;ed=jump.ed;la_blocks+=1u;la_skipped+=jump.length;if(s.status!=0u){break;}continue;}
  if(s.m+1u>=args[1]){s.status=2u;break;}
  let z=reference_z(s.m);let q=reference_q(s.m);let next=reference_z(s.m+1u);let physical=pn_add(z,s.delta);
  let af=pn_magnitude_upper(physical);let ad=pn_magnitude_upper(derivative);let ax=pn_magnitude_upper(s.delta);let aw=pn_magnitude_upper(z);
  let ef=radd(radd(e,q),rounding(radd(aw,ax)));
  ed=radd(radd(rmul(cf_scale(radd(af,ef),2.0),ed),rmul(cf_scale(ef,2.0),ad)),rounding(radd(cf_scale(rmul(af,ad),2.0),cf_one())));
  let rho=radd(radd(rounding(radd(radd(cf_scale(rmul(aw,ax),2.0),rmul(ax,ax)),pn_magnitude_upper(dc))),rmul(cf_scale(q,2.0),ax)),rc);
  let w=radd(radd(af,q),rounding(radd(aw,ax)));e=radd(rmul(e,radd(cf_scale(w,2.0),e)),rho);
  derivative=pn_add(pn_scale(pn_mul(physical,derivative),2.0),pn_one());let before=s;
  s=perturb_step_radius(s,dc,z,next,s.m+2u==args[1],radius);
  if(s.rebases!=before.rebases){e=radd(radd(e,reference_q(before.m+1u)),rounding(radd(pn_magnitude_upper(next),pn_magnitude_upper(s.delta))));}
  if(cf_bad(e)||cf_bad(ed)||pn_bad(derivative)||e.e< -16000||e.e>16000||ed.e< -16000||ed.e>16000||!pn_in_range(derivative)){s.status=2u;break;}
  if(s.status!=0u){break;}
  let upper=radd(radd(pn_magnitude_upper(pn_add(reference_z(s.m),s.delta)),e),reference_q(s.m));
  if(!pn_norm_le(CE(ff(upper.m.x),ff(upper.m.y),upper.e),radius)){s.status=1u;break;}
 }
 let b=8u*i;
 result[b]=vec4<u32>(bitcast<u32>(s.delta.re.hi),bitcast<u32>(s.delta.im.hi),bitcast<u32>(s.delta.re.lo),bitcast<u32>(s.delta.im.lo));result[b+1u]=vec4<u32>(bitcast<u32>(s.delta.e),0u,0u,0u);
 result[b+2u]=vec4<u32>(bitcast<u32>(derivative.re.hi),bitcast<u32>(derivative.im.hi),bitcast<u32>(derivative.re.lo),bitcast<u32>(derivative.im.lo));result[b+3u]=vec4<u32>(bitcast<u32>(derivative.e),0u,0u,0u);
 result[b+4u]=packed_cf(e);result[b+5u]=packed_cf(ed);result[b+6u]=vec4<u32>(s.m,s.n,s.status,s.rebases);
 result[b+7u]=vec4<u32>(0x47505502u,i,la_blocks,la_skipped);
}
