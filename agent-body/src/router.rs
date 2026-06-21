use anyhow::{bail, Context, Result};
use std::process::{Command, Stdio};

/// Organ alias → binary name on PATH.
pub const ORGANS: &[(&str, &str)] = &[
    ("brain", "agent-brain"),
    ("spine", "agent-spine"),
    ("heart", "agent-heart"),
    ("nerves", "agent-nerves"),
    ("muscle", "agent-muscle"),
    ("immune", "agent-immune"),
    ("eyes", "agent-eyes"),
    ("mouth", "agent-mouth"),
];

pub const BUILTIN_COMMANDS: &[&str] = &[
    "init",
    "start",
    "stop",
    "restart",
    "supervise",
    "update",
    "doctor",
    "status",
    "log",
    "tui",
    "help",
    "--help",
    "-h",
    "--version",
    "-V",
];

pub fn is_builtin(arg: &str) -> bool {
    BUILTIN_COMMANDS.contains(&arg)
}

pub fn resolve_binary(organ: &str) -> Option<&'static str> {
    ORGANS
        .iter()
        .find(|(alias, _)| *alias == organ)
        .map(|(_, bin)| *bin)
}

pub fn exec_organ(args: &[String]) -> Result<()> {
    if args.is_empty() {
        bail!(
            "usage: autonomic <organ> [args...]  (organs: {})",
            organ_list()
        );
    }

    let organ = &args[0];
    let binary = resolve_binary(organ)
        .with_context(|| format!("unknown organ '{organ}'. Known: {}", organ_list()))?;

    let status = Command::new(binary)
        .args(&args[1..])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("failed to spawn {binary}"))?;

    if status.success() {
        Ok(())
    } else {
        std::process::exit(status.code().unwrap_or(1));
    }
}

pub fn organ_version(organ: &str) -> Result<Option<String>> {
    let Some(binary) = resolve_binary(organ) else {
        return Ok(None);
    };
    let output = Command::new(binary).arg("--version").output();
    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            Ok(Some(if text.is_empty() {
                String::from_utf8_lossy(&out.stderr).trim().to_string()
            } else {
                text
            }))
        }
        _ => Ok(None),
    }
}

pub fn organ_list() -> String {
    ORGANS
        .iter()
        .map(|(alias, _)| *alias)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_known_organs() {
        assert_eq!(resolve_binary("brain"), Some("agent-brain"));
        assert_eq!(resolve_binary("unknown"), None);
    }

    #[test]
    fn builtins_are_reserved() {
        assert!(is_builtin("init"));
        assert!(!is_builtin("brain"));
    }
}
