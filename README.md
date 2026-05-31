# Spore α plugin (v0.1.0)

The slash-command UX layer for the **Spore α** runtime — a Markdown-based AI memory harness for Obsidian vaults that runs inside Claude Code.

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

- **Obsidian** 1.12+ with the built-in CLI enabled (Settings → General → Command line interface).
- **Claude Code** (Max plan recommended).
- The **sporeAlpha runtime file** (`_sporeAlpha.v0.1.md` or later) dropped at the root of an Obsidian vault.

## Installation

```
/plugin marketplace add neuromance-admin/spore-claudecode-plugin
/plugin install spore@neuromance-co
```

## Usage

1. Open your Obsidian vault in Obsidian (with the CLI enabled).
2. Open Claude Code with your vault's root as the working directory.
3. Hand `_sporeAlpha.v0.1.md` (the runtime in your vault root) to Claude Code — *"read this file"*.
4. On first launch, Spore walks you through a brief first-use dialog (your name, what to call your AI, what this vault is for, whether to stamp the three starter rules). Once done, you're in **Ready** state.
5. Use the slash commands — or just ask in natural language. *"Save the session"* and `/spore:save` land at the same place.

## What this plugin does NOT do

- It does not contain the doctrine. The runtime markdown file does. The plugin is a UX layer.
- It does not modify your vault directly. Every vault write comes from the runtime-defined routines, going through the verb seam (Obsidian CLI), gated by the active-vault guard and read-after-write verification.
- It does not auto-update the runtime. The runtime is dropped per-vault by the owner; signed-manifest update channel is parked for v0.1.

## See also

- Runtime source: `_sporeAlpha.v0.1.md` (in any vault you've Spore-ified) — the complete doctrine, 993 lines, 12 sections + Changelog.
- Design rationale: the upstream `SporeSource` repo.

## License

MIT
