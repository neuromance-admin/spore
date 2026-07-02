//! Vault resolution and the path-containment guard.
//!
//! The vault root is the directory containing the runtime file (`_sporeAlpha.md`
//! — the frozen canonical name, v0.3+). It is resolved by walking up from cwd
//! (or taken from `--vault`). Every path a verb touches is guarded: it must
//! resolve *inside* the root. This makes Hard Floor #1 structural — the tool
//! will not cross the boundary.

use crate::{ErrKind, Result, SporeError};
use std::path::{Component, Path, PathBuf};

/// The frozen canonical runtime filename (v0.3+). The version lives in the
/// file's frontmatter and in `spore version` — never in the name, so a refresh
/// is always a clean single-file overwrite.
pub const RUNTIME_FILENAME: &str = "_sporeAlpha.md";

/// A file is a runtime iff it carries exactly the frozen name. Versioned names
/// (`_sporeAlpha.v*.md`), refresh backups (`_sporeAlpha.md.bak-*`) and the
/// transient `_sporeAlpha.shedding.tmp` are deliberately not runtimes.
pub fn is_runtime_file(name: &str) -> bool {
    name == RUNTIME_FILENAME
}

fn dir_has_runtime(dir: &Path) -> bool {
    match std::fs::read_dir(dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).any(|e| {
            e.file_name()
                .to_str()
                .map(is_runtime_file)
                .unwrap_or(false)
        }),
        Err(_) => false,
    }
}

/// Resolve the vault root. With `--vault`, verify it actually holds a runtime.
/// Otherwise walk up from cwd to the nearest ancestor that does.
pub fn resolve(override_root: Option<&Path>) -> Result<PathBuf> {
    if let Some(o) = override_root {
        let abs = absolutize(o)?;
        if !dir_has_runtime(&abs) {
            return Err(SporeError::new(
                ErrKind::State,
                format!("no runtime file (_sporeAlpha.md) in {}", abs.display()),
            ));
        }
        return Ok(abs);
    }

    let cwd = std::env::current_dir()?;
    let mut cur: &Path = &cwd;
    loop {
        if dir_has_runtime(cur) {
            return Ok(cur.to_path_buf());
        }
        match cur.parent() {
            Some(p) => cur = p,
            None => {
                return Err(SporeError::new(
                    ErrKind::State,
                    "not inside a Spore vault (no _sporeAlpha.md found in this directory or any ancestor)"
                        .to_string(),
                ));
            }
        }
    }
}

/// Make a path absolute without requiring it to exist (joins cwd if relative).
fn absolutize(p: &Path) -> Result<PathBuf> {
    if p.is_absolute() {
        Ok(lexical_normalize(p))
    } else {
        let cwd = std::env::current_dir()?;
        Ok(lexical_normalize(&cwd.join(p)))
    }
}

/// Resolve `.` and `..` lexically — no filesystem access, so it works for
/// files that don't exist yet (e.g. `create`). Leading `/` is preserved.
pub fn lexical_normalize(p: &Path) -> PathBuf {
    let mut out: Vec<Component> = Vec::new();
    for comp in p.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                match out.last() {
                    Some(Component::Normal(_)) => {
                        out.pop();
                    }
                    // A `..` at root, or after another `..`, is kept — the guard
                    // below will reject anything that lands outside the vault.
                    _ => out.push(comp),
                }
            }
            other => out.push(other),
        }
    }
    let mut buf = PathBuf::new();
    for c in out {
        buf.push(c.as_os_str());
    }
    buf
}

/// Resolve symlinks on the longest existing prefix of `p`, then rejoin the
/// (possibly not-yet-existing) tail. This lets the guard compare real paths on
/// both sides even when the leaf (e.g. a `create` target) doesn't exist yet and
/// even when the vault sits under a symlink (macOS `/var` -> `/private/var`).
fn canonical_ish(p: &Path) -> PathBuf {
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(p)
    };
    let norm = lexical_normalize(&abs);
    for anc in norm.ancestors() {
        if let Ok(c) = std::fs::canonicalize(anc) {
            return match norm.strip_prefix(anc) {
                Ok(rest) => lexical_normalize(&c.join(rest)),
                Err(_) => c,
            };
        }
    }
    norm
}

/// Guard a path argument against the vault root. Returns the resolved absolute
/// path if it is inside the root; otherwise a Guard error (Category A STOP).
pub fn guard(root: &Path, arg: &str) -> Result<PathBuf> {
    let candidate = Path::new(arg);
    let joined = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    };
    let abs = canonical_ish(&joined);
    let root_c = canonical_ish(root);

    if !abs.starts_with(&root_c) {
        return Err(SporeError::new(
            ErrKind::Guard,
            format!(
                "path escapes the vault root: {} is not inside {}",
                abs.display(),
                root_c.display()
            ),
        ));
    }
    Ok(abs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_name_is_frozen() {
        assert!(is_runtime_file("_sporeAlpha.md"));
        assert!(!is_runtime_file("_sporeAlpha.v0.2.md")); // versioned names are not runtimes (v0.3 freeze, no legacy)
        assert!(!is_runtime_file("_sporeAlpha.md.bak-0.2.0")); // refresh backups are not runtimes
        assert!(!is_runtime_file("_sporeAlpha.shedding.tmp"));
    }

    #[test]
    fn normalize_resolves_dotdot() {
        assert_eq!(lexical_normalize(Path::new("/a/b/../c")), PathBuf::from("/a/c"));
        assert_eq!(lexical_normalize(Path::new("/a/./b")), PathBuf::from("/a/b"));
    }

    #[test]
    fn guard_accepts_inside() {
        let root = std::env::temp_dir();
        let ok = guard(&root, "notes/x.md");
        assert!(ok.is_ok());
        assert!(ok.unwrap().starts_with(lexical_normalize(
            &std::fs::canonicalize(&root).unwrap()
        )));
    }

    #[test]
    fn guard_rejects_escape() {
        let root = std::env::temp_dir().join("spore-guard-test-root");
        std::fs::create_dir_all(&root).unwrap();
        assert!(guard(&root, "../../../etc/passwd").is_err());
        assert!(guard(&root, "/etc/passwd").is_err());
    }
}
