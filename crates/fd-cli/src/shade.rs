//! `fd shade` / `fd info`: consume `.fds` files. `fd shade` reads each input once (the
//! union of the columns its looks need) and applies every look to that one result.
use crate::args::Args;
use fd_atlas::Sha256;
use fd_samples::{Column, ColumnSet, Kind, Reader};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let a = Args::parse(argv, &["look", "o"])?;
    if a.str("look").is_none() && a.str("o").is_none() && a.positional.len() == 3 {
        return single(&a.positional);
    }
    let ([input], Some(out)) = (a.positional.as_slice(), a.str("o")) else {
        return Err("usage: fd shade IN.fds|DIR [--look L[,L...]] -o OUTDIR".into());
    };
    let names: Vec<&str> = a.str("look").unwrap_or(fd_shade::NAMES[0]).split(',').collect();
    let looks = names
        .iter()
        .map(|n| fd_shade::by_name(n).ok_or_else(|| format!("unknown look {n:?}; known: {}", fd_shade::NAMES.join(", "))))
        .collect::<Result<Vec<_>, _>>()?;
    let frames = inputs(Path::new(input))?;
    std::fs::create_dir_all(out).map_err(|e| format!("{out}: {e}"))?;
    let want = looks.iter().fold(ColumnSet::default(), |m, l| ColumnSet(m.0 | l.columns().0));
    let (mut read_s, mut shade_s, mut write_s, mut images) = (0.0, 0.0, 0.0, 0);
    for f in &frames {
        let name = f.display();
        let bytes = std::fs::read(f).map_err(|e| format!("{name}: {e}"))?;
        let sha = hex(&Sha256::digest(&bytes));
        drop(bytes);
        let t = Instant::now();
        let mut r = Reader::open(f).map_err(|e| format!("{name}: {e}"))?;
        let samples = r.read(want).map_err(|e| format!("{name}: {e}"))?;
        let read = t.elapsed().as_secs_f64();
        read_s += read;
        let h = &r.header;
        let cols: Vec<String> = want.with(Column::Class).iter().map(|c| format!("{c:?}").to_lowercase()).collect();
        println!(
            "input {name} sha256 {sha} samples {}x{} ss {} kernel {} max_iter {} columns {} read_seconds {read:.6}",
            h.nx,
            h.ny,
            h.ss,
            h.kernel,
            h.max_iter,
            cols.join(",")
        );
        let stem = f.file_stem().and_then(|s| s.to_str()).unwrap_or("frame");
        for (look, pass) in names.iter().zip(&looks) {
            let t = Instant::now();
            let img = pass.shade(h, &samples);
            let shade = t.elapsed().as_secs_f64();
            let t = Instant::now();
            let png = format!("{}/{stem}.{look}.png", out.trim_end_matches('/'));
            std::fs::write(&png, fd_shade::png(&img)).map_err(|e| format!("{png}: {e}"))?;
            let write = t.elapsed().as_secs_f64();
            (shade_s, write_s, images) = (shade_s + shade, write_s + write, images + 1);
            println!(
                "look {look} input_sha256 {sha} out {png} px {}x{} shade_seconds {shade:.6} write_seconds {write:.6} iterations 0",
                img.w, img.h
            );
        }
    }
    // Zero by construction: fd-shade links no kernel (its own test holds that), and
    // this command calls nothing but the sample reader, the looks and the PNG writer.
    println!(
        "totals frames {} looks {} images {images} read_seconds {read_s:.6} shade_seconds {shade_s:.6} \
         write_seconds {write_s:.6} iterations 0 kernel_calls 0",
        frames.len(),
        looks.len()
    );
    Ok(())
}

/// Legacy form `fd shade <look> in.fds out.png`: one look, one image.
fn single(p: &[String]) -> Result<(), String> {
    let [name, input, out] = p else { unreachable!() };
    let pass = fd_shade::by_name(name)
        .ok_or_else(|| format!("unknown look {name:?}; known: {}", fd_shade::NAMES.join(", ")))?;
    let t = Instant::now();
    let mut r = Reader::open(Path::new(input)).map_err(|e| format!("{input}: {e}"))?;
    let samples = r.read(pass.columns()).map_err(|e| format!("{input}: {e}"))?;
    let img = pass.shade(&r.header, &samples);
    std::fs::write(out, fd_shade::png(&img)).map_err(|e| format!("{out}: {e}"))?;
    println!("{out}: {name}, {}x{} px, {:.3} s", img.w, img.h, t.elapsed().as_secs_f64());
    Ok(())
}

/// One `.fds` file, or every `*.fds` in a directory (e.g. `fd control -o DIR`), sorted.
fn inputs(p: &Path) -> Result<Vec<PathBuf>, String> {
    if !p.is_dir() {
        return Ok(vec![p.to_path_buf()]);
    }
    let rd = std::fs::read_dir(p).map_err(|e| format!("{}: {e}", p.display()))?;
    let mut v: Vec<PathBuf> =
        rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|f| f.extension().is_some_and(|x| x == "fds")).collect();
    v.sort();
    if v.is_empty() {
        return Err(format!("{}: no .fds files", p.display()));
    }
    Ok(v)
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
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
