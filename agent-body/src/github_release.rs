use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub fn detect_target() -> Option<&'static str> {
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

pub fn parse_version(raw: &str) -> Vec<u32> {
    raw.trim()
        .trim_start_matches('v')
        .split('.')
        .filter_map(|part| part.parse::<u32>().ok())
        .collect()
}

pub fn version_is_newer(latest: &str, current: &str) -> bool {
    parse_version(latest) > parse_version(current)
}

pub fn fetch_latest_tag(repo: &str) -> Result<String> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let output = Command::new("curl")
        .args(["-fsSL", &url])
        .output()
        .context("failed to run curl, is curl installed?")?;
    if !output.status.success() {
        bail!("GitHub API request failed for {repo}");
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

pub fn version_stamp_path(binary_path: &Path) -> PathBuf {
    binary_path.with_extension("version")
}

pub fn read_installed_version(binary_path: &Path) -> Option<String> {
    let stamp = version_stamp_path(binary_path);
    std::fs::read_to_string(stamp).ok().map(|s| s.trim().to_string())
}

fn write_installed_version(binary_path: &Path, tag: &str) -> Result<()> {
    let stamp = version_stamp_path(binary_path);
    std::fs::write(&stamp, format!("{tag}\n")).with_context(|| format!("write {}", stamp.display()))
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

pub fn download_release_binary(repo: &str, binary: &str, dest: &Path, tag: &str) -> Result<()> {
    let Some(target) = detect_target() else {
        bail!(
            "unsupported platform: {}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        );
    };

    let artifact = if target.contains("windows") {
        format!("{binary}-{target}.exe")
    } else {
        format!("{binary}-{target}")
    };
    let url = format!("https://github.com/{repo}/releases/download/{tag}/{artifact}");
    let tmp = dest.with_extension("download");
    let tmp_str = tmp.to_string_lossy().to_string();

    println!("Downloading {binary} {tag} for {target}...");
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
    write_installed_version(dest, tag)?;

    #[cfg(target_os = "macos")]
    codesign(dest);

    println!("Installed {binary} {tag} to {}", dest.display());
    Ok(())
}

pub fn ensure_release_binary(
    repo: &str,
    binary: &str,
    dest: &Path,
    force: bool,
) -> Result<bool> {
    let latest = fetch_latest_tag(repo)?;
    let latest_ver = latest.trim_start_matches('v');

    if dest.is_file() {
        if let Some(current) = read_installed_version(dest) {
            let current_ver = current.trim_start_matches('v');
            if !force && !version_is_newer(latest_ver, current_ver) {
                return Ok(false);
            }
        } else if !force {
            // Binary exists but no stamp — keep it unless forced.
            return Ok(false);
        }
    }

    download_release_binary(repo, binary, dest, &latest)?;
    Ok(true)
}
