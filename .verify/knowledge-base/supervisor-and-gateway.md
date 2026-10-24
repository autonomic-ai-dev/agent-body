# Supervisor and gateway

## What it does

agent-body is the single entry point for the Autonomic stack. It creates the workspace, starts and supervises the daemons in dependency order, reports health, installs a macOS LaunchAgent, and serves one MCP gateway that proxies every organ tool. It also defines the shared `ExecuteResult` payload that agent-spine consumes.

## How it works

1. **Workspace.** `autonomic init` creates `~/.autonomic/` and `config.toml`.
2. **Lifecycle.** `autonomic start`, `stop`, and `restart` start daemons in order (nats-server, nerves, heart) and track each PID under `~/.autonomic/state/supervisor/`.
3. **Supervision.** `autonomic supervise` polls each daemon and restarts one that fails.
4. **Health.** `GET /organs/health` (`agent-body/src/health_mesh.rs`) aggregates each organ's health into one JSON summary.
5. **Gateway.** `autonomic serve-mcp` serves every organ tool through one MCP endpoint. It spawns stopped daemons on demand and proxies `autonomic <organ> …` to `agent-<organ>`. It exits when its host closes stdin. The startup health check runs on a detached thread, so shutdown does not wait for it (PR #66). Before PR #66, shutdown waited on that check, and a gateway whose host had exited kept running.
6. **Results.** `ExecuteResult` (`core/src/nats.rs`) carries `degraded: bool`, which is `false` when absent from older payloads. Immune sets it when it falls back from Firecracker to a subprocess.

## Where the code is

- `agent-body/src/`: supervisor, health mesh, and the gateway.
- `core/src/nats.rs`: shared subjects and payloads, including `ExecuteResult`.
- Tests: `cargo test --workspace` runs the crate unit tests. The `ExecuteResult` tests are `payload_without_degraded_defaults_to_false` and `degraded_flag_round_trips`.

## How to use it

- `autonomic init`, then `autonomic start`.
- `autonomic status` lists each daemon with its PID and health.
- `autonomic serve-mcp` for an MCP host.

## How it is verified

- `cargo test --workspace` runs the workspace suite. Last recorded run for `v1.0.0-beta.3.10`: 29 and 23 unit tests passed, with no failures.
- `cargo clippy --workspace --all-targets -- -D warnings` passed for the same release.
- The retry and timeout behaviour in the gateway has no test yet.

## Known limits

- `retry-and-timeout` is documented but not confirmed by a test.
- The gateway auto-spawns stopped daemons, so a test that calls a tool can start a real process.
- `launchd-persistence` writes to the user's LaunchAgents folder. Test it only against a temporary home directory.
- Immune sets `degraded` only after it re-pins to a tag that includes the field. Until then, the flag stays `false` in immune's results.

## Related

- Feature map: `.verify/features/supervisor-and-gateway.md`
- Organ consumers: agent-spine (`ExecuteResult`), agent-immune (sets `degraded`)
