use super::*;
use fd_kernel::{reference, Params, Plane};
use fd_samples::{Column, ColumnSet, View};
use fd_shade::gpu::testkit::{bytes, dispatch, words, Binding};

const ENTRY: &str = r#"
@group(0) @binding(0) var<storage, read> refs: array<vec4<u32>>;
@group(0) @binding(1) var<storage, read> cases: array<u32>;
@group(0) @binding(2) var<storage, read_write> out: array<u32>;
fn load_cf(a: u32) -> CF { return CF(vec2<f32>(bitcast<f32>(cases[a]), bitcast<f32>(cases[a+1u])), bitcast<i32>(cases[a+2u])); }
fn reference_at(a: u32) -> CF { let v = refs[a]; return CF(vec2<f32>(bitcast<f32>(v.x), bitcast<f32>(v.y)), bitcast<i32>(v.z)); }
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (16u*i >= arrayLength(&cases)) { return; }
    let a = 16u*i;
    let dc = load_cf(a);
    var s = PerturbState(load_cf(a+4u), cases[a+8u], cases[a+9u], cases[a+10u], cases[a+11u]);
    let base = cases[a+12u];
    let len = cases[a+13u];
    for (var k=0u; k<cases[a+14u]; k+=1u) {
        if (s.status != 0u) { break; }
        if (len < 2u || s.m >= len-1u || base > arrayLength(&refs) || len > arrayLength(&refs)-base) { s.status=2u; break; }
        s = perturb_step(s, dc, reference_at(base+s.m), reference_at(base+s.m+1u), s.m+2u==len);
    }
    let b=8u*i;
    out[b]=bitcast<u32>(s.delta.m.x); out[b+1u]=bitcast<u32>(s.delta.m.y); out[b+2u]=bitcast<u32>(s.delta.e);
    out[b+3u]=s.m; out[b+4u]=s.n; out[b+5u]=s.rebases; out[b+6u]=s.status; out[b+7u]=0u;
}
"#;

fn cf(x: (f64, f64), exponent: i32) -> [u32; 4] {
    let a = x.0.abs().max(x.1.abs());
    let k = if a == 0.0 {
        0
    } else {
        a.log2().floor() as i32 + 1
    };
    [
        ((x.0 * 2f64.powi(-k)) as f32).to_bits(),
        ((x.1 * 2f64.powi(-k)) as f32).to_bits(),
        (exponent + k) as u32,
        0,
    ]
}
fn case(dc: [u32; 4], delta: [u32; 4], base: u32, len: u32, steps: u32) -> Vec<u32> {
    [
        dc.to_vec(),
        delta.to_vec(),
        vec![0, 0, 0, 0, base, len, steps, 0],
    ]
    .concat()
}
fn value(w: &[u32]) -> (f64, f64) {
    let e = 2f64.powi(w[2] as i32);
    (
        f64::from(f32::from_bits(w[0])) * e,
        f64::from(f32::from_bits(w[1])) * e,
    )
}
fn mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}
fn norm(a: (f64, f64)) -> f64 {
    a.0.hypot(a.1)
}

#[test]
fn gpu_perturb_shader_validates() {
    let source = format!(
        "{}\n{CF}\n{CF_ADAPTER}\n{STEPPER}\n{ENTRY}",
        crate::zone_ff::FF
    );
    let module = naga::front::wgsl::parse_str(&source).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
}

