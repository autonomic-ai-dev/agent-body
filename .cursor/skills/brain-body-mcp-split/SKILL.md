---
name: brain-body-mcp-split
description: Mitigate dual MCP when both agent-brain and agent-body are installed — which server for route_task vs organ tools, avoid duplicate work, route_to_mcp federation, conflict checklist.
---

# Brain vs body MCP (dual install)

Both **agent-brain** and **agent-body** may appear in Cursor/OpenCode `mcp.json`. That is **supported**. Mitigation is **role split**, not removing one.

## Roles (hard split)

| Server | Owns | Use for | Do **not** use for |
|--------|------|---------|---------------------|
| **agent-brain** | Turn routing, memory, skills, verify ledger | `route_task`, `store_memory`, `grep_search`, `file_summary`, `route_to_mcp` | Shell execute, workflows, budget, sandbox, UI QA |
| **agent-body** | Control plane + organ gateway | `muscle_*`, `spine_*`, `heart_*`, `eyes_*`, `immune_*`, `mouth_*`, `autonomic` CLI | Replacing `route_task` / memory / skill routing |

## Recommended topology

1. **Host MCP:** both registered (`agent-brain` + `agent-body` serve-mcp).
2. **Federation:** `[brain.upstream_mcp]` points at body so brain can suggest/call body tools via **`route_to_mcp`**.
3. **Every turn:** call **brain** `route_task` first (route gate). Then either:
   - call **body** tools directly when listed, or
   - call **brain** `route_to_mcp` with `server=agent-body` when `suggested_tools` says so.

```text
User message
    → agent-brain route_task          (skills/rules/memory + suggested_tools)
    → agent-body muscle_/spine_/…     (or route_to_mcp → agent-body)
    → agent-brain store_memory        (end of task)
```

## How the agent mitigates confusion

1. **Name prefixes:** body tools are `muscle_*` / `spine_*` / … — never `route_task`. Brain tools never execute shell via muscle.
2. **One owner per concern:** routing → brain; execution → body; don’t run the same test via ad-hoc Shell **and** `muscle_execute_bash`.
3. **Prefer `suggested_tools`:** if `route_task` returns `server=agent-body`, use that path instead of inventing CLI.
4. **Gate:** if brain MCP tools are denied, re-call `route_task` — do not “switch” to body to bypass the gate.
5. **Doctor:** `agent-brain doctor --fix` keeps both mcp.json entries + upstream in sync.

## Anti-patterns

| Bad | Why | Good |
|-----|-----|------|
| Skip `route_task` because body MCP is enough | Loses skills/memory/must_apply | Always `route_task` on brain |
| Use body for “which skill?” | Body doesn’t route skills | Brain `route_task` / `registry list` |
| Use brain Shell to replace `muscle_execute_bash` | No sandbox/budget mesh | Body `muscle_execute_bash` |
| Call the same organ tool twice (direct + route_to_mcp) | Duplicate side effects | Pick one path per action |
| Disable body MCP “to simplify” | Loses gateway | Keep both; follow the table |

## Install / repair

```bash
agent-brain doctor --fix          # brain + body in mcp.json, upstream enabled
agent-brain doctor --mcp           # schema audit of body serve-mcp
autonomic status                   # organs up
```

If only one MCP is present: install the missing side (`install-agent-body` skill or `agent-brain install --global`).

## Related

- Skill **install-agent-body** — install/wire body
- Skill **rmcp-mcp-gateway** — schema/debug
- `agent-brain docs/AGENT-CONTEXT.md` — prefer body for organ work
