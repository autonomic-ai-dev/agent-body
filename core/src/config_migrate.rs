use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::global_workspace;

/// Migrate `~/.agent_brain/config.yaml` into `[brain]` in unified config.
pub fn migrate_brain_yaml_if_needed() -> Result<bool> {
    let legacy_home = global_workspace::legacy_brain_home();
    let candidates = ["config.yaml", "config.yml", "config.json"];
    let mut legacy_path = None;
    for name in candidates {
        let p = legacy_home.join(name);
        if p.is_file() {
            legacy_path = Some(p);
            break;
        }
    }
    let Some(legacy_path) = legacy_path else {
        return Ok(false);
    };

    let unified = global_workspace::config_path();
    if unified.exists() {
        let raw = fs::read_to_string(&unified)?;
        let root: toml::Table = toml::from_str(&raw).unwrap_or_default();
        if root.contains_key("brain") {
            return Ok(false);
        }
    }

    let content = fs::read_to_string(&legacy_path)?;
    let json: serde_json::Value =
        if legacy_path.extension().and_then(|e| e.to_str()) == Some("json") {
            serde_json::from_str(&content)?
        } else {
            serde_yaml::from_str(&content)?
        };
    let table: toml::Table = serde_json::from_value(json)?;

    let mut root: toml::Table = if unified.exists() {
        toml::from_str(&fs::read_to_string(&unified)?).unwrap_or_default()
    } else {
        toml::Table::new()
    };
    root.insert("brain".into(), toml::Value::Table(table));
    if let Some(parent) = unified.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(unified, toml::to_string_pretty(&root)?)?;
    eprintln!(
        "Migrated brain config from {} into {}",
        legacy_path.display(),
        global_workspace::config_path().display()
    );
    Ok(true)
}

/// Migrate `~/.config/agent-spine/config.toml` into `[spine]` and workflows dir.
pub fn migrate_spine_legacy_if_needed() -> Result<bool> {
    let legacy_dir = global_workspace::legacy_spine_config_dir();
    let legacy_config = legacy_dir.join("config.toml");
    if !legacy_config.is_file() {
        return Ok(false);
    }

    let unified = global_workspace::config_path();
    if unified.exists() {
        let raw = fs::read_to_string(&unified)?;
        let root: toml::Table = toml::from_str(&raw).unwrap_or_default();
        if root.contains_key("spine") {
            return Ok(false);
        }
    }

    let content = fs::read_to_string(&legacy_config)?;
    let spine_table: toml::Table = toml::from_str(&content).unwrap_or_default();

    let mut root: toml::Table = if unified.exists() {
        toml::from_str(&fs::read_to_string(&unified)?).unwrap_or_default()
    } else {
        toml::Table::new()
    };
    root.insert("spine".into(), toml::Value::Table(spine_table));
    if let Some(parent) = unified.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(unified, toml::to_string_pretty(&root)?)?;

    let workflows_src = legacy_dir.join("workflows");
    let workflows_dst = global_workspace::spine_config_dir().join("workflows");
    if workflows_src.is_dir() && !workflows_dst.exists() {
        copy_dir_recursive(&workflows_src, &workflows_dst)?;
    }

    eprintln!(
        "Migrated spine config from {} into {}",
        legacy_config.display(),
        global_workspace::config_path().display()
    );
    Ok(true)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dest)?;
        } else {
            fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}

pub fn run_legacy_migrations() -> Result<()> {
    let _ = migrate_brain_yaml_if_needed();
    let _ = migrate_spine_legacy_if_needed();
    Ok(())
}
