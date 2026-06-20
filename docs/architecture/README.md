# agent-body architecture documentation

## Design goals

agent-body is the **meta-orchestrator** — it doesn't do its own work, it ensures all other organs can do theirs. Three responsibilities:

1. **Install** — download and configure all organ binaries from GitHub releases
2. **Supervise** — start organs in dependency order, monitor health, restart on failure
3. **Route** — proxy CLI commands to the correct organ binary

### Install architecture

The `install-all-organs.sh` script follows a consistent pattern:

```
Download OS/arch-detected release binaries from GitHub
  → agent-body, agent-brain, agent-spine, agent-heart
  → agent-nerves, agent-muscle, agent-immune, agent-eyes, agent-mouth
  → nats-server (NATS official release)
  → ln -sf ~/.local/bin/agent-body ~/.local/bin/autonomic
  → agent-body init          # create ~/.autonomic/ workspace
  → agent-brain install      # set up MCP, hooks, permissions
```

Each organ's `scripts/install.sh` follows the same pattern independently — the all-in-one script just calls them in sequence.

### Supervisor architecture

```
autonomic start
  1. Check ~/.autonomic/config.toml exists
  2. Start nats-server -js (JetStream mode, port 4222, monitoring port 8222)
  3. Wait for NATS health endpoint
  4. Start agent-nerves serve (port 3102)
  5. Wait for nerves /health
  6. Start agent-heart serve (port 3101)
  7. Wait for heart /health
  8. Record PIDs in ~/.autonomic/state/supervisor/
  9. Enter supervise loop: poll /health every 30s, restart if unhealthy
```

The ordered startup ensures dependencies are available before dependent organs start. The supervise loop handles the common case of daemons dying after laptop sleep.

### CLI routing

```
autonomic brain serve
  → translated to: agent-brain serve --config ~/.autonomic/config.toml
autonomic spine run workflow.yaml
  → translated to: agent-spine run workflow.yaml --config ~/.autonomic/config.toml
autonomic immune scan ./Cargo.toml
  → translated to: agent-immune scan ./Cargo.toml --config ~/.autonomic/config.toml
```

The `--config` flag is injected automatically so all organs read from the unified config.

### Key design decisions

| Decision | Rationale |
|----------|-----------|
| **Separate organ binaries, not plugins** | Each organ can be developed, tested, and installed independently. No plugin API needed. |
| **Shell install script, not package manager** | Works on any OS/arch without npm, pip, brew, or apt. Single curl pipe. |
| **HTTP health probes for supervision** | Simpler than process signals. `/health` endpoints are language-agnostic. |
| **Symlink for `autonomic`** | The `autonomic` name is shorter and more memorable than `agent-body`. The symlink avoids renaming the binary. |

### Alternatives considered

| Option | Why rejected |
|--------|-------------|
| **Single monolithic binary** | Defeats the purpose of a modular architecture. Organs should be independently replaceable. |
| **npm/pip package** | Would require Node.js or Python runtime. Rust binaries are self-contained. |
| **systemd/launchd supervision** | Platform-specific and requires root. The user-space supervisor works everywhere. |
| **Docker Compose** | Heavy for local development. The supervisor starts native binaries with near-zero overhead. |
