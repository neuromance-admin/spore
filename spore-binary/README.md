# `spore` — the Spore filesystem seam helper

The deterministic verb executor behind the sporeAlpha v0.2 runtime. It replaces the Obsidian CLI as the substrate seam (§7): the runtime talks to the vault *only* through this binary. Rust, **zero external dependencies**, single ~450 KB static binary.

It is a **dumb executor** — no doctrine, no "when to write," no auto-anything. All judgment stays in the runtime and the AI. Its guarantees are structural, not advisory:

- **Vault-root guard** — every path is resolved and checked to be inside the vault root; an escape is refused (exit 3). Hard Floor #1 becomes a wall the tool won't cross.
- **Atomic + verified writes** — temp file → fsync → atomic rename → read-back compare. A mismatch is exit 5 (Category E).
- **Wikilink integrity** — `move`/`rename` rewrite `[[links]]` across the vault (the job Obsidian used to do).

See `../design-seam-swap.md` for the full contract and rationale.

## Verbs

```
spore [--vault <root>] <verb> [key=value ...]

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

init  [path]                           stamp a new vault with the embedded runtime
version                                binary version + supported runtime schema
```

`content=-` reads the note body from stdin (avoids argv escaping). The vault root is resolved by walking up from cwd to the nearest `_sporeAlpha.v*.md`, or given via `--vault`.

## Resolved edge-case behaviour

- **Wikilink matching is case-insensitive on the basename** (Obsidian-like). Preserves `|alias`, `#heading`, `^block`, and embeds (`![[ ]]`).
- **Cross-folder moves repoint path-qualified links.** A bare link (`[[note]]`) stays bare (it resolves by basename anywhere); a path-qualified link (`[[Drafts/note]]`) is repointed to the note's new vault-relative path (`[[Published/note]]`), or made bare if the note moved to the vault root. A rewrite that would produce identical text is left untouched and uncounted.
- **Links inside fenced code blocks (```` ``` ````/`~~~`) are never rewritten.**
- **Basename collision → STOP** (exit 6): if more than one note shares the old basename, link rewriting by basename is ambiguous, so the tool refuses rather than guess.
- **Search / frontmatter-query / tags exclude** the runtime file(s), `_sporeAlpha.shedding.tmp`, and write temps — the §7 exclusion is enforced by the tool.
- **Dotfolders** (`.git`, `.obsidian`, `.trash`) are skipped by the walk.

### Known alpha limitations

- **Symlink containment** relies on canonicalizing the longest existing path prefix; exotic symlink-into-vault setups aren't specially handled.
- **`tags`** parses inline `#tag` and simple `tags:` frontmatter (scalar / inline list); block-list `tags:` across multiple lines aren't aggregated yet.

## Build

```
cargo build --release      # -> target/release/spore
cargo test                 # 23 unit tests
```

The binary **embeds the canonical runtime** (`../_sporeAlpha.v0.2.md`) at build time via `include_str!`, so `init` always stamps the runtime version this binary shipped with. Editing that runtime file requires a rebuild to re-embed.

## Install (alpha, local)

```
mkdir -p ~/.spore/bin
cp target/release/spore ~/.spore/bin/spore
# ensure ~/.spore/bin is on PATH (add to your shell profile)
```

A public one-line bootstrap (GitHub release / Homebrew tap) is a packaging task for when v0.2 ships beyond the author's machine.