/// Requires an actual dispatch: no adapter is a failure, never a zero-row pass.
#[test]
#[ignore = "actual selected wgpu device; run with --ignored --nocapture"]
fn gpu_perturb_checkpoints() {
    let source = format!(
        "{}\n{CF}\n{CF_ADAPTER}\n{STEPPER}\n{ENTRY}",
        crate::zone_ff::FF
    );
    let mut refs = Vec::new();
    let mut input = Vec::new();
    let mut expected = Vec::new();
    // Exact-camera arbitrary locations; no zone files or specially placed nucleus.
    for (re, im, width) in [
        ("-0.7453", "0.1127", "0.0065"),
        ("0.25", "0", "1e-8"),
        (
            "-0.743643887037158704752191506114774",
            "0.131825904205311970493132056385139",
            "1e-28",
        ),
        ("0", "1", "1e-50"),
        ("-2", "0", "1e-200"),
    ] {
        let view = View {
            center_re: re.into(),
            center_im: im.into(),
            width: width.into(),
            rotation: 0.37,
        };
        let p = Params {
            nx: 32,
            ny: 18,
            max_iter: 256,
            ss: 1,
            escape_radius: 2.0,
            columns: ColumnSet::of(&[Column::Nu]),
            threads: 1,
            tier: None,
        };
        let (r, _) = reference(&view, &p).unwrap();
        let plane = Plane::new(&view, 32, 18).unwrap();
        let base = refs.len() as u32 / 4;
        let len = r.len() as u32;
        for (&x, &y) in r.re.iter().zip(&r.im) {
            refs.extend(cf((x, y), 0));
        }
        for (i, j) in [(0, 0), (15, 8), (31, 17)] {
            let u = plane.unit_offset(i, j);
            let dc = (u.0 * plane.h(), u.1 * plane.h());
            for warm in [0, 32, 128] {
                let mut admitted = false;
                let (mut d, mut m, mut n, mut reb, mut status, mut err) =
                    ((0.0, 0.0), 0usize, 0, 0, 0, 0.0);
                for step in 0..warm + 8 {
                    if step == warm {
                        let mut row = case(cf(dc, 0), cf(d, 0), base, len, 8);
                        row[8] = m as u32;
                        row[9] = n;
                        row[10] = reb;
                        input.extend(row);
                        admitted = true;
                        err = 64.0 * 2f64.powi(-24) * norm(d);
                    }
                    let z = (r.re[m], r.im[m]);
                    let zz = mul(z, d);
                    let dd = mul(d, d);
                    // Conservative local operation/input-rounding envelope, u=2^-24.
                    // This is a checkpoint diagnostic, NOT a pixel-error certificate.
                    err = (2.0 * norm(z) + 2.0 * norm(d) + err) * err
                        + 64.0
                            * 2f64.powi(-24)
                            * (2.0 * norm(z) * norm(d) + norm(d).powi(2) + norm(dc));
                    d = (2.0 * zz.0 + dd.0 + dc.0, 2.0 * zz.1 + dd.1 + dc.1);
                    m += 1;
                    n += 1;
                    let physical = (r.re[m] + d.0, r.im[m] + d.1);
                    if norm(physical) > 2.0 {
                        status = 1;
                        break;
                    }
                    if m + 1 == r.len() || norm(physical) <= norm(d) {
                        err += 64.0 * 2f64.powi(-24) * (norm((r.re[m], r.im[m])) + norm(d));
                        d = physical;
                        m = 0;
                        reb += 1;
                    }
                }
                if admitted {
                    expected.push((
                        format!("{re}/{width} pixel {i},{j} checkpoint {warm}"),
                        d,
                        m as u32,
                        n,
                        reb,
                        status,
                        err,
                        plane.h(),
                    ));
                }
            }
        }
    }
    let count = expected.len();
    // Exact powers of two far outside f32 AND f64 physical range, and explicit
    // decline states. Use a zero reference, so delta' = delta^2 + dc.
    let base = refs.len() as u32 / 4;
    refs.extend([0u32; 8]);
    let mut special = Vec::new();
    for e in [-16001i32, -16000, -3322, -1000, 0, 16000, 16001] {
        let dc = [0.5f32.to_bits(), 0, e as u32, 0];
        special.push((input.len() / 16, e));
        input.extend(case(dc, cf((0.0, 0.0), 0), base, 2, 1));
    }
    let invalid = input.len() / 16;
    input.extend(case(
        [f32::NAN.to_bits(), 0, 0, 0],
        cf((0.0, 0.0), 0),
        base,
        2,
        1,
    ));
    let bad_index = input.len() / 16;
    let mut bad = case(cf((0.0, 0.0), 0), cf((0.0, 0.0), 0), base, 2, 1);
    bad[8] = 99;
    input.extend(bad);
    let (_, device, _) =
        fd_shade::gpu::testkit::open_device("GPU-31 capabilities").expect("actual wgpu device");
    eprintln!(
        "GPU-31 device enabled features {:?} (no SHADER_F64 requested)",
        device.features()
    );
    let (adapter, result) = dispatch(
        "GPU-31 perturbation checkpoints",
        &source,
        "main",
        &[
            Binding::Read(&bytes(&refs)),
            Binding::Read(&bytes(&input)),
            Binding::Out((input.len() / 16 * 32) as u64),
        ],
        ((input.len() / 16).div_ceil(64) as u32, 1),
    )
    .unwrap();
    let output = words(&result);
    let mut max_ratio = 0.0f64;
    for (k, (name, want, m, n, reb, status, budget, h)) in expected.iter().enumerate() {
        let row = &output[k * 8..k * 8 + 8];
        let got = value(row);
        let error = norm((got.0 - want.0, got.1 - want.1));
        let ratio = error / budget.max(f64::MIN_POSITIVE);
        max_ratio = max_ratio.max(ratio);
        assert_eq!(
            (row[3], row[4], row[5], row[6]),
            (*m, *n, *reb, *status),
            "{name}"
        );
        assert!(
            error <= *budget,
            "{name}: error {error:e} > local budget {budget:e}"
        );
        eprintln!("GPU-31 {name}: steps {n}, rebases {reb}, status {status}, delta error {error:e}, local budget {budget:e}, delta error / initial sample spacing {:e}",error/h);
    }
    for (k, e) in special {
        let row = &output[8 * k..8 * k + 8];
        assert_eq!(
            row[6],
            if e.abs() > 16000 {
                3
            } else if e > 2 {
                1
            } else {
                0
            },
            "exponent {e}"
        );
        if e.abs() <= 16000 {
            assert_eq!(
                (row[0], row[1], row[2]),
                (0.5f32.to_bits(), 0, e as u32),
                "exponent {e}"
            );
        }
    }
    assert_eq!(output[8 * invalid + 6], 2);
    assert_eq!(output[8 * bad_index + 6], 2);
    eprintln!("GPU-31 adapter {adapter}: {count} camera checkpoints + 9 range/invalid cases; worst local-budget ratio {max_ratio:e}; no film or speed claim");
}

