//! Opt-in style graph. Compilation is independent of devices and fractal iteration.
//! Values are linear-light colors, scalars, or vectors; only reachable work is emitted.
use std::collections::{BTreeMap, BTreeSet, HashMap};
pub mod finishing;
mod image_gpu;
pub mod images;
pub mod render;

/// Compatible scalar, spatial-vector and linear-color graph values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ValueType {
    Scalar,
    Vector,
    Color,
}
impl ValueType {
    fn wgsl(self) -> &'static str {
        match self {
            Self::Scalar => "f32",
            Self::Vector => "vec2<f32>",
            Self::Color => "vec3<f32>",
        }
    }
}

#[derive(Clone, Debug)]
struct Node {
    op: String,
    args: Vec<String>,
}

/// Versioned, dependency-free text representation. Node order is immaterial.
#[derive(Clone, Debug)]
pub struct Graph {
    nodes: BTreeMap<String, Node>,
    surface: String,
    emission: String,
    output: Option<String>,
}

/// Reusable generated pointwise body. Neighborhood passes are explicit extensions,
/// never hidden inside a node. Uniform values do not participate in pipeline keys.
#[derive(Clone, Debug)]
pub struct Compiled {
    pub images: Option<images::Plan>,
    pub body: String,
    pub helpers: String,
    pub support_pixels: u32,
    pub evaluations_per_sample: u32,
    pub surface: String,
    pub emission: String,
    pub reachable_nodes: usize,
    pub unique_operations: usize,
    pub parameters: Vec<(String, f32)>,
    pub sources: BTreeSet<String>,
    /// STYL-08: phase-spread evaluations per sample (1, or STYLE_TAPS when some periodic
    /// value can change faster than a sample).
    pub taps: u32,
}

/// Evaluations per sample when a graph spreads fast periodic values over their cycle.
pub const STYLE_TAPS: u32 = 4;

impl Graph {
    /// Read graph v1 text without creating a GPU device.
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.len() > 262_144 {
            return Err("style exceeds 256 KiB".into());
        }
        let mut nodes = BTreeMap::new();
        let (mut surface, mut emission, mut version) = (None, None, false);
        let mut output = None;
        for (i, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let f: Vec<_> = line.split_whitespace().collect();
            let error = || {
                format!("style line {}: expected version 1, surface NAME, emission NAME, or NAME = OP INPUTS", i+1)
            };
            match f.as_slice() {
                ["version", "1"] if !version => version = true,
                ["output", name] if output.is_none() => output = Some((*name).to_string()),
                ["surface", name] if surface.is_none() => surface = Some((*name).to_string()),
                ["emission", name] if emission.is_none() => emission = Some((*name).to_string()),
                [name, "=", op, args @ ..] => {
                    if !name.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
                        || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
                    {
                        return Err(format!("line {}: invalid node name", i + 1));
                    }
                    let n = Node {
                        op: (*op).into(),
                        args: args.iter().map(|x| (*x).into()).collect(),
                    };
                    if nodes.insert((*name).into(), n).is_some() {
                        return Err(format!("duplicate node {name}"));
                    }
                    if nodes.len() > 1024 {
                        return Err("at most 1024 style nodes".into());
                    }
                }
                _ => return Err(error()),
            }
        }
        if !version {
            return Err("style needs version 1".into());
        }
        if output.is_some() && (surface.is_some() || emission.is_some()) {
            return Err("use output IMAGE or surface/emission, not both".into());
        }
        if output.is_none() && (surface.is_none() || emission.is_none()) {
            return Err("style needs surface/emission or output IMAGE".into());
        }
        Ok(Self {
            nodes,
            surface: surface.unwrap_or_default(),
            emission: emission.unwrap_or_default(),
            output,
        })
    }

    /// DEC-33: resolve reachable values once and fuse pure shared expressions.
    fn compiler(&self) -> Compiler<'_> {
        Compiler {
            graph: self,
            values: HashMap::new(),
            visiting: BTreeSet::new(),
            expressions: HashMap::new(),
            rates: HashMap::new(),
            result: Compiled {
                images: None,
                body: String::new(),
                helpers: String::new(),
                support_pixels: 0,
                evaluations_per_sample: 1,
                surface: String::new(),
                emission: String::new(),
                reachable_nodes: 0,
                unique_operations: 0,
                parameters: Vec::new(),
                sources: BTreeSet::new(),
                taps: 1,
            },
        }
    }
    pub fn compile(&self) -> Result<Compiled, String> {
        if let Some(output) = &self.output {
            return images::compile(self, output);
        }
        let mut c = self.compiler();
        let (surface, st) = c.visit(&self.surface)?;
        let (emission, et) = c.visit(&self.emission)?;
        if st != ValueType::Color || et != ValueType::Color {
            return Err("surface and emission must be colors".into());
        }
        c.result.surface = surface;
        c.result.emission = emission;
        c.result.reachable_nodes = c.values.len();
        Ok(c.result)
    }
}

