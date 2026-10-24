# Supervisor and gateway

## What it does

agent-body is the single entry point for the Autonomic stack. It creates the shared workspace, starts and stops the daemons in dependency order, reports their health, can register them as a macOS LaunchAgent, and serves one MCP gateway that proxies every organ's tools.

## How it works

1. **Workspace.** `autonomic init` creates `~/.autonomic/` and `config.toml`. Every organ reads config, memory, state, and logs from this one layout.
2. **Lifecycle.** `autonomic start`, `stop`, and `restart` manage one PID file per daemon under `~/.autonomic/state/supervisor/`. Start order is nats-server, then nerves, then heart.
3. **Supervision.** `autonomic supervise` polls each daemon's health and restarts one that fails.
4. **Health mesh.** `GET /organs/health` (`agent-body/agent-body/src/health_mesh.rs`) returns one JSON report covering every organ. `GET /health` returns the gateway's own status.
5. **Gateway.** `autonomic serve-mcp` serves one MCP endpoint. A call to an organ tool is proxied to `agent-<organ>`. A stopped daemon is spawned on demand. `autonomic <organ> ...` does the same from the CLI.
6. **Persistence.** `autonomic launchd install` writes `~/Library/LaunchAgents/com.autonomic.daemons.plist`. `status` and `uninstall` manage the same file.

## Where the code is

- Health mesh: `agent-body/agent-body/src/health_mesh.rs`, route at line 11
- Supervisor: `agent-body/agent-body/src/supervisor.rs`
- Routes: the HTTP routes table in `agent-body/README.md`

## How to use it

- `autonomic init`, then `autonomic start`, then `autonomic status`
- `autonomic doctor` lists missing binaries with a non-zero exit
- `autonomic serve-mcp` gives a host agent one endpoint for every organ
- `curl http://localhost:<port>/organs/health` for the whole stack

## How it is verified

- `cargo test` in agent-body: the supervisor, config, and gateway logic (50 tests in the recorded baseline)
- `bash .verify/checks/route_drift.sh`: the README route table matches the `.route()` calls in source, in both directions

## Known limits

- The response shape of `/organs/health` has no test. Check the JSON before depending on exact keys.
- `retry-and-timeout` (three retries per call) is documented in `AGENTS.md` but no test confirms it.
- The gateway spawns stopped daemons, so a test that calls a tool can start a real process. Stop only the PIDs the test recorded.
- `launchd-persistence` writes to the real LaunchAgents folder. Test it only against a temporary home directory.

## Related

- Feature map: `agent-body/.verify/features/supervisor-and-gateway.md`
- Organ health is fed by every organ's `/health` route, for example [budget-gate](../../../agent-heart/.verify/knowledge-base/budget-gate.md)
