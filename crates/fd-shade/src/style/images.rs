//! Image DAG lowering: typed ports, common work, dependency order and buffer liveness.
use super::{Compiled, Compiler, Graph, ValueType};
use std::collections::{BTreeSet, HashMap};

/// Meaning of an image's channels, independent of its shared RGBA32F storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Scalar,
    Vector,
    Color,
}
/// A rasterized field; all reachable sources share one fused sample dispatch.
#[derive(Clone, Debug)]
pub struct Source {
    pub label: String,
    pub value: String,
    pub kind: Kind,
    pub spatial: bool,
    pub masked: bool,
}
/// One logical image with either an atlas source or a reusable transient slot.
#[derive(Clone, Debug)]
pub struct Image {
    pub kind: Kind,
    pub source: Option<usize>,
    pub slot: usize,
    pub support: u32,
}
/// One GPU operation. A blur lowers to two axis passes, never an implicit loop.
#[derive(Clone, Debug)]
pub struct Fused {
    pub inputs: Vec<usize>,
    pub body: String,
    pub result: String,
}

#[derive(Clone, Debug)]
pub struct Step {
    pub op: u32,
    pub inputs: [usize; 3],
    pub output: usize,
    pub fused: Option<Fused>,
}
impl Step {
    pub fn dependencies(&self) -> &[usize] {
        self.fused
            .as_ref()
            .map_or(&self.inputs, |f| f.inputs.as_slice())
    }
}
/// Reachable image execution plan; source images are kept in a shared atlas.
#[derive(Clone, Debug)]
pub struct Plan {
    pub sources: Vec<Source>,
    pub images: Vec<Image>,
    pub steps: Vec<Step>,
    pub output: usize,
    pub slots: usize,
}

