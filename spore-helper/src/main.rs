//! `spore` — the Spore filesystem seam helper.
//!
//! A dumb, deterministic verb executor. It carries no doctrine, decides nothing
//! about *when* to write, and performs no auto-anything — all judgment stays in
//! the runtime and the AI. Its CLI *is* the verb seam (§7 of the runtime): each
//! subcommand maps 1:1 to a runtime abstract verb.
//!
//! Guarantees baked into the tool (so they are structural, not just doctrine):
//!   - the vault-root guard: no path outside the vault root is ever touched;
//!   - atomic writes: temp file -> fsync -> atomic rename over the target;
//!   - read-after-write verify: every write is read back and compared.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

mod fsops;
mod frontmatter;
mod init;
mod links;
mod query;
mod refresh;
mod vault;

/// Categorised failure. The message on stderr + a non-zero exit is what the
/// runtime reads to render a STOP block (§2 of the runtime).
#[derive(Debug)]
pub struct SporeError {
    pub kind: ErrKind,
    pub msg: String,
}

#[derive(Debug, Clone, Copy)]
pub enum ErrKind {
    Usage,     // malformed invocation
    Guard,     // path escaped the vault root (runtime Category A)
    NotFound,  // target missing
    Verify,    // read-after-write mismatch (runtime Category E)
    State,     // malformed frontmatter / unreadable state (runtime Category B)
    Io,        // underlying IO failure
}

impl ErrKind {
    fn tag(self) -> &'static str {
        match self {
            ErrKind::Usage => "usage",
            ErrKind::Guard => "guard",
            ErrKind::NotFound => "not-found",
            ErrKind::Verify => "verify",
            ErrKind::State => "state",
            ErrKind::Io => "io",
        }
    }
    fn code(self) -> u8 {
        match self {
            ErrKind::Usage => 2,
            ErrKind::Guard => 3,
            ErrKind::NotFound => 4,
            ErrKind::Verify => 5,
            ErrKind::State => 6,
            ErrKind::Io => 7,
        }
    }
}

impl SporeError {
    pub fn new(kind: ErrKind, msg: impl Into<String>) -> Self {
        SporeError { kind, msg: msg.into() }
    }
}

