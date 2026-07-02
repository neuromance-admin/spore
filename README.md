# Spore α plugin (v0.3.0)

The slash-command UX layer for the **Spore α** runtime — a Markdown-based AI memory harness that runs inside Claude Code. The substrate seam is the self-owned **`spore` helper** (a small local binary); Obsidian is an optional Markdown viewer, not a requirement.

This plugin provides six namespaced slash commands:

| Command | What it does |
|---|---|
| `/spore:save` | Save the current session — write a session node + refresh the Map |
| `/spore:rules` | View or manage this vault's rules |
| `/spore:inbox` | Work this vault's Inbox: list contents, propose filing |
| `/spore:map-rebuild` | Rebuild the Map from session history (with ⏸ preview) |
| `/spore:refresh` | Update this vault's runtime to the one the `spore` helper carries (backup first; no-op when current; refuses downgrade) |
| `/spore:help` | Show the command list, with a state-aware header |

Each command is a thin trigger that delegates to the doctrine in the runtime markdown file (`_sporeAlpha.md`) at your vault's root.

## Architecture

The plugin and the runtime are **separate distribution artifacts**:

- The **plugin** is installed once per user, via plugin marketplace. It provides the slash-command UX (autocomplete, native command surface, namespacing).
- The **runtime** is stamped once per vault at the vault's root (`spore init`). It carries the doctrine — what each command actually means, how the AI should behave during a session, the schemas for session nodes / rules / Maps / personas. As of v0.3 its filename is **frozen at `_sporeAlpha.md`** — the version lives in the file's frontmatter and in `spore version`, so runtime updates are a clean single-file swap (`/spore:refresh`).

The plugin commands assume the runtime is in context. If it isn't, the commands surface a friendly *"hand the runtime to Claude Code first"* message instead of failing silently.

## Requirements

- The **`spore` helper** installed on PATH (via the install bootstrap — the per-user foundation, once). This is the substrate seam the runtime drives.
- **Claude Code** (Max plan recommended).
- The **sporeAlpha runtime file** (`_sporeAlpha.md`, v0.3.0 or later) at the root of a vault — stamped by `spore init`.
- **Obsidian is optional** — a nice Markdown viewer for the graph/backlinks, but nothing requires it.

## Installation

Two one-time steps. **1 — the `spore` helper** (a prebuilt binary; no build tools needed), in a terminal:

```
curl -fsSL https://raw.githubusercontent.com/neuromance-admin/spore-claudecode-plugin/main/install.sh | sh
```

It installs to `~/.spore/bin` and puts it on your PATH. Verify with `spore version`. (Developers can build from source instead — see the `spore-helper/` crate.)

**2 — the plugin** (the `/spore:*` slash commands), inside any Claude Code session:

```
/plugin marketplace add neuromance-admin/spore-claudecode-plugin
/plugin install spore@neuromance-co
```

Then create a vault with `spore init ~/path/to/MyVault` and hand the runtime to Claude Code.

## Usage

1. Create a vault with `spore init <path>`.
2. Open Claude Code with your vault's root as the working directory.
3. Hand `_sporeAlpha.md` (the runtime in your vault root) to Claude Code — *"read this file"*.
4. On first launch, Spore walks you through a brief first-use dialog (your name, what to call your AI, what this vault is for, whether to stamp the three starter rules). Once done, you're in **Ready** state.
5. Use the slash commands — or just ask in natural language. *"Save the session"* and `/spore:save` land at the same place.

**Updating a vault's runtime:** after updating the `spore` helper (re-run the installer), any vault on an older runtime shows a gentle notice at boot; run `/spore:refresh` to update it. The old runtime is backed up beside the new one (`_sporeAlpha.md.bak-<version>`), and nothing else in the vault is touched.

## What this plugin does NOT do

- It does not contain the doctrine. The runtime markdown file does. The plugin is a UX layer.
- It does not modify your vault directly. Every vault write comes from the runtime-defined routines, going through the verb seam (the `spore` helper), gated by the helper's structural vault-root guard and read-after-write verification.
- It does not auto-update the runtime. Boot *tells* you when a newer runtime is available; only `/spore:refresh`, run by you, applies it. A signed remote update channel remains parked.

## See also

- Runtime source: `_sporeAlpha.md` (in any vault you've Spore-ified) — the complete doctrine, ~1091 lines, §1–§13 + Changelog.
- Seam helper source: the `spore-helper/` crate in this repo (authored canonically in the upstream `SporeSource` workshop).
- Design rationale: the upstream `SporeSource` repo.

## License

MIT
