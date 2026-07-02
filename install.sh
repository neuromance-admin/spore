#!/bin/sh
# Spore α — one-command installer for the `spore` binary (the substrate seam).
#
#   curl -fsSL https://raw.githubusercontent.com/neuromance-admin/spore/main/install.sh | sh
#
# Downloads the prebuilt binary for your platform from the latest GitHub release,
# installs it to ~/.spore/bin, and puts that on your PATH. No build tools needed.
#
# Env overrides:
#   SPORE_VERSION=vX.Y.Z   install a specific release (default: latest)
#   SPORE_BIN_DIR=/path    install location (default: ~/.spore/bin)
set -eu

REPO="neuromance-admin/spore"
BIN_DIR="${SPORE_BIN_DIR:-$HOME/.spore/bin}"
VERSION="${SPORE_VERSION:-latest}"

say() { printf '▸ %s\n' "$1"; }
die() { printf 'spore-install: %s\n' "$1" >&2; exit 1; }

command -v curl >/dev/null 2>&1 || die "curl is required"

# --- detect platform --------------------------------------------------------
os="$(uname -s)"
arch="$(uname -m)"
case "$os" in
  Darwin) os_tag="apple-darwin" ;;
  Linux)  os_tag="unknown-linux-gnu" ;;
  *)      die "unsupported OS '$os' (macOS and Linux are supported)" ;;
esac
case "$arch" in
  arm64|aarch64) arch_tag="aarch64" ;;
  x86_64|amd64)  arch_tag="x86_64" ;;
  *)             die "unsupported architecture '$arch'" ;;
esac
# macOS ships for Apple Silicon (arm64) only.
if [ "$os" = Darwin ] && [ "$arch_tag" != aarch64 ]; then
  die "Spore ships for Apple Silicon (arm64) Macs only — this looks like an Intel Mac."
fi
# Asset name == the Rust target triple, e.g. spore-aarch64-apple-darwin
asset="spore-${arch_tag}-${os_tag}"

if [ "$VERSION" = latest ]; then
  base="https://github.com/$REPO/releases/latest/download"
else
  base="https://github.com/$REPO/releases/download/$VERSION"
fi

# --- download ---------------------------------------------------------------
say "Downloading $asset ($VERSION)…"
tmp="$(mktemp)"
trap 'rm -f "$tmp" "$tmp.sha"' EXIT
curl -fSL --proto '=https' --tlsv1.2 "$base/$asset" -o "$tmp" \
  || die "download failed — no published release with asset '$asset'?
         url: $base/$asset"

# --- verify checksum (if the release ships one) -----------------------------
if curl -fsSL "$base/$asset.sha256" -o "$tmp.sha" 2>/dev/null; then
  want="$(awk '{print $1}' "$tmp.sha")"
  if command -v shasum >/dev/null 2>&1; then
    got="$(shasum -a 256 "$tmp" | awk '{print $1}')"
  elif command -v sha256sum >/dev/null 2>&1; then
    got="$(sha256sum "$tmp" | awk '{print $1}')"
  else
    got=""
  fi
  if [ -n "$got" ] && [ "$got" != "$want" ]; then
    die "checksum mismatch — refusing to install (expected $want, got $got)"
  fi
  [ -n "$got" ] && say "Checksum verified."
fi

# --- install ----------------------------------------------------------------
mkdir -p "$BIN_DIR"
chmod +x "$tmp"
mv "$tmp" "$BIN_DIR/spore"
trap - EXIT
# curl-downloaded files aren't Gatekeeper-quarantined, but strip defensively.
[ "$os" = Darwin ] && xattr -d com.apple.quarantine "$BIN_DIR/spore" 2>/dev/null || true
say "Installed to $BIN_DIR/spore"

# --- ensure PATH ------------------------------------------------------------
case ":$PATH:" in
  *":$BIN_DIR:"*) on_path=1 ;;
  *)              on_path=0 ;;
esac
if [ "$on_path" -eq 0 ]; then
  line='export PATH="$HOME/.spore/bin:$PATH"'
  case "${SHELL:-}" in
    */zsh)  profile="$HOME/.zshrc" ;;
    */bash) profile="$HOME/.bashrc" ;;
    *)      profile="$HOME/.profile" ;;
  esac
  if ! grep -qF "$line" "$profile" 2>/dev/null; then
    printf '\n# Spore binary (spore α)\n%s\n' "$line" >> "$profile"
    say "Added ~/.spore/bin to PATH in $profile"
  fi
  say "Run:  source $profile   (or open a new terminal) so 'spore' is available."
else
  "$BIN_DIR/spore" version || true
fi

cat <<EOF

✓ spore installed.

Next:
  spore init ~/path/to/MyVault      # create a vault
  # then open Claude Code in that folder and say: "read _sporeAlpha.md"

Uninstall: rm -rf ~/.spore/bin   (and remove the "Spore binary" line from your shell profile)
EOF