impl From<std::io::Error> for SporeError {
    fn from(e: std::io::Error) -> Self {
        let kind = if e.kind() == std::io::ErrorKind::NotFound {
            ErrKind::NotFound
        } else {
            ErrKind::Io
        };
        SporeError::new(kind, e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, SporeError>;

/// Parsed command line.
struct Cli {
    vault_override: Option<PathBuf>,
    verb: String,
    /// `key=value` args, in a stable order for deterministic output.
    kvs: BTreeMap<String, String>,
    /// bare positional args (e.g. the path for `init`).
    positionals: Vec<String>,
}

fn parse_args() -> Result<Cli> {
    let mut args = std::env::args().skip(1);
    let mut vault_override = None;
    let mut verb: Option<String> = None;
    let mut kvs = BTreeMap::new();
    let mut positionals = Vec::new();

    while let Some(tok) = args.next() {
        if tok == "--vault" {
            let v = args.next().ok_or_else(|| {
                SporeError::new(ErrKind::Usage, "--vault requires a path argument")
            })?;
            vault_override = Some(PathBuf::from(v));
        } else if let Some(rest) = tok.strip_prefix("--vault=") {
            vault_override = Some(PathBuf::from(rest));
        } else if verb.is_none() {
            verb = Some(tok);
        } else if let Some(eq) = tok.find('=') {
            let (k, v) = tok.split_at(eq);
            kvs.insert(k.to_string(), v[1..].to_string());
        } else {
            positionals.push(tok);
        }
    }

    let verb = verb.ok_or_else(|| SporeError::new(ErrKind::Usage, USAGE.to_string()))?;
    Ok(Cli { vault_override, verb, kvs, positionals })
}

const USAGE: &str = "\
spore — the Spore filesystem seam helper

USAGE:
  spore [--vault <root>] <verb> [key=value ...]

SEAM VERBS (operate on the resolved vault):
  vault                                  print the resolved vault root + name
  read              path=P               print a note's content
  create            path=P content=-     create/overwrite (body via stdin)
  append            path=P content=-     append to end (body via stdin)
  prepend           path=P content=-     prepend after frontmatter (body via stdin)
  move              from=A to=B          move a note, rewriting [[links]]
  rename            path=P newname=N     rename a note, rewriting [[links]]
  search            query=Q              free-text scan (excludes the runtime file)
  frontmatter-query name=K [value=V]     files whose frontmatter has key K
  tags                                   list tags across the vault
  property-set      path=P key=K value=V surgical frontmatter write (preserves other keys)
  property-remove   path=P key=K         surgical frontmatter delete

COLD-START / MAINTENANCE / METADATA:
  init  [path] [force=1]                 stamp a new vault with the runtime
  refresh [path] [force=1]               update the vault's runtime to the one this
                                         binary carries (backs up first; no-op when
                                         current; refuses downgrade; force re-stamps)
  version                                print binary version + supported runtime
                                         schema + embedded runtime version

Content of `content=-` is read from stdin.";

fn read_stdin() -> Result<String> {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s)?;
    Ok(s)
}

/// If `content` is present and equals `-`, replace it with stdin.
fn resolve_content(kvs: &BTreeMap<String, String>) -> Result<String> {
    match kvs.get("content") {
        Some(c) if c == "-" => read_stdin(),
        Some(c) => Ok(c.clone()),
        None => Err(SporeError::new(ErrKind::Usage, "missing required arg: content=<text|->")),
    }
}

fn need<'a>(kvs: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str> {
    kvs.get(key)
        .map(|s| s.as_str())
        .ok_or_else(|| SporeError::new(ErrKind::Usage, format!("missing required arg: {}=…", key)))
}

fn run() -> Result<()> {
    let cli = parse_args()?;

    // Verbs that do not require an existing vault.
    match cli.verb.as_str() {
        "version" | "--version" | "-V" => {
            init::print_version();
            return Ok(());
        }
        "help" | "--help" | "-h" => {
            println!("{}", USAGE);
            return Ok(());
        }
        "init" => {
            let target = cli.positionals.first().cloned();
            return init::init(target, cli.kvs.contains_key("force"));
        }
        "refresh" => {
            // Vault from positional path, --vault, or walk-up from cwd.
            let target = cli
                .positionals
                .first()
                .map(PathBuf::from)
                .or(cli.vault_override.clone());
            return refresh::refresh(target, cli.kvs.contains_key("force"));
        }
        _ => {}
    }

    // Everything else operates on the resolved vault.
    let root = vault::resolve(cli.vault_override.as_deref())?;

    match cli.verb.as_str() {
        "vault" => {
            let name = root.file_name().and_then(|n| n.to_str()).unwrap_or("");
            println!("{}\t{}", name, root.display());
        }
        "read" => {
            let p = vault::guard(&root, need(&cli.kvs, "path")?)?;
            print!("{}", fsops::read(&p)?);
        }
        "create" => {
            let p = vault::guard(&root, need(&cli.kvs, "path")?)?;
            fsops::create(&p, &resolve_content(&cli.kvs)?)?;
            eprintln!("spore: wrote {}", p.display());
        }
        "append" => {
            let p = vault::guard(&root, need(&cli.kvs, "path")?)?;
            fsops::append(&p, &resolve_content(&cli.kvs)?)?;
            eprintln!("spore: appended {}", p.display());
        }
        "prepend" => {
            let p = vault::guard(&root, need(&cli.kvs, "path")?)?;
            fsops::prepend(&p, &resolve_content(&cli.kvs)?)?;
            eprintln!("spore: prepended {}", p.display());
        }
        "property-set" => {
            let p = vault::guard(&root, need(&cli.kvs, "path")?)?;
            let key = need(&cli.kvs, "key")?;
            let value = need(&cli.kvs, "value")?;
            let old = fsops::read(&p)?;
            let new = frontmatter::set(&old, key, value)?;
            fsops::atomic_write(&p, &new)?;
            eprintln!("spore: set {}={} in {}", key, value, p.display());
        }
        "property-remove" => {
            let p = vault::guard(&root, need(&cli.kvs, "path")?)?;
            let key = need(&cli.kvs, "key")?;
            let old = fsops::read(&p)?;
            let new = frontmatter::remove(&old, key)?;
            fsops::atomic_write(&p, &new)?;
            eprintln!("spore: removed {} from {}", key, p.display());
        }
        "rename" => {
            let p = vault::guard(&root, need(&cli.kvs, "path")?)?;
            let newname = need(&cli.kvs, "newname")?;
            let n = links::rename(&root, &p, newname)?;
            eprintln!("spore: renamed -> {} ({} link(s) rewritten)", n.path.display(), n.links);
        }
        "move" => {
            let from = vault::guard(&root, need(&cli.kvs, "from")?)?;
            let to = vault::guard(&root, need(&cli.kvs, "to")?)?;
            let n = links::mv(&root, &from, &to)?;
            eprintln!("spore: moved -> {} ({} link(s) rewritten)", to.display(), n);
        }
        "search" => {
            let q = need(&cli.kvs, "query")?;
            query::search(&root, q)?;
        }
        "frontmatter-query" => {
            let name = need(&cli.kvs, "name")?;
            query::frontmatter_query(&root, name, cli.kvs.get("value").map(|s| s.as_str()))?;
        }
        "tags" => {
            query::tags(&root)?;
        }
        other => {
            return Err(SporeError::new(
                ErrKind::Usage,
                format!("unknown verb: {}\n\n{}", other, USAGE),
            ));
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("spore: error[{}]: {}", e.kind.tag(), e.msg);
            ExitCode::from(e.kind.code())
        }
    }
}
