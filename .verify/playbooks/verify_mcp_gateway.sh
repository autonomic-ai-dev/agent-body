#!/usr/bin/env bash
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$DIR/../.." && pwd)"
cd "$REPO_ROOT"

echo "==> Running MCP gateway tests..."
cargo test --package agent-body --lib mcp_gateway::tests -- --nocapture

echo "==> Running E2E live stdio MCP gateway verification (with RUST_LOG=info)..."
python3 - <<'EOF'
import subprocess, os, json, sys

env = os.environ.copy()
env["RUST_LOG"] = "info"

p = subprocess.Popen(["agent-body", "serve-mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=env)

def call(req):
    p.stdin.write(json.dumps(req) + "\n")
    p.stdin.flush()
    line = p.stdout.readline()
    if not line:
        err = p.stderr.read()
        raise RuntimeError(f"agent-body closed transport unexpectedly. Stderr: {err}")
    return json.loads(line)

init_resp = call({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"e2e-test","version":"1.0"}}})
assert "result" in init_resp, f"Initialize failed: {init_resp}"

p.stdin.write(json.dumps({"jsonrpc":"2.0","method":"notifications/initialized"}) + "\n")
p.stdin.flush()

# Test muscle organ routing
muscle_resp = call({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"muscle_execute_bash","arguments":{"command":"echo test"}}})
assert "result" in muscle_resp and not muscle_resp.get("isError", False), f"Muscle tool call failed: {muscle_resp}"

# Test mouth organ routing
mouth_resp = call({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"mouth_validate_ast","arguments":{"command":"ls -la"}}})
assert "result" in mouth_resp and not mouth_resp.get("isError", False), f"Mouth tool call failed: {mouth_resp}"

p.stdin.close()
p.wait()
print("==> Live stdio JSON-RPC framing & multi-organ E2E verification passed.")
EOF
echo "==> MCP gateway tests passed."
