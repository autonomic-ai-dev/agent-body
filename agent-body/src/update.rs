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

    show_versions()
}

pub fn show_versions() -> Result<()> {
    println!("Autonomic organ versions on PATH:\n");
    println!("{:<10} {:<16} status", "organ", "binary");
    println!("{}", "-".repeat(42));

    for (alias, binary) in ORGANS {
        match router::organ_version(alias)? {
            Some(version) => println!("{alias:<10} {binary:<16} {version}"),
            None => println!("{alias:<10} {binary:<16} not installed"),
        }
    }

    println!();
    println!("Install missing organs from GitHub releases or `cargo install --path <repo>`.");
    Ok(())
}
