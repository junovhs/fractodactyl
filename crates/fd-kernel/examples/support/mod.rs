//! Reusable frozen-oracle loader/comparison for development probes, not a renderer.
use fd_kernel::{
    bla_dc_max, reference, render_bla, render_with, Bla, Params, Plane, Tier, BLA_EPS_MAX,
};
use fd_samples::{Column, ColumnSet, Kind, Samples, View};
use std::path::Path;
use std::str::FromStr;

fn number<T: FromStr>(value: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("invalid numeric field {value:?}"))
}

#[derive(Clone)]
pub struct Point {
    pub class: char,
    pub iteration: u64,
    pub values: [f64; 5],
}

pub struct Case {
    pub name: String,
    pub view: View,
    pub params: Params,
    pub points: Vec<Point>,
}

pub fn load(path: &Path, threads: usize) -> Result<Case, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut lines = text.lines();
    if lines.next() != Some("fd-truth\t1") {
        return Err("unsupported truth schema".into());
    }
    let fields: Vec<_> = lines.next().ok_or("missing view")?.split('\t').collect();
    if fields.len() != 12 || fields[0] != "view" {
        return Err("invalid view header".into());
    }
    if !fields[1]
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err("invalid name".into());
    }
    let view = View {
        center_re: fields[2].into(),
        center_im: fields[3].into(),
        width: fields[4].into(),
        rotation: number(fields[10])?,
    };
    let params = Params {
        nx: number(fields[5])?,
        ny: number(fields[6])?,
        ss: number(fields[7])?,
        max_iter: number(fields[8])?,
        escape_radius: number(fields[9])?,
        threads,
        columns: ColumnSet::of(&[Column::Nu, Column::De, Column::Normal]),
        tier: None,
    };
    let bits: u32 = number(fields[11])?;
    if params.nx == 0
        || params.ny == 0
        || params.ss != 1
        || params.max_iter == 0
        || bits < 128
        || !params.escape_radius.is_finite()
        || params.escape_radius < 2.0
    {
        return Err("invalid truth domain".into());
    }
    let mut points = Vec::new();
    for line in lines.filter(|l| !l.starts_with('#')) {
        let f: Vec<_> = line.split('\t').collect();
        if f.len() != 8 || number::<usize>(f[0])? != points.len() {
            return Err("invalid point index/fields".into());
        }
        let class = match f[1] {
            "E" => 'E',
            "A" => 'A',
            "U" => 'U',
            _ => return Err("invalid truth class".into()),
        };
        let iteration = number(f[2])?;
        let mut values = [0.0f64; 5];
        for (to, from) in values.iter_mut().zip(&f[3..]) {
            *to = number(from)?;
        }
        if values.iter().any(|v| !v.is_finite())
            || (class == 'E'
                && (iteration == 0
                    || iteration > params.max_iter
                    || values[1] <= 0.0
                    || values[3] < 0.0
                    || values[4] < 0.0))
            || (class == 'A' && iteration != 0)
            || (class == 'U' && iteration != params.max_iter)
        {
            return Err("invalid truth observation".into());
        }
        points.push(Point {
            class,
            iteration,
            values,
        });
    }
    if points.len() != params.nx as usize * params.ny as usize {
        return Err("point count does not match camera grid".into());
    }
    Ok(Case {
        name: fields[1].into(),
        view,
        params,
        points,
    })
}

/// Exact identities of independently investigated finite-precision edge samples.
/// This is deliberately a versioned list, never a general distance-based waiver.
pub struct Edge {
    case: String,
    index: usize,
    camera: [String; 3],
    nx: u32,
    ny: u32,
    cap: u64,
    iteration: u64,
    bits: [u64; 4],
}

