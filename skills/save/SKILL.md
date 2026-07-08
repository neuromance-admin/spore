---
name: save
description: Save the current Spore session — write a session node and refresh the Map per the runtime's doctrine
disable-model-invocation: true
---

The owner has invoked `/spore:save`. They are consenting to perform the save ritual on the current vault.

Execute per the loaded sporeAlpha runtime:

- **§7 (Verb-Seam Mapping)** — every write goes through the `spore` binary, which enforces the vault-root guard structurally (any path escaping the vault root is refused, exit 3). A guard refusal → 🛑 Category A STOP.
- **§8 (Memory Discipline)** — the save ritual: write the session node (schema §8.6), refresh the Map's Threads (re-synthesise from session content), regenerate the Map's Recent (query-driven), bump `updated`.
- **§8.6/§8.8 (gated fan-out)** — on heavy saves (≳3 touched areas or ≳10 session nodes to synthesise), delegate the vault-side work to read-only workers per §8.8; the session node's conversation-derived content is never delegated. Below the gate, work inline.
- **§10 (Map)** — per-part regen policy for refresh.

Every write goes through the verb seam (Hard Floor #3) and is read-after-write verified (Hard Floor #6). Any mismatch → 🛑 Category E STOP; report what landed, what didn't, what's recoverable.

If the sporeAlpha runtime is not loaded in this session, surface that to the owner first:

> "The Spore runtime isn't loaded — hand `_sporeAlpha.md` (in your vault root) to Claude Code first, then re-run `/spore:save`."
