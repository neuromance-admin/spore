---
name: map-rebuild
description: Rebuild this vault's Map from session history, with ⏸ preview before write
disable-model-invocation: true
---

The owner has invoked `/spore:map-rebuild`. Execute the rebuild per the loaded sporeAlpha runtime's **§10 (Map → `/spore:map-rebuild` command)**:

1. Read all session nodes in `<vault>/Sessions/`.
2. Re-synthesise Threads from session history.
3. Regenerate Recent from query.
4. **Preserve Purpose verbatim** (owner-authored; never auto-touched).
5. Bump `updated`.
6. Show preview in a ⏸ ASK block (template in §10) — request explicit consent.

On consent → active-vault guard (§7), write via the verb seam, read-after-write verified.
On cancel → nothing changes.

**Edge case:** if `Map.md` doesn't exist when `/spore:map-rebuild` is invoked → vault first-boot semantics (§4 Moment 2). Ask the owner for Purpose; nothing to preserve.

If the sporeAlpha runtime is not loaded:

> "The Spore runtime isn't loaded — hand `_sporeAlpha.v*.md` (in your vault root) to Claude Code first, then re-run `/spore:map-rebuild`."