pub fn load_edges(path: &Path) -> Result<Vec<Edge>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut lines = text.lines();
    if lines.next() != Some("fd-truth-edges\t1") {
        return Err("invalid edge schema".into());
    }
    let mut edges = Vec::new();
    for line in lines.filter(|l| !l.starts_with('#')) {
        let f: Vec<_> = line.split('\t').collect();
        if f.len() != 13 {
            return Err("invalid edge fields".into());
        }
        let edge = Edge {
            case: f[0].into(),
            index: number(f[1])?,
            camera: [f[2].into(), f[3].into(), f[4].into()],
            nx: number(f[5])?,
            ny: number(f[6])?,
            cap: number(f[7])?,
            iteration: number(f[8])?,
            bits: [
                number(f[9])?,
                number(f[10])?,
                number(f[11])?,
                number(f[12])?,
            ],
        };
        if edges
            .iter()
            .any(|e: &Edge| e.case == edge.case && e.index == edge.index)
        {
            return Err("duplicate edge".into());
        }
        edges.push(edge);
    }
    Ok(edges)
}

fn validate_edges(case: &Case, edges: &[Edge]) -> Result<(), String> {
    let plane = Plane::new(&case.view, case.params.nx, case.params.ny)?;
    for edge in edges.iter().filter(|e| e.case == case.name) {
        let point = case
            .points
            .get(edge.index)
            .ok_or("edge index outside pack")?;
        let (ux, uy) = plane.unit_offset(
            edge.index % case.params.nx as usize,
            edge.index / case.params.nx as usize,
        );
        let bits = [
            plane.c_re.to_bits(),
            plane.c_im.to_bits(),
            (ux * plane.h()).to_bits(),
            (uy * plane.h()).to_bits(),
        ];
        if edge.camera
            != [
                case.view.center_re.clone(),
                case.view.center_im.clone(),
                case.view.width.clone(),
            ]
            || edge.nx != case.params.nx
            || edge.ny != case.params.ny
            || edge.cap != case.params.max_iter
            || case.params.ss != 1
            || case.view.rotation != 0.0
            || case.params.escape_radius != 1e10
            || edge.bits != bits
            || point.class != 'E'
            || point.iteration != edge.iteration
        {
            return Err(format!(
                "{} precision-edge identity changed at {}",
                case.name, edge.index
            ));
        }
    }
    Ok(())
}

#[derive(Default)]
pub struct Score {
    pub escaped: usize,
    pub analytic: usize,
    pub finite_cap: usize,
    pub finite_cap_interior: usize,
    pub class_mismatches: usize,
    pub known_edges: usize,
    pub scalar_failures: usize,
    pub nu_px: f64,
    pub de_px: f64,
    pub normal_px: f64,
    pub first_failure: Option<usize>,
}

fn displacement(excess: f64, rate: f64) -> f64 {
    if excess <= 0.0 {
        0.0
    } else if rate > 0.0 {
        excess / rate
    } else {
        f64::INFINITY
    }
}

