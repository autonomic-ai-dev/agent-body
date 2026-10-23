# agent-body features

## Baseline preconditions

- Rust toolchain matching `Cargo.lock`.
- Daemon tests need `nats-server` on PATH and a writable `~/.autonomic/`. Tests must use a temporary home directory.

## Driving conventions

- Unit tests run through `cargo test`. Logs go to `.verify/artifacts/<RUN_ID>/`. Baseline logs are in `.verify/artifacts/feature-map-baseline/`.

## Proof standards

- A feature is proven when its unit tests pass and, for the gateway, a proxied call returns the organ's result.

## Feature index

| Feature file | Sub-features | Primary surface |
|---|---|---|
| `supervisor-and-gateway.md` | 7 | `autonomic init/start/stop/doctor/status/launchd/serve-mcp` |
