//! `refresh` — update an existing vault's runtime to the one this binary
//! carries (§6 `/spore:refresh`, design: design-runtime-refresh.md).
//!
//! Safe by construction: the runtime is identity-free (same bytes in every
//! vault), so a refresh is a file *swap*, never a merge. This verb touches
//! exactly two paths — the runtime file and its timestamp-free versioned
//! backup — and nothing else. Map/Rules/Sessions/Inbox/personas are never
//! in scope.
//!
//! Decision ladder:
//!   engine < vault           → refuse (downgrade guard; update the binary)
//!   engine == vault, !force  → no-op ("already current")
//!   engine == vault, force   → re-stamp (recovery: restores the shed scaffold)
//!   engine > vault           → back up, then stamp the embedded seed
//!
//! Both writes ride `fsops::atomic_write` (temp → fsync → atomic rename →
//! read-back verify), the same path `init` trusts.

use crate::init::{embedded_runtime_version, RUNTIME};
use crate::vault::RUNTIME_FILENAME;
use crate::{ErrKind, Result, SporeError};
use std::path::{Path, PathBuf};

/// Parse a strict `MAJOR.MINOR.PATCH` semver into a comparable triple.
fn parse_semver(v: &str) -> Result<(u64, u64, u64)> {
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() == 3 {
        if let (Ok(ma), Ok(mi), Ok(pa)) = (
            parts[0].parse::<u64>(),
            parts[1].parse::<u64>(),
            parts[2].parse::<u64>(),
        ) {
            return Ok((ma, mi, pa));
        }
    }
    Err(SporeError::new(
        ErrKind::State,
        format!("not a MAJOR.MINOR.PATCH version: {:?}", v),
    ))
}

pub fn refresh(target: Option<PathBuf>, force: bool) -> Result<()> {
    // Resolve the vault: explicit path, or the standard walk-up from cwd.
    // Resolution requires the frozen-name runtime to be present (no legacy names).
    let root = crate::vault::resolve(target.as_deref())?;
    let runtime_path = root.join(RUNTIME_FILENAME);

    let current = crate::fsops::read(&runtime_path)?;
    let vault_ver = crate::frontmatter::get(&current, "version").ok_or_else(|| {
        SporeError::new(
            ErrKind::State,
            format!(
                "{} carries no `version:` frontmatter — cannot compare versions; if the file is damaged, restore a backup or re-init a fresh vault",
                runtime_path.display()
            ),
        )
    })?;
    let engine_ver = embedded_runtime_version()?;
    let v = parse_semver(&vault_ver)?;
    let e = parse_semver(&engine_ver)?;

    // Downgrade guard — holds even under force.
    if e < v {
        return Err(SporeError::new(
            ErrKind::State,
            format!(
                "downgrade refused: this vault's runtime is v{} but the binary carries v{} — update the spore binary first",
                vault_ver, engine_ver
            ),
        ));
    }

    if e == v && !force {
        println!("already current (v{}) — nothing to do", vault_ver);
        return Ok(());
    }

    // Back up the old runtime, then stamp the embedded seed. Both atomic + verified.
    let backup_path = backup_path_for(&root, &vault_ver);
    crate::fsops::atomic_write(&backup_path, &current)?;
    crate::fsops::atomic_write(&runtime_path, RUNTIME)?;

    if e == v {
        println!(
            "🍄→🌱 Re-stamped this vault's runtime (v{}, forced) — the full seed is back, setup scaffold restored.",
            engine_ver
        );
    } else {
        println!(
            "🍄→🌱 Refreshed this vault's runtime: v{} → v{}",
            vault_ver, engine_ver
        );
    }
    println!("Old runtime backed up to {}", backup_path.display());
    println!(
        "Next launch applies v{} (a seed — it re-slims to established form after boot).",
        engine_ver
    );
    Ok(())
}

