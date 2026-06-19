# agent-body

**The sovereign meta-framework and unified CLI installer for the Autonomic AI ecosystem.**

agent-body is the central nervous system wrapper. It provides the unified `autonomic` CLI tool to scaffold, manage, and install all underlying organs (`brain`, `spine`, `heart`, etc.) so developers never have to piece the ecosystem together manually.

Rust is the body; the developer is the mind.

```bash
curl -fsSL https://raw.githubusercontent.com/autonomic-ai-dev/agent-body/master/scripts/install.sh | bash -s -- --global
autonomic init --full
```

**MCP is live immediately** — it exposes ecosystem-level health checks and orchestration tools to the agent.

---

## Why agent-body?

The Autonomic AI ecosystem is intentionally decoupled into highly specialized daemons. While this Unix-philosophy approach provides unmatched scalability and resilience, it can be intimidating to set up from scratch.

1. **Fragmentation:** Asking a user to download and configure 7 different Rust binaries is a terrible onboarding experience.
2. **Version Skew:** If `agent-spine` v2.1 requires `agent-nerves` v1.4, the user shouldn't have to manage that dependency graph.
3. **Unified Telemetry:** Developers need a single command to check the health of the entire organism.

**agent-body fixes this with a unified meta-framework:**

| Problem | agent-body answer |
|---------|-------------------|
| "This ecosystem is too complex to install" | **Unified Installer** — one bash script downloads `agent-body`, which then orchestrates the download and configuration of all other organs. |
| "I don't know what versions are compatible" | **Version Pinning** — `autonomic update` manages compatibility matrices across all daemons. |
| "Is the background daemon actually running?" | **Ecosystem Health** — `autonomic doctor` checks the status of the `heart`, `immune`, and `nerves` background processes. |

---

## Architectural Deep Dive

`agent-body` acts as a package manager and process supervisor for the Autonomic AI ecosystem.

### 1. The `core` Crate
While `agent-body` is a CLI tool, it also publishes the `autonomic-core` Rust crate.
- This crate contains the shared types, `nats.rs` message schemas, and MCP tool definitions used by all other organs.
- By centralizing these types, we guarantee that `agent-spine` can always talk to `agent-muscle` without schema mismatch errors.

### 2. Process Supervision
On macOS and Linux, `agent-body` configures `launchd` or `systemd` to ensure that critical background daemons (like `agent-heart`) restart automatically if they crash.

---

## Complete Setup (Copy & Paste)

### 1. Install the meta-framework

```bash
curl -fsSL https://raw.githubusercontent.com/autonomic-ai-dev/agent-body/master/scripts/install.sh | bash -s -- --global
```

### 2. Scaffold a new project

```bash
autonomic init --name my-ai-project
cd my-ai-project
autonomic start
```

### 3. Verify Health

```bash
autonomic doctor
```

---

## Commands

| Command | Description |
|---------|-------------|
| `autonomic init` | Scaffold a new project and initialize required organs |
| `autonomic start` | Start all required background daemons |
| `autonomic update` | Upgrade all ecosystem binaries to the latest compatible versions |
| `autonomic doctor` | Verify MCP connections and daemon health |

---

## Development

```bash
cargo test --release -p agent-body
cargo build --release -p agent-body
```

## License
MIT
