#!/usr/bin/env bash
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$DIR/.." && pwd)"

echo "==> [agent-body] Starting verification suite..."
cd "$REPO_ROOT"

echo "==> Checking cargo clippy with -D warnings..."
cargo clippy --workspace --all-targets -- -D warnings

echo "==> Checking cargo check..."
cargo check --workspace

echo "==> Running Playbook: MCP Gateway..."
"$DIR/playbooks/verify_mcp_gateway.sh"

echo "==> Running Playbook: Degradation & Supervisor..."
"$DIR/playbooks/verify_degradation.sh"

echo "==> Running Playbook: Full Suite..."
"$DIR/playbooks/verify_full_suite.sh"

echo "==> [agent-body] All verification playbooks passed cleanly (exit 0)."
