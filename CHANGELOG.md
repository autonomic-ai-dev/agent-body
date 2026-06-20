# Changelog

## [Unreleased]

## [0.1.0] - 2026-06-20

### Added

- **Initial project scaffold** — workspace with `core` and `agent-body` crates
- **Shared types** — `agent-body-core` crate with `BrainProvenance`, `ExecutionId`, `ContextBundle`, `RouteLimits`, `ScoredItem`, `TaskKind`, `WorkflowTrigger`
- **CLI** — `autonomic init` (scaffold project), `start` (daemon placeholder), `update` (upgrade placeholder), `doctor` (health check), `status`
- **Doctor module** — checks agent-brain and agent-heart binary availability
- **Config** — auto-created YAML config in `~/.config/autonomic/config.yaml`
