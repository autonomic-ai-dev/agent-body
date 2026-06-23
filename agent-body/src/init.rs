use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use agent_body_core::ui::ProgressRun;

use crate::config::Config;

pub fn init_project(name: Option<&str>) -> Result<()> {
    let total = if name.is_some() { 5 } else { 4 };
    let mut progress = ProgressRun::new("Initializing Autonomic workspace").with_total_hint(total);

    let dirs = progress.step("workspace directories");
    agent_body_core::ensure_dirs().context("create ~/.autonomic workspace")?;
    agent_body_core::run_legacy_migrations().ok();
    agent_body_core::ensure_default_ecosystem_sections().ok();
    agent_body_core::scaffold_agents_dir().ok();
    agent_body_core::write_default_gitignore().ok();
    dirs.done();

    let agents = progress.step("AGENTS.md");
    match agent_body_core::compose_agents_md() {
        Ok(path) => {
            println!("  agents:    {}", path.display());
            agents.done();
        }
        Err(err) => {
            agents.warn(format!("compose skipped: {err:#}"));
        }
    }

    let config = progress.step("config");
    let _ = Config::load()?;
    config.done();

    if let Some(project) = name {
        let scaffold = progress.step(format!("project '{project}'"));
        let dir = PathBuf::from(&project);
        if dir.exists() {
            scaffold.fail(format!("project directory '{}' already exists", dir.display()));
            progress.finish()?;
            anyhow::bail!("project directory '{}' already exists", dir.display());
        }
        fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
        let readme = format!(
            "# {project}\n\nAutonomic AI project scaffold.\n\n- Config: `~/.autonomic/config.toml`\n- Run `autonomic doctor` to verify organ binaries.\n- Run `autonomic start` to launch local daemons.\n",
            project = project
        );
        fs::write(dir.join("README.md"), readme)?;
        scaffold.done();
    }

    progress.finish()?;

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
    println!("  autonomic start          # broker + daemons");
    println!("  autonomic doctor         # verify organ binaries");
    println!("  autonomic agents compose --install  # AGENTS.md + host links");
    println!("  autonomic brain serve    # route to agent-brain");
    Ok(())
}
