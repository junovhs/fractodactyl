//! Saved places (EXPL-08): a location found in `fd explore`, kept as `places/NAME.place`
//! so `fd film --place NAME` can zoom to it. The text format is one `key value` per line
//! (`#` comments): `re` and `im` exact decimals (any number of digits), `width` a positive
//! decimal or `1.5e-40`, `iter` the explorer's iteration count when it was saved.
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Place {
    pub re: String,
    pub im: String,
    pub width: String,
    pub iter: u64,
}

/// `$FD_PLACES` if set, else the repo's `places/` (not the cwd), as `shade::looks_dir`.
pub(crate) fn places_dir() -> PathBuf {
    std::env::var_os("FD_PLACES").map(PathBuf::from).unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../places"))
}

/// `[+-]digits[.digits]`.
fn is_decimal(s: &str) -> bool {
    let s = s.strip_prefix(['-', '+']).unwrap_or(s);
    let (i, f) = s.split_once('.').unwrap_or((s, "0"));
    !i.is_empty() && !f.is_empty() && i.bytes().chain(f.bytes()).all(|b| b.is_ascii_digit())
}

/// A positive `digits[.digits][e[+-]digits]` that is not zero.
fn is_width(s: &str) -> bool {
    let (m, e) = s.split_once(['e', 'E']).unwrap_or((s, "0"));
    let e = e.strip_prefix(['-', '+']).unwrap_or(e);
    is_decimal(m) && !m.starts_with(['-', '+']) && m.bytes().any(|b| (b'1'..=b'9').contains(&b)) && !e.is_empty() && e.bytes().all(|b| b.is_ascii_digit())
}

impl Place {
    pub(crate) fn parse(text: &str) -> Result<Place, String> {
        let (mut re, mut im, mut width, mut iter) = (None, None, None, 20_000);
        for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
            let (k, v) = line.split_once(char::is_whitespace).map(|(k, v)| (k, v.trim())).unwrap_or((line, ""));
            match k {
                "re" | "im" if !is_decimal(v) => return Err(format!("{k}: not a decimal number: {v:?}")),
                "re" => re = Some(v.to_string()),
                "im" => im = Some(v.to_string()),
                "width" if !is_width(v) => return Err(format!("width: not a positive number: {v:?}")),
                "width" => width = Some(v.to_string()),
                "iter" => iter = v.parse().ok().filter(|n| (100..=2_000_000).contains(n)).ok_or_else(|| format!("iter: expected 100..2000000, got {v:?}"))?,
                _ => return Err(format!("unknown place key {k:?}")),
            }
        }
        Ok(Place { re: re.ok_or("place has no re")?, im: im.ok_or("place has no im")?, width: width.ok_or("place has no width")?, iter })
    }

    pub(crate) fn to_text(&self) -> String {
        format!("# fractadactyl place\nre {}\nim {}\nwidth {}\niter {}\n", self.re, self.im, self.width, self.iter)
    }
}

/// Saved places, sorted by name (unreadable or invalid files are skipped).
pub(crate) fn list() -> Vec<(String, Place)> {
    let mut out: Vec<(String, Place)> = std::fs::read_dir(places_dir())
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter_map(|e| {
                    let name = e.file_name().to_str()?.strip_suffix(".place")?.to_string();
                    let place = Place::parse(&std::fs::read_to_string(e.path()).ok()?).ok()?;
                    Some((name, place))
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// `places/NAME.place`, or a file path.
pub(crate) fn load(name: &str) -> Result<Place, String> {
    let path = [PathBuf::from(name), places_dir().join(format!("{name}.place"))]
        .into_iter()
        .find(|p| p.is_file())
        .ok_or_else(|| format!("no place {name:?} (looked for a file and {})", places_dir().join(format!("{name}.place")).display()))?;
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    Place::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Write `places/NAME.place`; returns the absolute path written.
pub(crate) fn save(name: &str, place: &Place) -> Result<String, String> {
    let dir = places_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(format!("{name}.place"));
    std::fs::write(&path, place.to_text()).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path.canonicalize().unwrap_or(path).display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_round_trips_and_validates() {
        let deep = format!("-0.7432918908524302029316241585089040394625440130877230883413356446722846985935{}", "1".repeat(1000));
        let p = Place::parse(&format!("# x\nre {deep}\nim 0.13124\nwidth 1.5e-1000\niter 50000\n")).unwrap();
        assert_eq!((p.re.as_str(), p.width.as_str(), p.iter), (deep.as_str(), "1.5e-1000", 50000));
        assert_eq!(Place::parse(&p.to_text()).unwrap(), p);
        assert_eq!(Place::parse("re 0\nim 1\nwidth 4.2").unwrap().iter, 20_000);
        for bad in [
            "re 1x\nim 0\nwidth 1",
            "re 0\nim 0\nwidth 0",
            "re 0\nim 0\nwidth -1",
            "re 0\nim 0\nwidth 1e",
            "re 0\nim 0\nwidth 1\niter 5",
            "re 0\nim 0",
            "re 0\nim 0\nwidth 1\nlook ice",
            "re .5\nim 0\nwidth 1",
        ] {
            assert!(Place::parse(bad).is_err(), "{bad:?} accepted");
        }
    }
}
