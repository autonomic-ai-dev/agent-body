#!/usr/bin/env bash
# Install agent-body from latest GitHub release.
set -euo pipefail

REPO="autonomic-ai-dev/agent-body"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/github.sh
source "${SCRIPT_DIR}/lib/github.sh"

detect_arch() {
  local arch
  arch="$(uname -m)"
  case "$arch" in
    x86_64|amd64) echo "x86_64" ;;
    aarch64|arm64) echo "aarch64" ;;
    *) echo "unsupported-arch-$arch" >&2; exit 1 ;;
  esac
}

detect_os() {
  local os
  os="$(uname -s)"
  case "$os" in
    Darwin) echo "apple-darwin" ;;
    Linux)  echo "unknown-linux-gnu" ;;
    *)      echo "unsupported-os-$os" >&2; exit 1 ;;
  esac
}

sign_macos_binary() {
  local bin="$1"
  if [[ "$(uname -s)" != "Darwin" ]]; then
    return 0
  fi
  xattr -cr "$bin" 2>/dev/null || true
  codesign --force --sign - "$bin" 2>/dev/null || true
}

main() {
  local os arch asset tag url target
  os="$(detect_os)"
  arch="$(detect_arch)"
  asset="agent-body-${arch}-${os}"
  target="${arch}-${os}"

  echo "Fetching latest release for $os/$arch ..." >&2
  tag="$(github_latest_tag "$REPO" "agent-body" "$target")"
  if [[ -z "$tag" ]]; then
    echo "Failed to fetch latest tag for $REPO" >&2
    echo "Set GITHUB_TOKEN or GH_TOKEN if GitHub API rate-limits you." >&2
    exit 1
  fi
  url="https://github.com/$REPO/releases/download/$tag/$asset"

  echo "Downloading $asset ($tag) ..." >&2
  install -d "$HOME/.local/bin"
  curl -fsSL "$url" -o "$HOME/.local/bin/agent-body"
  chmod +x "$HOME/.local/bin/agent-body"
  sign_macos_binary "$HOME/.local/bin/agent-body"
  ln -sf "$HOME/.local/bin/agent-body" "$HOME/.local/bin/autonomic"

  echo "Installed agent-body $tag to $HOME/.local/bin/agent-body (autonomic symlink)" >&2
}

main "$@"
