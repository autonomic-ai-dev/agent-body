use anyhow::Result;

use crate::ecosystem_config;

pub fn should_update_organ(organ: &str, force: bool) -> Result<bool> {
    if force {
        return Ok(true);
    }
    let cfg = ecosystem_config::effective_update(organ)?;
    Ok(cfg.enabled)
}

pub fn organ_alias_for_binary(binary: &str) -> Option<&'static str> {
    match binary {
        "agent-brain" => Some("brain"),
        "agent-spine" => Some("spine"),
        "agent-heart" => Some("heart"),
        "agent-nerves" => Some("nerves"),
        "agent-muscle" => Some("muscle"),
        "agent-immune" => Some("immune"),
        "agent-eyes" => Some("eyes"),
        "agent-mouth" => Some("mouth"),
        "agent-body" | "autonomic" => Some("autonomic"),
        _ => None,
    }
}

pub fn should_update_binary(binary: &str, force: bool) -> Result<bool> {
    if force {
        return Ok(true);
    }
    match organ_alias_for_binary(binary) {
        Some(organ) => should_update_organ(organ, false),
        None => Ok(true),
    }
}
