use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::config::Config;

pub fn init_project(name: Option<&str>) -> Result<()> {
    agent_body_core::ensure_dirs().context("create ~/.autonomic workspace")?;
    let _ = Config::load()?;

    if let Some(project) = name {
        let dir = PathBuf::from(project);
        if dir.exists() {
            anyhow::bail!("project directory '{}' already exists", dir.display());
        }
        fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
        let readme = format!(
            "# {project}\n\nAutonomic AI project scaffold.\n\n- Config: `~/.autonomic/config.toml`\n- Run `autonomic doctor` to verify organ binaries.\n- Run `autonomic start` to launch local daemons.\n",
            project = project
        );
        fs::write(dir.join("README.md"), readme)?;
        println!("Created project '{}'", dir.display());
    }

    println!("Autonomic workspace ready.");
    println!("  config:    {}", agent_body_core::config_path().display());
    println!("  memory:    {}", agent_body_core::memory_dir().display());
    println!("  broker:    {}", agent_body_core::broker_dir().display());
    println!(
        "  spine log: {}",
        agent_body_core::spine_logs_dir().display()
    );
    println!();
    println!("Next steps:");
    println!("  autonomic start          # broker + heart daemons");
    println!("  autonomic doctor         # verify organ binaries");
    println!("  autonomic brain serve    # route to agent-brain");
    Ok(())
}
