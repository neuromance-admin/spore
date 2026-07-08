---
name: refresh
description: Update this vault's runtime to the newer one the spore binary carries — backed up first, versions compared, nothing else touched
disable-model-invocation: true
---

The owner has invoked `/spore:refresh`. They are consenting to update this vault's runtime file to the runtime the `spore` binary carries baked in.

Execute per the loaded sporeAlpha runtime's **§6 (Commands → `/spore:refresh` discipline)**:

1. Run `spore refresh`. **The binary owns the entire file operation** — never hand-write runtime content yourself (Hard Floor #2). The binary compares the vault runtime's `version:` frontmatter against its embedded runtime, then:
   - **newer available** → backs up the current runtime (`_sporeAlpha.md.bak-<oldversion>`) and stamps the new one (atomic + read-back verified);
   - **already current** → no-op, nothing written;
   - **vault ahead of the binary** → refuses (downgrade guard).
2. **Relay the binary's outcome verbatim**: refreshed (old → new version, backup path, "applies next launch"), already current, or refused (direct the owner to update the `spore` binary via its installer first).
3. After a successful refresh, finish the current session normally on the runtime already in context — the **next launch** boots the new runtime (a seed; it re-slims to established form after boot, runtime §13).

Refresh touches **only** the runtime file and its backup. `Map.md`, `Rules/`, `Sessions/`, `Inbox/`, and personas are never in scope.

If the sporeAlpha runtime is not loaded in this session, `spore refresh` still works standalone — run it from the vault directory (or pass the vault path) and relay the outcome; then suggest the owner hand `_sporeAlpha.md` to Claude Code to boot the refreshed runtime.
