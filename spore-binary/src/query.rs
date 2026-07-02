//! Read-only queries: search, frontmatter-query, tags. All driven by the
//! binary's own recursive file walk — no external `grep`/ripgrep dependency.
//!
//! The runtime file, refresh backups (`_sporeAlpha.md.bak-*`) and the transient
//! `_sporeAlpha.shedding.tmp` are excluded from every walk, so the §7
//! search-exclusion is enforced by the tool and the AI can never forget it.

use crate::frontmatter;
use crate::Result;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Names the walk always skips.
fn is_excluded(name: &str) -> bool {
    crate::vault::is_runtime_file(name)               // _sporeAlpha.md (frozen)
        || name.starts_with("_sporeAlpha.md.bak")     // refresh backups (full runtime copies)
        || name.starts_with("_sporeAlpha.v")          // stale versioned runtimes (pre-freeze)
        || name == "_sporeAlpha.shedding.tmp"         // transient shed temp
        || name.starts_with(".spore-tmp-")            // our own write temps
}

fn is_hidden_dir(name: &str) -> bool {
    // Skip dotfolders (.git, .obsidian, .trash) — not vault content.
    name.starts_with('.')
}

/// All `.md` notes in the vault, excluding runtime/transient files. Sorted for
/// deterministic output.
pub fn walk_md(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk_into(root, &mut out);
    out.sort();
    out
}

fn walk_into(dir: &Path, out: &mut Vec<PathBuf>) {
    let rd = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };
    for entry in rd.filter_map(|e| e.ok()) {
        let name = entry.file_name();
        let name = match name.to_str() {
            Some(n) => n,
            None => continue,
        };
        let path = entry.path();
        if path.is_dir() {
            if !is_hidden_dir(name) {
                walk_into(&path, out);
            }
        } else if name.ends_with(".md") && !is_excluded(name) {
            out.push(path);
        }
    }
}

/// Free-text, case-insensitive substring search over note bodies.
/// Prints `relpath:lineno: line` for each hit.
pub fn search(root: &Path, query: &str) -> Result<()> {
    let needle = query.to_lowercase();
    for file in walk_md(root) {
        let content = match std::fs::read_to_string(&file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let rel = file.strip_prefix(root).unwrap_or(&file);
        for (n, line) in content.lines().enumerate() {
            if line.to_lowercase().contains(&needle) {
                println!("{}:{}: {}", rel.display(), n + 1, line.trim());
            }
        }
    }
    Ok(())
}

/// Files whose *frontmatter* has key `name` (optionally == `value`).
/// Prints `relpath\tvalue`.
pub fn frontmatter_query(root: &Path, name: &str, value: Option<&str>) -> Result<()> {
    for file in walk_md(root) {
        let content = match std::fs::read_to_string(&file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if !frontmatter::has_key(&content, name) {
            continue;
        }
        let got = frontmatter::get(&content, name).unwrap_or_default();
        if let Some(want) = value {
            if got != want {
                continue;
            }
        }
        let rel = file.strip_prefix(root).unwrap_or(&file);
        println!("{}\t{}", rel.display(), got);
    }
    Ok(())
}

/// List unique tags across the vault: inline `#tag` in bodies plus `tags:`
/// frontmatter values (bare or `- item` list form). Sorted, one per line.
pub fn tags(root: &Path) -> Result<()> {
    let mut set: BTreeSet<String> = BTreeSet::new();
    for file in walk_md(root) {
        let content = match std::fs::read_to_string(&file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        collect_inline_tags(&content, &mut set);
        collect_frontmatter_tags(&content, &mut set);
    }
    for t in set {
        println!("#{}", t);
    }
    Ok(())
}

fn collect_inline_tags(content: &str, set: &mut BTreeSet<String>) {
    // Naive but adequate: `#` preceded by start/whitespace, followed by tag chars.
    let bytes = content.as_bytes();
    let mut i = 0;
    while i < content.len() {
        if bytes[i] == b'#' && (i == 0 || bytes[i - 1].is_ascii_whitespace()) {
            let mut j = i + 1;
            while j < content.len() {
                let c = bytes[j];
                if c.is_ascii_alphanumeric() || c == b'-' || c == b'_' || c == b'/' {
                    j += 1;
                } else {
                    break;
                }
            }
            if j > i + 1 {
                set.insert(content[i + 1..j].to_string());
            }
            i = j;
        } else {
            i += 1;
        }
    }
}

fn collect_frontmatter_tags(content: &str, set: &mut BTreeSet<String>) {
    if let Some(v) = frontmatter::get(content, "tags") {
        // Inline list form: `tags: [a, b]` or `tags: a`
        let cleaned = v.trim_matches(|c| c == '[' || c == ']');
        for part in cleaned.split(',') {
            let t = part.trim().trim_matches('"').trim_matches('\'');
            if !t.is_empty() {
                set.insert(t.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excludes_runtime_and_temps() {
        assert!(is_excluded("_sporeAlpha.md")); // frozen canonical runtime
        assert!(is_excluded("_sporeAlpha.md.bak-0.2.0")); // refresh backup
        assert!(is_excluded("_sporeAlpha.v0.2.md")); // stale pre-freeze runtime
        assert!(is_excluded("_sporeAlpha.shedding.tmp"));
        assert!(is_excluded(".spore-tmp-123-x.md"));
        assert!(!is_excluded("Map.md"));
    }

    #[test]
    fn inline_tags_collected() {
        let mut s = BTreeSet::new();
        collect_inline_tags("a #alpha and #beta/child but not c#sharp", &mut s);
        assert!(s.contains("alpha"));
        assert!(s.contains("beta/child"));
        assert!(!s.contains("sharp"));
    }
}
