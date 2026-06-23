use anyhow::{Context, Result};
use std::process::{Command, Stdio};

use agent_body_core::github_release::github_api_token;
use agent_body_core::ui::ProgressRun;

use crate::router::{self, ORGANS};

pub fn run_update(force: bool) -> Result<()> {
    if github_api_token().is_none() {
        eprintln!(
            "Note: GITHUB_TOKEN not set — updates use release redirects; API fallback may rate-limit."
        );
    }

    let dashboard_steps = 2u32;
    let organ_steps = ORGANS.len() as u32;
    let total = organ_steps + dashboard_steps;
    let mut progress = ProgressRun::new("Updating Autonomic stack").with_total_hint(total as usize);

    let mut failed = 0u32;

    for (alias, binary) in ORGANS {
        let step = progress.step(*alias);
        let which = Command::new("which").arg(binary).output();
        if !which.is_ok_and(|o| o.status.success()) {
            step.warn(format!("{binary} not installed — skipping"));
            continue;
        }

        let mut cmd = Command::new(binary);
        cmd.arg("update");
        if force {
            cmd.arg("--force");
        }
        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .with_context(|| format!("spawn {binary} update"))?;

        if output.status.success() {
            step.done();
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let mut detail = stderr.trim().to_string();
            if detail.is_empty() {
                detail = format!("{binary} update exited with {}", output.status);
            }
            if detail.contains("403") || detail.contains("rate limit") {
                detail.push_str(
                    "\nSet GITHUB_TOKEN or GH_TOKEN (public repo read is enough) and retry `autonomic update --force`.",
                );
            }
            step.fail(detail);
            failed += 1;
        }
    }

    let tui_step = progress.step("agent-tui");
    match crate::tui_install::update(force) {
        Ok(true) => {
            tui_step.done();
        }
        Ok(false) => tui_step.cached(),
        Err(err) => {
            tui_step.warn(format!("{err:#}"));
        }
    }

    let ui_step = progress.step("agent-ui relay");
    match crate::ui_relay::update(force) {
        Ok(true) => {
            ui_step.done();
        }
        Ok(false) => ui_step.cached(),
        Err(err) => {
            ui_step.warn(format!("{err:#}"));
        }
    }

    let summary = progress.finish()?;

    if summary.failed > 0 {
        eprintln!("{failed} organ update(s) failed. Retry with `autonomic update --force`.");
        show_versions()?;
        anyhow::bail!("update failed");
    }

    show_versions()
}

pub fn show_versions() -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(show_versions_async())
}

async fn show_versions_async() -> Result<()> {
    println!("Autonomic organ versions on PATH:\n");
    println!("{:<10} {:<16} status", "organ", "binary");
    println!("{}", "-".repeat(42));

    let mut join_set = tokio::task::JoinSet::new();
    for (alias, _) in ORGANS {
        let a = alias.to_string();
        join_set.spawn(async move { router::organ_version_async(&a).await });
    }

    let mut results = Vec::new();
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok((alias, version)) => results.push((alias, version)),
            Err(e) => tracing::warn!("version check failed: {e}"),
        }
    }
    results.sort_by(|a, b| a.0.cmp(&b.0));

    for (alias, version) in &results {
        let binary = router::resolve_binary(alias).unwrap_or(alias);
        match version {
            Some(v) => println!("{alias:<10} {binary:<16} {v}"),
            None => println!("{alias:<10} {binary:<16} not installed"),
        }
    }

    println!();
    println!("Install missing organs from GitHub releases or `cargo install --path <repo>`.");
    Ok(())
}