#[test]
#[ignore = "actual GPU outward norms at exponent extremes"]
fn gpu_perturb_outward_norms() {
    let entry = r#"
@group(0) @binding(0) var<storage, read> input: array<vec4<u32>>;
@group(0) @binding(1) var<storage, read_write> output: array<vec4<u32>>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
 let i=id.x;if(i>=arrayLength(&input)){return;}
 let v=input[i];let r=cf_magnitude_upper(CF(vec2<f32>(bitcast<f32>(v.x),bitcast<f32>(v.y)),bitcast<i32>(v.z)));
 output[i]=vec4<u32>(bitcast<u32>(r.m.x),bitcast<u32>(r.m.y),bitcast<u32>(r.e),0u);
}
"#;
    let mut input = Vec::new();
    let mut expected = Vec::new();
    for e in [-16000, -3322, -127, -1, 0, 1, 127, 3000, 16000] {
        for (x, y) in [
            (0.0f32, 0.0f32),
            (0.5, 0.0),
            (0.0, 0.5),
            (0.5, 0.5),
            (0.999, -0.75),
        ] {
            input.extend([x.to_bits(), y.to_bits(), e as u32, 0]);
            expected.push((f64::from(x).hypot(f64::from(y)), e));
        }
    }
    let source = format!("{}\n{CF}\n{entry}", crate::zone_ff::FF);
    let (adapter, out) = dispatch(
        "GPU-32 outward norms",
        &source,
        "main",
        &[
            Binding::Read(&bytes(&input)),
            Binding::Out((input.len() * 4) as u64),
        ],
        ((expected.len() as u32).div_ceil(64), 1),
    )
    .unwrap();
    for ((want, e), row) in expected.iter().zip(words(&out).chunks_exact(4)) {
        let got = if *want == 0.0 {
            f64::from(f32::from_bits(row[0]))
        } else {
            f64::from(f32::from_bits(row[0])) * 2f64.powi(row[2] as i32 - e)
        };
        assert!(
            got >= *want && got <= want * 1.0002,
            "norm at exponent {e}: {got} vs {want}"
        );
    }
    eprintln!("GPU-32 {adapter}:45 outward normalized norms at exponent -16000..16000 passed");
}