impl Plan {
    pub fn report(&self) -> String {
        let mut s = format!(
            "image_sources {} image_passes {} transient_slots {} output_support_px {}\n",
            self.sources.len(),
            self.steps.len(),
            self.slots,
            if self.images[self.output].support == u32::MAX {
                "full-frame".to_string()
            } else {
                self.images[self.output].support.to_string()
            }
        );
        for (i, source) in self.sources.iter().enumerate() {
            s += &format!(
                "field {i} name {} type {:?} scope {}\n",
                source.label,
                source.kind,
                if source.spatial { "spatial" } else { "global" }
            );
        }
        for (i, p) in self.steps.iter().enumerate() {
            let op = if p.fused.is_some() {
                "fused-pointwise"
            } else {
                match p.op {
                    4 => "warp",
                    5 => "blur-x",
                    6 => "blur-y",
                    7 => "relight",
                    8 => "stream",
                    9 => "zoom",
                    _ => "unknown",
                }
            };
            s += &format!(
                "pass {i} op {op} inputs {:?} output {} slot {}\n",
                p.dependencies(),
                p.output,
                self.images[p.output].slot
            );
        }
        s
    }
    pub(super) fn layout(&self, alignment: u64, w: usize, h: usize) -> (Vec<(u64, u64)>, u64) {
        let mut end = 0u64;
        let mut layout = Vec::new();
        for source in &self.sources {
            let offset = end.next_multiple_of(alignment);
            let bytes = if source.spatial {
                w as u64 * h as u64 * 16
            } else {
                16
            };
            layout.push((offset, bytes));
            end = offset + bytes;
        }
        (layout, end)
    }
    fn fuse(&mut self) {
        let producers: HashMap<usize, usize> = self
            .steps
            .iter()
            .enumerate()
            .map(|(i, s)| (s.output, i))
            .collect();
        let mut users = vec![BTreeSet::new(); self.images.len()];
        for (i, s) in self.steps.iter().enumerate() {
            for &input in &s.inputs {
                users[input].insert(i);
            }
        }
        users[self.output].insert(self.steps.len());
        struct Tree<'a> {
            plan: &'a Plan,
            producers: &'a HashMap<usize, usize>,
            users: &'a [BTreeSet<usize>],
            root: usize,
            deep: bool,
            leaves: Vec<usize>,
            body: String,
            emitted: BTreeSet<usize>,
            absorbed: BTreeSet<usize>,
        }
        impl Tree<'_> {
            fn value(&mut self, id: usize) -> String {
                if self.emitted.contains(&id) {
                    return format!("v{id}");
                }
                let producer = self.producers.get(&id).copied();
                if let Some(index) = producer.filter(|&i| {
                    self.plan.steps[i].op <= 3
                        && (i == self.root || (self.deep && self.users[id].len() == 1))
                }) {
                    let step = &self.plan.steps[index];
                    let op = step.op;
                    let inputs = step.inputs;
                    let x = self.value(inputs[0]);
                    let y = self.value(inputs[1]);
                    let expression = match op {
                        0 => format!("{x}+{y}"),
                        1 => format!("{x}*{y}"),
                        2 => format!("max({x},{y})"),
                        3 => {
                            let z = self.value(inputs[2]);
                            format!("mix({x},{y},clamp({z}.x,0.0,1.0))")
                        }
                        _ => unreachable!(),
                    };
                    self.body += &format!("let v{id}={expression};\n");
                    self.emitted.insert(id);
                    if index != self.root {
                        self.absorbed.insert(index);
                    }
                    format!("v{id}")
                } else {
                    let index = if let Some(i) = self.leaves.iter().position(|&x| x == id) {
                        i
                    } else {
                        self.leaves.push(id);
                        self.leaves.len() - 1
                    };
                    format!("leaf{index}")
                }
            }
        }
        let mut removed = BTreeSet::new();
        let mut fused = HashMap::new();
        for i in (0..self.steps.len()).rev() {
            if removed.contains(&i) || self.steps[i].op > 3 {
                continue;
            }
            let build = |deep| {
                let mut t = Tree {
                    plan: self,
                    producers: &producers,
                    users: &users,
                    root: i,
                    deep,
                    leaves: Vec::new(),
                    body: String::new(),
                    emitted: BTreeSet::new(),
                    absorbed: BTreeSet::new(),
                };
                let result = t.value(self.steps[i].output);
                (
                    Fused {
                        inputs: t.leaves,
                        body: t.body,
                        result,
                    },
                    t.absorbed,
                )
            };
            let (mut f, mut absorbed) = build(true);
            if f.inputs.len() > 7 {
                (f, absorbed) = build(false);
            }
            removed.extend(absorbed);
            fused.insert(i, f);
        }
        self.steps = self
            .steps
            .drain(..)
            .enumerate()
            .filter_map(|(i, mut step)| {
                if removed.contains(&i) {
                    None
                } else {
                    step.fused = fused.remove(&i);
                    Some(step)
                }
            })
            .collect();
    }
    pub(super) fn fused_shader(&self, f: &Fused) -> String {
        let mut declarations = String::new();
        let mut reads = String::new();
        for (i, &id) in f.inputs.iter().enumerate() {
            declarations +=
                &format!("@group(0) @binding({i}) var<storage,read> src{i}:array<vec4<f32>>;\n");
            let image = &self.images[id];
            let index = if image.source.is_some_and(|s| !self.sources[s].spatial) {
                "0u"
            } else {
                "index"
            };
            let read = format!("src{i}[{index}]");
            let read = if image.kind == Kind::Scalar {
                format!("vec4<f32>({read}.x)")
            } else {
                read
            };
            reads += &format!("let leaf{i}={read};\n");
        }
        let output = f.inputs.len();
        let uniform = output + 1;
        format!(
            r#"{declarations}
struct Params {{dims:vec4<u32>,kinds:vec4<u32>}}
@group(0) @binding({output}) var<storage,read_write> result_image:array<vec4<f32>>;
@group(0) @binding({uniform}) var<uniform> p:Params;
@compute @workgroup_size(8,8)
fn apply(@builtin(global_invocation_id) gid:vec3<u32>){{
 if(any(gid.xy>=p.dims.xy)){{return;}}let index=gid.y*p.dims.x+gid.x;
 {reads}{body}
 let value={result};result_image[index]=select(vec4<f32>(0.0),clamp(value,vec4<f32>(-65504.0),vec4<f32>(65504.0)),abs(value)<vec4<f32>(1e30));
}}
"#,
            body = f.body,
            result = f.result
        )
    }
    fn assign_slots(&mut self) {
        let mut last = vec![0; self.images.len()];
        for (i, step) in self.steps.iter().enumerate() {
            for &input in step.dependencies() {
                last[input] = i + 1;
            }
        }
        last[self.output] = self.steps.len() + 1;
        let mut free = Vec::new();
        for (i, step) in self.steps.iter().enumerate() {
            self.images[step.output].slot = free.pop().unwrap_or_else(|| {
                let slot = self.slots;
                self.slots += 1;
                slot
            });
            for input in step.dependencies().iter().copied().collect::<BTreeSet<_>>() {
                if last[input] == i + 1 && self.images[input].source.is_none() {
                    free.push(self.images[input].slot);
                }
            }
        }
    }
    /// Generate the source-field rasterizer using the existing sample/precision helpers.
    pub(super) fn shader(&self, c: &Compiled) -> String {
        let template = include_str!("shade.wgsl");
        let prefix = template
            .split("fn evaluate(")
            .next()
            .unwrap()
            .replace(
                "var<storage, read_write> hdr: array<Layer>;",
                "var<storage, read_write> atlas: array<vec4<f32>>;",
            )
            .replace("// GRAPH_HELPERS", &c.helpers);
        let members = self
            .sources
            .iter()
            .enumerate()
            .map(|(i, _)| format!("f{i}: vec4<f32>"))
            .collect::<Vec<_>>()
            .join(",");
        let values = self
            .sources
            .iter()
            .map(|source| {
                let value = match source.kind {
                    Kind::Scalar => format!("vec4<f32>({})", source.value),
                    Kind::Vector => format!("vec4<f32>({},0.0,0.0)", source.value),
                    Kind::Color => format!("vec4<f32>(clean_color({}),0.0)", source.value),
                };
                if source.masked {
                    format!("select(vec4<f32>(mark,0.0), {value}, vec4<bool>((s.w>>16u)==0u))")
                } else {
                    value
                }
            })
            .collect::<Vec<_>>()
            .join(",");
        let mut accum = String::new();
        let mut writes = String::new();
        let mut variables = String::new();
        for i in 0..self.sources.len() {
            variables += &format!("var f{i}=vec4<f32>(0.0);\n");
            accum += &format!("f{i}+=v.f{i}*weight;\n");
            if self.sources[i].spatial {
                writes += &format!("atlas[p.image_offsets[{i}].x+index]=f{i}/max(total,1e-20);\n");
            } else {
                writes += &format!(
                    "if(index==0u){{atlas[p.image_offsets[{i}].x]=f{i}/max(total,1e-20);}}\n"
                );
            }
        }
        format!(
            r#"{prefix}
struct Raster {{ {members} }}
fn evaluate(i:u32,position:vec2<f32>)->Raster {{
 let s=samples[i];
 var mark=vec3<f32>(0.0);
 if ((s.w>>16u)!=0u && (s.w>>16u)!=1u && p.dims.w==0u) {{mark=vec3<f32>(0.25,0.0,0.25);}}
 let fields=style_fields(i);let de=fields.de;let normal=fields.normal;let local_nu=fields.local_nu;
 {body}
 return Raster({values});
}}
@compute @workgroup_size(8,8)
fn shade(@builtin(global_invocation_id) gid:vec3<u32>) {{
 let ss=p.dims.z;let size=p.dims.xy/ss;
 if (any(gid.xy>=size)) {{return;}}
 {variables}
 let footprint=style_footprint(gid.xy);
 let lo=vec2<u32>(floor(footprint.xy));let hi=vec2<u32>(ceil(footprint.zw));
 var total=0.0;
 for(var y=lo.y;y<hi.y;y++) {{for(var x=lo.x;x<hi.x;x++) {{
  let xy=vec2<u32>(x,y);
  let weight=style_weight(xy,gid.xy);
  if(weight<=0.0) {{continue;}}
  total+=weight;
  let v=evaluate(xy.y*p.dims.x+xy.x,(vec2<f32>(xy)+0.5)/f32(ss));
  {accum}
 }}}}
 let index=gid.y*size.x+gid.x;
 {writes}
}}
"#,
            body = c.body
        )
    }
}

