use anyhow::Result;

pub async fn check_all() -> Result<bool> {
    let mut all_healthy = true;

    agent_body_core::ensure_dirs().ok();
    println!(
        "  workspace: {}",
        agent_body_core::autonomic_root().display()
    );
    println!("  config:    {}", agent_body_core::config_path().display());

    if let Err(e) = check_agent_brain().await {
        println!("  ✗ agent-brain: {e}");
        all_healthy = false;
    } else {
        println!("  ✓ agent-brain");
    }

    if let Err(e) = check_agent_heart().await {
        println!("  ✗ agent-heart: {e}");
        all_healthy = false;
    } else {
        println!("  ✓ agent-heart");
    }

    Ok(all_healthy)
}

async fn check_agent_brain() -> Result<()> {
    let output = tokio::process::Command::new("agent-brain")
        .arg("--version")
        .output()
        .await?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        println!("    version: {version}");
        Ok(())
    } else {
        anyhow::bail!("not found or not executable");
    }
}

async fn check_agent_heart() -> Result<()> {
    let output = tokio::process::Command::new("agent-heart")
        .arg("--version")
        .output()
        .await?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        println!("    version: {version}");
        Ok(())
    } else {
        anyhow::bail!("not found or not executable");
    }
}
