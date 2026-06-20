#!/usr/bin/env bash
# Sign macOS binaries for distribution.
set -euo pipefail

NAME="agent-body"
IDENTITY="${1:-"Developer ID Application"}"

for target in x86_64-apple-darwin aarch64-apple-darwin; do
  binary="dist/${NAME}-${target}"
  if [[ -f "$binary" ]]; then
    echo "Signing $binary ..."
    codesign --force --options runtime --sign "$IDENTITY" "$binary"
    codesign -vvv "$binary"
  fi
done
