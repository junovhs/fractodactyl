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
    let source = format!("{}\n{CF}\n{STEPPER}\n{ENTRY}", crate::zone_ff::FF);
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
    let source = format!("{}\n{CF}\n{STEPPER}\n{ENTRY}", crate::zone_ff::FF);
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
