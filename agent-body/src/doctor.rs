use anyhow::Result;

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
