#!/usr/bin/env bash
set -euo pipefail

echo "==> Running MCP gateway tests..."
cargo test --package agent-body --lib mcp_gateway::tests -- --nocapture
echo "==> MCP gateway tests passed."
