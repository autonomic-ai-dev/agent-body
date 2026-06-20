#!/usr/bin/env bash
# Extract the changelog entry for the latest version.
set -euo pipefail

CHANGELOG="${1:-CHANGELOG.md}"

awk '
  /^## \[/ { if (found) exit; found=1; next }
  /^## \[/ { next }
  found { print }
' "$CHANGELOG"
