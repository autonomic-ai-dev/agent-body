# agent-body

**Meta-framework and unified CLI for the Autonomic AI ecosystem.**

`agent-body` installs as the `agent-body` binary and exposes the **`autonomic`** CLI (symlinked on install). It scaffolds the shared workspace, supervises core daemons, routes commands to peripheral organs, and ships `agent-body-core` — the shared types and NATS schemas every organ uses.

---

## Architecture

Each organ is **independently useful** (its own binary, config section, CLI, and HTTP API) and **designed to integrate** through shared paths, NATS JetStream, and the agent-spine event bus.

```text
                         autonomic (agent-body)
                    init · start · doctor · <organ> …
                                    │
          ┌─────────────────────────┼─────────────────────────┐
          │                         │                         │
    agent-brain                 agent-spine               agent-heart
    MCP / memory                workflows · events          GC scheduler
          │                         │                         │
          └─────────────┬───────────┴───────────┬─────────────┘
                        │                       │
                  agent-nerves              agent-muscle
                  NATS / JetStream          exec · finetune
                        │
        ┌───────────────┼───────────────┬───────────────┐
        │               │               │               │
  agent-immune    agent-eyes      agent-mouth     (your agents)
  scan / sandbox  vision / DOM    approvals
```

| Integration surface | Purpose |
|---------------------|---------|
| `~/.autonomic/config.toml` | One file, `[organ]` sections per daemon |
| `agent-body-core` | Shared NATS subjects, JetStream stream, workspace paths |
| agent-spine `:3100` | Organ registration, heartbeats, domain events |
| NATS (`agent-nerves`) | Async jobs: compute, train requests, bus messages |

---

## Quick start

### 1. Install all organs (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/autonomic-ai-dev/agent-body/master/scripts/install-all-organs.sh | bash
export PATH="$HOME/.local/bin:$PATH"
```

This downloads release binaries for **agent-body** and all eight peripheral organs, links `autonomic` → `agent-body`, runs **`autonomic init`**, and verifies with **`autonomic doctor`**.

Install only the meta CLI:

```bash
curl -fsSL https://raw.githubusercontent.com/autonomic-ai-dev/agent-body/master/scripts/install.sh | bash
ln -sf ~/.local/bin/agent-body ~/.local/bin/autonomic   # optional symlink
```

### 2. Initialize workspace

```bash
autonomic init                  # creates ~/.autonomic/ and config.toml sections
autonomic init --name my-app    # optional project directory
```

### 3. Verify and start core daemons

```bash
autonomic doctor                # all organ binaries on PATH?
autonomic update                # print installed versions
autonomic start                 # nerves + heart (ordered, health-checked)
autonomic status                # workspace paths + supervisor state
```

### 4. Route to any organ

```bash
autonomic brain serve           # → agent-brain …
autonomic spine serve           # → agent-spine …
autonomic muscle train --backend auto
autonomic eyes dom stats
```

---

## Commands

| Command | Description |
|---------|-------------|
| `autonomic init [--name DIR]` | Create `~/.autonomic/` workspace and unified config |
| `autonomic start` | Start supervised daemons (nerves, heart) |
| `autonomic stop` / `restart` | Stop or restart supervised daemons |
| `autonomic supervise` | Watch and restart unhealthy daemons |
| `autonomic doctor` | Verify organ binaries and workspace |
| `autonomic update` | List installed organ versions on PATH |
| `autonomic status` | Workspace paths + supervisor table |
| `autonomic tui` | Live CPU/RAM for autonomic processes |
| `autonomic <organ> …` | Proxy to `agent-{organ}` (brain, spine, heart, …) |

---

## Integration smoke test

From a local checkout:

```bash
bash scripts/smoke-integration.sh
# optional: start daemons and probe HTTP health
AUTONOMIC_SMOKE_HTTP=1 bash scripts/smoke-integration.sh
```

---

## Workspace layout

| Path | Purpose |
|------|---------|
| `~/.autonomic/config.toml` | Unified organ configuration |
| `~/.autonomic/memory/` | agent-brain store |
| `~/.autonomic/broker/` | NATS / JetStream persistence |
| `~/.autonomic/logs/spine/` | Workflow and execution state |
| `~/.autonomic/state/<organ>/` | Per-organ runtime state |

---

## Peripheral organs

| Organ | Port | Role |
|-------|------|------|
| [agent-brain](../agent-brain) | MCP stdio | Context routing, memory, MCP |
| [agent-spine](../agent-spine) | 3100 | YAML workflows, event bus |
| [agent-heart](../agent-heart) | 3101 | Scheduled agent-brain GC |
| [agent-nerves](../agent-nerves) | 3102 | NATS / JetStream bus |
| [agent-muscle](../agent-muscle) | 3103 | Command execution, LoRA training |
| [agent-mouth](../agent-mouth) | 3104 | Slack approvals, webhooks |
| [agent-eyes](../agent-eyes) | 3105 | Capture, DOM index, VLM |
| [agent-immune](../agent-immune) | 3106 | OSV scan, sandbox |

Each repo includes its own `scripts/install.sh` if you prefer to install organs individually.

---

## Development

```bash
cargo test --release -p agent-body -p agent-body-core
cargo build --release -p agent-body
```

---

## License

MIT
