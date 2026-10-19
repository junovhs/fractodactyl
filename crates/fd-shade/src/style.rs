//! Opt-in style graph. Compilation is independent of devices and fractal iteration.
//! Values are linear-light colors, scalars, or vectors; only reachable work is emitted.
use std::collections::{BTreeMap, BTreeSet, HashMap};
pub mod finishing;
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
}

/// Reusable generated pointwise body. Neighborhood passes are explicit extensions,
/// never hidden inside a node. Uniform values do not participate in pipeline keys.
#[derive(Clone, Debug)]
pub struct Compiled {
    pub body: String,
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
        Ok(Self {
            nodes,
            surface: surface.ok_or("style needs surface NAME")?,
            emission: emission.ok_or("style needs emission NAME")?,
        })
    }

    /// DEC-33: resolve reachable values once and fuse pure shared expressions.
    pub fn compile(&self) -> Result<Compiled, String> {
        let mut c = Compiler {
            graph: self,
            values: HashMap::new(),
            visiting: BTreeSet::new(),
            expressions: HashMap::new(),
            result: Compiled {
                body: String::new(),
                surface: String::new(),
                emission: String::new(),
                reachable_nodes: 0,
                unique_operations: 0,
                parameters: Vec::new(),
                sources: BTreeSet::new(),
            },
        };
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
        format!("style_version 1 reachable_nodes {} unique_operations {} pointwise_passes 1 neighborhood_passes 0 support_px 0 parameters {} sources {}\ncoordinates sample-local,output-pixel time_dependent {}\n", self.reachable_nodes, self.unique_operations, self.parameters.len(), self.sources.iter().cloned().collect::<Vec<_>>().join(","), self.sources.contains("time"))
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
                    | "phase" => {
                        if op == "phase" {
                            1
                        } else {
                            2
                        }
                    }
                    "mix" | "smoothstep" | "clamp" | "light" => 3,
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
                } else if op == "light" {
                    if xt != Vector || v[1].1 != Scalar || v[2].1 != Vector {
                        return Err(bad());
                    }
                    (Scalar, format!("0.15 + 0.85 * max(dot(normalize(vec3<f32>({x} * {}, 1.0)), normalize(vec3<f32>({}, 1.0))), 0.0)",v[1].0,v[2].0))
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
