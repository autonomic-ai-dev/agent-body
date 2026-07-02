# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.17] - 2026-07-02

### Added

- **agent-body-core 0.3.7 — `wasm` feature** — shared `wasm_engine` module with Wasmtime engine singleton and blake3-keyed module cache
- **`SandboxExecute` / `ExecuteResult` fuel fields** — `wasm_path`, `fuel_limit`, `memory_limit_bytes`, `fuel_consumed`, `memory_peak_bytes` for WASM sandbox accounting

## [0.5.16] - 2026-06-30

### Fixed

- **`autonomic update` — missing organ subcommand** — check `{binary} update --help` before calling it; skip organs that don't support the `update` subcommand instead of failing the entire update stack

### Changed

- **agent-body-core 0.3.5** — `cli::progress` helpers (`apply_progress_env`, `strip_progress_argv`); ProgressTree on `run_organ_self_update`

## [0.5.15] - 2026-06-29

### Fixed

- **MCP gateway tool registration** — rewrite `serve-mcp` with rmcp `#[tool_router]` / `#[tool_handler]` (matching agent-brain and agent-muscle) so tools expose JSON Schema draft 2020-12 `inputSchema` with `properties` and `required`; fixes empty `{}` schemas that prevented Cursor and OpenCode from registering agent-body tools
- **`serverInfo` identity** — MCP `initialize` handshake reports `name: agent-body` and package version instead of generic `rmcp`

### Changed

- Gateway uses `server.serve(stdio())` instead of `serve_server` for transport consistency with other Autonomic organs

## [0.5.14] - 2026-06-27

### Added

- **MCP API Gateway** (`autonomic serve-mcp`) — unified entry point aggregating all organ MCP tools via persistent child-process connections
- **Persistent connections** — gateway spawns each organ binary once (lazy connect), keeps `RunningService` handle for reuse across tool calls
- **`serve-mcp` builtin** — added to `BUILTIN_COMMANDS` in router.rs so it routes to gateway instead of organ proxy
- **Integration tests** — gateway route resolution, tool-to-organ mapping, tool definition validation

### Changed

- Gateway routes updated: heart, eyes, immune now use `serve-mcp` subcommand (MCP-only, no HTTP daemon)

## [0.5.12] - 2026-06-23

### Added

- **agent-body-core 0.3.4** — shared `github_release` (redirect-first tag fetch, `GITHUB_TOKEN` API fallback, macOS adhoc sign)
- **ProgressTree** — Docker BuildKit-style progress on `autonomic update`, `doctor`, `init`, `start`, and `stop`; global `--progress auto|plain|quiet`
- **`doctor --quick`** — binaries-only health check (skips supervisor log scan); used at end of `install-all-organs.sh`
- **`scripts/lib/github.sh`** — shared GitHub release helpers for install scripts

### Changed

- Install scripts: optional `nats-server` (skip when working; `FORCE_NATS=1` to override), macOS adhoc codesign, quieter organ installs
- CI: `pipeline-compose-run@v1.17.1` with PR head ref (fixes `refs/pull/N/merge` dispatch failures)
- Removed GHA `concurrency:` groups so in-flight runs are not cancelled

## [0.5.11] - 2026-06-21

### Changed

- **Parallel version checks** — `autonomic update` and `autonomic status` now run all 8 organ `--version` subprocesses concurrently via `tokio::task::JoinSet` with a 3s per-binary timeout, reducing total latency from ~6s to ~0.3s

## [0.5.10] - 2026-06-21

### Fixed

- **GitHub Release Parser** — allowed trailing spaces after colon when extracting `tag_name` from GitHub API JSON
- **NATS Server Syntax** — properly wrapped configurations inside `authorization { users: [ { ... } ] }` array in `nats_auth.rs`

## [0.5.9] - 2026-06-21

### Added

- `autonomic tui --force` and `autonomic ui --force` refresh dashboard installs
- `autonomic ui --open-only` opens the hosted dashboard without starting the local relay
- `autonomic update` also refreshes agent-tui (GitHub release) and agent-ui relay (git pull + bun install)

### Fixed

- `autonomic ui` opens the hosted dashboard at `https://ui-autonomic-ai.vercel.app` instead of the old domain
- `autonomic tui` checks GitHub release version and upgrades when a newer tag is published
- `autonomic ui` validates git/bun availability, checks clone/install exit codes, and supports Linux browser launch via `xdg-open`

## [0.5.8] - 2026-06-21

### Added

- `autonomic tui` downloads `agent-tui` from GitHub releases into `~/.local/bin` (matching other organ install paths)
- `autonomic ui` clones and runs the local agent-ui relay via Bun

### Fixed

- `autonomic tui` no longer downloads a non-existent raw GitHub URL or exits immediately after install

## [0.5.7] - 2026-06-21

### Added

- **V2 NATS security** — per-organ ACL credentials, secure `nats-server.conf` generation, optional mTLS (`AUTONOMIC_NATS_TLS=1`), and `agent-body-core` `connect_nats()` helper
- Supervisor bootstraps broker credentials on `autonomic start` and injects `AUTONOMIC_NATS_*` env vars into spine, nerves, and heart

