#!/bin/sh
#
# txt2tex installer.
#
#   curl -fsSL https://raw.githubusercontent.com/whynotdeep7/txt2tex/main/install.sh | sh
#
# Downloads the prebuilt binary for this machine from the latest GitHub
# release and puts it on your PATH. That is all it does: no Rust toolchain, no
# editor, no changes to your shell config.
#
# POSIX sh, and it never reads stdin — stdin is the pipe when curled.

set -eu

REPO="whynotdeep7/txt2tex"
INSTALL_DIR="${TXT2TEX_INSTALL_DIR:-$HOME/.local/bin}"

if [ -t 1 ]; then
    BOLD=$(printf '\033[1m'); DIM=$(printf '\033[2m')
    RED=$(printf '\033[31m'); GREEN=$(printf '\033[32m'); YELLOW=$(printf '\033[33m')
    RESET=$(printf '\033[0m')
else
    BOLD=""; DIM=""; RED=""; GREEN=""; YELLOW=""; RESET=""
fi

say()  { printf '  %s\n' "$1"; }
ok()   { printf '  %s✓%s %s\n' "$GREEN" "$RESET" "$1"; }
warn() { printf '  %s!%s %s\n' "$YELLOW" "$RESET" "$1"; }
fail() { printf '\n  %s✗ %s%s\n\n' "$RED" "$1" "$RESET" >&2; exit 1; }

# ── which binary does this machine need? ──────────────────────
os=$(uname -s)
arch=$(uname -m)

case "$os/$arch" in
    Darwin/arm64)  asset="txt2tex-macos-arm64"   ;;
    Linux/x86_64)  asset="txt2tex-linux-x86_64"  ;;
    Darwin/x86_64)
        fail "Intel Macs have no prebuilt binary yet.
     Build from source instead:  cargo install --git https://github.com/$REPO"
        ;;
    *)
        fail "No prebuilt binary for $os on $arch.
     Build from source instead:  cargo install --git https://github.com/$REPO"
        ;;
esac

# 'latest/download' always redirects to the newest release, so this URL never
# needs updating when a version ships.
url="https://github.com/$REPO/releases/latest/download/$asset"

printf '\n  %stxt2tex%s  %s%s on %s%s\n\n' "$BOLD" "$RESET" "$DIM" "$os" "$arch" "$RESET"

command -v curl >/dev/null 2>&1 || fail "curl is required."

# ── download ──────────────────────────────────────────────────
tmp=$(mktemp) || fail "could not create a temp file."
# Clean up the partial download on any exit, successful or not.
trap 'rm -f "$tmp"' EXIT INT TERM

say "downloading $asset"
# curl's own error is silenced; the message below says more.
curl -fsSL "$url" -o "$tmp" 2>/dev/null || fail "download failed — no binary at
     $url

     Check that a release exists:  https://github.com/$REPO/releases"

[ -s "$tmp" ] || fail "downloaded an empty file."

# ── install ───────────────────────────────────────────────────
mkdir -p "$INSTALL_DIR" || fail "could not create $INSTALL_DIR"
chmod +x "$tmp"
# Move into place last, so a failure never leaves a broken binary on PATH.
mv -f "$tmp" "$INSTALL_DIR/txt2tex" || fail "could not write to $INSTALL_DIR"
trap - EXIT INT TERM

ok "installed to $INSTALL_DIR/txt2tex"

# ── things the user still needs to know ───────────────────────
case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
        warn "$INSTALL_DIR is not on your PATH. Add this to your shell config:"
        say ""
        say "    export PATH=\"$INSTALL_DIR:\$PATH\""
        say ""
        ;;
esac

# tectonic is a real requirement, but installing it is not this script's
# business — it is ~100 MB and every platform wants it a different way.
if ! command -v tectonic >/dev/null 2>&1; then
    warn "tectonic is not installed. txt2tex needs it to typeset PDFs:"
    say ""
    say "    macOS    brew install tectonic"
    say "    Linux    cargo install tectonic"
    say ""
fi

printf '  Run %stxt2tex%s to get started.\n\n' "$BOLD" "$RESET"
