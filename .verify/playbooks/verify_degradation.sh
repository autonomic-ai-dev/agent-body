#!/usr/bin/env bash
set -euo pipefail

echo "==> Running degradation and supervisor resilience tests..."
cargo test --package agent-body --lib degradation::tests -- --nocapture
cargo test --package agent-body --lib supervisor::tests -- --nocapture
echo "==> Degradation and supervisor resilience tests passed."
