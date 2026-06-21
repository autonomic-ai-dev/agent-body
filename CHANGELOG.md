# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
