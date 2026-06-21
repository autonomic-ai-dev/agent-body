use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use anyhow::Result;

fn log_dir() -> PathBuf {
    agent_body_core::organ_state_dir("supervisor").join("logs")
}

pub async fn check_all() -> Result<bool> {
    let mut all_healthy = true;

    agent_body_core::ensure_dirs().ok();
    println!(
        "  workspace: {}",
        agent_body_core::autonomic_root().display()
    );
    println!("  config:    {}", agent_body_core::config_path().display());
    println!();

    for (alias, binary) in crate::router::ORGANS {
        match check_binary(binary).await {
            Ok(version) => println!("  ✓ {alias} ({binary}) — {version}"),
            Err(e) => {
                println!("  ✗ {alias} ({binary}) — {e}");
                all_healthy = false;
            }
        }
    }

    Ok(all_healthy)
}

async fn check_binary(binary: &str) -> Result<String> {
    let output = tokio::process::Command::new(binary)
        .arg("--version")
        .output()
        .await?;

    if !output.status.success() {
        anyhow::bail!("not found or not executable");
    }

    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if version.is_empty() {
        Ok(String::from_utf8_lossy(&output.stderr).trim().to_string())
    } else {
        Ok(version)
    }
}

/// Check log files for recent ERROR/WARN entries and print findings.
pub fn check_logs() -> Result<bool> {
    let dir = log_dir();
    if !dir.exists() {
        println!("  ~ no log directory at {}", dir.display());
        return Ok(true);
    }

    let mut all_clean = true;
    let mut entries: Vec<_> = fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "log"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in &entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let errors = scan_log_for_errors(&path, 100)?;

        if errors.is_empty() {
            println!("  ✓ {name} — no recent errors");
        } else {
            all_clean = false;
            println!("  ⚠ {name} — {} recent error(s):", errors.len());
            for err in &errors {
                println!("      {err}");
            }
        }
    }

    Ok(all_clean)
}

/// Scan the last `max_lines` lines of a log file for ERROR and WARN entries.
fn scan_log_for_errors(path: &PathBuf, max_lines: usize) -> Result<Vec<String>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let all_lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();
    let tail = all_lines.iter().rev().take(max_lines).rev();

    let mut found = Vec::new();
    for line in tail {
        let upper = line.to_uppercase();
        if upper.contains(" ERROR") || upper.starts_with("ERROR") || upper.contains(" PANIC") {
            let trimmed = line.trim();
            if trimmed.len() > 150 {
                found.push(format!("{}...", &trimmed[..150]));
            } else {
                found.push(trimmed.to_string());
            }
        }
    }

    Ok(found)
}