/// Independent binary64 iteration windows plus exact exponent/range cases for CE.
/// This validates the empirical RN model on this adapter, not all WGSL executions.
#[test]
#[ignore = "actual CE GPU checkpoints and exponent extremes"]
fn gpu_perturb_ce_checkpoints() {
    let entry = r#"
@group(0) @binding(0) var<storage,read> refs:array<vec4<u32>>;
@group(0) @binding(1) var<storage,read> cases:array<u32>;
@group(0) @binding(2) var<storage,read_write> out:array<u32>;
fn load(a:u32)->CE{return CE(FF(bitcast<f32>(cases[a]),bitcast<f32>(cases[a+2u])),FF(bitcast<f32>(cases[a+1u]),bitcast<f32>(cases[a+3u])),bitcast<i32>(cases[a+4u]));}
fn reference_at(a:u32)->CE{let hi=refs[2u*a];let lo=refs[2u*a+1u];return CE(FF(bitcast<f32>(hi.x),bitcast<f32>(hi.z)),FF(bitcast<f32>(hi.y),bitcast<f32>(hi.w)),bitcast<i32>(lo.x));}
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>){let i=id.x;let a=24u*i;if(a>=arrayLength(&cases)){return;}
 ff_init(cases[a+23u]);var s=PerturbState(load(a+8u),cases[a+16u],cases[a+17u],cases[a+18u],cases[a+19u]);
 let base=cases[a+20u];let len=cases[a+21u];
 if(!pn_capability_ok()){s.status=4u;}else{for(var k=0u;k<cases[a+22u];k+=1u){if(s.status!=0u){break;}if(len<2u||s.m>=len-1u||base+len>arrayLength(&refs)/2u){s.status=2u;break;}s=perturb_step(s,load(a),reference_at(base+s.m),reference_at(base+s.m+1u),s.m+2u==len);}}
 let b=12u*i;out[b]=bitcast<u32>(s.delta.re.hi);out[b+1u]=bitcast<u32>(s.delta.im.hi);out[b+2u]=bitcast<u32>(s.delta.re.lo);out[b+3u]=bitcast<u32>(s.delta.im.lo);out[b+4u]=bitcast<u32>(s.delta.e);
 out[b+8u]=s.m;out[b+9u]=s.n;out[b+10u]=s.rebases;out[b+11u]=s.status;
}
"#;
    let source = format!(
        "{}\n{CF}\n{CE_ADAPTER}\n{STEPPER}\n{entry}",
        crate::zone_ff::FF
    );
    let mut refs = Vec::new();
    let mut input = Vec::new();
    let mut expected = Vec::new();
    let eta = 256.0 * 2f64.powi(-48);
    for (re, im, width) in [
        ("-0.7453", "0.1127", "0.0065"),
        ("0.25", "0", "1e-8"),
        (
            "-0.743643887037158704752191506114774",
            "0.131825904205311970493132056385139",
            "1e-28",
        ),
    ] {
        let view = View {
            center_re: re.into(),
            center_im: im.into(),
            width: width.into(),
            rotation: 0.0,
        };
        let p = Params {
            nx: 128,
            ny: 72,
            max_iter: 10000,
            ss: 1,
            escape_radius: 2.0,
            columns: ColumnSet::of(&[Column::Nu]),
            threads: 1,
            tier: None,
        };
        let (r, _) = reference(&view, &p).unwrap();
        let plane = Plane::new(&view, 128, 72).unwrap();
        let base = refs.len() as u32 / 8;
        for (&re, &im) in r.re.iter().zip(&r.im) {
            refs.extend(frame::pack_ce((re, im), 0));
        }
        for (x, y) in [(0, 0), (63, 35), (127, 71)] {
            let u = plane.unit_offset(x, y);
            let dc = (u.0 * plane.h(), u.1 * plane.h());
            for warm in [0, 32, 128, 6800] {
                let (mut d, mut m, mut n, mut reb, mut status, mut error) =
                    ((0.0, 0.0), 0usize, 0u32, 0u32, 0u32, 0.0);
                let mut taken = false;
                for step in 0..warm + 8 {
                    if step == warm {
                        input.extend(frame::pack_ce(dc, 0));
                        input.extend(frame::pack_ce(d, 0));
                        input.extend([m as u32, n, reb, status, base, r.len() as u32, 8, 0]);
                        taken = true;
                        error = eta * norm(d);
                    }
                    let z = (r.re[m], r.im[m]);
                    let zd = mul(z, d);
                    let dd = mul(d, d);
                    error = (2.0 * norm(z) + 2.0 * norm(d) + error) * error
                        + eta * (2.0 * norm(z) * norm(d) + norm(d).powi(2) + norm(dc));
                    d = (2.0 * zd.0 + dd.0 + dc.0, 2.0 * zd.1 + dd.1 + dc.1);
                    m += 1;
                    n += 1;
                    let physical = (r.re[m] + d.0, r.im[m] + d.1);
                    if norm(physical) > 2.0 {
                        status = 1;
                        break;
                    }
                    if m + 1 == r.len() || norm(physical) <= norm(d) {
                        error += eta * (norm((r.re[m], r.im[m])) + norm(d));
                        d = physical;
                        m = 0;
                        reb += 1;
                    }
                }
                if taken {
                    expected.push((
                        format!("{width}:{x},{y} warm{warm}"),
                        d,
                        m as u32,
                        n,
                        reb,
                        status,
                        error,
                    ));
                }
            }
        }
    }
    let camera_count = expected.len();
    let base = refs.len() as u32 / 8;
    refs.extend([0u32; 16]);
    let mut special = Vec::new();
    for e in [-16001i32, -16000, -3322, -1000, 0, 16000, 16001] {
        let k = input.len() / 24;
        let mut dc = frame::pack_ce((0.5, 0.0), 0);
        dc[4] = e as u32;
        input.extend(dc);
        input.extend([0u32; 8]);
        input.extend([0, 0, 0, 0, base, 2, 1, 0]);
        special.push((k, e));
    }
    let invalid = input.len() / 24;
    let mut dc = [0u32; 8];
    dc[0] = f32::NAN.to_bits();
    input.extend(dc);
    input.extend([0u32; 8]);
    input.extend([0, 0, 0, 0, base, 2, 1, 0]);
    let (adapter, out) = dispatch(
        "GPU32 CE checkpoints",
        &source,
        "main",
        &[
            Binding::Read(&bytes(&refs)),
            Binding::Read(&bytes(&input)),
            Binding::Out((input.len() / 24 * 48) as u64),
        ],
        ((input.len() / 24).div_ceil(64) as u32, 1),
    )
    .unwrap();
    let output = words(&out);
    let mut worst = 0.0f64;
    for (row, (name, want, m, n, reb, status, budget)) in output.chunks_exact(12).zip(&expected) {
        let got = frame::unpack_ce(row, 0);
        let error = norm((got.0 - want.0, got.1 - want.1));
        assert_eq!(
            (row[8], row[9], row[10], row[11]),
            (*m, *n, *reb, *status),
            "{name}"
        );
        assert!(error <= *budget, "{name}: {error:e}>{budget:e}");
        worst = worst.max(error / budget.max(f64::MIN_POSITIVE));
    }
    for (k, e) in special {
        let row = &output[12 * k..12 * k + 12];
        assert_eq!(
            row[11],
            if e.abs() > 16000 {
                3
            } else if e > 2 {
                1
            } else {
                0
            },
            "e{e}"
        );
        if e.abs() <= 16000 {
            assert_eq!(
                (row[0], row[1], row[2], row[3], row[4]),
                (0.5f32.to_bits(), 0, 0, 0, e as u32)
            );
        }
    }
    assert_eq!(output[12 * invalid + 11], 2);
    eprintln!("GPU32 CE {adapter}: {camera_count} independent binary64 windows +8 exponent/invalid states, worst conditional-model ratio {worst:e}; empirical Heuristic only");
}
