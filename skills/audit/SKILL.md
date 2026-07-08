---
name: audit
description: Run the vault hygiene audit — read-only workers sweep for duplicate concepts, broken wikilinks, Map drift, schema gaps, and stale open loops; report in chat, fixes only on consent
disable-model-invocation: true
---

The owner has invoked `/spore:audit`. They are asking for the vault hygiene audit — a **read-only** sweep of the whole vault.

Execute per the loaded sporeAlpha runtime's **§8.8 (Delegation — the audit pipeline)**:

1. **Fan out read-only workers** (the `spore-worker` agent), each instantiated **verbatim** from the runtime's §8.9 worker brief with the vault root passed explicitly (`spore --vault …`). The five checks, in parallel: duplicate/overlapping concepts (§8.4) · wikilinks pointing at missing files (§8.5) · Map Threads unsupported by session-node reality (§10) · nodes missing `schemaVersion`/`summary` frontmatter · `status: open` session nodes whose loops read as settled (§8.6). If worker machinery is unavailable, run the same checks yourself inline — the audit degrades, never errors.
2. **Filter the findings yourself** (significance filter, §8.1) — no worker output reaches the owner unfiltered.
3. **Render the report in chat.** Never auto-write it to the vault.
4. **Offer an optional fix plan** as a §4.3-style write-plan. Fixes land only on the owner's consent, through the verb seam (vault-root guarded, atomic, read-after-write verified). No consent → the vault is untouched.

The audit itself writes nothing — it is pure read. Workers are propose-only, seam-only, conversation-blind, one-vault (runtime §8.8).

If the sporeAlpha runtime is not loaded in this session, surface: *"Hand this vault's `_sporeAlpha.md` to Claude Code first — the audit runs under the runtime's doctrine."*
