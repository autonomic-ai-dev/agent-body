# supervisor-and-gateway

Summary: agent-body is the single entry point for the Autonomic stack. It creates the workspace, starts and supervises the daemons in dependency order, reports health, installs a macOS LaunchAgent, and serves one MCP gateway that proxies every organ tool.

## Sub-features

- `workspace-init`: Creates `~/.autonomic/` and the unified config.
  - Intent: Give every organ one shared place for config, memory, state, and logs.
  - Behaviour: `autonomic init` creates the directory layout and `config.toml`.
  - Outcome: The paths in the README exist after the command runs.
- `daemon-lifecycle`: Starts, stops, and restarts daemons in order (nats-server, nerves, heart).
  - Intent: Bring the stack up with dependencies first.
  - Behaviour: `autonomic start`, `stop`, `restart` manage PID files under `~/.autonomic/state/supervisor/`.
  - Outcome: `status` lists each daemon with its PID and health.
- `supervise-restart`: Watches daemons and restarts unhealthy ones.
  - Intent: Recover from crashes without manual action.
  - Behaviour: `autonomic supervise` polls health and restarts a daemon that fails.
  - Outcome: A failed daemon returns to healthy state.
- `doctor-and-update`: Verifies organ binaries and workspace health, and lists installed versions.
  - Intent: Show missing or stale components before they break a run.
  - Behaviour: `autonomic doctor` checks binaries and paths. `autonomic update` lists versions on PATH.
  - Outcome: A printed report with a non-zero exit when a binary is missing.
- `launchd-persistence`: Registers the daemons as a macOS LaunchAgent.
  - Intent: Keep the stack running across logins.
  - Behaviour: `autonomic launchd install`, `status`, and `uninstall` manage `~/Library/LaunchAgents/com.autonomic.daemons.plist`.
  - Outcome: The plist is loaded and removed on request.
- `organ-health-mesh`: Reports health of every organ from one endpoint.
  - Intent: Let an operator or a dashboard see the whole stack without querying each organ.
  - Behaviour: `GET /organs/health` (`agent-body/agent-body/src/health_mesh.rs` line 11) aggregates organ health.
  - Outcome: A JSON summary per organ. The routes are in the README's HTTP routes table, and `.verify/checks/route_drift.sh` checks them against source. The response shape is not verified by a test in this pass.
- `mcp-gateway`: Serves all organ tools through one MCP endpoint.
  - Intent: Let a host agent call any organ through one server.
  - Behaviour: `autonomic serve-mcp` starts the gateway and spawns stopped daemons on demand. `autonomic <organ> …` proxies to `agent-<organ>`.
  - Outcome: Tool calls reach the organ and return its result.
- `retry-and-timeout`: Enforces per-call deadlines and retries failed calls up to three times.
  - Intent: Keep one slow organ from hanging a host session.
  - Behaviour: As stated in `AGENTS.md` section 5 for the gateway.
  - Outcome: Retries and the final error are returned to the caller. Not yet verified by a test in this pass.

## Code

- `agent-body`
- `core`

## How to get to it (user POV)

- CLI: `autonomic init`, `autonomic start`, `autonomic status`, `autonomic doctor`, `autonomic serve-mcp`, `autonomic tui`
- LaunchAgent: `autonomic launchd install`
- Proxy: `autonomic brain route_task …` (any organ name after `autonomic`)
- HTTP: `GET /organs/health`

## Verification Suites (Targeted Test Commands)

- **Unit Tests:** `cargo test` (whole crate, 50 tests in the baseline)
- **Functional API Tests (Hurl):** not yet written
- **Allowlisted Verification:** entries in `.verify/verify.toml` and `.verify/run.sh`
- **Preconditions:** Rust toolchain. The daemon tests need `nats-server` on PATH and a writable `~/.autonomic/`.

### Verification Steps

1. **gateway-and-supervisor-unit-tests** (sub-features: all)
   - Request: `cargo test`
   - Why: Proves the crate's supervisor, config, and gateway logic.
   - Expected: 50 passed, 0 failed (baseline, `.verify/artifacts/feature-map-baseline/cargo-test.log`).
   - Impact: A failure means lifecycle or routing changed. Fix the cause.
2. **route-drift** (sub-features: organ-health-mesh)
   - Request: `bash .verify/checks/route_drift.sh`
   - Why: Proves each README route row matches a `.route()` in source, and each source route has a README row.
   - Expected: `route drift: README routes and src routes agree`.
   - Impact: A failure means a route was added, renamed, or removed without its README row. Fix whichever side is wrong.

## Gotchas

- `launchd-persistence` writes to the user's LaunchAgents folder. Test it only against a temporary home directory, never the real one.
- `retry-and-timeout` is documented in `AGENTS.md` but not confirmed by a test in this pass. Treat it as an open item.
- The gateway auto-spawns stopped daemons, so a test that calls a tool can start a real process. Stop only the PIDs the test recorded.
