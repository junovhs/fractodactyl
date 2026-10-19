// One implementation shared by every image node; controls are image-valued ports.
struct PassParams { dims:vec4<u32>, kinds:vec4<u32> }
@group(0) @binding(0) var<storage,read> a:array<vec4<f32>>;
@group(0) @binding(1) var<storage,read> b:array<vec4<f32>>;
@group(0) @binding(2) var<storage,read> c:array<vec4<f32>>;
@group(0) @binding(3) var<storage,read_write> result_image:array<vec4<f32>>;
@group(0) @binding(4) var<uniform> p:PassParams;
fn fetch(which:u32,xy:vec2<i32>)->vec4<f32>{
 let q=vec2<u32>(clamp(xy,vec2<i32>(0),vec2<i32>(p.dims.xy)-1));
 let i=select(q.y*p.dims.x+q.x,0u,(p.kinds.w & (1u<<which))!=0u);
 if(which==0u){return a[i];} if(which==1u){return b[i];} return c[i];
}
fn sample_at(which:u32,xy:vec2<f32>)->vec4<f32>{
 let base=vec2<i32>(floor(xy));let f=fract(xy);
 return mix(mix(fetch(which,base),fetch(which,base+vec2<i32>(1,0)),f.x),mix(fetch(which,base+vec2<i32>(0,1)),fetch(which,base+vec2<i32>(1,1)),f.x),f.y);
}
@compute @workgroup_size(8,8)
fn apply(@builtin(global_invocation_id) gid:vec3<u32>){
 if(any(gid.xy>=p.dims.xy)){return;}
 let i=gid.y*p.dims.x+gid.x;let xy=vec2<f32>(gid.xy);
 var x=fetch(0u,vec2<i32>(gid.xy));var y=fetch(1u,vec2<i32>(gid.xy));
 if(p.kinds.x==0u){x=vec4<f32>(x.x);}if(p.kinds.y==0u){y=vec4<f32>(y.x);}
 var result=x;
 switch p.dims.z {
  case 4u:{
   let delta=y.xy*fetch(2u,vec2<i32>(gid.xy)).x;let magnitude=length(delta);
   let offset=delta*min(1.0,8.0/max(magnitude,1e-8));
   result=sample_at(0u,xy+offset);
  }
  case 5u,6u:{
   let radius=clamp(abs(y.x),0.0,8.0);
   if(radius>0.001){
    let axis=select(vec2<f32>(1.0,0.0),vec2<f32>(0.0,1.0),p.dims.z==6u);
    let weights=array<f32,5>(1.0,4.0,6.0,4.0,1.0);
    result=vec4<f32>(0.0);
    for(var k=0;k<5;k++){result+=sample_at(0u,xy+axis*radius*(f32(k)*0.5-1.0))*weights[k];}
    result/=16.0;
   }
  }
  case 7u:{
   let pos=vec2<i32>(gid.xy);
   let dx=(fetch(1u,pos+vec2<i32>(1,0)).x-fetch(1u,pos-vec2<i32>(1,0)).x)*0.5;
   let dy=(fetch(1u,pos+vec2<i32>(0,1)).x-fetch(1u,pos-vec2<i32>(0,1)).x)*0.5;
   let normal=normalize(vec3<f32>(-dx,-dy,1.0));
   let light=normalize(vec3<f32>(fetch(2u,vec2<i32>(gid.xy)).xy,1.0));
   result=x*(0.15+0.85*max(dot(normal,light),0.0));
  }
  case 8u:{
   let radius=clamp(abs(fetch(2u,vec2<i32>(gid.xy)).x),0.0,24.0);
   result=x;var total=1.0;
   for(var side=0;side<2;side++){
    let sign=select(-1.0,1.0,side==1);
    var pos=xy;
    for(var step=0;step<6;step++){
     let v=sample_at(1u,pos).xy;
     let dir=v/max(length(v),1.0);
     let mid=pos+dir*(sign*radius/12.0);
     let mv=sample_at(1u,mid).xy;
     pos+=mv/max(length(mv),1.0)*(sign*radius/6.0);
     let weight=exp(-pow(f32(step+1)/4.0,2.0));
     result+=sample_at(0u,pos)*weight;total+=weight;
    }
   }
   result/=total;
  }
  default:{}
 }
 result_image[i]=select(vec4<f32>(0.0),clamp(result,vec4<f32>(-65504.0),vec4<f32>(65504.0)),abs(result)<vec4<f32>(1e30));
}