### Changed

- `agent-body-core` 0.3.3 — `nats_auth`, `nats_client` modules; `system.dlq` subject; extended `SandboxExecute` resource fields

## [0.5.6] - 2026-06-21

### Added

- `autonomic update --force` now delegates to each organ's `update` subcommand instead of re-running the install script
- Each organ (spine, nerves, heart, muscle, eyes, mouth, brain) now has its own `update [--force]` subcommand that checks GitHub releases, compares versions, and downloads a new binary if available

### Fixed

- Clippy: `map_or` → `is_some_and`, `filter_map(|l| l.ok())` → `map_while(Result::ok)` for CI compliance

## [0.5.5] - 2026-06-21

### Changed

- Supervisor sets `RUST_LOG=debug` on all spawned daemon processes so all log levels (error, info, warn, debug) are captured to files

## [0.5.4] - 2026-06-21

### Added

- `autonomic log <name> [--follow] [--list]` — display or tail daemon logs from the supervisor log directory
- `autonomic doctor` now also scans log files for recent ERROR/PANIC entries

## [0.5.3] - 2026-06-21

### Changed

- README docs aligned with agent-brain template across peripheral organs; fixed GitHub links
- `install-all-organs.sh` documents and verifies `nats-server`; creates broker dir; clarifies `autonomic start` order

### Added

- agent-spine to supervisor DAEMONS — starts after NATS so nerves and heart can register at startup

## [0.5.2] - 2026-06-20

### Added

- **NATS server install** in `install-all-organs.sh` — downloads platform-specific nats-server v2.10.16 binary
- **NATS daemon** in supervisor — starts before nerves with JetStream and health probe on `:8222`

## [0.5.1] - 2026-06-20

### Added

- Mermaid architecture charts in README (`7dacf7b`)
- Legacy config migration in `install-all-organs.sh` — moves `~/.agent_brain`, `~/.agent_spine`, and peripheral organ dirs into `~/.autonomic/` (`8dd6fca`)
- Integration package install — `@supervisor` and `@starter` via agent-brain after organ install (`8dd6fca`)

### Changed

- `install-all-organs.sh` uses `agent-brain install --all --global` for multi-editor MCP wiring (`d588635`)
- Ecosystem README, full organ installer, and integration smoke test (`75c3bf8`)

## [0.5.0] - 2026-06-20

### Added

- **Daemon supervisor v1** — ordered start/stop, HTTP health probes, `autonomic restart`, `autonomic supervise`, `status.json`
- **Supervisor status in `autonomic status`** — PID, running, healthy columns per organ

### Changed

- Version bumped from `0.4.0` to `0.5.0`

## [0.4.0] - 2026-06-20

### Added

- **`autonomic` organ router** — `autonomic brain serve` proxies to `agent-brain` (and all peripheral organs)
- **`autonomic init`** — scaffolds `~/.autonomic/` workspace and optional project directory
- **`autonomic start` / `stop`** — supervises `agent-nerves` and `agent-heart` daemons with PID files
- **`autonomic update`** — lists installed organ binary versions
- **`autonomic tui`** — live CPU/RAM monitor for autonomic processes
- **Expanded doctor** — checks all eight organ binaries

### Changed

- Version bumped from `0.3.1` to `0.4.0`

## [0.3.1] - 2026-06-20

### Added

- **`default_nats_url()`** — reads `AUTONOMIC_NATS_URL` for cross-organ NATS configuration

### Changed

- Version bumped from `0.3.0` to `0.3.1`

## [0.3.0] - 2026-06-20

### Added

- **`nats` module** — shared JetStream subjects and message types (`StateTransitionEvent`, `ComputeJob`, `SandboxExecute`, etc.)
- **Dedup defaults** — `default_duplicate_window()` and `default_ack_wait()` for exactly-once publishing

### Changed

- Version bumped from `0.2.0` to `0.3.0`

## [0.2.0] - 2026-06-20

### Added

- **`global_workspace` module** — canonical paths under `~/.autonomic/` (or `AUTONOMIC_HOME`): memory, spine logs, broker, per-organ state, unified `config.toml`
- **`organ_config` loader** — reads each organ's settings from `[organ]` sections in unified TOML; auto-migrates legacy `~/.config/<organ>/config.yaml`

### Changed

- Version bumped from `0.1.0` to `0.2.0`

## [0.1.0] - 2026-06-20

### Added

- **Initial project scaffold** — workspace with `core` and `agent-body` crates
- **Shared types** — `agent-body-core` crate with `BrainProvenance`, `ExecutionId`, `ContextBundle`, `RouteLimits`, `ScoredItem`, `TaskKind`, `WorkflowTrigger`
- **CLI** — `autonomic init` (scaffold project), `start` (daemon placeholder), `update` (upgrade placeholder), `doctor` (health check), `status`
- **Doctor module** — checks agent-brain and agent-heart binary availability
- **Config** — auto-created YAML config in `~/.config/autonomic/config.yaml`
