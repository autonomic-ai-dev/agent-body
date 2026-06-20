# Changelog

## [Unreleased]

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
