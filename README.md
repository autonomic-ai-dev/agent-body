# agent-body

`agent-body` is the control plane for the Autonomic stack. It provides the `autonomic` CLI, creates the shared `~/.autonomic/` workspace, supervises the daemons, and serves one MCP gateway for every organ's tools.

Full docs: [autonomic-ai-dev.vercel.app](https://autonomic-ai-dev.vercel.app)

## Quick start

```bash
autonomic init      # create ~/.autonomic/ and config.toml
autonomic doctor    # check that every organ binary is on PATH
autonomic start     # start NATS, then nerves, then heart
autonomic status    # PID and health for each daemon
```

## Commands

| Command | What it does |
|---------|--------------|
| `autonomic init` | Creates the workspace and unified config |
| `autonomic start`, `stop`, `restart` | Manage the supervised daemons |
| `autonomic status` | Lists each daemon with its PID and health |
| `autonomic supervise` | Restarts a daemon that fails its health check |
| `autonomic serve-mcp` | Serves every organ tool through one MCP endpoint |
| `autonomic <organ> …` | Runs an organ's command, for example `autonomic brain route_task` |
| `autonomic launchd install` | Registers the daemons as a macOS LaunchAgent |

## Develop

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Feature map and knowledge base: `.verify/features/supervisor-and-gateway.md` and `.verify/knowledge-base/supervisor-and-gateway.md`.
