#!/usr/bin/env bash
# Install agent-body from latest GitHub release.
set -euo pipefail

REPO="autonomic-ai-dev/agent-body"
VERSION="${VERSION:-v1.0.0-beta.1}"

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

prepare_macos_binary() {
  local bin="$1"
  if [[ "$(uname -s)" != "Darwin" ]] || [[ ! -f "$bin" ]]; then
    return 0
  fi
  xattr -cr "$bin" 2>/dev/null || true
  if command -v codesign >/dev/null 2>&1; then
    codesign --verify --verbose "$bin" 2>/dev/null || true
  fi
}

main() {
  local os arch asset url target
  os="$(detect_os)"
  arch="$(detect_arch)"
  asset="agent-body-${arch}-${os}"
  target="${arch}-${os}"

  if [[ "$VERSION" == "latest" ]]; then
    url="https://github.com/$REPO/releases/latest/download/$asset"
    echo "Downloading latest $asset ..." >&2
  else
    url="https://github.com/$REPO/releases/download/$VERSION/$asset"
    echo "Downloading $asset ($VERSION) ..." >&2
  fi

  INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
  install -d "$INSTALL_DIR"
  curl -fsSL "$url" -o "$INSTALL_DIR/agent-body"
  chmod +x "$INSTALL_DIR/agent-body"
  prepare_macos_binary "$INSTALL_DIR/agent-body"
  ln -sf "$INSTALL_DIR/agent-body" "$INSTALL_DIR/autonomic"

  echo "Installed agent-body to $INSTALL_DIR/agent-body (autonomic symlink)" >&2
}

main "$@"
