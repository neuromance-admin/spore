# Spore α plugin (v0.2.0)

The slash-command UX layer for the **Spore α** runtime — a Markdown-based AI memory harness that runs inside Claude Code. As of runtime v0.2 the substrate seam is the self-owned **`spore` helper** (a small local binary), not the Obsidian CLI — Obsidian is now an optional Markdown viewer, no longer a requirement.

This plugin provides five namespaced slash commands:

| Command | What it does |
|---|---|
| `/spore:save` | Save the current session — write a session node + refresh the Map |
| `/spore:rules` | View or manage this vault's rules |
| `/spore:inbox` | Work this vault's Inbox: list contents, propose filing |
| `/spore:map-rebuild` | Rebuild the Map from session history (with ⏸ preview) |
| `/spore:help` | Show the command list, with a state-aware header |

Each command is a thin trigger that delegates to the doctrine in the runtime markdown file (`_sporeAlpha.v*.md`) at your active Obsidian vault's root.

## Architecture

The plugin and the runtime are **separate distribution artifacts**:

- The **plugin** is installed once per user, via plugin marketplace. It provides the slash-command UX (autocomplete, native command surface, namespacing).
- The **runtime** is downloaded once per vault and dropped at the vault's root. It carries the doctrine — what each command actually means, how the AI should behave during a session, the schemas for session nodes / rules / Maps / personas.

The plugin commands assume the runtime is in context. If it isn't, the commands surface a friendly *"hand the runtime to Claude Code first"* message instead of failing silently.

## Requirements

- The **`spore` helper** installed on PATH (via the install bootstrap — the per-user foundation, once). This is the substrate seam the runtime drives.
- **Claude Code** (Max plan recommended).
- The **sporeAlpha runtime file** (`_sporeAlpha.v0.2.md` or later) at the root of a vault — dropped by `spore init`, or by hand.
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

1. Create a vault with `spore init <path>` (or drop `_sporeAlpha.v0.2.md` into a folder by hand).
2. Open Claude Code with your vault's root as the working directory.
3. Hand `_sporeAlpha.v0.2.md` (the runtime in your vault root) to Claude Code — *"read this file"*.
4. On first launch, Spore walks you through a brief first-use dialog (your name, what to call your AI, what this vault is for, whether to stamp the three starter rules). Once done, you're in **Ready** state.
5. Use the slash commands — or just ask in natural language. *"Save the session"* and `/spore:save` land at the same place.

## What this plugin does NOT do

- It does not contain the doctrine. The runtime markdown file does. The plugin is a UX layer.
- It does not modify your vault directly. Every vault write comes from the runtime-defined routines, going through the verb seam (the `spore` helper), gated by the helper's structural vault-root guard and read-after-write verification.
- It does not auto-update the runtime. The runtime is dropped per-vault by the owner; signed-manifest update channel is parked for v0.1.

## See also

- Runtime source: `_sporeAlpha.v0.2.md` (in any vault you've Spore-ified) — the complete doctrine, ~1080 lines, §1–§13 + Changelog.
- Seam helper source: `build/sporeAlpha-v0.2/spore-helper/` in the upstream `SporeSource` repo.
- Design rationale: the upstream `SporeSource` repo.

## License

MIT
