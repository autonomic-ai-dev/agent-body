#!/usr/bin/env bash
# Copy the local release binary to ~/.local/bin
set -euo pipefail

NAME="agent-body"
TARGET="${1:-aarch64-apple-darwin}"
SOURCE="target/$TARGET/release/$NAME"
DEST="$HOME/.local/bin/$NAME"

if [[ ! -f "$SOURCE" ]]; then
  echo "Build first: cargo build --release --target $TARGET -p $NAME"
  exit 1
fi

cp "$SOURCE" "$DEST"
chmod +x "$DEST"
echo "Installed $SOURCE -> $DEST"
