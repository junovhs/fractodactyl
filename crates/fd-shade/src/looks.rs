//! Runtime storage for studio presets, shared by CLI and viewer callers.
use std::path::{Path, PathBuf};

/// `$FD_LOOKS`, otherwise the main checkout's `looks/`, discovered from the
/// executable and then the current directory, otherwise a release package's
/// `looks/` beside the executable. Git worktrees use their common Git directory
/// so saves survive worktree removal. Built-ins remain available without either.
pub fn looks_dir() -> Option<PathBuf> {
    resolve(
        std::env::var_os("FD_LOOKS").map(PathBuf::from),
        std::env::current_exe().ok().as_deref(),
        std::env::current_dir().ok().as_deref(),
    )
}

fn resolve(
    override_dir: Option<PathBuf>,
    exe: Option<&Path>,
    cwd: Option<&Path>,
) -> Option<PathBuf> {
    override_dir.or_else(|| {
        exe.and_then(Path::parent)
            .into_iter()
            .chain(cwd)
            .find_map(|start| start.ancestors().find_map(repo_looks))
            .or_else(|| {
                let looks = exe?.parent()?.join("looks");
                looks.is_dir().then_some(looks)
            })
    })
}

fn repo_looks(root: &Path) -> Option<PathBuf> {
    let marker = root.join(".git");
    let git_dir = if marker.is_dir() {
        marker
    } else {
        let text = std::fs::read_to_string(&marker).ok()?;
        root.join(text.trim().strip_prefix("gitdir: ")?)
    };
    let common = match std::fs::read_to_string(git_dir.join("commondir")) {
        Ok(text) => git_dir.join(text.trim()),
        Err(_) => git_dir,
    };
    let common = common.canonicalize().ok()?;
    // A bare repository has no checkout to save into.
    if common.file_name()? != ".git" {
        return None;
    }
    let looks = common.parent()?.join("looks");
    looks.is_dir().then_some(looks)
}

/// Save a validated look and return its full destination path for display.
pub fn save_look(name: &str, look: &crate::Look) -> Result<PathBuf, String> {
    let dir = looks_dir().ok_or("no repository looks folder; set FD_LOOKS to save presets")?;
    save_at(&dir, name, look)
}

fn save_at(dir: &Path, name: &str, look: &crate::Look) -> Result<PathBuf, String> {
    if name.is_empty()
        || name.len() > 64
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(format!(
            "bad preset name {name:?}: use letters, digits, - and _"
        ));
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(format!("{name}.look"));
    std::fs::write(&path, look.to_text()).map_err(|e| format!("{}: {e}", path.display()))?;
    path.canonicalize()
        .map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "fd-looks-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(path.join(".git/worktrees/probe")).unwrap();
            std::fs::create_dir_all(path.join("looks")).unwrap();
            Self(path.canonicalize().unwrap())
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn worktree_reads_and_saves_in_main_checkout() {
        let f = Fixture::new();
        let worktree = f.0.join("scratch/probe");
        std::fs::create_dir_all(worktree.join("looks")).unwrap();
        std::fs::write(
            worktree.join(".git"),
            "gitdir: ../../.git/worktrees/probe\n",
        )
        .unwrap();
        std::fs::write(f.0.join(".git/worktrees/probe/commondir"), "../..\n").unwrap();
        let dir = resolve(None, None, Some(&worktree)).unwrap();
        assert_eq!(dir, f.0.join("looks"));
        let look = crate::Look::builtin("ice").unwrap();
        let saved = save_at(&dir, "probe", &look).unwrap();
        assert_eq!(saved, dir.join("probe.look"));
        assert_eq!(std::fs::read_to_string(saved).unwrap(), look.to_text());
        assert!(!worktree.join("looks/probe.look").exists());
        std::fs::remove_dir_all(worktree).unwrap();
        assert_eq!(
            resolve(None, Some(&f.0.join("target/release/fd")), None),
            Some(dir)
        );
    }

    #[test]
    fn override_wins_and_cwd_is_a_fallback() {
        let f = Fixture::new();
        let override_dir = f.0.join("custom");
        assert_eq!(
            resolve(
                Some(override_dir.clone()),
                Some(&f.0.join("fd")),
                Some(&f.0)
            ),
            Some(override_dir)
        );
        assert_eq!(resolve(None, None, Some(&f.0)), Some(f.0.join("looks")));
        assert_eq!(resolve(None, None, None), None);
    }

    #[test]
    fn release_looks_follow_override_and_checkout_lookup() {
        let f = Fixture::new();
        let package = f.0.join("release");
        std::fs::create_dir_all(package.join("looks")).unwrap();
        let exe = package.join("fd");
        assert_eq!(resolve(None, Some(&exe), None), Some(f.0.join("looks")));
        std::fs::remove_dir_all(f.0.join(".git")).unwrap();
        assert_eq!(resolve(None, Some(&exe), None), Some(package.join("looks")));
        let custom = f.0.join("custom");
        assert_eq!(
            resolve(Some(custom.clone()), Some(&exe), None),
            Some(custom)
        );
        std::fs::remove_dir_all(package.join("looks")).unwrap();
        assert_eq!(resolve(None, Some(&exe), None), None);
    }

    #[test]
    fn save_rejects_paths() {
        let f = Fixture::new();
        let look = crate::Look::builtin("ice").unwrap();
        for name in ["", "../escape", "a/b", "a.look", "x y"] {
            assert!(save_at(&f.0.join("looks"), name, &look).is_err());
        }
    }
}
