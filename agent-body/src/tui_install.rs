use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const REPO: &str = "autonomic-ai-dev/agent-tui";
const BINARY: &str = "agent-tui";

fn install_dir() -> Result<PathBuf> {
    dirs::home_dir()
        .map(|home| home.join(".local").join("bin"))
        .context("resolve home directory")
}

fn detect_target() -> Option<&'static str> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    match (os, arch) {
        ("macos", "aarch64") => Some("aarch64-apple-darwin"),
        ("macos", "x86_64") => Some("x86_64-apple-darwin"),
        ("linux", "x86_64") => Some("x86_64-unknown-linux-gnu"),
        ("linux", "aarch64") => Some("aarch64-unknown-linux-gnu"),
        ("windows", "x86_64") => Some("x86_64-pc-windows-msvc"),
        _ => None,
    }
}

fn fetch_latest_version() -> Result<String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let output = Command::new("curl")
        .args(["-fsSL", &url])
        .output()
        .context("failed to run curl, is curl installed?")?;
    if !output.status.success() {
        bail!("GitHub API request failed — no agent-tui release published yet");
    }
    let body = String::from_utf8_lossy(&output.stdout);
    for line in body.lines() {
        if let Some(start) = line.find("\"tag_name\":\"") {
            let start = start + 12;
            if let Some(end) = line[start..].find('\"') {
                return Ok(line[start..start + end].to_string());
            }
        }
    }
    bail!("could not parse tag_name from GitHub API response");
}

#[cfg(target_os = "macos")]
fn codesign(path: &Path) {
    let path_str = path.to_string_lossy();
    let _ = Command::new("xattr")
        .args(["-cr", &path_str])
        .status();
    let _ = Command::new("codesign")
        .args(["--force", "--sign", "-", &path_str])
        .status();
}

fn download_binary(dest: &Path) -> Result<()> {
    let Some(target) = detect_target() else {
        bail!(
            "unsupported platform: {}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        );
    };

    let latest = fetch_latest_version()?;
    let url = format!("https://github.com/{REPO}/releases/latest/download/{BINARY}-{target}");
    let tmp = dest.with_extension("download");
    let tmp_str = tmp.to_string_lossy().to_string();

    println!("Downloading {BINARY} {latest} for {target}...");
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).context("create install directory")?;
    }

    let status = Command::new("curl")
        .args(["-fsSL", &url, "-o", &tmp_str])
        .status()
        .context("failed to run curl")?;
    if !status.success() {
        bail!("download failed — release may not exist for this platform ({target})");
    }

    #[cfg(unix)]
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))
        .context("set executable permissions")?;

    std::fs::rename(&tmp, dest).context("install binary")?;

    #[cfg(target_os = "macos")]
    codesign(dest);

    println!("Installed {BINARY} to {}", dest.display());
    Ok(())
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

    let dest = install_dir()?.join(BINARY);
    if dest.is_file() {
        return Ok(dest);
    }

    download_binary(&dest)?;
    Ok(dest)
}

/// Ensure `agent-tui` is installed, then run it in the foreground.
pub fn run() -> Result<()> {
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
