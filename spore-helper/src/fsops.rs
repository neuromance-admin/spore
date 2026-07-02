//! File operations: atomic, read-after-write-verified writes, plus read /
//! create / append / prepend. Every write goes temp -> fsync -> atomic rename,
//! then is read back and compared (Hard Floor #6 / §8.7). A mismatch is a
//! Verify error (Category E STOP).

use crate::{ErrKind, Result, SporeError};
use std::fs;
use std::io::Write;
use std::path::Path;

pub fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            SporeError::new(ErrKind::NotFound, format!("no such note: {}", path.display()))
        } else {
            SporeError::from(e)
        }
    })
}

/// Atomic, verified whole-file write. Creates parent directories as needed.
pub fn atomic_write(path: &Path, content: &str) -> Result<()> {
    let parent = path.parent().ok_or_else(|| {
        SporeError::new(ErrKind::Io, format!("path has no parent: {}", path.display()))
    })?;
    fs::create_dir_all(parent)?;

    // Temp file in the same directory so the rename is atomic (same filesystem).
    let tmp = parent.join(format!(
        ".spore-tmp-{}-{}",
        std::process::id(),
        path.file_name().and_then(|n| n.to_str()).unwrap_or("note")
    ));

    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(content.as_bytes())?;
        f.sync_all()?;
    }

    // Atomic replace: the canonical path is never half-written.
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        SporeError::from(e)
    })?;

    // Read-after-write verify.
    let back = fs::read_to_string(path)?;
    if back != content {
        return Err(SporeError::new(
            ErrKind::Verify,
            format!("read-after-write mismatch on {}", path.display()),
        ));
    }
    Ok(())
}

pub fn create(path: &Path, content: &str) -> Result<()> {
    atomic_write(path, content)
}

pub fn append(path: &Path, content: &str) -> Result<()> {
    let existing = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(SporeError::from(e)),
    };
    let mut combined = existing;
    if !combined.is_empty() && !combined.ends_with('\n') {
        combined.push('\n');
    }
    combined.push_str(content);
    atomic_write(path, &combined)
}

/// Prepend after the frontmatter block if present, else at the very start.
pub fn prepend(path: &Path, content: &str) -> Result<()> {
    let existing = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(SporeError::from(e)),
    };

    let (insert_at, need_nl) = frontmatter_end(&existing);
    let mut out = String::with_capacity(existing.len() + content.len() + 1);
    out.push_str(&existing[..insert_at]);
    if need_nl && !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(content);
    if !content.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&existing[insert_at..]);
    atomic_write(path, &out)
}

/// Byte index just after the closing `---\n` of a leading frontmatter block,
/// or 0 if there is none. Returns (index, whether a newline separator is wanted).
fn frontmatter_end(content: &str) -> (usize, bool) {
    if !content.starts_with("---\n") && content != "---" {
        return (0, false);
    }
    // Find the closing fence line after the opening one.
    let after_open = 4; // len("---\n")
    let rest = &content[after_open..];
    if let Some(pos) = find_fence(rest) {
        (after_open + pos, false)
    } else {
        (0, false)
    }
}

/// Find the byte index just past a line that is exactly `---` in `s`.
fn find_fence(s: &str) -> Option<usize> {
    let mut idx = 0;
    for line in s.split_inclusive('\n') {
        let trimmed = line.strip_suffix('\n').unwrap_or(line);
        if trimmed == "---" {
            return Some(idx + line.len());
        }
        idx += line.len();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("spore-fsops-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn atomic_write_then_read() {
        let p = tmpdir().join("a.md");
        atomic_write(&p, "hello\n").unwrap();
        assert_eq!(read(&p).unwrap(), "hello\n");
    }

    #[test]
    fn append_adds_newline_boundary() {
        let p = tmpdir().join("b.md");
        atomic_write(&p, "one").unwrap();
        append(&p, "two\n").unwrap();
        assert_eq!(read(&p).unwrap(), "one\ntwo\n");
    }

    #[test]
    fn prepend_after_frontmatter() {
        let p = tmpdir().join("c.md");
        atomic_write(&p, "---\nk: v\n---\nbody\n").unwrap();
        prepend(&p, "INSERTED\n").unwrap();
        assert_eq!(read(&p).unwrap(), "---\nk: v\n---\nINSERTED\nbody\n");
    }

    #[test]
    fn prepend_no_frontmatter() {
        let p = tmpdir().join("d.md");
        atomic_write(&p, "body\n").unwrap();
        prepend(&p, "TOP\n").unwrap();
        assert_eq!(read(&p).unwrap(), "TOP\nbody\n");
    }
}
