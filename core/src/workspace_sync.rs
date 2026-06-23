use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::ecosystem_config;
use crate::global_workspace;

pub const DEFAULT_GITIGNORE: &str = r#"# Autonomic workspace — whole-home sync ignores runtime noise
broker/
state/supervisor/*.pid
**/*.db-wal
**/*.db-shm
**/.DS_Store
logs/supervisor/*.log
"#;

pub fn write_default_gitignore() -> Result<()> {
    let path = global_workspace::workspace_gitignore_path();
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, DEFAULT_GITIGNORE).context("write .gitignore")?;
    Ok(())
}

fn git(cmd: &[&str], cwd: &Path) -> Result<String> {
    let out = Command::new("git")
        .args(cmd)
        .current_dir(cwd)
        .output()
        .with_context(|| format!("run git {}", cmd.join(" ")))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        bail!("git {} failed: {}", cmd.join(" "), stderr.trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn git_init(remote: Option<&str>, branch: Option<&str>) -> Result<()> {
    let root = global_workspace::autonomic_root();
    global_workspace::ensure_dirs().map_err(|e| anyhow::anyhow!(e))?;
    write_default_gitignore()?;

    if !root.join(".git").exists() {
        git(&["init"], &root)?;
    }

    if let Some(remote) = remote.filter(|s| !s.is_empty()) {
        let existing = Command::new("git")
            .args(["remote", "get-url", "origin"])
            .current_dir(&root)
            .output()
            .ok()
            .filter(|o| o.status.success());
        if existing.is_none() {
            git(&["remote", "add", "origin", remote], &root)?;
        }
    }

    if let Some(branch) = branch.filter(|s| !s.is_empty()) {
        let _ = git(&["checkout", "-B", branch], &root);
    }

    let _ = git(&["add", "-A"], &root);
    let status = git(&["status", "--porcelain"], &root)?;
    if !status.is_empty() {
        let _ = git(
            &["commit", "-m", "autonomic: workspace sync init"],
            &root,
        );
    }
    Ok(())
}

pub fn git_status() -> Result<String> {
    let root = global_workspace::autonomic_root();
    if !root.join(".git").exists() {
        bail!("no git repo at {} — run `autonomic sync git init`", root.display());
    }
    git(&["status", "-sb"], &root)
}

pub fn git_pull() -> Result<()> {
    let sync = ecosystem_config::effective_sync(None)?;
    if !sync.enabled {
        bail!("sync disabled in config ([sync] enabled = false)");
    }
    let root = global_workspace::autonomic_root();
    let branch = if sync.git.branch.is_empty() {
        "main"
    } else {
        sync.git.branch.as_str()
    };
    let _ = git(&["pull", "origin", branch], &root)?;
    Ok(())
}

pub fn git_push() -> Result<()> {
    let sync = ecosystem_config::effective_sync(None)?;
    if !sync.enabled {
        bail!("sync disabled in config ([sync] enabled = false)");
    }
    let root = global_workspace::autonomic_root();
    if !root.join(".git").exists() {
        git_init(
            (!sync.git.remote.is_empty()).then_some(sync.git.remote.as_str()),
            Some(sync.git.branch.as_str()),
        )?;
    }
    let branch = if sync.git.branch.is_empty() {
        "main"
    } else {
        sync.git.branch.as_str()
    };
    let _ = git(&["add", "-A"], &root);
    let porcelain = git(&["status", "--porcelain"], &root)?;
    if !porcelain.is_empty() {
        let _ = git(
            &["commit", "-m", "autonomic: workspace sync"],
            &root,
        );
    }
    let _ = git(&["push", "-u", "origin", branch], &root)?;
    Ok(())
}
