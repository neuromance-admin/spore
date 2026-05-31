---
name: rules
description: View or manage this vault's rules — list currently loaded rules with their scopes and offer edit / disable
disable-model-invocation: true
---

The owner has invoked `/spore:rules`. Surface this vault's rules per the loaded sporeAlpha runtime's **§9 (Rules)**.

1. List currently-loaded vault rules — file path, summary, `ruleScope`, `loadPriority`.
2. Offer actions:
   - Edit a rule (open the file for owner edits).
   - Disable a rule (rename / move it out of `<vault>/Rules/` per owner direction).
   - Propose a new rule (if the owner describes a pattern they want promoted).
3. After any rule mutation → re-run §3 Step 4 (rules sweep) including the collision check; surface any new collision per the standard ⏸ block.

Hard Floor still applies — proposals only; never write without consent.

If the sporeAlpha runtime is not loaded:

> "The Spore runtime isn't loaded — hand `_sporeAlpha.v*.md` (in your vault root) to Claude Code first, then re-run `/spore:rules`."
