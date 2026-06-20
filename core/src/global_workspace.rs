use std::path::PathBuf;

/// Root of the autonomic ecosystem (`~/.autonomic` or `AUTONOMIC_HOME`).
pub fn autonomic_root() -> PathBuf {
    std::env::var("AUTONOMIC_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home_dir().join(".autonomic"))
}

fn home_dir() -> PathBuf {
    dirs::home_dir()
        .or_else(|| std::env::var("HOME").ok().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// agent-brain memory store (`brain.db`, vectors, route logs).
pub fn memory_dir() -> PathBuf {
    autonomic_root().join("memory")
}

pub fn memory_logs_dir() -> PathBuf {
    memory_dir().join("logs")
}

/// agent-spine execution state and JSON graphs.
pub fn spine_logs_dir() -> PathBuf {
    autonomic_root().join("logs").join("spine")
}

pub fn default_state_db() -> PathBuf {
    spine_logs_dir().join("state.db")
}

pub fn executions_dir() -> PathBuf {
    spine_logs_dir().join("executions")
}

/// agent-nerves JetStream / broker persistence.
pub fn broker_dir() -> PathBuf {
    autonomic_root().join("broker")
}

/// Per-organ runtime state (e.g. agent-heart last_gc).
pub fn organ_state_dir(organ: &str) -> PathBuf {
    autonomic_root().join("state").join(organ)
}

/// Unified ecosystem config (Phase 2 source of truth).
pub fn config_path() -> PathBuf {
    autonomic_root().join("config.toml")
}

/// Legacy per-organ YAML config path under XDG config dir.
pub fn legacy_config_path(organ: &str) -> PathBuf {
    let legacy_autonomic = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("autonomic")
        .join("config.yaml");
    if organ == "autonomic" {
        return legacy_autonomic;
    }
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(organ)
        .join("config.yaml")
}

pub fn ensure_dirs() -> std::io::Result<()> {
    std::fs::create_dir_all(memory_dir())?;
    std::fs::create_dir_all(memory_logs_dir())?;
    std::fs::create_dir_all(spine_logs_dir())?;
    std::fs::create_dir_all(executions_dir())?;
    std::fs::create_dir_all(broker_dir())?;
    std::fs::create_dir_all(autonomic_root())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_live_under_autonomic_root() {
        let root = autonomic_root();
        assert!(memory_dir().starts_with(&root));
        assert!(broker_dir().starts_with(&root));
        assert!(config_path().starts_with(&root));
    }
}
