//! `fd shade` / `fd info`: consume a `.fds` file. Reads only the columns the pass needs.
use crate::args::Args;
use fd_samples::{ColumnSet, Kind, Reader};
use std::path::Path;
use std::time::Instant;

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let a = Args::parse(argv, &[])?;
    let [name, input, out] = a.positional.as_slice() else {
        return Err("usage: fd shade <pass> in.fds out.png".into());
    };
    let pass = fd_shade::by_name(name)
        .ok_or_else(|| format!("unknown pass {name:?}; known: {}", fd_shade::NAMES.join(", ")))?;
    let t = Instant::now();
    let mut r = Reader::open(Path::new(input)).map_err(|e| format!("{input}: {e}"))?;
    let samples = r.read(pass.columns()).map_err(|e| format!("{input}: {e}"))?;
    let img = pass.shade(&r.header, &samples);
    std::fs::write(out, fd_shade::png(&img)).map_err(|e| format!("{out}: {e}"))?;
    println!("{out}: {name}, {}x{} px, {:.3} s", img.w, img.h, t.elapsed().as_secs_f64());
    Ok(())
}

pub(crate) fn info(argv: &[String]) -> Result<(), String> {
    let [input] = argv else { return Err("usage: fd info in.fds".into()) };
    let mut r = Reader::open(Path::new(input)).map_err(|e| format!("{input}: {e}"))?;
    let h = r.header.clone();
    let s = r.read(ColumnSet::default()).map_err(|e| format!("{input}: {e}"))?;
    let count = |k| s.class.iter().filter(|c| c.kind() == Some(k)).count();
    println!("format   1.{} kernel {}", h.minor, h.kernel);
    println!("grid     {}x{} samples, ss {}, {}x{} px", h.nx, h.ny, h.ss, h.pixels().0, h.pixels().1);
    println!("view     re {} im {} width {} rot {}", h.view.center_re, h.view.center_im, h.view.width, h.view.rotation);
    println!("iter     max {} escape radius {:e}", h.max_iter, h.escape_radius);
    println!("columns  {:?}", h.columns.iter().collect::<Vec<_>>());
    println!(
        "classes  escaped {} interior {} unresolved {}",
        count(Kind::Escaped),
        count(Kind::Interior),
        count(Kind::Unresolved)
    );
    Ok(())
}
