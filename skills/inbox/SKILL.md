---
name: inbox
description: Work this vault's Inbox — list contents, propose filing into the vault structure, write on consent
disable-model-invocation: true
---

The owner has invoked `/spore:inbox`. Work the vault's Inbox per the Spore design principle: a passive dump zone, on-demand processing only — never auto-file.

1. Read `<vault>/Inbox/` — list contents (filenames + a short content preview of each).
2. For each item, propose where it could land in the vault structure (a category folder, an existing concept note to update, a new atomic note).
3. Apply atomic-note discipline (§8.4) — prefer updating an existing note over creating a new one; check for related notes via `frontmatter-query` / `search`.
4. Write only on explicit consent. Each write through the verb seam, read-after-write verified.
5. The owner may also direct items to be left in the Inbox indefinitely — that's fine, the Inbox is theirs.

If the sporeAlpha runtime is not loaded:

> "The Spore runtime isn't loaded — hand `_sporeAlpha.v*.md` (in your vault root) to Claude Code first, then re-run `/spore:inbox`."
