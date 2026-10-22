// GPU-33: sole GPU affine-block consumer, using CPU-built shared contracts.
@group(0) @binding(4) var<storage, read> bla_words:array<vec4<u32>>;
struct BlaAdvance {s:PerturbState, derivative:CE, e:CF, ed:CF, length:u32}
fn scalar_lt(a:CF,b:CF)->bool {
 if(b.m.x<=0.0){return false;}if(a.m.x==0.0){return true;}
 return a.e<b.e||(a.e==b.e&&a.m.x<b.m.x);
}
fn bla_advance(s:PerturbState,dc:CE,rc:CF,derivative:CE,e:CF,ed:CF)->BlaAdvance {
 var out=BlaAdvance(s,derivative,e,ed,0u);
 if(s.m==0u||args[13]==0u){return out;}
 let adc=pn_magnitude_upper(dc);let parameter=radd(adc,rc);
 if(!scalar_lt(parameter,unpack_cf(bla_words[0]))){return out;}
 let ax=pn_magnitude_upper(s.delta);let domain=radd(ax,e);let k=s.m-1u;
 var chosen=0u;var length=0u;
 for(var level=0u;level<args[13];level+=1u){
  let len=2u<<level;
  if(k%len!=0u||len>args[2]-s.n||s.m+len>=args[1]){break;}
  let index=args[14u+level]+9u*(k/len);
  if(index+8u>=arrayLength(&bla_words)){break;}
  if(!scalar_lt(domain,unpack_cf(bla_words[index+4u]))){break;}
  chosen=index;length=len;
 }
 if(length==0u){return out;}
 let a=unpack_ce(bla_words[chosen],bla_words[chosen+1u]);
 let b=unpack_ce(bla_words[chosen+2u],bla_words[chosen+3u]);
 let alpha=unpack_cf(bla_words[chosen+5u]);let beta=unpack_cf(bla_words[chosen+6u]);
 let da=unpack_cf(bla_words[chosen+7u]);let db=unpack_cf(bla_words[chosen+8u]);
 let aa=pn_magnitude_upper(a);let ab=pn_magnitude_upper(b);let ad=pn_magnitude_upper(derivative);
 // Existing e/ed and actual |delta|/|dc|; host reference/coefficient loss is in
 // envelopes, CE upload residual charged by host, conditional CE eval below.
 out.e=radd(radd(rmul(radd(aa,alpha),e),radd(rmul(alpha,ax),rmul(beta,adc))),radd(rmul(radd(ab,beta),rc),rounding(radd(rmul(aa,ax),rmul(ab,adc)))));
 out.ed=radd(radd(rmul(radd(aa,da),ed),radd(rmul(da,ad),db)),rounding(radd(rmul(aa,ad),ab)));
 out.s.delta=pn_add(pn_mul(a,s.delta),pn_mul(b,dc));
 out.derivative=pn_add(pn_mul(a,derivative),b);
 out.s.m+=length;out.s.n+=length;out.length=length;
 let next=reference_z(out.s.m);let landing=out.s.m;
 let radius=unpack_ce(vec4<u32>(args[4],args[5],args[6],args[7]),vec4<u32>(args[8],args[9],args[10],args[11]));
 out.s=perturb_finish(out.s,next,landing+1u==args[1],radius);
 if(out.s.rebases!=s.rebases){out.e=radd(radd(out.e,reference_q(landing)),rounding(radd(pn_magnitude_upper(next),pn_magnitude_upper(out.s.delta))));}
 if(cf_bad(out.e)||cf_bad(out.ed)||pn_bad(out.derivative)||!pn_in_range(out.derivative)){out.s.status=2u;}
 return out;
}
