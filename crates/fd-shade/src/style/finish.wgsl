struct Params {
    dims: vec4<u32>,
    clock: vec4<f32>,
    origin: vec4<u32>,
    knobs: array<vec4<f32>, 64>,
    optics: vec4<f32>, // bloom, grain amount, grain size, color variation
    grain: vec4<u32>, // seed, exposure index, bloom radius, dither
    tonal: vec4<f32>, // shadow, midtone, highlight grain response
}
struct Layer { surface: vec4<f32>, emission: vec4<f32> }
@group(0) @binding(0) var<storage, read> hdr: array<Layer>;
@group(0) @binding(1) var<storage, read_write> pixels: array<u32>;
@group(0) @binding(2) var<uniform> p: Params;
@group(0) @binding(3) var<storage, read> glow: array<vec4<f32>>;

fn bloom_at(xy: vec2<f32>, size: vec2<u32>) -> vec3<f32> {
    let half_size = (size + 1u) / 2u;
    let uv = xy * 0.5 - 0.25;
    let base = vec2<i32>(floor(uv));
    let f = fract(uv);
    var color = vec3<f32>(0.0);
    for (var y=0; y<2; y++) {
        for (var x=0; x<2; x++) {
            let c=vec2<u32>(clamp(base+vec2<i32>(x,y),vec2<i32>(0),vec2<i32>(half_size)-1));
            let w=select(1.0-f.x,f.x,x==1)*select(1.0-f.y,f.y,y==1);
            color += glow[c.y*half_size.x+c.x].rgb*w;
        }
    }
    return color;
}

fn lattice(c: vec2<i32>, seed: u32) -> f32 {
    return tpdf(u32(c.x)*0x9e3779b9u ^ u32(c.y)*0x85ebca6bu ^ seed);
}
fn grain_field(xy: vec2<f32>, seed: u32) -> f32 {
    let c=vec2<i32>(floor(xy));
    let q=fract(xy);
    let f=q*q*(3.0-2.0*q);
    let a=lattice(c,seed); let b=lattice(c+vec2<i32>(1,0),seed);
    let d=lattice(c+vec2<i32>(0,1),seed); let e=lattice(c+vec2<i32>(1,1),seed);
    // Normalize interpolation variance, so amount is independent of grain size.
    let norm=sqrt(dot(vec2<f32>(1.0-f.x,f.x),vec2<f32>(1.0-f.x,f.x))*dot(vec2<f32>(1.0-f.y,f.y),vec2<f32>(1.0-f.y,f.y)));
    return mix(mix(a,b,f.x),mix(d,e,f.x),f.y)*2.44948974/max(norm,0.001);
}

@compute @workgroup_size(8, 8)
fn finish(@builtin(global_invocation_id) gid: vec3<u32>) {
    let size = p.dims.xy / p.dims.z;
    if (any(gid.xy >= size)) { return; }
    let i = gid.y * size.x + gid.x;
    var c = max(hdr[i].surface.rgb + hdr[i].emission.rgb, vec3<f32>(0.0));
    if (p.optics.x > 0.0) { c += bloom_at(vec2<f32>(gid.xy),size)*p.optics.x; }
    let mapped = c / (vec3<f32>(1.0) + c);
    var srgb = select(1.055 * pow(mapped, vec3<f32>(1.0/2.4)) - 0.055, 12.92 * mapped, mapped <= vec3<f32>(0.0031308));
    if (p.optics.y > 0.0) {
        let y=dot(srgb,vec3<f32>(0.2126,0.7152,0.0722));
        let tone=select(mix(p.tonal.x,p.tonal.y,smoothstep(0.0,0.5,y)),mix(p.tonal.y,p.tonal.z,smoothstep(0.5,1.0,y)),y>0.5);
        let seed=p.grain.x ^ (p.grain.y*0x27d4eb2du);
        let xy=(vec2<f32>(gid.xy)+0.5)/p.optics.z;
        let mono=grain_field(xy,seed);
        var noise=vec3<f32>(mono);
        if (p.optics.w>0.0) {
            let colored=vec3<f32>(grain_field(xy,seed^0xabc98388u),grain_field(xy,seed^0x8da6b343u),grain_field(xy,seed^0xd8163841u));
            let w=p.optics.w;
            noise=mix(noise,colored,w)/sqrt((1.0-w)*(1.0-w)+w*w);
        }
        srgb += noise*p.optics.y*tone*2.0*sqrt(max(y*(1.0-y),0.0));
    }
    var dither=vec3<f32>(0.0);
    if (p.grain.w!=0u) {
        let key=gid.y*0x9e3779b9u ^ gid.x*0x85ebca6bu;
        dither=vec3<f32>(tpdf(key),tpdf(key^0x68bc21ebu),tpdf(key^0x02e5be93u));
    }
    let b = vec3<u32>(clamp(srgb * 255.0 + 0.5 + dither, vec3<f32>(0.0), vec3<f32>(255.0)));
    pixels[i] = b.x | (b.y << 8u) | (b.z << 16u);
}