impl Compiled {
    /// Describe pointwise work and explicit source dependencies for CLI inspection.
    pub fn report(&self) -> String {
        let mut report=format!("style_version 1 reachable_nodes {} unique_operations {} pointwise_passes 1 neighborhood_passes 0 support_px {} evaluations_per_sample_max {} parameters {} sources {}\ncoordinates sample-local,output-pixel time_dependent {}\n", self.reachable_nodes, self.unique_operations, self.support_pixels, self.evaluations_per_sample, self.parameters.len(), self.sources.iter().cloned().collect::<Vec<_>>().join(","), self.sources.contains("time"));
        if let Some(plan) = &self.images {
            report.push_str(&plan.report());
        }
        report
    }
}

struct Compiler<'a> {
    graph: &'a Graph,
    values: HashMap<String, (String, ValueType)>,
    visiting: BTreeSet<String>,
    expressions: HashMap<String, (String, ValueType)>,
    /// STYL-08: per-sample rate of change of each emitted value (an f32 WGSL variable),
    /// absent when the value is constant across samples.
    rates: HashMap<String, String>,
    result: Compiled,
}

/// Size of a value in the units its rate is measured in.
fn magnitude(x: &str, t: ValueType) -> String {
    match t {
        ValueType::Scalar => format!("abs({x})"),
        ValueType::Vector => format!("length({x})"),
        ValueType::Color => format!("max(max(abs({x}.x), abs({x}.y)), abs({x}.z))"),
    }
}

