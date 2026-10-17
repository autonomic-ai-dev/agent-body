use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use agent_body_core::organ_state_dir;

pub const PLIST_LABEL: &str = "com.autonomic.daemons";

pub fn plist_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine user home directory")?;
    Ok(home
        .join("Library/LaunchAgents")
        .join(format!("{PLIST_LABEL}.plist")))
}

pub fn resolve_binary_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        let exe_str = exe.to_string_lossy();
        if !exe_str.contains("/target/debug/") && !exe_str.contains("/target/release/") {
            return exe;
        }
    }

    for name in &["autonomic", "agent-body"] {
        if let Ok(output) = Command::new("which").arg(name).output() {
            if output.status.success() {
                let p = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !p.is_empty() {
                    return PathBuf::from(p);
                }
            }
        }
    }

    PathBuf::from("/opt/homebrew/bin/autonomic")
}

pub fn generate_plist(
    binary_path: &Path,
    stdout_log: &Path,
    stderr_log: &Path,
    path_env: &str,
) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{binary}</string>
        <string>supervise</string>
        <string>--interval</string>
        <string>5</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{stdout}</string>
    <key>StandardErrorPath</key>
    <string>{stderr}</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>{path}</string>
    </dict>
</dict>
</plist>
"#,
        label = PLIST_LABEL,
        binary = binary_path.display(),
        stdout = stdout_log.display(),
        stderr = stderr_log.display(),
        path = path_env,
    )
}

pub fn install() -> Result<()> {
    let plist = plist_path()?;
    if let Some(parent) = plist.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create dir {}", parent.display()))?;
    }

    let log_dir = organ_state_dir("supervisor").join("logs");
    fs::create_dir_all(&log_dir)?;
    let stdout_log = log_dir.join("launchd.stdout.log");
    let stderr_log = log_dir.join("launchd.stderr.log");

    let binary = resolve_binary_path();
    let path_env = std::env::var("PATH").unwrap_or_else(|_| {
        "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin".to_string()
    });

    let content = generate_plist(&binary, &stdout_log, &stderr_log, &path_env);
    fs::write(&plist, content)
        .with_context(|| format!("failed to write plist to {}", plist.display()))?;

    // Unload existing service if already loaded, then load -w
    let _ = Command::new("launchctl").arg("unload").arg(&plist).output();

    let status = Command::new("launchctl")
        .arg("load")
        .arg("-w")
        .arg(&plist)
        .status()
        .context("failed to execute launchctl load")?;

    if !status.success() {
        anyhow::bail!("launchctl load failed with status: {status}");
    }

    println!("✓ LaunchAgent installed: {}", plist.display());
    println!("  Label: {PLIST_LABEL}");
    println!("  Binary: {}", binary.display());
    println!("  Logs: {}", log_dir.display());
    Ok(())
}

pub fn uninstall() -> Result<()> {
    let plist = plist_path()?;
    if plist.exists() {
        let _ = Command::new("launchctl").arg("unload").arg(&plist).output();
        fs::remove_file(&plist).with_context(|| format!("failed to remove {}", plist.display()))?;
        println!("✓ LaunchAgent uninstalled: {}", plist.display());
    } else {
        println!("LaunchAgent plist does not exist: {}", plist.display());
    }
    Ok(())
}

pub fn status() -> Result<()> {
    let plist = plist_path()?;
    println!("LaunchAgent: {}", plist.display());
    println!("Installed: {}", if plist.exists() { "yes" } else { "no" });

    let output = Command::new("launchctl")
        .arg("list")
        .arg(PLIST_LABEL)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let info = String::from_utf8_lossy(&out.stdout);
            println!("Status in launchd:\n{}", info.trim());
        }
        Ok(_) => {
            println!("Status in launchd: not running or not loaded");
        }
        Err(e) => {
            println!("Error querying launchctl: {e}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_plist() {
        let binary = Path::new("/opt/homebrew/bin/autonomic");
        let stdout = Path::new("/tmp/test.stdout.log");
        let stderr = Path::new("/tmp/test.stderr.log");
        let path_env = "/usr/bin:/bin";

        let plist = generate_plist(binary, stdout, stderr, path_env);
        assert!(plist.contains("<string>com.autonomic.daemons</string>"));
        assert!(plist.contains("<string>/opt/homebrew/bin/autonomic</string>"));
        assert!(plist.contains("<string>supervise</string>"));
        assert!(plist.contains("<key>RunAtLoad</key>"));
        assert!(plist.contains("<true/>"));
        assert!(plist.contains("<string>/tmp/test.stdout.log</string>"));
    }
}
