//! Surgical frontmatter writes. `set` and `remove` touch only the named key and
//! preserve every other line verbatim — the tolerance principle (legacy fields
//! like `vmdId`, owner-added keys, comments are never clobbered).
//!
//! We treat frontmatter as a YAML *subset*: top-level `key: value` lines at
//! column 0. A block below a key (indented lines, `- list` items) is a
//! "continuation" belonging to that key. We never reserialize the whole doc —
//! we splice at the line level, so anything we don't understand is left as-is.

use crate::{ErrKind, Result, SporeError};

const FENCE: &str = "---";

struct Parsed {
    /// Lines of the frontmatter interior (between the fences), no trailing '\n'.
    fm_lines: Vec<String>,
    /// Everything after the closing fence (the body), verbatim.
    body: String,
    /// Whether the document had a frontmatter block at all.
    had_fm: bool,
    /// True if the interior lines ended with a newline before the closing fence.
    trailing_nl_body: bool,
}

fn split(content: &str) -> Result<Parsed> {
    // No leading frontmatter.
    let opens = content.starts_with("---\n") || content == FENCE || content.starts_with("---\r\n");
    if !opens {
        return Ok(Parsed {
            fm_lines: Vec::new(),
            body: content.to_string(),
            had_fm: false,
            trailing_nl_body: content.ends_with('\n'),
        });
    }

    let mut lines = content.split_inclusive('\n');
    let _open = lines.next(); // "---\n"

    let mut fm_lines = Vec::new();
    let mut body = String::new();
    let mut closed = false;
    for line in lines {
        let trimmed = line.strip_suffix('\n').unwrap_or(line);
        if !closed && trimmed == FENCE {
            closed = true;
            continue; // drop the closing fence; we re-emit it on serialize
        }
        if closed {
            body.push_str(line);
        } else {
            fm_lines.push(trimmed.to_string());
        }
    }

    if !closed {
        return Err(SporeError::new(
            ErrKind::State,
            "malformed frontmatter: opening `---` has no closing `---`".to_string(),
        ));
    }

    Ok(Parsed {
        fm_lines,
        body,
        had_fm: true,
        trailing_nl_body: content.ends_with('\n'),
    })
}

/// Is this a top-level `key:` line? Returns the key if so.
fn top_level_key(line: &str) -> Option<&str> {
    if line.starts_with(char::is_whitespace) {
        return None;
    }
    let colon = line.find(':')?;
    let key = &line[..colon];
    if key.is_empty() || !key.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return None;
    }
    Some(key)
}

/// Index range [start, end) of the key's lines (its `key:` line + continuations).
fn key_span(fm_lines: &[String], key: &str) -> Option<(usize, usize)> {
    let mut i = 0;
    while i < fm_lines.len() {
        if top_level_key(&fm_lines[i]) == Some(key) {
            let start = i;
            let mut end = i + 1;
            while end < fm_lines.len() && top_level_key(&fm_lines[end]).is_none() {
                end += 1;
            }
            return Some((start, end));
        }
        i += 1;
    }
    None
}

/// Render a scalar value, quoting only when YAML would otherwise misread it.
fn render_value(v: &str) -> String {
    let needs_quote = v.is_empty()
        || v.starts_with(|c: char| c.is_whitespace())
        || v.ends_with(|c: char| c.is_whitespace())
        || v.starts_with(['&', '*', '!', '|', '>', '%', '@', '`', '"', '\'', '#', '-', '?', ':', '[', '{'])
        || v.contains(": ")
        || v.contains(" #")
        || v.ends_with(':');
    if needs_quote {
        let escaped = v.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{}\"", escaped)
    } else {
        v.to_string()
    }
}

