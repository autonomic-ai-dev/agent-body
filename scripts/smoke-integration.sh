#!/usr/bin/env bash
# Integration smoke test for the Autonomic AI organ ecosystem.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALL_DIR="${INSTALL_DIR:-${HOME}/.local/bin}"
export PATH="${INSTALL_DIR}:${PATH}"

PASS=0
FAIL=0
SKIP=0

pass() { echo "  ✓ $*"; PASS=$((PASS + 1)); }
fail() { echo "  ✗ $*"; FAIL=$((FAIL + 1)); }
skip() { echo "  ~ $* (skipped)"; SKIP=$((SKIP + 1)); }

require_bin() {
  local bin="$1"
  if command -v "$bin" >/dev/null 2>&1; then
    pass "${bin} on PATH"
    return 0
  fi
  fail "${bin} not on PATH"
  return 1
}

section() {
  echo
  echo "━━ $* ━━"
}

section "Workspace"
if command -v agent-body >/dev/null 2>&1; then
  agent-body init >/dev/null 2>&1 || true
  pass "autonomic workspace initialized"
else
  skip "agent-body missing — cannot run autonomic init"
fi

if [[ -d "${HOME}/.autonomic" ]]; then
  pass "~/.autonomic exists"
else
  fail "~/.autonomic missing"
fi

if [[ -f "${HOME}/.autonomic/config.toml" ]]; then
  pass "unified config.toml present"
else
  fail "config.toml missing"
fi

section "Organ binaries"
BINS=(
  agent-body
  agent-brain
  agent-spine
  agent-heart
  agent-nerves
  agent-muscle
  agent-immune
  agent-eyes
  agent-mouth
)
missing=0
for b in "${BINS[@]}"; do
  require_bin "$b" || missing=$((missing + 1))
done

if command -v agent-body >/dev/null 2>&1; then
  section "autonomic doctor"
  if agent-body doctor; then
    pass "autonomic doctor"
  else
    fail "autonomic doctor reported issues"
  fi
fi

section "Standalone CLI smoke"
smoke_status() {
  local bin="$1"
  command -v "$bin" >/dev/null 2>&1 || return 0
  if "$bin" --version >/dev/null 2>&1 || "$bin" status >/dev/null 2>&1; then
    pass "${bin} responds to --version or status"
  else
    fail "${bin} did not respond to --version/status"
  fi
}

for b in agent-nerves agent-heart agent-muscle agent-eyes agent-mouth agent-immune; do
  smoke_status "$b"
done

section "Rust unit tests (local checkout)"
if command -v cargo >/dev/null 2>&1; then
  for repo in agent-body agent-nerves agent-heart agent-muscle agent-eyes agent-mouth agent-immune agent-spine; do
    dir="${ROOT}/../${repo}"
    if [[ -f "${dir}/Cargo.toml" ]]; then
      if (cd "$dir" && cargo test -q --release 2>/dev/null); then
        pass "cargo test ${repo}"
      else
        fail "cargo test ${repo}"
      fi
    else
      skip "no checkout at ${dir}"
    fi
  done
else
  skip "cargo not available"
fi

section "Integration HTTP (optional)"
if [[ "${AUTONOMIC_SMOKE_HTTP:-0}" == "1" ]]; then
  if command -v agent-body >/dev/null 2>&1; then
    agent-body start || true
    sleep 3
    curl -fsS "http://127.0.0.1:3102/health" >/dev/null && pass "nerves /health" || fail "nerves /health"
    curl -fsS "http://127.0.0.1:3101/health" >/dev/null && pass "heart /health" || fail "heart /health"
  else
    skip "agent-body missing for autonomic start"
  fi
else
  skip "set AUTONOMIC_SMOKE_HTTP=1 to probe daemons"
fi

section "Summary"
echo "  passed:  ${PASS}"
echo "  failed:  ${FAIL}"
echo "  skipped: ${SKIP}"

if [[ "$FAIL" -gt 0 ]]; then
  echo
  echo "Some checks failed. Install missing organs:"
  echo "  bash scripts/install-all-organs.sh"
  exit 1
fi

echo
echo "Integration smoke test passed."
