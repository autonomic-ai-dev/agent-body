# command-policy

Summary: One bash AST guard decides which shell commands are rejected before any organ runs them. agent-muscle, agent-mouth, and agent-immune all use the same rules, so an agent gets the same answer from each.

## Sub-features

- `guard-inspect`: Rejects destructive or dynamic shell commands.
  - Intent: One policy that every organ applies.
  - Behaviour: `inspect(source)` in `core/src/guard.rs` parses the command with tree-sitter bash. It rejects recursive `rm`, `find -delete`, `eval`/`source`/`.`, dynamic command names, raw device writes, `mkfs`/`newfs`, `diskutil` erase commands, pipe-to-shell from a fetcher, fork bombs, and any syntax error or parse failure.
  - Outcome: An empty result means allowed. Each rejection returns a line and a message.
- `parity-corpus`: A shared list of allowed and rejected commands.
  - Intent: Let every organ prove it agrees with the shared policy.
  - Behaviour: `COMMAND_POLICY_CORPUS` in `core/src/guard_corpus.rs` holds `(command, rejected)` pairs. The body-core test `guard_agrees_with_every_corpus_entry` checks each one against `inspect`.
  - Outcome: Every entry agrees with the guard. Organ tests run the same table against their own wrappers.

## Code

- `core/src/guard.rs`
- `core/src/guard_corpus.rs`

## How to get to it (user POV)

- This crate exposes `agent_body_core::guard::inspect`. Mouth, muscle, and immune call it at v1.0.0-beta.3.11.

## Verification Suites (Targeted Test Commands)

- `cargo test --workspace` in agent-body runs `guard::tests` and `guard_corpus::tests`.

## Gotchas

- The policy is stricter than before for agent-muscle. `rm -rf target/` and `find ... -delete` are now rejected everywhere, not only under protected roots. Clean build directories with a non-recursive command or a tool-specific clean.
- Mouth, muscle, and immune call this guard at v1.0.0-beta.3.11. Each organ's wrapper runs `COMMAND_POLICY_CORPUS` in its own tests.
