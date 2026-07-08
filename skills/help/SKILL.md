---
name: help
description: Show the Spore command list, with a state-aware header
disable-model-invocation: true
---

The owner has invoked `/spore:help`. Render the help per the loaded sporeAlpha runtime's **§6 (Commands → `/spore:help` derivation)**:

1. Read §6 of the runtime.
2. **Quote the command entries verbatim** — do not rephrase descriptions (anti-drift: runtime is the source of truth).
3. Prepend a one-line **state header** based on current state:
   - First-use in progress (§4 dialog active): *"Spore is in first-run setup. Answer the prompts to continue. Type `/spore:help` after setup for the full command list."* — no list shown.
   - Ready (default; post-handshake): *"Ready — <AI Name> · working in <vault> · N rules loaded."* — then the quoted command list.

If the sporeAlpha runtime is not loaded in this session, render this fallback list and tell the owner how to start:

```
sporeAlpha is not loaded in this session.

To start: hand `_sporeAlpha.md` (the runtime in your vault root) to Claude Code.

Once loaded, these commands are available:
  /spore:save        — save the session
  /spore:rules       — view or manage this vault's rules
  /spore:inbox       — work this vault's Inbox
  /spore:map-rebuild — rebuild the Map from session history
  /spore:audit       — vault hygiene audit (read-only; fixes only on consent)
  /spore:refresh     — update this vault's runtime to the one the binary carries
  /spore:help        — this help
```
