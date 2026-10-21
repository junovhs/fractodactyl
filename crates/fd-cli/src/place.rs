//! Saved places (EXPL-08): a location found in the viewer, kept in per-user storage
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

/// Personal saves stay outside the install/checkout (DEC-35).
fn places_dir() -> Result<PathBuf, String> {
    user_dir(
        cfg!(windows),
        std::env::var_os("FD_PLACES").map(PathBuf::from),
        std::env::var_os("APPDATA").map(PathBuf::from),
        std::env::var_os("XDG_DATA_HOME").map(PathBuf::from),
        std::env::var_os("HOME").map(PathBuf::from),
    )
    .ok_or_else(|| "no user data folder; set FD_PLACES to save places".to_string())
}

fn user_dir(
    windows: bool,
    override_dir: Option<PathBuf>,
    appdata: Option<PathBuf>,
    xdg: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Option<PathBuf> {
    let nonempty = |p: &PathBuf| !p.as_os_str().is_empty();
    override_dir.filter(nonempty).or_else(|| {
        let data = if windows {
            appdata.filter(nonempty)
        } else {
            xdg.filter(nonempty)
                .or_else(|| home.filter(nonempty).map(|h| h.join(".local/share")))
        }?;
        Some(data.join("fractodactyl/places"))
    })
}

/// Read-only curated places: package first, then a runtime-discovered checkout.
fn shipped_dirs(exe: Option<&Path>, cwd: Option<&Path>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(package) = exe.and_then(Path::parent) {
        dirs.push(package.join("places"));
    }
    for start in exe.and_then(Path::parent).into_iter().chain(cwd) {
        if let Some(repo) = start.ancestors().find(|p| p.join(".git").exists()) {
            let places = repo.join("places");
            if !dirs.contains(&places) {
                dirs.push(places);
            }
        }
    }
    dirs
}

fn read_dirs() -> Vec<PathBuf> {
    places_dir()
        .ok()
        .into_iter()
        .chain(shipped_dirs(
            std::env::current_exe().ok().as_deref(),
            std::env::current_dir().ok().as_deref(),
        ))
        .collect()
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
    is_decimal(m)
        && !m.starts_with(['-', '+'])
        && m.bytes().any(|b| (b'1'..=b'9').contains(&b))
        && !e.is_empty()
        && e.bytes().all(|b| b.is_ascii_digit())
}

impl Place {
    pub(crate) fn parse(text: &str) -> Result<Place, String> {
        let (mut re, mut im, mut width, mut iter) = (None, None, None, 20_000);
        for line in text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
        {
            let (k, v) = line
                .split_once(char::is_whitespace)
                .map(|(k, v)| (k, v.trim()))
                .unwrap_or((line, ""));
            match k {
                "re" | "im" if !is_decimal(v) => {
                    return Err(format!("{k}: not a decimal number: {v:?}"))
                }
                "re" => re = Some(v.to_string()),
                "im" => im = Some(v.to_string()),
                "width" if !is_width(v) => {
                    return Err(format!("width: not a positive number: {v:?}"))
                }
                "width" => width = Some(v.to_string()),
                "iter" => {
                    iter = v
                        .parse()
                        .ok()
                        .filter(|n| (100..=2_000_000).contains(n))
                        .ok_or_else(|| format!("iter: expected 100..2000000, got {v:?}"))?
                }
                _ => return Err(format!("unknown place key {k:?}")),
            }
        }
        Ok(Place {
            re: re.ok_or("place has no re")?,
            im: im.ok_or("place has no im")?,
            width: width.ok_or("place has no width")?,
            iter,
        })
    }

    pub(crate) fn to_text(&self) -> String {
        format!(
            "# fractodactyl place\nre {}\nim {}\nwidth {}\niter {}\n",
            self.re, self.im, self.width, self.iter
        )
    }
}

/// Saved places, sorted by name (unreadable or invalid files are skipped).
pub(crate) fn list() -> Vec<(String, Place)> {
    let mut out = std::collections::BTreeMap::new();
    for dir in read_dirs() {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path
                .file_name()
                .and_then(|s| s.to_str())
                .and_then(|s| s.strip_suffix(".place"))
            else {
                continue;
            };
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Ok(place) = Place::parse(&text) else {
                continue;
            };
            out.entry(name.to_string()).or_insert(place);
        }
    }
    out.into_iter().collect()
}

/// A personal or curated place name, or an explicit file path.
pub(crate) fn load(name: &str) -> Result<Place, String> {
    let paths: Vec<_> = std::iter::once(PathBuf::from(name))
        .chain(
            read_dirs()
                .into_iter()
                .map(|dir| dir.join(format!("{name}.place"))),
        )
        .collect();
    let path = paths.iter().find(|p| p.is_file()).ok_or_else(|| {
        format!(
            "no place {name:?} (looked in {})",
            paths
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Place::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Write a personal `NAME.place`; returns the absolute path written (DEC-35).
pub(crate) fn save(name: &str, place: &Place) -> Result<String, String> {
    let dir = places_dir()?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(format!("{name}.place"));
    std::fs::write(&path, place.to_text()).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path.canonicalize().unwrap_or(path).display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_user_roots() {
        let p = |s: &str| Some(PathBuf::from(s));
        for windows in [false, true] {
            assert_eq!(
                user_dir(windows, p("override"), p("app"), p("xdg"), p("home")),
                p("override")
            );
            assert_eq!(user_dir(windows, None, None, None, None), None);
        }
        assert_eq!(
            user_dir(true, None, p("app"), p("xdg"), p("home")),
            p("app/fractodactyl/places")
        );
        assert_eq!(user_dir(true, None, None, p("xdg"), p("home")), None);
        assert_eq!(
            user_dir(false, None, p("app"), p("xdg"), p("home")),
            p("xdg/fractodactyl/places")
        );
        assert_eq!(
            user_dir(false, None, p("app"), None, p("home")),
            p("home/.local/share/fractodactyl/places")
        );
        assert_eq!(
            user_dir(false, p(""), None, p(""), p("home")),
            p("home/.local/share/fractodactyl/places")
        );
    }

    #[test]
    fn curated_checkout_is_discovered_at_runtime() {
        let root = std::env::temp_dir().join(format!("fd-place-resolve-{}", std::process::id()));
        std::fs::create_dir_all(root.join("repo/.git")).unwrap();
        let exe = root.join("package/fd");
        let cwd = root.join("repo/nested");
        assert_eq!(
            shipped_dirs(Some(&exe), Some(&cwd)),
            vec![root.join("package/places"), root.join("repo/places")]
        );
        // A checkout can also be found above the executable, without relying on cwd.
        let exe = root.join("repo/target/release/fd");
        assert_eq!(
            shipped_dirs(Some(&exe), None),
            vec![
                root.join("repo/target/release/places"),
                root.join("repo/places")
            ]
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn parses_round_trips_and_validates() {
        let deep = format!(
            "-0.7432918908524302029316241585089040394625440130877230883413356446722846985935{}",
            "1".repeat(1000)
        );
        let p = Place::parse(&format!(
            "# x\nre {deep}\nim 0.13124\nwidth 1.5e-1000\niter 50000\n"
        ))
        .unwrap();
        assert_eq!(
            (p.re.as_str(), p.width.as_str(), p.iter),
            (deep.as_str(), "1.5e-1000", 50000)
        );
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
