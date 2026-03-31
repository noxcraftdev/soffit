#!/bin/bash
set -euo pipefail

REPO="noxcraftdev/soffit"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# Detect OS and architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)  TARGET_OS="unknown-linux-gnu" ;;
  Darwin) TARGET_OS="apple-darwin" ;;
  *) echo "Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64) TARGET_ARCH="x86_64" ;;
  aarch64|arm64) TARGET_ARCH="aarch64" ;;
  *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

TARGET="${TARGET_ARCH}-${TARGET_OS}"

# Get latest release tag
LATEST=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed 's/.*"tag_name": "\(.*\)".*/\1/')

if [ -z "$LATEST" ]; then
  echo "Could not determine latest release"
  exit 1
fi

URL="https://github.com/$REPO/releases/download/$LATEST/soffit-${TARGET}.tar.gz"

echo "Installing soffit $LATEST for $TARGET..."
mkdir -p "$INSTALL_DIR"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

curl -fsSL "$URL" -o "$TMP/soffit.tar.gz"
tar xzf "$TMP/soffit.tar.gz" -C "$TMP"
install -m 755 "$TMP/soffit" "$INSTALL_DIR/soffit"

echo "Installed soffit to $INSTALL_DIR/soffit"

# Check if INSTALL_DIR is in PATH
if ! echo "$PATH" | tr ':' '\n' | grep -q "^$INSTALL_DIR$"; then
  echo ""
  echo "Add $INSTALL_DIR to your PATH:"
  echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
fi

# Auto-configure Claude Code statusline
CLAUDE_SETTINGS="$HOME/.claude/settings.json"
STATUSLINE='{"type": "command", "command": "soffit render", "padding": 0}'

configure_claude() {
  if ! command -v python3 &>/dev/null; then
    echo ""
    echo "Add to ~/.claude/settings.json:"
    echo "  \"statusLine\": $STATUSLINE"
    return
  fi

  mkdir -p "$HOME/.claude"
  python3 -c "
import json, sys, os
path = '$CLAUDE_SETTINGS'
try:
    with open(path) as f:
        settings = json.load(f)
except (FileNotFoundError, json.JSONDecodeError):
    settings = {}

if 'statusLine' in settings:
    current = settings['statusLine']
    if current.get('command') == 'soffit render':
        print('Claude Code already configured for soffit.')
        sys.exit(0)
    print('Claude Code has an existing statusLine config.')
    print('To use soffit, update it to:')
    print('  \"statusLine\": $STATUSLINE')
    sys.exit(0)

settings['statusLine'] = json.loads('$STATUSLINE')
with open(path, 'w') as f:
    json.dump(settings, f, indent=2)
    f.write('\n')
print('Configured Claude Code to use soffit statusline.')
"
}

configure_claude
