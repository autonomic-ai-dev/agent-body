#!/usr/bin/env bash
# Install agent-body (meta CLI) and all peripheral organ binaries from GitHub releases.
set -euo pipefail

INSTALL_DIR="${INSTALL_DIR:-${HOME}/.local/bin}"
REPOS=(
  "autonomic-ai-dev/agent-body:agent-body"
  "autonomic-ai-dev/agent-brain:agent-brain"
  "autonomic-ai-dev/agent-spine:agent-spine"
  "autonomic-ai-dev/agent-heart:agent-heart"
  "autonomic-ai-dev/agent-nerves:agent-nerves"
  "autonomic-ai-dev/agent-muscle:agent-muscle"
  "autonomic-ai-dev/agent-immune:agent-immune"
  "autonomic-ai-dev/agent-eyes:agent-eyes"
  "autonomic-ai-dev/agent-mouth:agent-mouth"
)

detect_target() {
  local os arch
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"
  case "$os" in
    darwin)
      case "$arch" in
        x86_64) echo "x86_64-apple-darwin" ;;
        arm64|aarch64) echo "aarch64-apple-darwin" ;;
        *) echo "unsupported arch: $arch" >&2; exit 1 ;;
      esac
      ;;
    linux)
      case "$arch" in
        x86_64) echo "x86_64-unknown-linux-gnu" ;;
        aarch64|arm64) echo "aarch64-unknown-linux-gnu" ;;
        *) echo "unsupported arch: $arch" >&2; exit 1 ;;
      esac
      ;;
    *)
      echo "unsupported OS: $os" >&2
      exit 1
      ;;
  esac
}

install_binary() {
  local repo="$1"
  local binary="$2"
  local target="$3"
  local asset="${binary}-${target}"
  local url="https://github.com/${repo}/releases/latest/download/${asset}"

  echo "==> Installing ${binary} (${target})"
  if ! curl -fsSL "$url" -o "${INSTALL_DIR}/${binary}"; then
    echo "    failed: ${url}" >&2
    return 1
  fi
  chmod +x "${INSTALL_DIR}/${binary}"
  echo "    ok: ${INSTALL_DIR}/${binary}"
}

install_nats() {
  echo "==> Installing nats-server"
  local os arch asset
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"
  case "$os" in
    darwin)
      case "$arch" in
        x86_64) asset="nats-server-v2.10.16-darwin-amd64" ;;
        arm64|aarch64) asset="nats-server-v2.10.16-darwin-arm64" ;;
        *) echo "unsupported arch: $arch" >&2; return 1 ;;
      esac
      ;;
    linux)
      case "$arch" in
        x86_64) asset="nats-server-v2.10.16-linux-amd64" ;;
        aarch64|arm64) asset="nats-server-v2.10.16-linux-arm64" ;;
        *) echo "unsupported arch: $arch" >&2; return 1 ;;
      esac
      ;;
    *) echo "unsupported OS: $os" >&2; return 1 ;;
  esac

  local url="https://github.com/nats-io/nats-server/releases/download/v2.10.16/${asset}.zip"
  if ! curl -fsSL "$url" -o "/tmp/${asset}.zip"; then
    echo "    failed to download: ${url}" >&2
    return 1
  fi

  unzip -q -o "/tmp/${asset}.zip" -d "/tmp/nats_extract"
  cp "/tmp/nats_extract/${asset}/nats-server" "${INSTALL_DIR}/nats-server"
  chmod +x "${INSTALL_DIR}/nats-server"
  rm -rf "/tmp/${asset}.zip" "/tmp/nats_extract"
  if ! "${INSTALL_DIR}/nats-server" --version >/dev/null 2>&1; then
    echo "    installed but failed version check" >&2
    return 1
  fi
  echo "    ok: ${INSTALL_DIR}/nats-server"
}

migrate_configs() {
  echo "==> Migrating legacy configurations..."
  local auto_dir="$HOME/.autonomic"
  mkdir -p "$auto_dir/memory" "$auto_dir/logs/spine" "$auto_dir/state" "$auto_dir/legacy_configs"

  if [[ -d "$HOME/.agent_brain/data" ]]; then
    echo "    Migrating agent-brain data..."
    cp -Rn "$HOME/.agent_brain/data/"* "$auto_dir/memory/" 2>/dev/null || true
    mv "$HOME/.agent_brain/config.yaml" "$auto_dir/legacy_configs/brain_config.yaml" 2>/dev/null || true
    mv "$HOME/.agent_brain" "$HOME/.agent_brain.bak" 2>/dev/null || true
  fi

  if [[ -d "$HOME/.agent_spine/executions" ]]; then
    echo "    Migrating agent-spine executions..."
    cp -Rn "$HOME/.agent_spine/executions" "$auto_dir/logs/spine/" 2>/dev/null || true
    mv "$HOME/.agent_spine" "$HOME/.agent_spine.bak" 2>/dev/null || true
  fi

  for organ in heart nerves muscle immune eyes mouth; do
    if [[ -d "$HOME/.agent_${organ}" ]]; then
      echo "    Migrating agent-${organ} state..."
      mkdir -p "$auto_dir/state/${organ}"
      cp -Rn "$HOME/.agent_${organ}/"* "$auto_dir/state/${organ}/" 2>/dev/null || true
      mv "$HOME/.agent_${organ}" "$HOME/.agent_${organ}.bak" 2>/dev/null || true
    fi
  done
}

install_integration_packages() {
  echo "==> Installing integration packages..."
  if [[ -x "${INSTALL_DIR}/agent-brain" ]]; then
    echo "    Installing global MCP hooks and permissions for all editors..."
    "${INSTALL_DIR}/agent-brain" install --all --global || true
    echo "    Installing @supervisor package..."
    "${INSTALL_DIR}/agent-brain" add @supervisor || true
    echo "    Installing @starter package..."
    "${INSTALL_DIR}/agent-brain" add @starter || true
  fi
}

main() {
  local target
  target="$(detect_target)"
  mkdir -p "$INSTALL_DIR"

  local failed=0
  for entry in "${REPOS[@]}"; do
    IFS=: read -r repo binary <<<"$entry"
    if ! install_binary "$repo" "$binary" "$target"; then
      failed=$((failed + 1))
    fi
  done

  if [[ -x "${INSTALL_DIR}/agent-body" ]]; then
    ln -sf "${INSTALL_DIR}/agent-body" "${INSTALL_DIR}/autonomic"
    echo "==> Linked autonomic -> agent-body"
  fi

  install_nats || true
  mkdir -p "${HOME}/.autonomic/broker"

  echo
  if [[ "$failed" -gt 0 ]]; then
    echo "Installed with ${failed} failure(s). Missing releases may need a local \`cargo build --release\`." >&2
    exit 1
  fi

  echo "All organ binaries installed to ${INSTALL_DIR}"
  echo
  echo "==> Initializing Autonomic workspace"
  export PATH="${INSTALL_DIR}:${PATH}"
  if "${INSTALL_DIR}/agent-body" init; then
    migrate_configs
    install_integration_packages
    
    echo "==> Running autonomic doctor"
    "${INSTALL_DIR}/agent-body" doctor || true
  fi

  echo
  echo "Done. Start the local stack (NATS + core daemons):"
  echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
  echo "  autonomic start          # nats-server (JetStream) → agent-nerves → agent-heart"
  echo "  autonomic status         # supervisor PID / health table"
  echo
  echo "Optional:"
  echo "  bash scripts/smoke-integration.sh"
  echo "  AUTONOMIC_SMOKE_HTTP=1 bash scripts/smoke-integration.sh"
}

main "$@"
