use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const UI_REPO: &str = "https://github.com/autonomic-ai-dev/agent-ui.git";
const UI_REPO_SLUG: &str = "autonomic-ai-dev/agent-ui";
const DASHBOARD_URL: &str = "https://ui-autonomic-ai.vercel.app";

fn ui_dir() -> PathBuf {
    agent_body_core::autonomic_root().join("agent-ui")
}

fn require_tool(name: &str) -> Result<()> {
    Command::new("which")
        .arg(name)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|_| ())
        .ok_or_else(|| anyhow::anyhow!("{name} not found on PATH — install it and retry"))
}

fn run_checked(mut cmd: Command, what: &str) -> Result<()> {
    let status = cmd
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("run {what}"))?;
    if !status.success() {
        bail!("{what} failed with {status}");
    }
    Ok(())
}

fn clone_repo(dir: &Path) -> Result<()> {
    println!("Cloning agent-ui into {}...", dir.display());
    if let Some(parent) = dir.parent() {
        std::fs::create_dir_all(parent).context("create autonomic workspace")?;
    }

    let mut cmd = Command::new("git");
    cmd.args(["clone", UI_REPO]).arg(dir);
    run_checked(cmd, "git clone agent-ui")
}

fn install_dependencies(dir: &Path) -> Result<()> {
    println!("Installing agent-ui relay dependencies with Bun...");
    let mut cmd = Command::new("bun");
    cmd.arg("install").current_dir(dir);
    run_checked(cmd, "bun install")
}

fn pull_latest(dir: &Path) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.args(["pull", "--ff-only"]).current_dir(dir);
    run_checked(cmd, "git pull agent-ui")
}

fn relay_version(dir: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(dir)
        .output()
        .context("read agent-ui git revision")?;
    if !output.status.success() {
        bail!("agent-ui checkout is not a git repository");
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Clone or update the local NATS→WebSocket relay used by the hosted dashboard.
pub fn update(force: bool) -> Result<bool> {
    require_tool("git")?;
    require_tool("bun")?;

    let dir = ui_dir();
    let had_relay = dir.join("server.js").is_file();

    if !had_relay {
        if dir.exists() {
            bail!(
                "{} exists but is not a valid agent-ui checkout (missing server.js)",
                dir.display()
            );
        }
        clone_repo(&dir)?;
        install_dependencies(&dir)?;
        println!("agent-ui relay installed ({})", relay_version(&dir)?);
        return Ok(true);
    }

    if !dir.join("node_modules").is_dir() {
        install_dependencies(&dir)?;
    }

    let before = relay_version(&dir)?;
    if !force && !remote_has_updates(&dir)? {
        println!("agent-ui relay already up to date ({before})");
        return Ok(false);
    }

    pull_latest(&dir)?;
    install_dependencies(&dir)?;
    let after = relay_version(&dir)?;
    if before == after && !force {
        println!("agent-ui relay already up to date ({after})");
        Ok(false)
    } else {
        println!("agent-ui relay updated to {after}");
        Ok(true)
    }
}

fn remote_has_updates(dir: &Path) -> Result<bool> {
    let fetch = Command::new("git")
        .args(["fetch", "--quiet", "origin"])
        .current_dir(dir)
        .status()
        .context("git fetch agent-ui")?;
    if !fetch.success() {
        return Ok(true);
    }

    let output = Command::new("git")
        .args(["rev-list", "--count", "HEAD..origin/master"])
        .current_dir(dir)
        .output()
        .context("compare agent-ui revisions")?;
    if !output.status.success() {
        return Ok(true);
    }

    let count = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u32>()
        .unwrap_or(1);
    Ok(count > 0)
}

fn ensure_relay(force: bool) -> Result<()> {
    if !ui_dir().join("server.js").is_file() {
        update(true)?;
        return Ok(());
    }

    if force {
        update(true)?;
    } else if !ui_dir().join("node_modules").is_dir() {
        install_dependencies(&ui_dir())?;
    }

    Ok(())
}

fn open_dashboard(url: &str) -> Result<()> {
    let status = match std::env::consts::OS {
        "macos" => Command::new("open").arg(url).status(),
        "linux" => Command::new("xdg-open").arg(url).status(),
        "windows" => Command::new("cmd").args(["/C", "start", "", url]).status(),
        other => bail!("unsupported OS for opening browser: {other}"),
    }
    .with_context(|| format!("open {url}"))?;

    if !status.success() {
        println!("Could not open browser automatically. Visit {url}");
    }
    Ok(())
}

/// Open the hosted dashboard. Optionally start the local relay for live NATS data.
pub fn run(open_only: bool, force: bool) -> Result<()> {
    println!("Opening Autonomic Web Dashboard at {DASHBOARD_URL}...");

    if open_only {
        println!(
            "Live NATS data requires the local relay. Run `autonomic ui` without --open-only."
        );
        open_dashboard(DASHBOARD_URL)?;
        return Ok(());
    }

    require_tool("git")?;
    require_tool("bun")?;
    ensure_relay(force)?;

    println!("Starting local WebSocket relay (ws://127.0.0.1:8080)...");
    let mut child = Command::new("bun")
        .arg("server.js")
        .current_dir(ui_dir())
        .env(
            "AUTONOMIC_NATS_URL",
            std::env::var("AUTONOMIC_NATS_URL").unwrap_or_else(|_| "nats://127.0.0.1:4222".into()),
        )
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("start bun server.js")?;

    std::thread::sleep(std::time::Duration::from_millis(500));
    open_dashboard(DASHBOARD_URL)?;

    println!(
        "Relay repo: {UI_REPO_SLUG} (managed at {})",
        ui_dir().display()
    );
    println!("Press Ctrl+C to stop the local relay.");
    let status = child.wait().context("wait for bun server.js")?;
    if !status.success() {
        bail!("agent-ui relay exited with {status}");
    }
    Ok(())
}
