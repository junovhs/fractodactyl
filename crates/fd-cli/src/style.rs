//! CLI entry point for the opt-in style engine. Existing look routes are untouched.
use crate::args::Args;
use fd_shade::style::render::{columns, StyleRenderer};
use fd_shade::style::Graph;
use std::time::Instant;

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let a = Args::parse(argv, &["o", "time", "runs", "set", "unresolved"])?;
    match a.positional.as_slice() {
        [command, path] if command == "inspect" => {
            let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
            let c = Graph::parse(&text)?.compile()?;
            print!("{}", c.report());
            if let Some(out) = a.str("o") {
                std::fs::write(out, &c.body).map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        [command, path, input] if command == "render" => {
            let output=a.need("o")?;
            let time:f32=a.num("time",0.0)?;
            if !time.is_finite(){return Err("style time must be finite".into());}
            let runs:u32=a.num("runs",1)?;
            if !(1..=128).contains(&runs){return Err("style runs must be 1..128".into());}
            let unresolved=match a.str("unresolved").unwrap_or("mark") {"mark"=>false,"interior"=>true,_=>return Err("--unresolved expects mark or interior".into())};
            let compiled=Graph::parse(&std::fs::read_to_string(path).map_err(|e|e.to_string())?)?.compile()?;
            print!("{}",compiled.report());
            let load=Instant::now();
            let mut reader=fd_samples::Reader::open(std::path::Path::new(input)).map_err(|e|e.to_string())?;
            let samples=reader.read(columns(&compiled)).map_err(|e|e.to_string())?;
            let load_seconds=load.elapsed().as_secs_f64();
            let mut renderer=StyleRenderer::new(compiled,&reader.header,&samples)?;
            if let Some(settings)=a.str("set"){
                for setting in settings.split(','){
                    let (name,value)=setting.split_once('=').ok_or("--set expects name=value pairs")?;
                    renderer.set_parameter(name,value.parse().map_err(|_|"invalid --set number")?)?;
                }
            }
            let mut image=None;
            for run in 0..runs {
                image=Some(renderer.frame(time,unresolved)?);
                let s=&renderer.stats;
                println!("style_frame {run} seconds {:.6} dispatches {} pipelines {} allocations {} sample_uploads {} working_bytes_estimate {} iterations 0",s.frame_seconds,s.dispatches,s.pipelines,s.allocations,s.sample_uploads,s.working_bytes);
            }
            let img=image.unwrap();
            std::fs::write(output,fd_shade::png(&img)).map_err(|e|e.to_string())?;
            println!("style_output {output} px {}x{} adapter {:?} load_seconds {load_seconds:.6} initialization_seconds {:.6}",img.w,img.h,renderer.stats.adapter,renderer.stats.initialization_seconds);
            Ok(())
        }
        _ => Err("usage: fd style inspect GRAPH [-o BODY.wgsl] | fd style render GRAPH INPUT.fds -o OUTPUT.png [--time T --runs N --set name=value,... --unresolved mark|interior]".into()),
    }
}
