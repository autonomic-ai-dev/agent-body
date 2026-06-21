use anyhow::{Context, Result};
use std::process::Command;

use crate::router::{self, ORGANS};

pub fn run_update(force: bool) -> Result<()> {
    println!("Updating all Autonomic organs from latest GitHub releases...\n");

    let mut updated = 0u32;
    let mut skipped = 0u32;
    let mut failed = 0u32;

    for (alias, binary) in ORGANS {
        let which = Command::new("which").arg(binary).output();
        if !which.is_ok_and(|o| o.status.success()) {
            println!("  {alias:<10} {binary:<16} not installed — skipping");
            skipped += 1;
            continue;
        }

        print!("  {alias:<10} {binary:<16} ");
        let mut cmd = Command::new(binary);
        cmd.arg("update");
        if force {
            cmd.arg("--force");
        }
        let result = cmd
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .with_context(|| format!("spawn {binary} update"))?;

        if result.success() {
            updated += 1;
        } else {
            // The organ already printed its own error message
            failed += 1;
        }
    }

    println!();
    if updated > 0 || failed > 0 {
        println!("{updated} updated, {failed} failed, {skipped} skipped");
    }
    if failed > 0 {
        println!("Some updates failed. You can retry with `autonomic update --force`.");
    }

    update_dashboards(force)?;

    show_versions()
}

fn update_dashboards(force: bool) -> Result<()> {
    println!("\nOptional dashboards:");

    match crate::tui_install::update(force) {
        Ok(true) => println!("  agent-tui  updated"),
        Ok(false) => println!("  agent-tui  already up to date"),
        Err(err) => println!("  agent-tui  update failed: {err:#}"),
    }

    match crate::ui_relay::update(force) {
        Ok(true) => println!("  agent-ui   relay updated"),
        Ok(false) => println!("  agent-ui   relay already up to date"),
        Err(err) => println!("  agent-ui   relay update failed: {err:#}"),
    }

    Ok(())
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
