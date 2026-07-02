//! Cold-start commands — the moments before any Claude Code session exists.
//!
//! `init` stamps the embedded canonical runtime into a new vault (it does NOT
//! germinate — personas/Purpose/rules need owner consent in-session, §4 of the
//! runtime). `version` drives the boot version handshake (§3 Step 0).

use crate::{ErrKind, Result, SporeError};
use std::io::Write;
use std::path::{Path, PathBuf};

/// The canonical runtime, embedded at build time. The build therefore couples
/// the binary to a specific runtime version — `init` always stamps *this* one.
const RUNTIME: &str = include_str!("../../_sporeAlpha.v0.2.md");
const RUNTIME_FILENAME: &str = "_sporeAlpha.v0.2.md";

/// Runtime schemaVersion(s) this binary understands (for the handshake).
const SUPPORTED_SCHEMA: &str = "1";

pub fn print_version() {
    println!("spore {}", env!("CARGO_PKG_VERSION"));
    println!("runtime-file: {}", RUNTIME_FILENAME);
    println!("runtime-schema: {}", SUPPORTED_SCHEMA);
}

fn prompt(label: &str) -> Result<String> {
    print!("{}", label);
    std::io::stdout().flush().ok();
    let mut s = String::new();
    std::io::stdin().read_line(&mut s)?;
    Ok(s.trim().to_string())
}

pub fn init(target: Option<String>, force: bool) -> Result<()> {
    // Resolve the target folder — interactive if not supplied.
    let folder = match target {
        Some(t) => t,
        None => {
            let f = prompt("Vault folder (path to create or use): ")?;
            if f.is_empty() {
                return Err(SporeError::new(ErrKind::Usage, "no vault folder given".to_string()));
            }
            f
        }
    };
    let root = PathBuf::from(&folder);
    let runtime_path = root.join(RUNTIME_FILENAME);

    if runtime_path.exists() && !force {
        return Err(SporeError::new(
            ErrKind::Io,
            format!(
                "{} already exists — this looks like a Spore vault already (pass force=1 to overwrite)",
                runtime_path.display()
            ),
        ));
    }

    std::fs::create_dir_all(&root)?;

    // Stamp the runtime (atomic + verified via fsops).
    crate::fsops::atomic_write(&runtime_path, RUNTIME)?;

    // Scaffold the standard vault folders (empty).
    for sub in ["Sessions", "Inbox"] {
        std::fs::create_dir_all(root.join(sub))?;
    }

    print_handoff(&root);
    Ok(())
}

fn print_handoff(root: &Path) {
    let name = root.file_name().and_then(|n| n.to_str()).unwrap_or("your vault");
    println!("🌱 Stamped a new Spore vault at {}", root.display());
    println!();
    println!("  {}", RUNTIME_FILENAME);
    println!("  Sessions/");
    println!("  Inbox/");
    println!();
    println!("Next: open Claude Code with {} as the working directory and hand it", name);
    println!("the runtime — \"read {}\". First launch sets up your", RUNTIME_FILENAME);
    println!("personas, this vault's purpose, and the starter rules.");
}
