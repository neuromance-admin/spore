//! Move / rename with wikilink integrity — the job Obsidian used to do for free.
//!
//! On rename/move we update every `[[link]]` / `![[embed]]` whose target
//! basename matches the old note, preserving `|alias`, `#heading`, `^block`.
//! Matching is case-insensitive on the basename (Obsidian-like).
//!
//! Prefix handling — the fix for cross-folder moves:
//!   - a **bare** link (`[[note]]`) stays bare (it resolves by basename
//!     regardless of folder) — only its basename is swapped;
//!   - a **path-qualified** link (`[[Drafts/note]]`) is repointed to the note's
//!     **new** vault-relative path (`[[Published/note]]`), or made bare if the
//!     note moved to the vault root. So a cross-folder move no longer leaves a
//!     stale prefix.
//! A rewrite that would produce identical text (e.g. a bare link when only the
//! folder changed) is left untouched and not counted.
//!
//! Links inside fenced code blocks are left alone. Ambiguity guard: if more
//! than one note shares the old basename, basename-matching is unsafe — we STOP
//! rather than guess (design §4).

use crate::query::walk_md;
use crate::{ErrKind, Result, SporeError};
use std::path::{Component, Path, PathBuf};

pub struct Renamed {
    pub path: PathBuf,
    pub links: usize,
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string()
}

fn ensure_md(name: &str) -> String {
    if name.ends_with(".md") {
        name.to_string()
    } else {
        format!("{}.md", name)
    }
}

/// New note path relative to the vault root, `/`-separated, without the `.md`
/// extension — the canonical target for a repointed path-qualified link.
fn vault_rel_no_ext(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    let mut parts: Vec<String> = rel
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => s.to_str().map(|s| s.to_string()),
            _ => None,
        })
        .collect();
    if let Some(last) = parts.last_mut() {
        if let Some(stripped) = last.strip_suffix(".md") {
            *last = stripped.to_string();
        }
    }
    parts.join("/")
}

/// Count vault notes whose basename (stem) equals `base`, case-insensitively.
fn basename_count(root: &Path, base: &str) -> usize {
    let lb = base.to_lowercase();
    walk_md(root)
        .iter()
        .filter(|p| stem(p).to_lowercase() == lb)
        .count()
}

pub fn rename(root: &Path, path: &Path, newname: &str) -> Result<Renamed> {
    if !path.exists() {
        return Err(SporeError::new(
            ErrKind::NotFound,
            format!("no such note: {}", path.display()),
        ));
    }
    let old_base = stem(path);
    // `newname` is a name in the same directory; ignore any stray path parts.
    let new_file = ensure_md(
        Path::new(newname)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(newname),
    );
    let new_path = path.with_file_name(&new_file);

    guard_collision(root, &old_base)?;

    if new_path.exists() {
        return Err(SporeError::new(
            ErrKind::Io,
            format!("target already exists: {}", new_path.display()),
        ));
    }
    std::fs::rename(path, &new_path)?;

    let new_rel = vault_rel_no_ext(root, &new_path);
    let links = rewrite_all(root, &old_base, &new_rel)?;
    Ok(Renamed { path: new_path, links })
}

pub fn mv(root: &Path, from: &Path, to: &Path) -> Result<usize> {
    if !from.exists() {
        return Err(SporeError::new(
            ErrKind::NotFound,
            format!("no such note: {}", from.display()),
        ));
    }
    let old_base = stem(from);

    guard_collision(root, &old_base)?;

    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if to.exists() {
        return Err(SporeError::new(
            ErrKind::Io,
            format!("target already exists: {}", to.display()),
        ));
    }
    std::fs::rename(from, to)?;

    // Always rewrite: a move can change the folder even when the basename is
    // unchanged, and path-qualified links must follow. rewrite_line no-ops any
    // link whose text wouldn't actually change, so the count stays honest.
    let new_rel = vault_rel_no_ext(root, to);
    let links = rewrite_all(root, &old_base, &new_rel)?;
    Ok(links)
}

fn guard_collision(root: &Path, base: &str) -> Result<()> {
    if basename_count(root, base) > 1 {
        return Err(SporeError::new(
            ErrKind::State,
            format!(
                "ambiguous basename '{}' — more than one note shares it; refusing to rewrite links by basename (rename them unique first)",
                base
            ),
        ));
    }
    Ok(())
}

fn rewrite_all(root: &Path, old_base: &str, new_rel: &str) -> Result<usize> {
    let mut total = 0;
    for file in walk_md(root) {
        let content = match std::fs::read_to_string(&file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let (new_content, n) = rewrite_links(&content, old_base, new_rel);
        if n > 0 {
            crate::fsops::atomic_write(&file, &new_content)?;
            total += n;
        }
    }
    Ok(total)
}

/// Rewrite `[[...]]` / `![[...]]` links whose basename matches `old_base` so
/// they point at `new_rel` (the note's new vault-relative path). Bare links
/// keep only their basename swapped; qualified links are repointed in full.
/// Returns (new_content, count). Skips fenced code blocks.
pub fn rewrite_links(content: &str, old_base: &str, new_rel: &str) -> (String, usize) {
    let lb_old = old_base.to_lowercase();
    let mut out = String::with_capacity(content.len());
    let mut count = 0;
    let mut in_fence = false;

    for line in content.split_inclusive('\n') {
        let body = line.strip_suffix('\n').unwrap_or(line);
        let t = body.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_fence = !in_fence;
            out.push_str(line);
            continue;
        }
        if in_fence {
            out.push_str(line);
            continue;
        }
        let (rewritten, n) = rewrite_line(line, &lb_old, new_rel);
        count += n;
        out.push_str(&rewritten);
    }
    (out, count)
}

