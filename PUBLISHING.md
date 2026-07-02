# Publishing the Spore helper (`spore`) — release + one-command install

The one-command installer (`install.sh`) downloads a **prebuilt** `spore` binary from this repo's GitHub Releases. This doc is how those binaries get built and published.

## Repo layout this expects

```
spore-claudecode-plugin/
├── .claude-plugin/           ← the Claude Code plugin (slash commands)
├── skills/                   ← plugin skills
├── spore-helper/             ← the Rust crate (SEAM binary source) — synced from SporeSource
├── _sporeAlpha.md            ← the runtime — synced from SporeSource; the crate embeds it
├── install.sh                ← the one-command installer users curl
├── PUBLISHING.md             ← this file
└── .github/workflows/release.yml
```

`spore-helper/` and `_sporeAlpha.md` are authored canonically in the **SporeSource
workshop** (`build/sporeAlpha-v0.3/`). They are *synced* into this repo before a release
(same dual-source discipline as rules/personas). SporeSource stays the source of truth.

**Why the runtime lives at the repo root:** the crate embeds it with
`include_str!("../../_sporeAlpha.md")` — a path that resolves to one level *above*
the crate, i.e. the repo root. Keep the two in step, or `cargo build` fails.

## One-time / per-release: sync the crate + runtime

From the SporeSource workshop, copy both into the plugin repo (excluding build output):

```sh
SRC=/path/to/SporeSource/build/sporeAlpha-v0.3
DST=/path/to/spore-claudecode-plugin

rsync -a --delete --exclude target/ "$SRC/spore-helper/" "$DST/spore-helper/"
cp "$SRC/_sporeAlpha.md" "$DST/_sporeAlpha.md"
```

Commit both. The crate's `Cargo.toml` version (currently `0.3.0`) is what ships.

## Cut a release

The version should match the runtime's `minHelper` and the binary's `Cargo.toml` version.

```sh
git tag v0.3.0
git push origin v0.3.0
```

That fires `.github/workflows/release.yml`, which:
1. builds `spore` on `macos-14` (Apple Silicon / arm64 — the only supported target),
2. packages it as `spore-aarch64-apple-darwin` + a `.sha256`,
3. creates the GitHub Release and uploads the assets,
4. verifies the expected asset names are present (guards against drift from `install.sh`).

You can also trigger it manually from the **Actions → release → Run workflow** button (`workflow_dispatch`).

## After the release

The one-liner is live:

```sh
curl -fsSL https://raw.githubusercontent.com/neuromance-admin/spore-claudecode-plugin/main/install.sh | sh
```

`install.sh` resolves `releases/latest`, so it always fetches the newest tagged release. Pin a specific one with `SPORE_VERSION=v0.3.0`.

## Asset-name contract (don't break this)

`install.sh` builds the asset name as `spore-<arch>-<os>` where:
- arch ∈ {`aarch64`, `x86_64`}, os ∈ {`apple-darwin`, `unknown-linux-gnu`}

…which equals the Rust target triple. The workflow names assets `spore-${{ matrix.target }}`, so they line up. If you add a platform, add it to **both** the workflow matrix and `install.sh`'s `case` blocks. The workflow's "Verify expected assets" step is a backstop.

## Not included (by choice / macOS-first)

- **Intel Macs (`x86_64-apple-darwin`)** — intentionally dropped; Spore ships for Apple Silicon only. `install.sh` gives Intel Macs a clear message rather than a 404.
- **Linux** builds — add `x86_64-unknown-linux-gnu` / `aarch64-unknown-linux-gnu` to the matrix (`install.sh` already handles the names).
- **Windows** — would need a `.exe` asset + a PowerShell installer.
- **A double-click GUI installer** (`.pkg`/`.dmg`) — needs Apple Developer ID signing + notarization; deferred by design (`curl | sh` avoids Gatekeeper because curl-downloaded files aren't quarantined).
