//! Opt-in style commands; rendering resources live across repeated frames.
use crate::args::Args;
use fd_shade::style::{
    finishing::Finishing,
    render::{columns, StyleRenderer},
    Graph,
};
use std::time::Instant;

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let a = Args::parse(
        argv,
        &[
            "o",
            "time",
            "runs",
            "frames",
            "fps",
            "zoom",
            "zoom-end",
            "set",
            "unresolved",
            "bloom",
            "bloom-radius",
            "grain",
            "grain-size",
            "grain-color",
            "grain-seed",
            "exposure",
            "dither",
            "grain-shadow",
            "grain-mid",
            "grain-highlight",
        ],
    )?;
    match a.positional.as_slice(){
        [command,path] if command=="inspect"=>{
            let c=Graph::parse(&std::fs::read_to_string(path).map_err(|e|e.to_string())?)?.compile()?;
            print!("{}",c.report());
            if let Some(out)=a.str("o"){std::fs::write(out,format!("{}\n{}",c.helpers,c.body)).map_err(|e|e.to_string())?;}
            Ok(())
        }
        [command,path,input] if command=="render" || command=="sequence"=>render(&a,command,path,input),
        _=>Err("usage: fd style inspect GRAPH [-o BODY.wgsl] | fd style render GRAPH INPUT.fds -o IMAGE.png | fd style sequence GRAPH INPUT.fds -o DIR [--frames N --fps F --zoom 1 --zoom-end 1.3] [--time T --runs N --set name=value,... --bloom AMOUNT --bloom-radius R --grain AMOUNT --grain-size PX --grain-color MIX --grain-seed N --exposure N --dither on|off --unresolved mark|interior]".into()),
    }
}

fn render(a: &Args, command: &str, path: &str, input: &str) -> Result<(), String> {
    let started = Instant::now();
    let output = a.need("o")?;
    let time: f32 = a.num("time", 0.0)?;
    if !time.is_finite() {
        return Err("style time must be finite".into());
    }
    let runs: u32 = a.num("runs", 1)?;
    if !(1..=128).contains(&runs) {
        return Err("style runs must be 1..128".into());
    }
    let sequence = command == "sequence";
    let frames: u32 = a.num("frames", if sequence { 48 } else { 1 })?;
    if !(1..=600).contains(&frames) || (!sequence && frames != 1) {
        return Err("sequence frames must be 1..600; render uses one frame".into());
    }
    let fps: f32 = a.num("fps", 24.0)?;
    if !fps.is_finite() || fps <= 0.0 || fps > 240.0 {
        return Err("style fps must be in (0,240]".into());
    }
    let reconstruct = a.str("zoom").is_some() || a.str("zoom-end").is_some();
    let zoom: f32 = a.num("zoom", 1.0)?;
    let zoom_end: f32 = a.num("zoom-end", zoom)?;
    if [zoom, zoom_end]
        .iter()
        .any(|x| !x.is_finite() || !(1.0..=2.0).contains(x))
    {
        return Err("style zoom must be in 1..2".into());
    }
    let unresolved = match a.str("unresolved").unwrap_or("mark") {
        "mark" => false,
        "interior" => true,
        _ => return Err("--unresolved expects mark or interior".into()),
    };
    let mut finishing = Finishing {
        bloom: a.num("bloom", 0.0)?,
        bloom_radius: a.num("bloom-radius", 12)?,
        grain: a.num("grain", 0.0)?,
        grain_size: a.num("grain-size", 1.0)?,
        grain_color: a.num("grain-color", 0.0)?,
        seed: a.num("grain-seed", 1)?,
        exposure: a.num("exposure", 0)?,
        grain_tonal: [
            a.num("grain-shadow", 0.25)?,
            a.num("grain-mid", 1.0)?,
            a.num("grain-highlight", 0.3)?,
        ],
        dither: match a.str("dither").unwrap_or("off") {
            "on" => true,
            "off" => false,
            _ => return Err("--dither expects on or off".into()),
        },
    };
    finishing.validate()?;
    finishing
        .exposure
        .checked_add(frames - 1)
        .ok_or("exposure index overflow")?;
    let compiled =
        Graph::parse(&std::fs::read_to_string(path).map_err(|e| e.to_string())?)?.compile()?;
    print!("{}", compiled.report());
    let load = Instant::now();
    let mut reader =
        fd_samples::Reader::open(std::path::Path::new(input)).map_err(|e| e.to_string())?;
    let samples = reader.read(columns(&compiled)).map_err(|e| e.to_string())?;
    let load_seconds = load.elapsed().as_secs_f64();
    let mut renderer = StyleRenderer::new(compiled, &reader.header, &samples)?;
    if let Some(settings) = a.str("set") {
        for setting in settings.split(',') {
            let (name, value) = setting
                .split_once('=')
                .ok_or("--set expects name=value pairs")?;
            renderer.set_parameter(name, value.parse().map_err(|_| "invalid --set number")?)?;
        }
    }
    if sequence {
        std::fs::create_dir_all(output).map_err(|e| e.to_string())?;
    }
    let first_exposure = finishing.exposure;
    let mut times = Vec::new();
    let mut write_seconds = 0.0;
    for frame in 0..frames {
        finishing.exposure = first_exposure + frame;
        renderer.set_finishing(&finishing)?;
        if reconstruct {
            let t = frame as f32 / (frames - 1).max(1) as f32;
            renderer.set_reconstruction(zoom * (zoom_end / zoom).powf(t), [0.5, 0.5])?;
        }
        let at = time + frame as f32 / fps;
        let mut image = None;
        for run in 0..runs {
            image = Some(renderer.frame(at, unresolved)?);
            let s = &renderer.stats;
            times.push(s.frame_seconds);
            println!("style_frame {frame} run {run} time {at:.6} exposure {} seconds {:.6} dispatches {} pipelines {} allocations {} sample_uploads {} working_bytes_estimate {} bloom_support_px {} graph_support_px {} evaluations_per_sample_max {} iterations 0",finishing.exposure,s.frame_seconds,s.dispatches,s.pipelines,s.allocations,s.sample_uploads,s.working_bytes,s.bloom_support_pixels,s.graph_support_pixels,s.evaluations_per_sample);
        }
        let image = image.unwrap();
        let target = if sequence {
            std::path::Path::new(output).join(format!("frame-{frame:05}.png"))
        } else {
            output.into()
        };
        let write = Instant::now();
        std::fs::write(&target, fd_shade::png(&image)).map_err(|e| e.to_string())?;
        write_seconds += write.elapsed().as_secs_f64();
        if frame == 0 {
            println!("style_output {} px {}x{} adapter {:?} load_seconds {load_seconds:.6} initialization_seconds {:.6}",target.display(),image.w,image.h,renderer.stats.adapter,renderer.stats.initialization_seconds);
        }
    }
    let first = times[0];
    let mut warm = times[1..].to_vec();
    warm.sort_by(f64::total_cmp);
    let p50 = warm.get(warm.len() / 2).copied().unwrap_or(0.0);
    let max = warm.last().copied().unwrap_or(0.0);
    println!("style_summary frames {frames} runs {runs} first_frame_seconds {first:.6} warm_samples {} warm_p50_seconds {p50:.6} warm_max_seconds {max:.6} output_write_seconds {write_seconds:.6} total_seconds {:.6}",warm.len(),started.elapsed().as_secs_f64());
    Ok(())
}