/// `_sporeAlpha.md.bak-<oldversion>` beside the runtime. If that exists already
/// (a repeated same-version re-stamp), suffix `-2`, `-3`, … — never overwrite a
/// backup.
fn backup_path_for(root: &Path, old_version: &str) -> PathBuf {
    let base = root.join(format!("{}.bak-{}", RUNTIME_FILENAME, old_version));
    if !base.exists() {
        return base;
    }
    let mut n = 2u32;
    loop {
        let candidate = root.join(format!("{}.bak-{}-{}", RUNTIME_FILENAME, old_version, n));
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_vault(name: &str, runtime_content: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("spore-refresh-{}-{}", std::process::id(), name));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join(RUNTIME_FILENAME), runtime_content).unwrap();
        d
    }

    fn fake_runtime(version: &str) -> String {
        format!(
            "---\nschemaVersion: 1\nversion: {}\ncodename: sporeAlpha\nstage: established\n---\n\n# fake runtime body\n",
            version
        )
    }

    #[test]
    fn semver_parses_and_rejects() {
        assert_eq!(parse_semver("0.3.0").unwrap(), (0, 3, 0));
        assert_eq!(parse_semver("10.20.30").unwrap(), (10, 20, 30));
        assert!(parse_semver("0.3").is_err());
        assert!(parse_semver("v0.3.0").is_err());
        assert!(parse_semver("0.3.0-beta").is_err());
    }

    #[test]
    fn upgrades_older_vault_with_backup() {
        let old = fake_runtime("0.2.0");
        let root = scratch_vault("upgrade", &old);
        refresh(Some(root.clone()), false).unwrap();

        let now = std::fs::read_to_string(root.join(RUNTIME_FILENAME)).unwrap();
        assert_eq!(now, RUNTIME, "runtime must be the embedded seed");
        let bak = std::fs::read_to_string(root.join("_sporeAlpha.md.bak-0.2.0")).unwrap();
        assert_eq!(bak, old, "backup must hold the pre-refresh bytes");
    }

    #[test]
    fn same_version_is_noop_without_force() {
        let engine_ver = embedded_runtime_version().unwrap();
        let cur = fake_runtime(&engine_ver);
        let root = scratch_vault("noop", &cur);
        refresh(Some(root.clone()), false).unwrap();

        let now = std::fs::read_to_string(root.join(RUNTIME_FILENAME)).unwrap();
        assert_eq!(now, cur, "no-op must leave the runtime untouched");
        assert!(
            !root
                .join(format!("_sporeAlpha.md.bak-{}", engine_ver))
                .exists(),
            "no-op must not create a backup"
        );
    }

    #[test]
    fn same_version_force_restamps_seed() {
        let engine_ver = embedded_runtime_version().unwrap();
        let cur = fake_runtime(&engine_ver); // simulated established form
        let root = scratch_vault("force", &cur);
        refresh(Some(root.clone()), true).unwrap();

        let now = std::fs::read_to_string(root.join(RUNTIME_FILENAME)).unwrap();
        assert_eq!(now, RUNTIME, "force must re-stamp the embedded seed");
        let bak = std::fs::read_to_string(
            root.join(format!("_sporeAlpha.md.bak-{}", engine_ver)),
        )
        .unwrap();
        assert_eq!(bak, cur, "force must still back up first");
    }

    #[test]
    fn downgrade_refused_even_forced() {
        let root = scratch_vault("downgrade", &fake_runtime("9.9.9"));
        let plain = refresh(Some(root.clone()), false);
        assert!(matches!(plain, Err(ref e) if matches!(e.kind, ErrKind::State)));
        let forced = refresh(Some(root), true);
        assert!(matches!(forced, Err(ref e) if matches!(e.kind, ErrKind::State)));
    }

    #[test]
    fn missing_version_frontmatter_is_state_error() {
        let root = scratch_vault("nover", "---\nschemaVersion: 1\n---\nbody\n");
        let r = refresh(Some(root), false);
        assert!(matches!(r, Err(ref e) if matches!(e.kind, ErrKind::State)));
    }

    #[test]
    fn backups_never_overwrite() {
        let engine_ver = embedded_runtime_version().unwrap();
        let cur = fake_runtime(&engine_ver);
        let root = scratch_vault("bakstack", &cur);
        refresh(Some(root.clone()), true).unwrap();
        // Re-stamp again: the second backup (of the now-seed runtime) must land
        // beside the first, not clobber it.
        refresh(Some(root.clone()), true).unwrap();
        assert!(root.join(format!("_sporeAlpha.md.bak-{}", engine_ver)).exists());
        assert!(root
            .join(format!("_sporeAlpha.md.bak-{}-2", engine_ver))
            .exists());
    }
}
