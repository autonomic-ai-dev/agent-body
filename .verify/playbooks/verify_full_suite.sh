#!/usr/bin/env bash
set -euo pipefail

echo "==> Running agent-body full workspace test suite..."
cargo test --workspace -- --nocapture
echo "==> agent-body full workspace test suite passed."
