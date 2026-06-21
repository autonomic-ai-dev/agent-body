use anyhow::{Context, Result, bail};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::github_release;

const REPO: &str = "autonomic-ai-dev/agent-tui";
const BINARY: &str = "agent-tui";

fn install_dir() -> Result<PathBuf> {
    dirs::home_dir()
        .map(|home| home.join(".local").join("bin"))
        .context("resolve home directory")
}

fn canonical_binary() -> Result<PathBuf> {
    Ok(install_dir()?.join(BINARY))
}

fn resolve_binary() -> Result<PathBuf> {
    if Command::new("which")
        .arg(BINARY)
        .output()
        .is_ok_and(|o| o.status.success())
    {
        let output = Command::new("which").arg(BINARY).output()?;
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Ok(PathBuf::from(path));
        }
    }

    let dest = canonical_binary()?;
    if dest.is_file() {
        return Ok(dest);
    }

    update(true)?;
    Ok(dest)
}

/// Download or upgrade `agent-tui` from GitHub releases.
pub fn update(force: bool) -> Result<bool> {
    let dest = canonical_binary()?;
    github_release::ensure_release_binary(REPO, BINARY, &dest, force)
}

/// Ensure `agent-tui` is installed, optionally refresh, then run it.
pub fn run(force: bool) -> Result<()> {
    let dest = canonical_binary()?;
    if !dest.is_file() {
        update(true)?;
    } else if force {
        update(true)?;
    } else {
        let _ = update(false)?;
    }

    let binary = resolve_binary()?;
    let status = Command::new(&binary)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("run {}", binary.display()))?;

    if !status.success() {
        bail!("{BINARY} exited with {status}");
    }
    Ok(())
}
