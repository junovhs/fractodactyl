//! `fd shade` / `fd info`: consume `.fds` files. `fd shade` reads each input once (the
//! union of the columns its looks need) and applies every look to that one result.
//!
//! Appearance flags (FX-01, shared with `fd play`): `--flow`, `--breathe`, `--brate`,
//! `--drift` animate the bands (the explorer's knobs and maths), `--aa on` fades bands
//! finer than a sample, `--unresolved interior` draws unresolved samples as interior.
//! Time is `--time T` plus, over a directory, frame index / `--fps F`; with every flag
//! at its default the looks are exactly the still looks.
//!
//! Studio looks (FX-02): `--preset NAME|FILE` picks a `.look` (a file path, else
//! `looks/NAME.look`, else a built-in: ice, coral, steel, zebra, smoke) and implies
//! `--look studio`; `--density`, `--terrace`, `--slope`, `--light`, `--lines`, `--line-px`
//! override its knobs, and its animation defaults apply where the flags are absent.
use crate::args::Args;
use fd_atlas::Sha256;
use fd_samples::{Column, ColumnSet, Kind, Reader};
use fd_shade::Appearance;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let known: Vec<&str> = ["look", "o", "time", "fps"]
        .into_iter()
        .chain(APPEARANCE_FLAGS)
        .chain(LOOK_FLAGS)
        .collect();
    let a = Args::parse(argv, &known)?;
    let mut base = appearance(&a)?;
    let (t0, fps): (f64, f64) = (a.num("time", 0.0)?, a.num("fps", 0.0)?);
    if !t0.is_finite() || !fps.is_finite() || fps < 0.0 {
        return Err("--time must be finite and --fps non-negative".into());
    }
    if a.str("look").is_none() && a.str("o").is_none() && a.positional.len() == 3 {
        return single(&a.positional);
    }
    let ([input], Some(out)) = (a.positional.as_slice(), a.str("o")) else {
        return Err(format!("usage: fd shade IN.fds|DIR [--look L[,L...]] [--preset NAME|FILE] [--density D] [--terrace T] [--slope S] [--light DEG] [--lines L] [--line-px W] -o OUTDIR [--time T] [--fps F] {APPEARANCE_USAGE}"));
    };
    let default = if a.str("preset").is_some() {
        "studio"
    } else {
        fd_shade::NAMES[0]
    };
    let (names, looks): (Vec<String>, Vec<_>) =
        passes(a.str("look").unwrap_or(default), &a, &mut base)?
            .into_iter()
            .unzip();
    let frames = inputs(Path::new(input))?;
    std::fs::create_dir_all(out).map_err(|e| format!("{out}: {e}"))?;
    let want = looks
        .iter()
        .fold(ColumnSet::default(), |m, l| ColumnSet(m.0 | l.columns().0));
    let (mut read_s, mut shade_s, mut write_s, mut images) = (0.0, 0.0, 0.0, 0);
    for (k, f) in frames.iter().enumerate() {
        let time = t0 + if fps > 0.0 { k as f64 / fps } else { 0.0 };
        let look_at = Appearance { time, ..base };
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
        let cols: Vec<String> = want
            .with(Column::Class)
            .iter()
            .map(|c| format!("{c:?}").to_lowercase())
            .collect();
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
            let img = pass.shade_with(h, &samples, &look_at);
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

/// Flags of [`appearance`].
pub(crate) const APPEARANCE_FLAGS: [&str; 7] = [
    "flow",
    "breathe",
    "brate",
    "drift",
    "aa",
    "unresolved",
    "dither",
];
pub(crate) const APPEARANCE_USAGE: &str =
    "[--flow C/S] [--breathe A] [--brate HZ] [--drift C/S] [--aa on|off] [--unresolved mark|interior] [--dither on|off]";

/// The appearance named by the flags, at time 0 (callers set the time per frame).
pub(crate) fn appearance(a: &Args) -> Result<Appearance, String> {
    let num = |k: &str| -> Result<f64, String> {
        let v: f64 = a.num(k, 0.0)?;
        if v.is_finite() {
            Ok(v)
        } else {
            Err(format!("--{k} must be finite"))
        }
    };
    let aa = match a.str("aa").unwrap_or("off") {
        "on" => true,
        "off" => false,
        v => return Err(format!("--aa: expected on or off, got {v:?}")),
    };
    let unresolved_interior = match a.str("unresolved").unwrap_or("mark") {
        "interior" => true,
        "mark" => false,
        v => {
            return Err(format!(
                "--unresolved: expected mark or interior, got {v:?}"
            ))
        }
    };
    Ok(Appearance {
        time: 0.0,
        flow: num("flow")?,
        breathe: num("breathe")?,
        brate: num("brate")?,
        drift: num("drift")?,
        aa,
        unresolved_interior,
        dither: match a.str("dither").unwrap_or("off") {
            "on" => true,
            "off" => false,
            v => return Err(format!("--dither: expected on or off, got {v:?}")),
        },
    })
}

/// A look's name and its pass.
pub(crate) type NamedPass = (String, Box<dyn fd_shade::Pass>);

/// Flags of [`studio_look`].
pub(crate) const LOOK_FLAGS: [&str; 7] = [
    "preset", "density", "terrace", "slope", "light", "lines", "line-px",
];

/// Saved studio presets: `$FD_LOOKS` if set, else the repo's `looks/` (not the cwd), so
/// `fd explore` saves and `fd film --preset` finds the same files from anywhere.
pub(crate) fn looks_dir() -> PathBuf {
    std::env::var_os("FD_LOOKS")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../looks"))
}

/// The studio look named by `--preset` (default the built-in `ice`), with knob flags
/// applied. `NAME` is a file path if one exists, else `looks/NAME.look` (see
/// [`looks_dir`]), else built-in.
pub(crate) fn studio_look(a: &Args) -> Result<fd_shade::Look, String> {
    let name = a.str("preset").unwrap_or("ice");
    let file = [
        std::path::PathBuf::from(name),
        looks_dir().join(format!("{name}.look")),
    ]
    .into_iter()
    .find(|p| p.is_file());
    let mut l = match file {
        Some(p) => {
            let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
            fd_shade::Look::parse(&text).map_err(|e| format!("{}: {e}", p.display()))?
        }
        None => fd_shade::Look::builtin(name).ok_or_else(|| {
            let b: Vec<&str> = fd_shade::BUILTIN.iter().map(|(n, _)| *n).collect();
            format!(
                "--preset {name:?}: no such file, no looks/{name}.look, and not a built-in ({})",
                b.join(", ")
            )
        })?,
    };
    let num = |k: &str| -> Result<Option<f64>, String> {
        a.str(k)
            .map(|v| {
                v.parse::<f64>()
                    .ok()
                    .filter(|x| x.is_finite())
                    .ok_or_else(|| format!("--{k}: bad number {v:?}"))
            })
            .transpose()
    };
    if let Some(v) = num("density")? {
        l.density = v;
    }
    for (k, slot) in [
        ("terrace", &mut l.terrace),
        ("slope", &mut l.slope),
        ("light", &mut l.light),
        ("lines", &mut l.lines),
        ("line-px", &mut l.line_px),
    ] {
        if let Some(v) = num(k)? {
            *slot = v as f32;
        }
    }
    // Re-validate the overridden knobs through the parser.
    fd_shade::Look::parse(&l.to_text())
}

/// Passes for a comma-separated look list; `studio` is built from [`studio_look`], whose
/// animation defaults fill in `base` where the animation flags are absent.
pub(crate) fn passes(
    names: &str,
    a: &Args,
    base: &mut Appearance,
) -> Result<Vec<NamedPass>, String> {
    names
        .split(',')
        .map(|n| {
            let pass: Box<dyn fd_shade::Pass> = if n == "studio" {
                let l = studio_look(a)?;
                for (k, slot, v) in [
                    ("flow", &mut base.flow, l.flow),
                    ("breathe", &mut base.breathe, l.breathe),
                    ("brate", &mut base.brate, l.brate),
                    ("drift", &mut base.drift, l.drift),
                ] {
                    if a.str(k).is_none() {
                        *slot = v;
                    }
                }
                Box::new(fd_shade::Studio(l))
            } else {
                fd_shade::by_name(n).ok_or_else(|| {
                    format!("unknown look {n:?}; known: {}", fd_shade::NAMES.join(", "))
                })?
            };
            Ok((n.to_string(), pass))
        })
        .collect()
}

/// Legacy form `fd shade <look> in.fds out.png`: one look, one image.
fn single(p: &[String]) -> Result<(), String> {
    let [name, input, out] = p else {
        unreachable!()
    };
    let pass = fd_shade::by_name(name).ok_or_else(|| {
        format!(
            "unknown look {name:?}; known: {}",
            fd_shade::NAMES.join(", ")
        )
    })?;
    let t = Instant::now();
    let mut r = Reader::open(Path::new(input)).map_err(|e| format!("{input}: {e}"))?;
    let samples = r
        .read(pass.columns())
        .map_err(|e| format!("{input}: {e}"))?;
    let img = pass.shade(&r.header, &samples);
    std::fs::write(out, fd_shade::png(&img)).map_err(|e| format!("{out}: {e}"))?;
    println!(
        "{out}: {name}, {}x{} px, {:.3} s",
        img.w,
        img.h,
        t.elapsed().as_secs_f64()
    );
    Ok(())
}

/// One `.fds` file, or every `*.fds` in a directory (e.g. `fd control -o DIR`), sorted.
fn inputs(p: &Path) -> Result<Vec<PathBuf>, String> {
    if !p.is_dir() {
        return Ok(vec![p.to_path_buf()]);
    }
    let rd = std::fs::read_dir(p).map_err(|e| format!("{}: {e}", p.display()))?;
    let mut v: Vec<PathBuf> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|f| f.extension().is_some_and(|x| x == "fds"))
        .collect();
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
    let [input] = argv else {
        return Err("usage: fd info in.fds".into());
    };
    let mut r = Reader::open(Path::new(input)).map_err(|e| format!("{input}: {e}"))?;
    let h = r.header.clone();
    let s = r
        .read(ColumnSet::default())
        .map_err(|e| format!("{input}: {e}"))?;
    let count = |k| s.class.iter().filter(|c| c.kind() == Some(k)).count();
    println!("format   1.{} kernel {}", h.minor, h.kernel);
    println!(
        "grid     {}x{} samples, ss {}, {}x{} px",
        h.nx,
        h.ny,
        h.ss,
        h.pixels().0,
        h.pixels().1
    );
    println!(
        "view     re {} im {} width {} rot {}",
        h.view.center_re, h.view.center_im, h.view.width, h.view.rotation
    );
    println!(
        "iter     max {} escape radius {:e}",
        h.max_iter, h.escape_radius
    );
    println!("columns  {:?}", h.columns.iter().collect::<Vec<_>>());
    println!(
        "classes  escaped {} interior {} unresolved {}",
        count(Kind::Escaped),
        count(Kind::Interior),
        count(Kind::Unresolved)
    );
    Ok(())
}
