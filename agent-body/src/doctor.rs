use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use agent_body_core::ui::ProgressRun;
use anyhow::Result;

fn log_dir() -> PathBuf {
    agent_body_core::organ_state_dir("supervisor").join("logs")
}

pub async fn run(quick: bool) -> Result<()> {
    let organ_count = crate::router::ORGANS.len();
    let total = if quick { organ_count } else { organ_count + 1 };
    let mut progress = ProgressRun::new("Autonomic health check").with_total_hint(total);

    agent_body_core::ensure_dirs().ok();

    let mut all_healthy = true;
    for (alias, binary) in crate::router::ORGANS {
        let step = progress.step(format!("{alias}"));
        match check_binary(binary).await {
            Ok(_version) => step.done(),
            Err(e) => {
                step.fail(e.to_string());
                all_healthy = false;
            }
        }
    }

    let mut logs_ok = true;
    if !quick {
        let step = progress.step("supervisor logs");
        logs_ok = check_logs_quiet()?;
        if logs_ok {
            step.done();
        } else {
            step.warn(
                "historical errors in supervisor logs (may be from prior runs; run `autonomic start` then re-check)",
            );
        }
    }

    let _summary = progress.finish()?;

    if all_healthy && logs_ok {
        println!("\nAll systems healthy.");
        Ok(())
    } else {
        println!("\nSome checks failed. Run `autonomic update` for version details.");
        std::process::exit(1);
    }
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

/// Check log files for recent ERROR entries (full output for explicit doctor).
pub fn check_logs() -> Result<bool> {
    let dir = log_dir();
    if !dir.exists() {
        println!("  ~ no log directory at {}", dir.display());
        return Ok(true);
    }

    println!("Supervisor log scan (may include errors from prior runs; start daemons and re-check if stack was stopped).");

    let mut all_clean = true;
    let mut entries: Vec<_> = fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "log"))
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

fn check_logs_quiet() -> Result<bool> {
    let dir = log_dir();
    if !dir.exists() {
        return Ok(true);
    }

    let mut all_clean = true;
    for entry in fs::read_dir(&dir)?.filter_map(|e| e.ok()) {
        if !entry.path().extension().is_some_and(|ext| ext == "log") {
            continue;
        }
        let errors = scan_log_for_errors(&entry.path(), 100)?;
        if !errors.is_empty() {
            all_clean = false;
        }
    }
    Ok(all_clean)
}

fn scan_log_for_errors(path: &PathBuf, max_lines: usize) -> Result<Vec<String>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let all_lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
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