fn serialize(p: &Parsed) -> String {
    let mut out = String::new();
    if p.had_fm || !p.fm_lines.is_empty() {
        out.push_str("---\n");
        for l in &p.fm_lines {
            out.push_str(l);
            out.push('\n');
        }
        out.push_str("---\n");
    }
    out.push_str(&p.body);
    // Preserve a body that intentionally had no trailing newline.
    if !p.trailing_nl_body && out.ends_with('\n') && !p.body.is_empty() && !p.body.ends_with('\n') {
        out.pop();
    }
    out
}

/// Set `key` to a scalar `value`, preserving all other frontmatter verbatim.
pub fn set(content: &str, key: &str, value: &str) -> Result<String> {
    let mut p = split(content)?;
    let line = format!("{}: {}", key, render_value(value));
    match key_span(&p.fm_lines, key) {
        Some((start, end)) => {
            // Replace the key's span with a single scalar line.
            p.fm_lines.splice(start..end, std::iter::once(line));
        }
        None => {
            p.fm_lines.push(line);
        }
    }
    p.had_fm = true;
    Ok(serialize(&p))
}

/// Remove `key` (and its continuation lines) if present. A no-op if absent.
pub fn remove(content: &str, key: &str) -> Result<String> {
    let mut p = split(content)?;
    if let Some((start, end)) = key_span(&p.fm_lines, key) {
        p.fm_lines.drain(start..end);
    }
    Ok(serialize(&p))
}

/// Read a top-level scalar value for `key`, if present (used by queries).
pub fn get(content: &str, key: &str) -> Option<String> {
    let p = split(content).ok()?;
    let (start, _end) = key_span(&p.fm_lines, key)?;
    let line = &p.fm_lines[start];
    let colon = line.find(':')?;
    let raw = line[colon + 1..].trim();
    let unquoted = raw
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .map(|s| s.replace("\\\"", "\"").replace("\\\\", "\\"))
        .unwrap_or_else(|| raw.to_string());
    Some(unquoted)
}

/// Does the frontmatter carry this top-level key at all?
pub fn has_key(content: &str, key: &str) -> bool {
    split(content)
        .ok()
        .map(|p| key_span(&p.fm_lines, key).is_some())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "---\nschemaVersion: 1\nsummary: \"hi\"\nvmdId: legacy-123\n---\nbody text\n";

    #[test]
    fn set_existing_preserves_others() {
        let out = set(DOC, "summary", "new summary").unwrap();
        assert!(out.contains("summary: new summary"));
        assert!(out.contains("vmdId: legacy-123"), "must preserve legacy field");
        assert!(out.contains("schemaVersion: 1"));
        assert!(out.ends_with("body text\n"));
    }

    #[test]
    fn set_new_key_appends() {
        let out = set(DOC, "updated", "2026-07-02").unwrap();
        assert!(out.contains("updated: 2026-07-02"));
        assert!(out.contains("vmdId: legacy-123"));
    }

    #[test]
    fn set_quotes_when_needed() {
        let out = set(DOC, "summary", "a: colon here").unwrap();
        assert!(out.contains("summary: \"a: colon here\""), "got: {}", out);
    }

    #[test]
    fn remove_drops_key() {
        let out = remove(DOC, "vmdId").unwrap();
        assert!(!out.contains("vmdId"));
        assert!(out.contains("schemaVersion: 1"));
    }

    #[test]
    fn set_creates_frontmatter_when_absent() {
        let out = set("just a body\n", "schemaVersion", "1").unwrap();
        assert!(out.starts_with("---\nschemaVersion: 1\n---\n"));
        assert!(out.ends_with("just a body\n"));
    }

    #[test]
    fn get_reads_scalar() {
        assert_eq!(get(DOC, "summary").as_deref(), Some("hi"));
        assert_eq!(get(DOC, "schemaVersion").as_deref(), Some("1"));
        assert_eq!(get(DOC, "missing"), None);
    }

    #[test]
    fn unclosed_frontmatter_errors() {
        assert!(set("---\nk: v\nno close\n", "k", "x").is_err());
    }
}