fn compare(case: &Case, samples: &Samples, edges: &[Edge]) -> Score {
    let mut score = Score::default();
    for (i, (truth, got)) in case.points.iter().zip(&samples.class).enumerate() {
        let kind = got.kind();
        let class_ok = match truth.class {
            'E' => {
                score.escaped += 1;
                kind == Some(Kind::Escaped)
            }
            'A' => {
                score.analytic += 1;
                kind == Some(Kind::Interior)
            }
            'U' => {
                score.finite_cap += 1;
                score.finite_cap_interior += usize::from(kind == Some(Kind::Interior));
                matches!(kind, Some(Kind::Interior | Kind::Unresolved))
            }
            _ => unreachable!(),
        };
        if !class_ok {
            eprintln!(
                "{} index={} oracle={} escape_iteration={} production={:?} nu={:?}",
                case.name,
                i,
                truth.class,
                truth.iteration,
                kind,
                samples.nu.as_ref().map(|v| v[i])
            );
            score.class_mismatches += 1;
            if matches!(case.params.tier, None | Some(Tier::F64))
                && kind == Some(Kind::Unresolved)
                && truth.class == 'E'
                && edges.iter().any(|e| e.case == case.name && e.index == i)
            {
                score.known_edges += 1;
            }
            score.first_failure.get_or_insert(i);
            continue;
        }
        if truth.class != 'E' {
            continue;
        }
        let [nu, de, angle, de_rate, normal_rate] = truth.values;
        let got_nu = samples.nu.as_ref().expect("requested nu")[i];
        let got_de = f64::from(samples.de.as_ref().expect("requested de")[i]);
        let got_angle = f64::from(samples.normal.as_ref().expect("requested normal")[i])
            * std::f64::consts::TAU
            / 65536.0;
        if !got_nu.is_finite() || !got_de.is_finite() {
            score.scalar_failures += 1;
            score.first_failure.get_or_insert(i);
            continue;
        }
        let nu_px = (got_nu - nu).abs() * de * std::f64::consts::LN_2 / 2.0;
        let de_diff = (got_de - de).abs();
        let de_px = if de_diff / de > 1e-2 {
            displacement(de_diff - de * 2f64.powi(-24), de_rate)
        } else {
            0.0
        };
        let normal_diff = ((got_angle - angle + std::f64::consts::PI)
            .rem_euclid(std::f64::consts::TAU)
            - std::f64::consts::PI)
            .abs();
        let normal_px = if normal_diff > 1e-3 {
            displacement(normal_diff - std::f64::consts::PI / 65536.0, normal_rate)
        } else {
            0.0
        };
        score.nu_px = score.nu_px.max(nu_px);
        score.de_px = score.de_px.max(de_px);
        score.normal_px = score.normal_px.max(normal_px);
        if nu_px > 1e-3 || de_px > 1e-3 || normal_px > 1e-3 {
            score.scalar_failures += 1;
            score.first_failure.get_or_insert(i);
        }
    }
    score
}

pub type RouteScore = (&'static str, String, u64, Score);

pub fn run(case: &Case, edges: &[Edge]) -> Result<Vec<RouteScore>, String> {
    validate_edges(case, edges)?;
    let mut rows = run_routes(case, edges)?;
    let expected = edges.iter().filter(|e| e.case == case.name).count();
    if expected > 0 && rows.iter().any(|r| r.3.known_edges != expected) {
        return Err(
            "precision-edge observed class pattern changed; review the explicit edge record".into(),
        );
    }
    if expected > 0 {
        let fixed = Case {
            name: case.name.clone(),
            view: case.view.clone(),
            params: Params {
                tier: Some(Tier::Fixed),
                ..case.params
            },
            points: case.points.clone(),
        };
        for (route, kernel, blocks, score) in run_routes(&fixed, &[])? {
            let label = if route == "plain" {
                "fixed-recheck-plain"
            } else {
                "fixed-recheck-bla"
            };
            rows.push((label, kernel, blocks, score));
        }
    }
    Ok(rows)
}

fn run_routes(case: &Case, edges: &[Edge]) -> Result<Vec<RouteScore>, String> {
    let orbit = reference(&case.view, &case.params)?;
    let (header, plain, _) = render_with(&case.view, &case.params, Some(orbit.clone()))?;
    let mut rows = vec![(
        "plain",
        header.kernel.clone(),
        0,
        compare(case, &plain, edges),
    )];
    match bla_dc_max(&case.view, &case.params) {
        Ok(radius) => {
            let bla = Bla::build(&orbit.0, BLA_EPS_MAX, radius)?;
            let (header, samples, _, stats) = render_bla(&case.view, &case.params, orbit, &bla)?;
            rows.push((
                "bla",
                header.kernel,
                stats.blocks,
                compare(case, &samples, edges),
            ));
        }
        Err(reason) => {
            // The scaled tier has no BLA implementation yet (FIX-03); do not
            // silently turn other preparation errors into successful fallback.
            if !reason.contains("not available on the scaled tier") {
                return Err(reason);
            }
            rows.push((
                "bla-unavailable-plain-fallback",
                header.kernel,
                0,
                compare(case, &plain, edges),
            ));
        }
    }
    Ok(rows)
}