struct Lower<'a> {
    graph: &'a Graph,
    fields: Compiler<'a>,
    plan: Plan,
    visited: HashMap<String, usize>,
    visiting: BTreeSet<String>,
    common: HashMap<String, usize>,
}

pub(super) fn compile(graph: &Graph, output: &str) -> Result<Compiled, String> {
    let mut c = Lower {
        graph,
        fields: graph.compiler(),
        plan: Plan {
            sources: Vec::new(),
            images: Vec::new(),
            steps: Vec::new(),
            output: 0,
            slots: 0,
        },
        visited: HashMap::new(),
        visiting: BTreeSet::new(),
        common: HashMap::new(),
    };
    c.plan.output = c.visit(output)?;
    if c.plan.images[c.plan.output].kind != Kind::Color {
        return Err("output must be a color image".into());
    }
    c.plan.fuse();
    c.plan.assign_slots();
    let mut result = c.fields.result;
    result.reachable_nodes = c.fields.values.len() + c.visited.len();
    result.support_pixels = c.plan.images[c.plan.output].support;
    result.images = Some(c.plan);
    Ok(result)
}

impl Lower<'_> {
    fn image(&mut self, kind: Kind, source: Option<usize>, support: u32) -> usize {
        let id = self.plan.images.len();
        self.plan.images.push(Image {
            kind,
            source,
            slot: 0,
            support,
        });
        id
    }
    fn step(&mut self, op: u32, inputs: [usize; 3], kind: Kind, support: u32) -> usize {
        let output = self.image(kind, None, support);
        self.plan.steps.push(Step {
            op,
            inputs,
            output,
            fused: None,
        });
        output
    }
    fn visit(&mut self, name: &str) -> Result<usize, String> {
        if let Some(&id) = self.visited.get(name) {
            return Ok(id);
        }
        if !self.visiting.insert(name.into()) {
            return Err(format!("image cycle at {name}"));
        }
        let node = self
            .graph
            .nodes
            .get(name)
            .ok_or_else(|| format!("missing image node {name}"))?
            .clone();
        let args = &node.args;
        let bad = || format!("image {name}: incompatible ports for {}", node.op);
        let id = if node.op == "image" {
            if args.len() != 1 {
                return Err(bad());
            }
            let (value, ty) = self.fields.visit(&args[0])?;
            let key = format!("source:{value}");
            if let Some(&id) = self.common.get(&key) {
                id
            } else {
                if self.plan.sources.len() >= 16 {
                    return Err("POC allows at most 16 rasterized fields".into());
                }
                let kind = match ty {
                    ValueType::Scalar => Kind::Scalar,
                    ValueType::Vector => Kind::Vector,
                    ValueType::Color => Kind::Color,
                };
                let source = self.plan.sources.len();
                fn spatial(graph: &Graph, name: &str) -> bool {
                    let n = &graph.nodes[name];
                    match n.op.as_str() {
                        "scalar" | "param" | "color" | "vector" => false,
                        "source" => n.args[0] != "time",
                        "phase" | "band" | "light" | "flow" => true,
                        _ => n.args.iter().any(|arg| spatial(graph, arg)),
                    }
                }
                let spatial = kind == Kind::Color || spatial(self.graph, &args[0]);
                fn reads_class(graph: &Graph, name: &str) -> bool {
                    let n = &graph.nodes[name];
                    match n.op.as_str() {
                        "scalar" | "param" | "color" | "vector" => false,
                        "source" => {
                            matches!(n.args[0].as_str(), "escaped" | "interior" | "unresolved")
                        }
                        _ => n.args.iter().any(|arg| reads_class(graph, arg)),
                    }
                }
                let masked = spatial && !reads_class(self.graph, &args[0]);
                self.plan.sources.push(Source {
                    label: name.into(),
                    value,
                    kind,
                    spatial,
                    masked,
                });
                let id = self.image(kind, Some(source), self.fields.result.support_pixels);
                self.common.insert(key, id);
                id
            }
        } else {
            let count = match node.op.as_str() {
                "add" | "mul" | "max" | "blur" => 2,
                "mix" | "warp" | "relight" | "stream" | "zoom" => 3,
                _ => return Err(format!("image {name}: unknown image operation {}", node.op)),
            };
            if args.len() != count {
                return Err(bad());
            }
            let mut inputs = [0; 3];
            for (i, arg) in args.iter().enumerate() {
                inputs[i] = self.visit(arg)?;
            }
            if count == 2 {
                inputs[2] = inputs[1];
            }
            let (a, b, d) = (
                self.plan.images[inputs[0]].kind,
                self.plan.images[inputs[1]].kind,
                self.plan.images[inputs[2]].kind,
            );
            let support = inputs
                .iter()
                .map(|&i| self.plan.images[i].support)
                .max()
                .unwrap_or(0);
            let key = format!("{}:{inputs:?}", node.op);
            if let Some(&id) = self.common.get(&key) {
                id
            } else {
                let id = match node.op.as_str() {
                    "add" | "mul" | "max" => {
                        if a != b && a != Kind::Scalar && b != Kind::Scalar {
                            return Err(bad());
                        }
                        let kind = if a == Kind::Scalar { b } else { a };
                        let op = match node.op.as_str() {
                            "add" => 0,
                            "mul" => 1,
                            _ => 2,
                        };
                        self.step(op, inputs, kind, support)
                    }
                    "mix" => {
                        if a != b || d != Kind::Scalar {
                            return Err(bad());
                        }
                        self.step(3, inputs, a, support)
                    }
                    "warp" => {
                        if b != Kind::Vector || d != Kind::Scalar {
                            return Err(bad());
                        }
                        self.step(4, inputs, a, support.saturating_add(9))
                    }
                    "stream" => {
                        if b != Kind::Vector || d != Kind::Scalar {
                            return Err(bad());
                        }
                        // Six midpoint steps in each direction, max 24 px.
                        // Curved paths can sample a different direction field at
                        // every step; this is convolution, never persistent fluid state.
                        self.step(8, inputs, a, support.saturating_add(26))
                    }
                    "zoom" => {
                        if b != Kind::Vector || d != Kind::Scalar {
                            return Err(bad());
                        }
                        // Arbitrary pivot magnification reads the whole image;
                        // u32::MAX is the explicit full-frame support marker.
                        self.step(9, inputs, a, u32::MAX)
                    }
                    "blur" => {
                        if b != Kind::Scalar {
                            return Err(bad());
                        }
                        let horizontal = self.step(5, inputs, a, support.saturating_add(9));
                        self.step(
                            6,
                            [horizontal, inputs[1], inputs[1]],
                            a,
                            support.saturating_add(18),
                        )
                    }
                    "relight" => {
                        if a != Kind::Color || b != Kind::Scalar || d != Kind::Vector {
                            return Err(bad());
                        }
                        self.step(7, inputs, Kind::Color, support.saturating_add(1))
                    }
                    _ => unreachable!(),
                };
                self.common.insert(key, id);
                id
            }
        };
        if self.plan.steps.len() > 128 {
            return Err("at most 128 image passes".into());
        }
        self.visiting.remove(name);
        self.visited.insert(name.into(), id);
        Ok(id)
    }
}
