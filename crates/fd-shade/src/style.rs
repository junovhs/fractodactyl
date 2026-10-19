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
}

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
    result: Compiled,
}

impl Compiler<'_> {
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
        let (ty, expr) = match node.op.as_str() {
            "scalar" if a.len() == 1 => (Scalar, format!("{:?}", number(&a[0])?)),
            "vector" if a.len() == 2 => (
                Vector,
                format!("vec2<f32>({:?}, {:?})", number(&a[0])?, number(&a[1])?),
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
                )
            }
            "param" if a.len() == 1 => {
                let value = number(&a[0])?;
                let index = self.result.parameters.len();
                if index >= 64 {
                    return Err("at most 64 style parameters".into());
                }
                self.result.parameters.push((name.into(), value));
                (Scalar, format!("p.knobs[{index}].x"))
            }
            "source" if a.len() == 1 => {
                self.result.sources.insert(a[0].clone());
                match a[0].as_str() {
                    "de" => (Scalar, "de".into()),
                    "escaped" => (Scalar, "select(0.0,1.0,(s.w>>16u)==0u)".into()),
                    "interior" => (Scalar, "select(0.0,1.0,(s.w>>16u)==1u)".into()),
                    "unresolved" => (Scalar, "select(0.0,1.0,(s.w>>16u)==2u)".into()),
                    "nu" => (Scalar, "local_nu".into()),
                    "normal" => (Vector, "normal".into()),
                    "gradient" => (
                        Vector,
                        "-normal * (2.0 / (max(de, 1e-20) * 0.69314718))".into(),
                    ),
                    "tangent" => (Vector, "vec2<f32>(-normal.y, normal.x)".into()),
                    "position" => (Vector, "position".into()),
                    "time" => (Scalar, "p.clock.x".into()),
                    _ => return Err(bad()),
                }
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
                if count == 1 {
                    match op {
                        "phase" if xt == Scalar => {
                            self.result.sources.insert("phase".into());
                            (Scalar, format!("style_phase(s.xy, {x})"))
                        }
                        "length" if xt == Vector => (Scalar, format!("length({x})")),
                        "normalize" if xt == Vector => {
                            (Vector, format!("{x} / max(length({x}), 1e-8)"))
                        }
                        "sin" | "cos" | "abs" | "exp" | "fract" => (xt, format!("{op}({x})")),
                        "log" => (xt, format!("log(max({x}, {}(1e-20)))", xt.wgsl())),
                        _ => return Err(bad()),
                    }
                } else if op == "compose" {
                    if xt != Scalar || v[1].1 != Scalar {
                        return Err(bad());
                    }
                    (Vector, format!("vec2<f32>({x},{})", v[1].0))
                } else if op == "band" {
                    if xt != Scalar || v[1].1 != Scalar {
                        return Err(bad());
                    }
                    self.result.sources.insert("de".into());
                    self.result.sources.insert("phase".into());
                    (Scalar, format!("style_band(s.xy, de, {x}, {})", v[1].0))
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
fn style_field(i: u32, position: vec2<f32>) -> {ty} {{
    let s=samples[i];
    if ((s.w >> 16u)!=0u) {{return {ty}(0.0);}}
    let fields=style_fields(i);
    let de=fields.de;
    let normal=fields.normal;
    let local_nu=fields.local_nu;
{body}
    return {x};
}}
fn style_flow(i: u32, direction: vec2<f32>, distance: f32, current: {ty}) -> {ty} {{
    let magnitude=length(direction);
    if (abs(distance)<0.001 || magnitude<0.001) {{return current;}}
    let delta=direction/magnitude*clamp(distance,-8.0,8.0)*f32(p.dims.z)/max(p.clock.y,1.0);
    let origin=vec2<f32>(f32(i % p.dims.x),f32(i / p.dims.x));
    var value=current*6.0;
    for (var k=0; k<5; k++) {{
        if (k==2) {{continue;}}
        let xy=vec2<u32>(clamp(round(origin+delta*(f32(k)*0.5-1.0)),vec2<f32>(0.0),vec2<f32>(p.dims.xy-1u)));
        let weight=select(1.0,4.0,k==1 || k==3);
        value+=style_field(xy.y*p.dims.x+xy.x,(vec2<f32>(xy)+0.5)/f32(p.dims.z))*weight;
    }}
    return value/16.0;
}}
"#,
                        ty = xt.wgsl(),
                        body = self.result.body,
                        x = x
                    );
                    (xt, format!("style_flow(i, {}, {}, {x})", v[1].0, v[2].0))
                } else if op == "light" {
                    if xt != Vector || v[1].1 != Scalar || v[2].1 != Vector {
                        return Err(bad());
                    }
                    self.result.sources.insert("de".into());
                    (
                        Scalar,
                        format!("style_light({x}, {}, {}, de)", v[1].0, v[2].0),
                    )
                } else if op == "boundary" {
                    if xt != Scalar || v[1].1 != Scalar {
                        return Err(bad());
                    }
                    (
                        Scalar,
                        format!("exp(-max({x}, 0.0) / max({}, 1e-5))", v[1].0),
                    )
                } else if op == "dot" {
                    if xt != Vector || v[1].1 != Vector {
                        return Err(bad());
                    }
                    (Scalar, format!("dot({x}, {})", v[1].0))
                } else if count == 3 {
                    if op == "mix" {
                        if xt != v[1].1 || v[2].1 != Scalar {
                            return Err(bad());
                        }
                    } else if v.iter().any(|(_, t)| *t != Scalar) {
                        return Err(bad());
                    }
                    (xt, format!("{op}({x}, {}, {})", v[1].0, v[2].0))
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
                    let e = match op {
                        "add" => format!("{xx} + {yy}"),
                        "sub" => format!("{xx} - {yy}"),
                        "mul" => format!("{xx} * {yy}"),
                        "div" => format!("{xx} / select(select({t}(-1e-8), {t}(1e-8), {yy} >= {t}(0.0)), {yy}, abs({yy}) > {t}(1e-8))", t=t.wgsl()),
                        "pow" => format!("pow(max({xx}, {}(0.0)), {yy})", t.wgsl()),
                        "min" | "max" => format!("{op}({xx}, {yy})"),
                        _ => return Err(bad()),
                    };
                    (t, e)
                }
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
