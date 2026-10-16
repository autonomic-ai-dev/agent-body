---
name: install-agent-body
description: Install and wire agent-body (autonomic control plane) — curl/brew install, organ stack, serve-mcp, Cursor/OpenCode mcp.json, doctor --fix upstream federation with agent-brain.
---

# Install agent-body

Use when the user needs the Autonomic **control plane** (`autonomic` CLI + organ gateway), cluster install, or `serve-mcp` registration.

## Goal

After this skill: `autonomic` on PATH, organs healthy, **agent-body** available as MCP (direct and/or via agent-brain `route_to_mcp`).

## Step 1 — Install binaries

**Full stack (recommended):**

```bash
curl -fsSL https://raw.githubusercontent.com/autonomic-ai-dev/agent-body/master/scripts/install-all-organs.sh | bash
```

**Meta CLI only:**

```bash
curl -fsSL https://raw.githubusercontent.com/autonomic-ai-dev/agent-body/master/scripts/install.sh | bash
```

**Homebrew:**

```bash
brew tap autonomic-ai-dev/tap
brew install autonomic-stack   # all organs + nats-server
# or: brew install autonomic   # agent-body / autonomic symlink only
```

Confirm:

```bash
which autonomic agent-body
autonomic --version
```

## Step 2 — Init workspace + start

```bash
autonomic init
autonomic start          # nats → nerves → heart → …
autonomic status
autonomic doctor
```

Workspace: `~/.autonomic/` (`config.toml`, memory, state, logs).

## Step 3 — Register MCP (host)

**Preferred with agent-brain already installed:**

```bash
agent-brain install --global
agent-brain doctor --fix
```

That:

1. Puts **agent-brain** in host `mcp.json` (turn router)
2. Puts **agent-body** in host `mcp.json` when the binary exists (`serve-mcp`)
3. Enables `[brain.upstream_mcp]` → `agent-body serve-mcp` for **`route_to_mcp`**

**Body-only host entry** (if brain is absent): add Cursor `mcpServers.agent-body` with `command: agent-body`, `args: ["serve-mcp"]` — or run `agent-brain install --global` after installing brain.

## Step 4 — Verify gateway

```bash
agent-brain doctor --mcp
# or: autonomic serve-mcp  (stdio; use doctor audit instead of hanging the session)
```

Expect organ tools: `muscle_execute_bash`, `spine_*`, `heart_*`, `eyes_*`, `immune_*`, `mouth_*`.

## Step 5 — Agent usage after install

1. Every turn: **`route_task`** on **agent-brain**
2. Organ work: **`route_to_mcp`** → `server=agent-body` **or** direct body MCP tools
3. Ops CLI: `autonomic status` / `autonomic spine run` / `autonomic doctor`

See skill **brain-body-mcp-split** when both MCPs are registered.

## Related

- `docs/CONTEXT.md` in agent-body — full architecture
- `agent-brain registry list` — utility catalog
- Skill **rmcp-mcp-gateway** — schema / stdio debugging