fn rewrite_line(line: &str, lb_old: &str, new_rel: &str) -> (String, usize) {
    let new_base = new_rel.rsplit('/').next().unwrap_or(new_rel);
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    let mut count = 0;

    while i < line.len() {
        if bytes[i] == b'[' && i + 1 < line.len() && bytes[i + 1] == b'[' {
            // Find the closing "]]".
            if let Some(close_rel) = line[i + 2..].find("]]") {
                let inner_start = i + 2;
                let inner_end = inner_start + close_rel;
                let inner = &line[inner_start..inner_end];

                // Split off the first of `|`/`#`/`^` as the suffix (alias/anchor/block).
                let split_at = inner
                    .find(|c| c == '|' || c == '#' || c == '^')
                    .unwrap_or(inner.len());
                let target = &inner[..split_at];
                let suffix = &inner[split_at..];

                // Basename is the last '/'-segment of the target.
                let (prefix, base) = match target.rfind('/') {
                    Some(p) => (&target[..=p], &target[p + 1..]),
                    None => ("", target),
                };

                if base.to_lowercase() == *lb_old {
                    // Bare link -> keep bare (basename only). Qualified link ->
                    // repoint to the note's new vault-relative path.
                    let new_target = if prefix.is_empty() {
                        new_base.to_string()
                    } else {
                        new_rel.to_string()
                    };
                    let new_inner = format!("{}{}", new_target, suffix);
                    if new_inner != inner {
                        out.push_str("[[");
                        out.push_str(&new_inner);
                        out.push_str("]]");
                        count += 1;
                        i = inner_end + 2;
                        continue;
                    }
                    // Identical (e.g. a bare link during a folder-only move) —
                    // fall through and copy verbatim, uncounted.
                }
            }
        }
        // Default: copy this byte (as a char to stay UTF-8 safe).
        let ch = line[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    (out, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_link() {
        let (o, n) = rewrite_links("see [[old note]] here", "old note", "new note");
        assert_eq!(n, 1);
        assert_eq!(o, "see [[new note]] here");
    }

    #[test]
    fn alias_and_anchor_preserved() {
        let (o, n) = rewrite_links("[[old|Display]] and [[old#Section]]", "old", "new");
        assert_eq!(n, 2);
        assert_eq!(o, "[[new|Display]] and [[new#Section]]");
    }

    #[test]
    fn rename_in_place_keeps_prefix() {
        // rename within Sessions/: new_rel carries the same folder.
        let (o, n) = rewrite_links("[[Sessions/old]]", "old", "Sessions/new");
        assert_eq!(n, 1);
        assert_eq!(o, "[[Sessions/new]]");
    }

    #[test]
    fn cross_folder_move_repoints_qualified_keeps_bare() {
        // note moved Drafts/ -> Published/. Qualified link follows; bare stays bare.
        let src = "path [[Drafts/note]] and bare [[note]]";
        let (o, n) = rewrite_links(src, "note", "Published/note");
        assert_eq!(n, 1, "only the path-qualified link changes");
        assert_eq!(o, "path [[Published/note]] and bare [[note]]");
    }

    #[test]
    fn move_to_root_makes_qualified_bare() {
        let (o, n) = rewrite_links("[[Drafts/note]]", "note", "note");
        assert_eq!(n, 1);
        assert_eq!(o, "[[note]]");
    }

    #[test]
    fn embed_and_block() {
        let (o, n) = rewrite_links("![[old]] ref [[old^abc123]]", "old", "new");
        assert_eq!(n, 2);
        assert_eq!(o, "![[new]] ref [[new^abc123]]");
    }

    #[test]
    fn code_fence_skipped() {
        let src = "[[old]]\n```\n[[old]]\n```\n[[old]]\n";
        let (o, n) = rewrite_links(src, "old", "new");
        assert_eq!(n, 2, "the fenced [[old]] must be left alone");
        assert_eq!(o, "[[new]]\n```\n[[old]]\n```\n[[new]]\n");
    }

    #[test]
    fn non_matching_left_alone() {
        let (o, n) = rewrite_links("[[other]]", "old", "new");
        assert_eq!(n, 0);
        assert_eq!(o, "[[other]]");
    }

    #[test]
    fn case_insensitive_match() {
        let (o, n) = rewrite_links("[[Old Note]]", "old note", "new");
        assert_eq!(n, 1);
        assert_eq!(o, "[[new]]");
    }
}
