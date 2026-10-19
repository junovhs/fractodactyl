//! Native Dioxus controls over the ONE persistent WGSL styling implementation.
use base64::Engine;
use dioxus::prelude::*;
use fd_shade::style::{
    render::{columns, StyleRenderer},
    Graph,
};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{mpsc, Arc, Mutex, OnceLock},
    time::Duration,
};

const GRAPH: &str = include_str!("../../../looks/styles/workbench.style");
const PRESETS: &str = include_str!("../../../looks/styles/workbench-presets.json");
static INITIAL: OnceLock<String> = OnceLock::new();

#[derive(Clone, Deserialize)]
struct Preset {
    name: String,
    description: String,
    settings: BTreeMap<String, f32>,
}
fn presets() -> Vec<Preset> {
    serde_json::from_str(PRESETS).expect("embedded styling presets")
}
fn defaults() -> BTreeMap<String, f32> {
    Graph::parse(GRAPH)
        .and_then(|g| g.compile())
        .expect("embedded material graph")
        .parameters
        .into_iter()
        .collect()
}
fn settings(p: &Preset) -> BTreeMap<String, f32> {
    let mut v = defaults();
    v.extend(p.settings.clone());
    v
}
fn export(graph: &str, values: &BTreeMap<String, f32>) -> String {
    graph
        .lines()
        .map(|line| {
            if let Some((name, rhs)) = line.split_once(" = param ") {
                format!(
                    "{name} = param {}",
                    values
                        .get(name)
                        .map_or_else(|| rhs.to_string(), ToString::to_string)
                )
            } else {
                line.into()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

#[derive(Clone, Copy)]
struct Control {
    name: &'static str,
    label: &'static str,
    group: &'static str,
    min: f32,
    max: f32,
    step: f32,
}
const CONTROLS: &[Control] = &[
    Control {
        name: "fold_frequency",
        label: "Fold frequency",
        group: "Shape",
        min: 0.01,
        max: 0.7,
        step: 0.005,
    },
    Control {
        name: "fold_depth",
        label: "Fold depth",
        group: "Shape",
        min: 0.,
        max: 8.,
        step: 0.05,
    },
    Control {
        name: "detail_frequency",
        label: "Crease frequency",
        group: "Shape",
        min: 0.05,
        max: 3.,
        step: 0.025,
    },
    Control {
        name: "detail_strength",
        label: "Crease strength",
        group: "Shape",
        min: 0.,
        max: 2.,
        step: 0.025,
    },
    Control {
        name: "phase",
        label: "Fold offset",
        group: "Shape",
        min: 0.,
        max: 1.,
        step: 0.005,
    },
    Control {
        name: "cavity",
        label: "Groove darkness",
        group: "Shape",
        min: 0.,
        max: 1.,
        step: 0.01,
    },
    Control {
        name: "light_angle",
        label: "Light direction",
        group: "Light & material",
        min: -3.141593,
        max: 3.141593,
        step: 0.025,
    },
    Control {
        name: "light_height",
        label: "Light elevation",
        group: "Light & material",
        min: 0.05,
        max: 3.,
        step: 0.025,
    },
    Control {
        name: "ambient",
        label: "Shadow fill",
        group: "Light & material",
        min: 0.,
        max: 0.5,
        step: 0.01,
    },
    Control {
        name: "surface_gain",
        label: "Surface brightness",
        group: "Light & material",
        min: 0.,
        max: 3.,
        step: 0.025,
    },
    Control {
        name: "gloss",
        label: "Highlight tightness",
        group: "Light & material",
        min: 2.,
        max: 96.,
        step: 1.,
    },
    Control {
        name: "metal",
        label: "Surface sheen",
        group: "Light & material",
        min: 0.,
        max: 3.,
        step: 0.025,
    },
    Control {
        name: "cool_hue",
        label: "Blue → violet",
        group: "Color & emission",
        min: 0.,
        max: 1.,
        step: 0.01,
    },
    Control {
        name: "warm_hue",
        label: "Gold → rose",
        group: "Color & emission",
        min: 0.,
        max: 1.,
        step: 0.01,
    },
    Control {
        name: "cool_gain",
        label: "Cool backlight",
        group: "Color & emission",
        min: 0.,
        max: 4.,
        step: 0.025,
    },
    Control {
        name: "warm_gain",
        label: "Warm edge light",
        group: "Color & emission",
        min: 0.,
        max: 5.,
        step: 0.025,
    },
    Control {
        name: "edge_width",
        label: "Warm edge width · px",
        group: "Color & emission",
        min: 0.02,
        max: 3.,
        step: 0.02,
    },
    Control {
        name: "atmosphere_width",
        label: "Backlight reach · px",
        group: "Mist",
        min: 1.,
        max: 150.,
        step: 1.,
    },
    Control {
        name: "mist_radius",
        label: "Curved spread · px",
        group: "Mist",
        min: 0.,
        max: 24.,
        step: 0.25,
    },
    Control {
        name: "mist_gain",
        label: "Mist brightness",
        group: "Mist",
        min: 0.,
        max: 3.,
        step: 0.025,
    },
    Control {
        name: "exposure",
        label: "Exposure",
        group: "Finish",
        min: 0.1,
        max: 4.,
        step: 0.025,
    },
];

#[derive(Clone)]
struct Request {
    id: u64,
    source: String,
    graph: String,
    values: BTreeMap<String, f32>,
}
struct Reply {
    id: u64,
    result: Result<(Vec<u8>, String), String>,
}
#[derive(Clone)]
struct Worker {
    send: mpsc::Sender<Request>,
    receive: Arc<Mutex<mpsc::Receiver<Reply>>>,
}
struct Controller {
    source: String,
    graph: String,
    renderer: Option<StyleRenderer>,
}
impl Controller {
    fn new() -> Self {
        Self {
            source: String::new(),
            graph: String::new(),
            renderer: None,
        }
    }
    fn frame(
        &mut self,
        source: &str,
        graph: &str,
        values: &BTreeMap<String, f32>,
    ) -> Result<(fd_shade::Rgb8, String), String> {
        if self.renderer.is_none() || self.source != source || self.graph != graph {
            let compiled = Graph::parse(graph)?.compile()?;
            let mut reader = fd_samples::Reader::open(std::path::Path::new(source))
                .map_err(|e| e.to_string())?;
            let samples = reader.read(columns(&compiled)).map_err(|e| e.to_string())?;
            self.renderer = Some(StyleRenderer::new(compiled, &reader.header, &samples)?);
            self.source = source.into();
            self.graph = graph.into();
        }
        let renderer = self.renderer.as_mut().unwrap();
        for (name, value) in values {
            renderer.set_parameter(name, *value)?;
        }
        let image = renderer.frame(0., true)?;
        let s = &renderer.stats;
        let text = format!(
            "{} × {} · {:.1} ms shading · {} passes · {:.1} MiB buffers · {} sample upload",
            image.w,
            image.h,
            s.frame_seconds * 1000.,
            s.dispatches,
            s.working_bytes as f64 / 1048576.,
            s.sample_uploads
        );
        Ok((image, text))
    }
}
impl Worker {
    fn new() -> Self {
        let (send, receiver) = mpsc::channel::<Request>();
        let (results, receive) = mpsc::channel();
        std::thread::spawn(move || {
            let mut controller = Controller::new();
            while let Ok(mut request) = receiver.recv() {
                // Coalesce drag events. Only one GPU evaluation is in flight.
                while let Ok(newer) = receiver.try_recv() {
                    request = newer;
                }
                let result = controller
                    .frame(&request.source, &request.graph, &request.values)
                    .map(|(image, stats)| (fd_shade::png(&image), stats));
                if results
                    .send(Reply {
                        id: request.id,
                        result,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            send,
            receive: Arc::new(Mutex::new(receive)),
        }
    }
}

/// Launch the native material workspace, or verify its shared renderer without a window.
pub(crate) fn run(
    source: Option<PathBuf>,
    check: bool,
    output: Option<PathBuf>,
) -> Result<(), String> {
    if check {
        return check_workspace(
            source.ok_or("--style --check needs --samples INPUT.fds")?,
            output,
        );
    }
    if output.is_some() {
        return Err("--style-output requires --check".into());
    }
    let _ = INITIAL.set(
        source
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default(),
    );
    let window = dioxus::desktop::WindowBuilder::new()
        .with_title("Fractodactyl · Styling")
        .with_inner_size(dioxus::desktop::LogicalSize::new(1440., 960.))
        .with_min_inner_size(dioxus::desktop::LogicalSize::new(940., 640.));
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            dioxus::desktop::Config::new()
                .with_window(window)
                .with_menu(None),
        )
        .launch(Workspace);
    Ok(())
}

#[component]
fn Workspace() -> Element {
    let worker = use_hook(Worker::new);
    let mut source = use_signal(|| INITIAL.get().cloned().unwrap_or_default());
    let mut values = use_signal(defaults);
    let mut graph = use_signal(|| GRAPH.to_string());
    let mut selected = use_signal(|| 0usize);
    let mut image = use_signal(String::new);
    let mut pinned = use_signal(String::new);
    let mut compare = use_signal(|| false);
    let mut status = use_signal(|| "Choose a sample file to start.".to_string());
    let mut request_id = use_signal(|| 0u64);
    let mut finished_id = use_signal(|| 0u64);
    let mut error = use_signal(String::new);
    let mut note = use_signal(String::new);
    let sender = worker.send.clone();
    use_effect(move || {
        let path = source();
        let controls = values();
        let program = graph();
        if path.is_empty() {
            return;
        }
        let id = *request_id.peek() + 1;
        request_id.set(id);
        error.set(String::new());
        if sender
            .send(Request {
                id,
                source: path,
                graph: program,
                values: controls,
            })
            .is_err()
        {
            error.set("The styling worker stopped. Reopen the styling window.".into());
            finished_id.set(id);
        }
    });
    let receive = worker.receive.clone();
    use_future(move || {
        let receive = receive.clone();
        async move {
            loop {
                let mut replies = Vec::new();
                if let Ok(lock) = receive.lock() {
                    while let Ok(reply) = lock.try_recv() {
                        replies.push(reply);
                    }
                }
                for reply in replies {
                    if reply.id != *request_id.peek() {
                        continue;
                    }
                    finished_id.set(reply.id);
                    match reply.result {
                        Ok((png, text)) => {
                            image.set(format!(
                                "data:image/png;base64,{}",
                                base64::engine::general_purpose::STANDARD.encode(png)
                            ));
                            status.set(text);
                            error.set(String::new());
                        }
                        Err(e) => error.set(e),
                    }
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        }
    });
    let choices = presets();
    let description = choices[selected()].description.clone();
    let showing = if compare() && !pinned().is_empty() {
        pinned()
    } else {
        image()
    };
    let busy = request_id() != finished_id();
    rsx! {
        style { {include_str!("style_workspace.css")} }
        main { class: "styling",
            header {
                div { class:"brand", span { "FRACTODACTYL" } h1 { "Styling" } }
                div { class:"actions",
                    button { onclick: move |_| async move {
                        if let Some(file) = rfd::AsyncFileDialog::new().add_filter("Fractal samples", &["fds"]).pick_file().await {
                            source.set(file.path().to_string_lossy().into_owned()); pinned.set(String::new()); compare.set(false);
                        }
                    }, "Open samples…" }
                    button { disabled: image().is_empty() || busy, onclick: move |_| { pinned.set(image()); compare.set(false); note.set("Reference pinned. Adjust controls, then hold Compare.".into()); }, "Pin reference" }
                    button { disabled: pinned().is_empty(), onpointerdown: move |_|compare.set(true), onpointerup: move |_|compare.set(false), onpointerleave: move |_|compare.set(false), onpointercancel:move |_|compare.set(false), onkeydown:move |e| {if e.key()==Key::Enter || e.key()==Key::Character(" ".into()){compare.set(true)}}, onkeyup:move |_|compare.set(false), "Hold to compare" }
                    button { class:"primary", onclick: move |_| async move {
                        let snapshot = export(&graph(), &values());
                        if let Some(file) = rfd::AsyncFileDialog::new().add_filter("Style graph", &["style"]).set_file_name("my-look.style").save_file().await {
                            match std::fs::write(file.path(),snapshot) {
                                Ok(()) => note.set(format!("Saved {}", file.path().display())),
                                Err(e) => error.set(format!("Could not save look: {e}")),
                            }
                        }
                    }, "Save look…" }
                    button { onclick: move |_| async move {
                        if let Some(file) = rfd::AsyncFileDialog::new().add_filter("Workspace look", &["style"]).pick_file().await {
                            let loaded = std::fs::read_to_string(file.path()).map_err(|e|e.to_string()).and_then(|text| {
                                let compiled = Graph::parse(&text)?.compile()?;
                                let imported = compiled.parameters.into_iter().collect::<BTreeMap<_,_>>();
                                if imported.keys().ne(defaults().keys()) {
                                    return Err("Open a look saved by this styling workspace. Other graphs can render with fd style render.".to_string());
                                }
                                Ok((text,imported))
                            });
                            match loaded { Ok((text,imported))=>{ graph.set(text);values.set(imported);note.set(format!("Opened {}",file.path().display())); }, Err(e)=>error.set(e) }
                        }
                    }, "Open look…" }
                    button { disabled:image().is_empty() || busy, onclick: move |_| async move {
                        let snapshot = image();
                        if let Some(file) = rfd::AsyncFileDialog::new().add_filter("PNG image", &["png"]).set_file_name("my-look.png").save_file().await {
                            let saved = snapshot.split_once(',').ok_or("No image available".to_string())
                                .and_then(|(_,data)|base64::engine::general_purpose::STANDARD.decode(data).map_err(|e|e.to_string()))
                                .and_then(|bytes|std::fs::write(file.path(),bytes).map_err(|e|e.to_string()));
                            match saved { Ok(())=>note.set(format!("Saved {}",file.path().display())),Err(e)=>error.set(e) }
                        }
                    }, "Save PNG…" }
                }
            }
            section { class:"workspace",
                div { class:"stage",
                    div { class:"stage-top", span { "MATERIAL STUDY" } span { if compare() { "PINNED REFERENCE" } else if !error().is_empty() { "LAST VALID PREVIEW" } else if busy { "UPDATING…" } else { "LIVE LOOK" } } }
                    div { class:"image-area",
                        if showing.is_empty() {
                            div { class:"empty", h2 { "Give the light something to follow." } p { "Open a .fds sample file, choose a material, and explore its shape and light." } }
                        } else {
                            img { src: showing, alt:"Fractal styled by the current graph settings", draggable:"false" }
                        }
                    }
                    div { class:"status", role:"status", "{status}" }
                    if !error().is_empty() { div { class:"error", role:"alert", "{error}" } }
                    if !note().is_empty() { div { class:"note", role:"status", "{note}" } }
                    div { class:"source", "{source}" }
                }
                aside {
                    div { class:"preset",
                        label { r#for:"preset", "STARTING POINT" }
                        select { id:"preset", value:"{selected}", onchange: move |e| {
                            if let Ok(index) = e.value().parse::<usize>() {
                                if let Some(p) = presets().get(index) { selected.set(index);graph.set(GRAPH.into());values.set(settings(p));note.set(String::new()); }
                            }
                        },
                            for (index,preset) in choices.iter().enumerate() { option { value:"{index}", "{preset.name}" } }
                        }
                        p { "{description}" }
                        button { class:"reset", onclick: move |_| {graph.set(GRAPH.into());values.set(settings(&presets()[selected()]));}, "Reset this preset" }
                    }
                    for group in ["Shape","Light & material","Color & emission","Mist","Finish"] {
                        details { open:true,
                            summary { "{group}" }
                            if group=="Mist" { p { class:"hint", "Mist spreads the cool backlight along curves. Raise Cool backlight to give it a source." } }
                            for control in CONTROLS.iter().filter(|c|c.group==group).copied() {
                                div { class:"control",
                                    label { r#for:control.name, "{control.label}" }
                                    output { "{values().get(control.name).copied().unwrap_or_default():.3}" }
                                    input { id:control.name, r#type:"range", min:"{control.min}",max:"{control.max}",step:"{control.step}",value:"{values().get(control.name).copied().unwrap_or_default()}",
                                        oninput:move |e| { if let Ok(value)=e.value().parse::<f32>() { values.write().insert(control.name.into(),value.clamp(control.min,control.max)); } }
                                    }
                                }
                            }
                        }
                    }
                    p { class:"hint", "Saved looks are ordinary .style graphs and also render with fd style render." }
                }
            }
        }
    }
}

fn check_workspace(source: PathBuf, output: Option<PathBuf>) -> Result<(), String> {
    let out = output.unwrap_or_else(|| std::env::temp_dir().join("fd-style-check"));
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let path = source.to_str().ok_or("sample path must be UTF-8")?;
    let mut controller = Controller::new();
    let before = std::fs::read(&source).map_err(|e| e.to_string())?;
    let mut reader = fd_samples::Reader::open(&source).map_err(|e| e.to_string())?;
    let classes = reader
        .read(fd_samples::ColumnSet::of(&[fd_samples::Column::Class]))
        .map_err(|e| e.to_string())?;
    let (width, height) = reader.header.pixels();
    let ss = reader.header.ss as usize;
    let nx = reader.header.nx as usize;
    let black_pixels: Vec<usize> = (0..width * height)
        .filter(|&i| {
            let x = i % width;
            let y = i / width;
            (0..ss).all(|dy| {
                (0..ss).all(|dx| !classes.class[(y * ss + dy) * nx + x * ss + dx].escaped())
            })
        })
        .collect();
    let mut images = Vec::new();
    for (index, p) in presets().iter().enumerate() {
        let values = settings(p);
        let (image, stats) = controller.frame(path, GRAPH, &values)?;
        if black_pixels
            .iter()
            .any(|&i| image.data[i * 3..i * 3 + 3] != [0, 0, 0])
        {
            return Err("transport illuminated a fully masked pixel".into());
        }
        let png = fd_shade::png(&image);
        if images.contains(&png) {
            return Err(format!("preset {} duplicates a previous image", p.name));
        }
        let replay = controller.frame(path, GRAPH, &values)?.0;
        if png != fd_shade::png(&replay) {
            return Err("preset replay changed pixels".into());
        }
        std::fs::write(out.join(format!("preset-{index}.png")), &png).map_err(|e| e.to_string())?;
        let saved = export(GRAPH, &values);
        let compiled = Graph::parse(&saved)?.compile()?;
        if compiled.parameters.into_iter().collect::<BTreeMap<_, _>>() != values {
            return Err("saved look changed parameters".into());
        }
        std::fs::write(out.join(format!("preset-{index}.style")), saved)
            .map_err(|e| e.to_string())?;
        println!(
            "style_preset {index} {:?} {stats} replay_equal true",
            p.name
        );
        images.push(png);
    }
    let base = defaults();
    let reference = fd_shade::png(&controller.frame(path, GRAPH, &base)?.0);
    let stats = &controller.renderer.as_ref().unwrap().stats;
    let resources = (stats.allocations, stats.pipelines, stats.sample_uploads);
    for control in CONTROLS {
        let mut values = base.clone();
        let current = values[control.name];
        values.insert(
            control.name.into(),
            if current > (control.min + control.max) * 0.5 {
                control.min
            } else {
                control.max
            },
        );
        let image = controller.frame(path, GRAPH, &values)?.0;
        if fd_shade::png(&image) == reference {
            return Err(format!("slider {} has no visible effect", control.name));
        }
        let stats = &controller.renderer.as_ref().unwrap().stats;
        if (stats.allocations, stats.pipelines, stats.sample_uploads) != resources {
            return Err("slider change rebuilt GPU resources".into());
        }
        println!(
            "style_control {} visible_change true resource_reuse true",
            control.name
        );
    }
    // Opening a saved look must preserve its graph, not silently substitute the
    // current bundled material. This also keeps earlier workspace exports usable.
    let custom = GRAPH.replace("cyan = color #139eea", "cyan = color #ff2020");
    let custom_image = fd_shade::png(&controller.frame(path, &custom, &base)?.0);
    if custom_image == reference {
        return Err("imported graph was ignored".into());
    }
    let reopened = export(&custom, &base);
    if fd_shade::png(&controller.frame(path, &reopened, &base)?.0) != custom_image {
        return Err("saved graph changed its appearance when reopened".into());
    }
    println!("style_saved_graph appearance_replay true");
    if std::fs::read(&source).map_err(|e| e.to_string())? != before {
        return Err("styling changed samples".into());
    }
    println!("style_workspace_check presets {} controls {} sample_uploads {} protected_black_pixels {} unchanged_input true output {}",presets().len(),CONTROLS.len(),resources.2,black_pixels.len(),out.display());
    Ok(())
}
