# agent-body Verification Suite

Verification playbooks and automated checks for `agent-body` (the Autonomic AI MCP gateway and daemon supervisor).

## Structure
- `run.sh`: Master entrypoint executing clippy, check, and all modular playbooks.
- `verify.toml`: Machine-readable descriptor of test commands.
- `playbooks/`:
  - `verify_full_suite.sh`: Comprehensive test suite across `agent-body` and `agent-body-core`.
  - `verify_mcp_gateway.sh`: Validates MCP gateway schema routing, parameter validation, and organ forwarding (`heart_record_usage`, `heart_quota_status`, `heart_usage_summary`, `muscle_execute`).
  - `verify_degradation.sh`: Validates progressive degradation ladder and supervisor health evictions.
