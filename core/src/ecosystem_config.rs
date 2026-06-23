use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;

use crate::global_workspace;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentsConfig {
    #[serde(default = "default_agent_mode")]
    pub mode: String,
    #[serde(default = "default_true")]
    pub compose_agents_md: bool,
}

fn default_agent_mode() -> String {
    "agent-brain".into()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitSyncConfig {
    #[serde(default)]
    pub remote: String,
    #[serde(default = "default_git_branch")]
    pub branch: String,
    #[serde(default)]
    pub auto_push: bool,
}

fn default_git_branch() -> String {
    "main".into()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub git: GitSyncConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub auto: bool,
    #[serde(default = "default_interval_hours")]
    pub interval_hours: u64,
}

fn default_interval_hours() -> u64 {
    24
}

fn default_true() -> bool {
    true
}

pub fn load_config_table() -> Result<toml::Table> {
    let path = global_workspace::config_path();
    if !path.exists() {
        return Ok(toml::Table::new());
    }
    let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    toml::from_str(&raw).context("parse ~/.autonomic/config.toml")
}

pub fn agents_config() -> Result<AgentsConfig> {
    let root = load_config_table()?;
    Ok(table_to_value(root.get("agents")))
}

pub fn effective_sync(organ: Option<&str>) -> Result<SyncConfig> {
    let root = load_config_table()?;
    let mut base: SyncConfig = table_to_value(root.get("sync"));
    if let Some(organ) = organ {
        if let Some(sync_table) = root.get("sync").and_then(|v| v.as_table()) {
            if let Some(override_val) = sync_table.get(organ) {
                let organ_cfg: SyncConfig = table_to_value(Some(override_val));
                merge_sync(&mut base, &organ_cfg);
            }
        }
    }
    Ok(base)
}

pub fn effective_update(organ: &str) -> Result<UpdateConfig> {
    let root = load_config_table()?;
    let mut base: UpdateConfig = table_to_value(root.get("update"));
    if let Some(update_table) = root.get("update").and_then(|v| v.as_table()) {
        if let Some(override_val) = update_table.get(organ) {
            let organ_cfg: UpdateConfig = table_to_value(Some(override_val));
            merge_update(&mut base, &organ_cfg);
        }
    }
    Ok(base)
}

pub fn update_enabled_for_organ(organ: &str) -> Result<bool> {
    Ok(effective_update(organ)?.enabled)
}

fn merge_sync(base: &mut SyncConfig, overlay: &SyncConfig) {
    if overlay.enabled {
        base.enabled = true;
    }
    if !overlay.git.remote.is_empty() {
        base.git.remote = overlay.git.remote.clone();
    }
    if !overlay.git.branch.is_empty() && overlay.git.branch != "main" {
        base.git.branch = overlay.git.branch.clone();
    }
    if overlay.git.auto_push {
        base.git.auto_push = true;
    }
}

fn merge_update(base: &mut UpdateConfig, overlay: &UpdateConfig) {
    if !overlay.enabled {
        base.enabled = false;
    }
    if overlay.auto {
        base.auto = true;
    }
    if overlay.interval_hours != 24 {
        base.interval_hours = overlay.interval_hours;
    }
}

fn table_to_value<T>(value: Option<&toml::Value>) -> T
where
    T: for<'de> Deserialize<'de> + Default,
{
    let Some(value) = value else {
        return T::default();
    };
    let Some(table) = value.as_table() else {
        return T::default();
    };
    let Ok(json) = serde_json::to_value(table) else {
        return T::default();
    };
    serde_json::from_value(json).unwrap_or_default()
}

pub fn read_organ_section_raw(organ: &str) -> Result<Option<toml::Table>> {
    let root = load_config_table()?;
    Ok(root
        .get(organ)
        .and_then(|v| v.as_table())
        .cloned())
}

pub fn write_organ_section_raw(organ: &str, section: &toml::Table) -> Result<()> {
    let path = global_workspace::config_path();
    let mut root = load_config_table()?;
    root.insert(organ.to_string(), toml::Value::Table(section.clone()));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, toml::to_string_pretty(&root)?)?;
    Ok(())
}

pub fn ensure_default_ecosystem_sections() -> Result<()> {
    let path = global_workspace::config_path();
    let mut root = load_config_table()?;
    let mut changed = false;

    if !root.contains_key("agents") {
        let agents = AgentsConfig::default();
        root.insert(
            "agents".into(),
            toml::Value::try_from(&agents).context("serialize [agents]")?,
        );
        changed = true;
    }
    if !root.contains_key("sync") {
        root.insert("sync".into(), toml::Table::new().into());
        changed = true;
    }
    if !root.contains_key("update") {
        let update = UpdateConfig {
            enabled: true,
            ..UpdateConfig::default()
        };
        root.insert(
            "update".into(),
            toml::Value::try_from(&update).context("serialize [update]")?,
        );
        changed = true;
    }

    if changed {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, toml::to_string_pretty(&root)?)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_update_respects_organ_override() {
        let mut base = UpdateConfig {
            enabled: true,
            auto: false,
            interval_hours: 24,
        };
        let overlay = UpdateConfig {
            enabled: true,
            auto: true,
            interval_hours: 12,
        };
        merge_update(&mut base, &overlay);
        assert!(base.auto);
        assert_eq!(base.interval_hours, 12);
    }
}
