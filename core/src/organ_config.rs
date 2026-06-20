use anyhow::{Context, Result};
use serde::{Serialize, de::DeserializeOwned};
use std::fs;
use std::path::Path;

use crate::global_workspace;

/// Load an organ's config from `~/.autonomic/config.toml` `[organ]` section,
/// migrating legacy `~/.config/<organ>/config.yaml` when present.
pub fn load<T>(organ: &str) -> Result<T>
where
    T: DeserializeOwned + Default + Serialize,
{
    global_workspace::ensure_dirs().map_err(|e| anyhow::anyhow!(e))?;
    let unified = global_workspace::config_path();

    if unified.exists() {
        if let Some(cfg) = read_section(&unified, organ)? {
            return Ok(cfg);
        }
    }

    let legacy = global_workspace::legacy_config_path(organ);
    if legacy.exists() {
        let content = fs::read_to_string(&legacy)?;
        let cfg: T = serde_yaml::from_str(&content)
            .with_context(|| format!("parse legacy config {}", legacy.display()))?;
        write_section(&unified, organ, &cfg)?;
        return Ok(cfg);
    }

    let cfg = T::default();
    write_section(&unified, organ, &cfg)?;
    Ok(cfg)
}

pub fn config_path() -> std::path::PathBuf {
    global_workspace::config_path()
}

fn read_section<T>(path: &Path, organ: &str) -> Result<Option<T>>
where
    T: DeserializeOwned,
{
    let content = fs::read_to_string(path)?;
    let root: toml::Table = toml::from_str(&content).context("parse unified config.toml")?;
    let Some(value) = root.get(organ) else {
        return Ok(None);
    };
    let table = value
        .as_table()
        .context(format!("config section [{organ}] must be a table"))?;
    let json = serde_json::to_value(table).context("convert organ config to JSON")?;
    let cfg: T = serde_json::from_value(json).context("decode organ config section")?;
    Ok(Some(cfg))
}

fn write_section<T>(path: &Path, organ: &str, cfg: &T) -> Result<()>
where
    T: Serialize,
{
    let mut root: toml::Table = if path.exists() {
        toml::from_str(&fs::read_to_string(path)?).unwrap_or_default()
    } else {
        toml::Table::new()
    };

    let value: toml::Table = toml::Value::try_from(cfg)
        .context("serialize organ config")?
        .as_table()
        .cloned()
        .context("organ config must serialize to table")?;
    root.insert(organ.to_string(), toml::Value::Table(value));

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, toml::to_string_pretty(&root)?)?;
    Ok(())
}
