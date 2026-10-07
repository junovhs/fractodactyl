//! `fd addr`: exact dyadic tile addresses (docs/spec/ADDRESS.md). Prints `name value`
//! lines; every coordinate is an exact decimal.
use crate::args::Args;
use fd_addr::Tile;

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let usage = || "usage: fd addr <locate|show|sample|owner> ...".to_string();
    let rest = argv.get(1..).ok_or_else(usage)?;
    match argv.first().map(String::as_str) {
        Some("locate") => {
            let a = Args::parse(rest, &["re", "im", "level"])?;
            let t = Tile::locate(a.need("re")?, a.need("im")?, a.num("level", 0)?)?;
            println!("key {t}");
            Ok(())
        }
        Some("show") => {
            let t = key(&Args::parse(rest, &[])?)?;
            println!("key {t}\nlevel {}", t.level);
            println!("re {}\nim {}\nside {}", t.center_re(), t.center_im(), t.side());
            println!("parent {}", t.parent().map_or("-".into(), |p| p.to_string()));
            let kids: Vec<String> = (0..4).map(|q| t.child(q).to_string()).collect();
            println!("children {}", kids.join(" "));
            Ok(())
        }
        Some("sample") => {
            let a = Args::parse(rest, &["grid", "at"])?;
            let t = key(&a)?;
            let (i, j) = a.need("at")?.split_once(',').ok_or("--at needs I,J")?;
            let parse = |v: &str| v.trim().parse::<u32>().map_err(|_| format!("--at: bad index {v:?}"));
            let c = t.sample(a.num("grid", 0)?, parse(i)?, parse(j)?)?;
            println!("key {c}\nre {}\nim {}", c.center_re(), c.center_im());
            Ok(())
        }
        Some("owner") => {
            let a = Args::parse(rest, &["grid"])?;
            let (t, i, j) = key(&a)?.owner(a.num("grid", 0)?)?;
            println!("key {t}\nat {i},{j}");
            Ok(())
        }
        _ => Err(usage()),
    }
}

/// The single positional tile key.
fn key(a: &Args) -> Result<Tile, String> {
    match a.positional.as_slice() {
        [k] => k.parse(),
        _ => Err("expected one tile key (level/xhex/yhex)".into()),
    }
}