impl Compiler<'_> {
    fn rate(&self, var: &str) -> String {
        self.rates.get(var).cloned().unwrap_or_else(|| "0.0".into())
    }
    fn visit(&mut self, name: &str) -> Result<(String, ValueType), String> {
        use ValueType::*;
        if let Some(v) = self.values.get(name) {
            return Ok(v.clone());
        }
        if !self.visiting.insert(name.into()) {
            return Err(format!("cycle at {name}"));
        }
        let node = self
            .graph
            .nodes
            .get(name)
            .ok_or_else(|| format!("missing node {name}"))?
            .clone();
        let a = &node.args;
        let bad = || format!("node {name}: invalid inputs for {}", node.op);
        let number = |s: &str| {
            s.parse::<f32>()
                .ok()
                .filter(|x| x.is_finite())
                .ok_or_else(bad)
        };
        let (ty, expr, rate): (ValueType, String, String) = match node.op.as_str() {
            "scalar" if a.len() == 1 => (Scalar, format!("{:?}", number(&a[0])?), "0.0".into()),
            "vector" if a.len() == 2 => (
                Vector,
                format!("vec2<f32>({:?}, {:?})", number(&a[0])?, number(&a[1])?),
                "0.0".into(),
            ),
            "color" if a.len() == 1 => {
                let rgb = a[0]
                    .strip_prefix('#')
                    .filter(|h| h.len() == 6)
                    .and_then(|h| u32::from_str_radix(h, 16).ok())
                    .ok_or_else(bad)?;
                let v = super::studio::linear(rgb);
                (
                    Color,
                    format!("vec3<f32>({:?}, {:?}, {:?})", v[0], v[1], v[2]),
                    "0.0".into(),
                )
            }
            "param" if a.len() == 1 => {
                let value = number(&a[0])?;
                let index = self.result.parameters.len();
                if index >= 64 {
                    return Err("at most 64 style parameters".into());
                }
                self.result.parameters.push((name.into(), value));
                (Scalar, format!("p.knobs[{index}].x"), "0.0".into())
            }
            "source" if a.len() == 1 => {
                self.result.sources.insert(a[0].clone());
                let (ty, expr) = match a[0].as_str() {
                    "de" => (Scalar, "de"),
                    "escaped" => (Scalar, "select(0.0,1.0,((s.w>>16u)&3u)==0u)"),
                    "interior" => (Scalar, "select(0.0,1.0,((s.w>>16u)&3u)==1u)"),
                    "unresolved" => (Scalar, "select(0.0,1.0,((s.w>>16u)&3u)==2u)"),
                    "nu" => (Scalar, "local_nu"),
                    // STYL-09: the stripe column, 14 bits above the class.
                    "stripe" => (Scalar, "f32(s.w >> 18u) / 16383.0"),
                    "normal" => (Vector, "normal"),
                    "gradient" => (Vector, "-normal * (2.0 / (max(de, 1e-20) * 0.69314718))"),
                    "tangent" => (Vector, "vec2<f32>(-normal.y, normal.x)"),
                    "position" => (Vector, "position"),
                    "time" => (Scalar, "p.clock.x"),
                    _ => return Err(bad()),
                };
                // A distance estimate changes by one sample spacing per sample; nu and
                // the normal turn once per distance-to-boundary (style_band's rule).
                let rate = match a[0].as_str() {
                    "de" => "style_de_rate()",
                    "nu" | "stripe" => "style_nu_rate(de)",
                    "normal" | "tangent" => "1.0 / style_samples(de)",
                    "gradient" => "{SELF_MAG} * 2.0 / style_samples(de)",
                    "position" => "1.0 / f32(p.dims.z)",
                    _ => "0.0",
                };
                (ty, expr.into(), rate.into())
            }
            op => {
                let count = match op {
                    "sin" | "cos" | "abs" | "exp" | "log" | "fract" | "length" | "normalize" => 1,
                    "add" | "sub" | "mul" | "div" | "pow" | "min" | "max" | "dot" | "boundary"
                    | "phase" | "band" | "compose" => {
                        if op == "phase" {
                            1
                        } else {
                            2
                        }
                    }
                    "mix" | "smoothstep" | "clamp" | "light" | "flow" => 3,
                    _ => return Err(format!("node {name}: unknown operation {op}")),
                };
                if a.len() != count {
                    return Err(bad());
                }
                let v: Vec<_> = a.iter().map(|n| self.visit(n)).collect::<Result<_, _>>()?;
                let x = &v[0].0;
                let xt = v[0].1;
                let r: Vec<String> = v.iter().map(|(var, _)| self.rate(var)).collect();
                let rx = &r[0];
                let still = |rates: &[&String]| rates.iter().all(|r| r.as_str() == "0.0");
                let m = |i: usize| magnitude(&v[i].0, v[i].1);
                let (t, e, rate) = if count == 1 {
                    // STYL-08: once the argument moves about a cycle per sample, the
                    // sample's evaluations spread it over (1 - k) of a cycle, so their
                    // average is the colour averaged over the cycle, not of its mean.
                    let mut spread = false;
                    let mut filtered =
                        |e: &dyn Fn(&str) -> String, period: &str, cycles: String| {
                            if still(&[rx]) {
                                (e(x), "0.0".to_string())
                            } else {
                                spread = true;
                                let k = format!("band_response({cycles})");
                                (
                                    e(&format!("({x} + {period} * style_j * (1.0 - {k}))")),
                                    format!("({rx}) * {k}"),
                                )
                            }
                        };
                    let out = match op {
                        "phase" if xt == Scalar => {
                            self.result.sources.insert("phase".into());
                            self.result.sources.insert("de".into());
                            (
                                Scalar,
                                format!("style_phase(s.xy, {x})"),
                                format!("abs({x}) * style_nu_rate(de)"),
                            )
                        }
                        "length" if xt == Vector => (Scalar, format!("length({x})"), rx.clone()),
                        "normalize" if xt == Vector => (
                            Vector,
                            format!("{x} / max(length({x}), 1e-8)"),
                            format!("({rx}) / max(length({x}), 1e-8)"),
                        ),
                        "sin" | "cos" => {
                            let (e, rate) = filtered(
                                &|a| format!("{op}({a})"),
                                "6.2831853",
                                format!("({rx}) * 0.15915494"),
                            );
                            (xt, e, rate)
                        }
                        "fract" => {
                            let (e, rate) = filtered(&|a| format!("fract({a})"), "1.0", rx.clone());
                            (xt, e, rate)
                        }
                        "abs" => (xt, format!("abs({x})"), rx.clone()),
                        "exp" => (xt, format!("exp({x})"), format!("{{SELF_MAG}} * ({rx})")),
                        "log" => (
                            xt,
                            format!("log(max({x}, {}(1e-20)))", xt.wgsl()),
                            format!("({rx}) / max({}, 1e-20)", m(0)),
                        ),
                        _ => return Err(bad()),
                    };
                    if spread {
                        self.result.taps = STYLE_TAPS;
                    }
                    out
                } else if op == "compose" {
                    if xt != Scalar || v[1].1 != Scalar {
                        return Err(bad());
                    }
                    (
                        Vector,
                        format!("vec2<f32>({x},{})", v[1].0),
                        format!("max({rx}, {})", r[1]),
                    )
                } else if op == "band" {
                    if xt != Scalar || v[1].1 != Scalar {
                        return Err(bad());
                    }
                    self.result.sources.insert("de".into());
                    self.result.sources.insert("phase".into());
                    (
                        Scalar,
                        format!("style_band(s.xy, de, {x}, {})", v[1].0),
                        format!("3.14159265 * abs({x}) * style_nu_rate(de) * band_response(abs({x}) * style_nu_rate(de))"),
                    )
                } else if op == "flow" {
                    if xt == Vector || v[1].1 != Vector || v[2].1 != Scalar {
                        return Err(bad());
                    }
                    if self.result.evaluations_per_sample > 1 {
                        return Err("POC supports one non-nested flow node per graph (bounded sampling cost)".into());
                    }
                    self.result.support_pixels = 9;
                    self.result.evaluations_per_sample = 5;
                    self.result.helpers = format!(
                        r#"
fn style_field(i: u32, position: vec2<f32>, style_j: f32) -> {ty} {{
    let s=samples[i];
    if (((s.w >> 16u) & 3u)!=0u) {{return {ty}(0.0);}}
    let fields=style_fields(i);
    let de=fields.de;
    let normal=fields.normal;
    let local_nu=fields.local_nu;
{body}
    return {x};
}}
fn style_flow(i: u32, style_j: f32, direction: vec2<f32>, distance: f32, current: {ty}) -> {ty} {{
    let magnitude=length(direction);
    if (abs(distance)<0.001 || magnitude<0.001) {{return current;}}
    let delta=direction/magnitude*clamp(distance,-8.0,8.0)*f32(p.dims.z)/style_scale();
    let origin=vec2<f32>(f32(i % p.dims.x),f32(i / p.dims.x));
    var value=current*6.0;
    for (var k=0; k<5; k++) {{
        if (k==2) {{continue;}}
        let xy=vec2<u32>(clamp(round(origin+delta*(f32(k)*0.5-1.0)),vec2<f32>(0.0),vec2<f32>(p.dims.xy-1u)));
        let weight=select(1.0,4.0,k==1 || k==3);
        value+=style_field(xy.y*p.dims.x+xy.x,(vec2<f32>(xy)+0.5)/f32(p.dims.z),style_j)*weight;
    }}
    return value/16.0;
}}
"#,
                        ty = xt.wgsl(),
                        body = self.result.body,
                        x = x
                    );
                    (
                        xt,
                        format!("style_flow(i, style_j, {}, {}, {x})", v[1].0, v[2].0),
                        rx.clone(),
                    )
                } else if op == "light" {
                    if xt != Vector || v[1].1 != Scalar || v[2].1 != Vector {
                        return Err(bad());
                    }
                    self.result.sources.insert("de".into());
                    // The lit value turns with the normal, scaled by the slope; a fast
                    // normal is spread over its turn like the periodic ops above.
                    let cycles = format!("abs({}) * ({rx}) * 0.15915494", v[1].0);
                    let normal = if still(&[rx]) {
                        x.clone()
                    } else {
                        self.result.taps = STYLE_TAPS;
                        format!(
                            "style_rotate({x}, 6.2831853 * style_j * (1.0 - band_response({cycles})))"
                        )
                    };
                    (
                        Scalar,
                        format!("style_light({normal}, {}, {}, de)", v[1].0, v[2].0),
                        format!("abs({}) * ({rx}) * band_response({cycles})", v[1].0),
                    )
                } else if op == "boundary" {
                    if xt != Scalar || v[1].1 != Scalar {
                        return Err(bad());
                    }
                    (
                        Scalar,
                        format!("exp(-max({x}, 0.0) / max({}, 1e-5))", v[1].0),
                        format!("{{SELF_MAG}} * ({rx}) / max({}, 1e-5)", v[1].0),
                    )
                } else if op == "dot" {
                    if xt != Vector || v[1].1 != Vector {
                        return Err(bad());
                    }
                    (
                        Scalar,
                        format!("dot({x}, {})", v[1].0),
                        format!("{} * ({}) + {} * ({rx})", m(0), r[1], m(1)),
                    )
                } else if count == 3 {
                    if op == "mix" {
                        if xt != v[1].1 || v[2].1 != Scalar {
                            return Err(bad());
                        }
                    } else if v.iter().any(|(_, t)| *t != Scalar) {
                        return Err(bad());
                    }
                    let rate = match op {
                        "mix" => format!(
                            "({rx}) + ({}) + {} * ({})",
                            r[1],
                            magnitude(&format!("({} - {x})", v[1].0), xt),
                            r[2]
                        ),
                        "smoothstep" => {
                            format!("1.5 * ({}) / max(abs({} - {x}), 1e-8)", r[2], v[1].0)
                        }
                        _ => rx.clone(),
                    };
                    (xt, format!("{op}({x}, {}, {})", v[1].0, v[2].0), rate)
                } else {
                    let yt = v[1].1;
                    if xt != yt && xt != Scalar && yt != Scalar {
                        return Err(bad());
                    }
                    let t = if xt == Scalar { yt } else { xt };
                    let cast = |s: &str, st| {
                        if st == Scalar && t != Scalar {
                            format!("{}({s})", t.wgsl())
                        } else {
                            s.to_string()
                        }
                    };
                    let (xx, yy) = (cast(x, xt), cast(&v[1].0, yt));
                    let ry = &r[1];
                    let rate = match op {
                        "add" | "sub" => format!("({rx}) + ({ry})"),
                        "mul" => format!("{} * ({ry}) + {} * ({rx})", m(0), m(1)),
                        "div" => format!(
                            "(({rx}) + {{SELF_MAG}} * ({ry})) / max({}, 1e-8)",
                            m(1)
                        ),
                        "pow" => format!(
                            "{{SELF_MAG}} * ({} * ({rx}) / max({}, 1e-20) + abs(log(max({}, 1e-20))) * ({ry}))",
                            m(1),
                            m(0),
                            m(0)
                        ),
                        _ => format!("max({rx}, {ry})"),
                    };
                    let e = match op {
                        "add" => format!("{xx} + {yy}"),
                        "sub" => format!("{xx} - {yy}"),
                        "mul" => format!("{xx} * {yy}"),
                        "div" => format!("{xx} / select(select({t}(-1e-8), {t}(1e-8), {yy} >= {t}(0.0)), {yy}, abs({yy}) > {t}(1e-8))", t=t.wgsl()),
                        "pow" => format!("pow(max({xx}, {}(0.0)), {yy})", t.wgsl()),
                        "min" | "max" => format!("{op}({xx}, {yy})"),
                        _ => return Err(bad()),
                    };
                    (t, e, rate)
                };
                // Constant inputs give constant results, except where nu enters.
                let rate = if still(&r.iter().collect::<Vec<_>>()) && op != "phase" && op != "band"
                {
                    "0.0".to_string()
                } else {
                    rate
                };
                (t, e, rate)
            }
        };
        let key = format!("{ty:?}:{expr}");
        let value = if let Some(value) = self.expressions.get(&key) {
            value.clone()
        } else {
            let var = format!("v{}", self.result.unique_operations);
            self.result
                .body
                .push_str(&format!("    let {var}: {} = {expr};\n", ty.wgsl()));
            if rate != "0.0" {
                let rate = rate.replace("{SELF_MAG}", &magnitude(&var, ty));
                let name = format!("r{}", self.result.unique_operations);
                self.result
                    .body
                    .push_str(&format!("    let {name}: f32 = {rate};\n"));
                self.rates.insert(var.clone(), name);
            }
            self.result.unique_operations += 1;
            let value = (var, ty);
            self.expressions.insert(key, value.clone());
            value
        };
        self.visiting.remove(name);
        self.values.insert(name.into(), value.clone());
        Ok(value)
    }
}
