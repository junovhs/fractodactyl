// Tent resample of two shaded keyframes into one film frame (SHAD-06): the WGSL port of
// keyframe::resample, f32 only, including rotated frames (FILM-14). Sources are
// full-frame packed RGB (r | g << 8 | b << 16) of which only `rect` was shaded; reads
// clamp to that rectangle, as the CPU gather does on its cropped image.

struct Params {
    dims: vec4<u32>,       // output width, output height, outer width, outer height
    more: vec4<u32>,       // inner width, inner height
    zoom: vec4<f32>,       // outer z, inner z, outer x, outer y
    blend: vec4<f32>,      // inner x, inner y, ramp, edge margin
    turn: vec4<f32>,       // sin, cos of the frame rotation
    outer_rect: vec4<u32>, // x, y, width, height shaded in the outer keyframe
    inner_rect: vec4<u32>, // the same for the inner keyframe
}

@group(0) @binding(0) var<storage, read> outer_rgb: array<u32>;
@group(0) @binding(1) var<storage, read> inner_rgb: array<u32>;
@group(0) @binding(2) var<storage, read_write> out_rgb: array<u32>;
@group(0) @binding(3) var<storage, read> gamma_lut: array<f32>;
@group(0) @binding(4) var<uniform> p: Params;

const TENT: f32 = 0.5;
// Rust's f32 1.0 / 2.2; a WGSL constant expression would fold it one ulp higher.
const INV_GAMMA_BITS: u32 = 0x3ee8ba2eu;

fn texel(inner: bool, i: u32) -> u32 {
    if (inner) {
        return inner_rgb[i];
    }
    return outer_rgb[i];
}

fn gather(inner: bool, x: f32, y: f32, radius: f32) -> vec3<f32> {
    var full = p.dims.zw;
    var rect = p.outer_rect;
    if (inner) {
        full = p.more.xy;
        rect = p.inner_rect;
    }
    let f = max(radius, 1.0);
    let cx = x * f32(full.x) + f32(full.x) / 2.0;
    let cy = y * f32(full.y) + f32(full.y) / 2.0;
    let x0 = i32(floor(cx - f));
    let x1 = i32(ceil(cx + f));
    let y0 = i32(floor(cy - f));
    let y1 = i32(ceil(cy + f));
    var acc = vec3<f32>(0.0);
    var wsum = 0.0;
    for (var sy = y0; sy <= y1; sy++) {
        let wy = 1.0 - abs(f32(sy) + 0.5 - cy) / f;
        if (wy <= 0.0) {
            continue;
        }
        let row = u32(clamp(sy - i32(rect.y), 0, i32(rect.w) - 1)) + rect.y;
        for (var sx = x0; sx <= x1; sx++) {
            let wx = 1.0 - abs(f32(sx) + 0.5 - cx) / f;
            if (wx <= 0.0) {
                continue;
            }
            let col = u32(clamp(sx - i32(rect.x), 0, i32(rect.z) - 1)) + rect.x;
            let v = texel(inner, row * full.x + col);
            let wgt = wx * wy;
            acc += vec3<f32>(
                gamma_lut[v & 255u],
                gamma_lut[(v >> 8u) & 255u],
                gamma_lut[(v >> 16u) & 255u],
            ) * wgt;
            wsum += wgt;
        }
    }
    return acc / max(wsum, 1e-12);
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let ow = p.dims.x;
    let oh = p.dims.y;
    if (gid.x >= ow || gid.y >= oh) {
        return;
    }
    let aspect = f32(ow) / f32(oh);
    let xn = (f32(gid.x) + 0.5) / f32(ow) - 0.5;
    let yn = (f32(gid.y) + 0.5) / f32(oh) - 0.5;
    let st = p.turn.x;
    let ct = p.turn.y;
    // keyframe::rotated: screen coordinates in the unrotated sample-grid axes.
    let rx = ct * xn + st * yn / aspect;
    let ry = ct * yn - st * xn * aspect;
    let zo = p.zoom.x;
    let zi = p.zoom.y;
    let xi = rx * zi + p.blend.x;
    let yi = ry * zi + p.blend.y;
    let d = max(2.0 * abs(xi), 2.0 * abs(yi));
    let wi = p.blend.z * clamp((1.0 - d) / p.blend.w, 0.0, 1.0);
    var col = vec3<f32>(0.0);
    if (wi < 1.0) {
        let o = gather(false, rx * zo + p.zoom.z, ry * zo + p.zoom.w,
                       TENT * zo * f32(p.dims.z) / f32(ow));
        col = o * (1.0 - wi);
    }
    if (wi > 0.0) {
        let i = gather(true, xi, yi, TENT * zi * f32(p.more.x) / f32(ow));
        col = col + i * wi;
    }
    var packed = 0u;
    for (var j = 0u; j < 3u; j++) {
        let v = clamp(col[j], 0.0, 1.0);
        let level = min(u32(pow(v, bitcast<f32>(INV_GAMMA_BITS)) * 255.0 + 0.5), 255u);
        packed |= level << (8u * j);
    }
    out_rgb[gid.y * ow + gid.x] = packed;
}
