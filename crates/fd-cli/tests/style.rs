use std::process::Command;

// GPU fixtures own device lifecycles; keep their setup/teardown from overlapping.
static GPU_FIXTURES: std::sync::Mutex<()> = std::sync::Mutex::new(());

const IMAGE_GRAPH:&str="version 1\noutput final\npos = source position\nr = length pos\nfreq = param 0.7\nx = mul r freq\ntime = source time\nt = add x time\ny = sin t\nhalf = scalar 0.5\ny0 = mul y half\nwave = add y0 half\na = color #103050\nb = color #ef9540\ncolor = mix a b wave\nheight_scale = param 8\nheight = mul wave height_scale\ntangent = source tangent\nsx = scalar -1\nsy = scalar -0.4\nsun = compose sx sy\ndistance = param 6\nradius = param 2\nbase = image color\nheights = image height\nvectors = image tangent\nlight = image sun\namount = image distance\nwidth = image radius\nwarped = warp base vectors amount\nlit = relight warped heights light\nblurred = blur lit width\nalias = blur lit width\nfinal = add blurred alias\n";

#[test]
fn image_graph_connections_control_order_modulation_and_work_sharing() {
    let _gpu_fixture = GPU_FIXTURES.lock().unwrap();
    use fd_shade::style::{render::StyleRenderer, Graph};
    let compiled = Graph::parse(IMAGE_GRAPH).unwrap().compile().unwrap();
    let fused = IMAGE_GRAPH.replace(
        "final = add blurred alias",
        "scaled = mul blurred amount\ncombined = add scaled alias\nfinal = add combined base",
    );
    let fused = Graph::parse(&fused).unwrap().compile().unwrap();
    let fused = fused.images.as_ref().unwrap();
    assert_eq!(
        fused.steps.len(),
        5,
        "three pointwise image operations fuse into one pass"
    );
    assert_eq!(
        fused
            .steps
            .last()
            .unwrap()
            .fused
            .as_ref()
            .unwrap()
            .inputs
            .len(),
        3
    );
    let mut wide = String::from("version 1\noutput sum8\n");
    for i in 0..9 {
        wide += &format!(
            "color{i} = color #{:06x}\nimage{i} = image color{i}\n",
            0x103050 + i * 0x050301
        );
    }
    wide += "sum1 = add image0 image1\n";
    for i in 2..9 {
        wide += &format!("sum{i} = add sum{} image{i}\n", i - 1);
    }
    let wide = Graph::parse(&wide).unwrap().compile().unwrap();
    assert_eq!(wide.images.as_ref().unwrap().steps.len(), 3);
    assert!(
        wide.images
            .as_ref()
            .unwrap()
            .steps
            .iter()
            .all(|p| p.dependencies().len() <= 7),
        "fusion respects portable storage binding limits"
    );
    let plan = compiled.images.as_ref().unwrap();
    assert_eq!(
        plan.steps.len(),
        5,
        "shared blur must execute once, as two passes"
    );
    assert_eq!(plan.slots, 2, "linear image chain uses two ping-pong slots");
    assert_eq!(
        plan.sources.iter().filter(|s| !s.spatial).count(),
        3,
        "sun/distance/radius are compact global images"
    );
    for step in &plan.steps {
        for &input in step.dependencies() {
            if plan.images[input].source.is_none() {
                assert_ne!(
                    plan.images[input].slot, plan.images[step.output].slot,
                    "no in-place read/write aliases"
                );
            }
        }
    }
    let reversed = IMAGE_GRAPH.lines().rev().collect::<Vec<_>>().join("\n");
    assert_eq!(
        compiled.report(),
        Graph::parse(&reversed).unwrap().compile().unwrap().report(),
        "source-line order is immaterial"
    );
    let broken = IMAGE_GRAPH.replace("warp base vectors amount", "warp base base amount");
    assert!(Graph::parse(&broken)
        .unwrap()
        .compile()
        .unwrap_err()
        .contains("incompatible ports"));
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root.join("crates/fd-shade/src/gpu/frozen/fixture.fds");
    let mut reader = fd_samples::Reader::open(&path).unwrap();
    let samples = reader.read(reader.header.columns).unwrap();
    let Some(mut renderer) = fd_shade::gpu::testkit::or_skip(
        StyleRenderer::new(compiled, &reader.header, &samples),
        "image graph",
    ) else {
        return;
    };
    let first = renderer.frame(0.0, true).unwrap();
    let classified="version 1\noutput image\nmask = source interior\ninside = color #ff0000\noutside = color #000080\ncolor = mix outside inside mask\nimage = image color\n";
    let mut single = reader.header.clone();
    single.ss = 1;
    let mut classification = StyleRenderer::new(
        Graph::parse(classified).unwrap().compile().unwrap(),
        &single,
        &samples,
    )
    .unwrap();
    let classified = classification.frame(0.0, true).unwrap();
    assert!(
        classified.data[0] > classified.data[2],
        "explicit interior mask can color the interior"
    );
    assert!(
        classified.data[2 * 3 + 2] > classified.data[2 * 3],
        "escaped region uses its separate color"
    );
    assert_eq!(renderer.stats.dispatches, 7);
    let resources = (
        renderer.stats.pipelines,
        renderer.stats.allocations,
        renderer.stats.working_bytes,
    );
    assert_eq!(first.data, renderer.frame(0.0, true).unwrap().data);
    assert_eq!(
        renderer.stats.dispatches, 1,
        "unchanged graph reuses all computed images"
    );
    renderer.set_parameter("radius", 6.0).unwrap();
    let changed = renderer.frame(0.0, true).unwrap();
    assert_ne!(first.data, changed.data);
    assert_eq!(
        resources,
        (
            renderer.stats.pipelines,
            renderer.stats.allocations,
            renderer.stats.working_bytes
        )
    );
    let other = IMAGE_GRAPH.replace(
        "warped = warp base vectors amount\nlit = relight warped heights light",
        "prelit = relight base heights light\nlit = warp prelit vectors amount",
    );
    let mut other = StyleRenderer::new(
        Graph::parse(&other).unwrap().compile().unwrap(),
        &reader.header,
        &samples,
    )
    .unwrap();
    assert_ne!(
        first.data,
        other.frame(0.0, true).unwrap().data,
        "warp then light differs from light then warp"
    );
    let dir = std::env::temp_dir().join(format!("fd-image-graph-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let graph = dir.join("image.style");
    let output = dir.join("image.png");
    std::fs::write(&graph, IMAGE_GRAPH).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_fd"))
        .args([
            "style",
            "render",
            graph.to_str().unwrap(),
            path.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--unresolved",
            "interior",
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(std::fs::read(output).unwrap(), fd_shade::png(&first));
    std::fs::remove_dir_all(dir).unwrap();
}

fn inspect(text: &str, name: &str) -> std::process::Output {
    let path = std::env::temp_dir().join(format!("fd-style-{}-{name}.style", std::process::id()));
    std::fs::write(&path, text).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_fd"))
        .args(["style", "inspect", path.to_str().unwrap()])
        .output()
        .unwrap();
    std::fs::remove_file(path).unwrap();
    result
}

#[test]
fn style_inspect_compiles_shared_reachable_work_and_rejects_invalid_graphs() {
    let graph="version 1\nsurface a\nemission b\nx = color #102030\ny = color #102030\na = add x y\nb = add x y\ndead = nonsense missing\n";
    let a = inspect(graph, "first");
    let b = inspect(graph, "second");
    assert!(a.status.success(), "{}", String::from_utf8_lossy(&a.stderr));
    assert_eq!(a.stdout, b.stdout);
    let report = String::from_utf8(a.stdout).unwrap();
    assert!(
        report.contains("reachable_nodes 4 unique_operations 2 pointwise_passes 1"),
        "{report}"
    );
    for (name, bad, expected) in [
        (
            "type",
            "version 1\nsurface a\nemission a\na = scalar 2\n",
            "must be colors",
        ),
        (
            "cycle",
            "version 1\nsurface a\nemission a\na = add b b\nb = add a a\n",
            "cycle",
        ),
        (
            "missing",
            "version 1\nsurface absent\nemission absent\n",
            "missing node",
        ),
        (
            "mixed",
            "version 1\nsurface a\nemission a\nx = color #000000\ny = vector 1 2\na = mul x y\n",
            "invalid inputs",
        ),
    ] {
        let out = inspect(bad, name);
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains(expected));
    }
}

#[test]
fn style_renderer_and_cli_reuse_samples_pipelines_and_hdr() {
    let _gpu_fixture = GPU_FIXTURES.lock().unwrap();
    use fd_shade::style::{render::StyleRenderer, Graph};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = root.join("crates/fd-shade/src/gpu/frozen/fixture.fds");
    let before = std::fs::read(&input).unwrap();
    let mut reader = fd_samples::Reader::open(&input).unwrap();
    let samples = reader.read(reader.header.columns).unwrap();
    let text="version 1\nsurface lit\nemission glow\nbase = color #102040\ngold = color #ffd080\nnormal = source normal\nslope = param 1.2\ndirection = vector -1 -1\nlight = light normal slope direction\nlit = mul base light\nwidth = param 0.1\nd = source de\nedge = boundary d width\nstrength = param 5\nenergy = mul edge strength\nglow = mul gold energy\n";
    let Some(mut renderer) = fd_shade::gpu::testkit::or_skip(
        StyleRenderer::new(
            Graph::parse(text).unwrap().compile().unwrap(),
            &reader.header,
            &samples,
        ),
        "style renderer",
    ) else {
        return;
    };
    let a = renderer.frame(0.0, false).unwrap();
    assert_eq!(renderer.stats.dispatches, 2);
    let b = renderer.frame(1.0, false).unwrap();
    assert_eq!(
        renderer.stats.dispatches, 1,
        "static HDR must not be recomputed"
    );
    assert_eq!(a.data, b.data);
    assert!(a.data.iter().any(|&v| v > 0));
    assert!(a.data.windows(2).any(|v| v[0] != v[1]));
    renderer.set_parameter("strength", 0.0).unwrap();
    let c = renderer.frame(1.0, false).unwrap();
    assert_eq!(renderer.stats.dispatches, 2);
    assert_ne!(a.data, c.data);
    assert_eq!(
        (
            renderer.stats.pipelines,
            renderer.stats.allocations,
            renderer.stats.sample_uploads
        ),
        (2, 6, 1)
    );
    let mut missing = samples.clone();
    missing.de = None;
    assert!(StyleRenderer::new(
        Graph::parse(text).unwrap().compile().unwrap(),
        &reader.header,
        &missing
    )
    .err()
    .unwrap()
    .contains("De"));
    let dir = std::env::temp_dir().join(format!("fd-style-render-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let graph = dir.join("test.style");
    let png = dir.join("test.png");
    std::fs::write(&graph, text).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_fd"))
        .args([
            "style",
            "render",
            graph.to_str().unwrap(),
            input.to_str().unwrap(),
            "-o",
            png.to_str().unwrap(),
            "--runs",
            "2",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report = String::from_utf8(out.stdout).unwrap();
    assert!(
        report.contains("dispatches 1 pipelines 2 allocations 6 sample_uploads 1"),
        "{report}"
    );
    assert_eq!(std::fs::read(&png).unwrap(), fd_shade::png(&a));
    assert_eq!(std::fs::read(&input).unwrap(), before);
    std::fs::remove_dir_all(dir).unwrap();

    // Precise phase and dynamic modulation use the same generated GPU route.
    let animated="version 1\nsurface rgb\nemission black\nfreq = param 0.05\nphase = phase freq\ntime = source time\nsignal = add phase time\nwave = cos signal\na = color #000020\nb = color #20d0ff\nrgb = mix a b wave\nblack = color #000000\n";
    let mut renderer = StyleRenderer::new(
        Graph::parse(animated).unwrap().compile().unwrap(),
        &reader.header,
        &samples,
    )
    .unwrap();
    let a = renderer.frame(0.0, true).unwrap();
    let b = renderer.frame(0.5, true).unwrap();
    assert_eq!(renderer.stats.dispatches, 2);
    assert_ne!(a.data, b.data);
    assert_eq!(a.data, renderer.frame(0.0, true).unwrap().data);
}

#[test]
fn style_finishing_has_bounded_bloom_and_reproducible_scaled_grain() {
    let _gpu_fixture = GPU_FIXTURES.lock().unwrap();
    use fd_samples::{Column, ColumnSet, Header, Samples, View, MINOR};
    use fd_shade::style::{finishing::Finishing, render::StyleRenderer, Graph};
    let n = 128;
    let columns = ColumnSet::of(&[Column::Class, Column::De]);
    let mut samples = Samples::alloc(n * n, columns);
    samples.de.as_mut().unwrap().fill(1000.0);
    samples.de.as_mut().unwrap()[64 * n + 64] = 0.0;
    let h = Header {
        minor: MINOR,
        columns,
        nx: n as u32,
        ny: n as u32,
        ss: 1,
        max_iter: 100,
        escape_radius: 1e10,
        view: View {
            center_re: "0".into(),
            center_im: "0".into(),
            width: "4".into(),
            rotation: 0.0,
        },
        kernel: "synthetic finishing fixture".into(),
    };
    let impulse="version 1\nsurface black\nemission emission\nblack = color #000000\nwhite = color #ffffff\nd = source de\nw = scalar 0.1\nedge = boundary d w\nenergy = scalar 64\nhot = mul edge energy\nemission = mul white hot\n";
    let Some(mut r) = fd_shade::gpu::testkit::or_skip(
        StyleRenderer::new(
            Graph::parse(impulse).unwrap().compile().unwrap(),
            &h,
            &samples,
        ),
        "style finishing",
    ) else {
        return;
    };
    let base = r.frame(0.0, false).unwrap();
    let options = Finishing {
        bloom: 8.0,
        bloom_radius: 8,
        ..Default::default()
    };
    r.set_finishing(&options).unwrap();
    let bloom = r.frame(0.0, false).unwrap();
    assert_eq!(
        r.stats.dispatches, 4,
        "cached HDR plus extraction/H/V/finish"
    );
    assert!(bloom.data[(64 * n + 68) * 3] > base.data[(64 * n + 68) * 3]);
    let support = r.stats.bloom_support_pixels as usize;
    for y in 0..n {
        for x in 0..n {
            if x.abs_diff(64) > support || y.abs_diff(64) > support {
                assert_eq!(
                    bloom.data[(y * n + x) * 3],
                    0,
                    "outside declared support ({x},{y})"
                );
            }
        }
    }
    let resources = (
        r.stats.pipelines,
        r.stats.allocations,
        r.stats.working_bytes,
    );
    r.frame(1.0, false).unwrap();
    assert_eq!(r.stats.dispatches, 1);
    r.set_finishing(&Finishing::default()).unwrap();
    assert_eq!(
        base.data,
        r.frame(0.0, false).unwrap().data,
        "disabled effects preserve bytes"
    );
    r.set_finishing(&options).unwrap();
    r.frame(0.0, false).unwrap();
    assert_eq!(
        resources,
        (
            r.stats.pipelines,
            r.stats.allocations,
            r.stats.working_bytes
        )
    );

    let flat =
        "version 1\nsurface gray\nemission black\ngray = color #808080\nblack = color #000000\n";
    let mut r =
        StyleRenderer::new(Graph::parse(flat).unwrap().compile().unwrap(), &h, &samples).unwrap();
    let plain = r.frame(0.0, false).unwrap();
    let mut options = Finishing {
        grain: 0.04,
        grain_size: 1.0,
        seed: 19,
        ..Default::default()
    };
    r.set_finishing(&options).unwrap();
    let a = r.frame(0.0, false).unwrap();
    assert_ne!(plain.data, a.data);
    assert_eq!(
        a.data,
        r.frame(3.0, false).unwrap().data,
        "same exposure replays"
    );
    options.exposure = 1;
    r.set_finishing(&options).unwrap();
    let b = r.frame(0.0, false).unwrap();
    assert_ne!(a.data, b.data);
    let differences: Vec<_> = a
        .data
        .iter()
        .zip(&plain.data)
        .map(|(&a, &b)| a as f64 - b as f64)
        .collect();
    let mean = differences.iter().sum::<f64>() / differences.len() as f64;
    let variance =
        differences.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / differences.len() as f64;
    assert!(mean.abs() < 0.7, "grain bias {mean}");
    assert!(variance > 20.0, "grain variance {variance}");
    let energy = |image: &fd_shade::Rgb8| -> u64 {
        image
            .data
            .chunks_exact(3)
            .enumerate()
            .filter(|(i, _)| i % n > 0)
            .map(|(i, p)| p[0].abs_diff(image.data[(i - 1) * 3]) as u64)
            .sum()
    };
    options.grain_size = 3.0;
    r.set_finishing(&options).unwrap();
    let coarse = r.frame(0.0, false).unwrap();
    assert!(
        energy(&coarse) < energy(&b),
        "larger grain must be more spatially correlated"
    );
    options.grain_tonal = [0.0; 3];
    r.set_finishing(&options).unwrap();
    assert_eq!(plain.data, r.frame(0.0, false).unwrap().data);
    options.grain = f32::NAN;
    assert!(r.set_finishing(&options).is_err());

    // Drive the actual user command with synthetic fields, not only its Rust API.
    let dir = std::env::temp_dir().join(format!("fd-style-finishing-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("impulse.fds");
    let graph = dir.join("impulse.style");
    let output = dir.join("bloom.png");
    fd_samples::write(std::fs::File::create(&input).unwrap(), &h, &samples).unwrap();
    std::fs::write(&graph, impulse).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_fd"))
        .args([
            "style",
            "render",
            graph.to_str().unwrap(),
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--bloom",
            "8",
            "--bloom-radius",
            "8",
            "--runs",
            "2",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(std::fs::read(&output).unwrap(), fd_shade::png(&bloom));
    assert!(String::from_utf8_lossy(&out.stdout).contains("bloom_support_px 20"));
    std::fs::write(&graph, flat).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_fd"))
        .args([
            "style",
            "render",
            graph.to_str().unwrap(),
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--grain",
            "0.04",
            "--grain-seed",
            "19",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(std::fs::read(&output).unwrap(), fd_shade::png(&a));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn style_saved_graphs_and_sequence_are_composable_and_replayable() {
    let _gpu_fixture = GPU_FIXTURES.lock().unwrap();
    use fd_shade::style::{render::StyleRenderer, Graph};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = root.join("crates/fd-shade/src/gpu/frozen/fixture.fds");
    let mut reader = fd_samples::Reader::open(&input).unwrap();
    let samples = reader.read(reader.header.columns).unwrap();
    let path = root.join("looks/styles/celestial.style");
    let text = std::fs::read_to_string(&path).unwrap();
    let compiled = Graph::parse(&text).unwrap().compile().unwrap();
    assert_eq!(
        (compiled.support_pixels, compiled.evaluations_per_sample),
        (9, 5)
    );
    let Some(mut renderer) = fd_shade::gpu::testkit::or_skip(
        StyleRenderer::new(compiled, &reader.header, &samples),
        "celestial style",
    ) else {
        return;
    };
    let celestial = renderer.frame(0.0, true).unwrap();
    let copper = std::fs::read_to_string(root.join("looks/styles/etched-copper.style")).unwrap();
    let mut other = StyleRenderer::new(
        Graph::parse(&copper).unwrap().compile().unwrap(),
        &reader.header,
        &samples,
    )
    .unwrap();
    assert_ne!(celestial.data, other.frame(0.0, true).unwrap().data);
    renderer.set_parameter("flow_length", 0.0).unwrap();
    let noflow = renderer.frame(0.0, true).unwrap();
    assert_ne!(celestial.data, noflow.data);
    assert!(
        Graph::parse(&text.replace(
            "ribbon_strength = param 1.2",
            "ribbon_strength = flow peak tangent flow_length"
        ))
        .unwrap()
        .compile()
        .is_err(),
        "nested/multiple flow must stay bounded"
    );
    let dir = std::env::temp_dir().join(format!("fd-style-sequence-{}", std::process::id()));
    let replay = dir.join("replay.png");
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_fd"))
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        out
    };
    let out = run(&[
        "style",
        "sequence",
        path.to_str().unwrap(),
        input.to_str().unwrap(),
        "-o",
        dir.to_str().unwrap(),
        "--frames",
        "3",
        "--fps",
        "2",
        "--grain",
        "0.03",
        "--bloom",
        "1",
        "--unresolved",
        "interior",
    ]);
    let report = String::from_utf8(out.stdout).unwrap();
    assert!(report.contains("warm_samples 2"));
    assert!(report.contains("sample_uploads 1"));
    run(&[
        "style",
        "render",
        path.to_str().unwrap(),
        input.to_str().unwrap(),
        "-o",
        replay.to_str().unwrap(),
        "--time",
        "0.5",
        "--exposure",
        "1",
        "--grain",
        "0.03",
        "--bloom",
        "1",
        "--unresolved",
        "interior",
    ]);
    assert_eq!(
        std::fs::read(dir.join("frame-00001.png")).unwrap(),
        std::fs::read(replay).unwrap()
    );
    std::fs::remove_dir_all(dir).unwrap();
}
