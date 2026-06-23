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

pub fn release_artifact_name(binary: &str, target: &str) -> String {
    if target.contains("windows") {
        format!("{binary}-{target}.exe")
    } else {
        format!("{binary}-{target}")
    }
}

/// Resolve latest release tag — redirect-first, GitHub API + token only as fallback.
pub fn fetch_latest_tag(repo: &str, binary: &str) -> Result<String> {
    let Some(target) = detect_target() else {
        bail!(
            "unsupported platform: {}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        );
    };
    let asset = release_artifact_name(binary, target);
    let latest_download = format!("https://github.com/{repo}/releases/latest/download/{asset}");
    match probe_release_tag_via_redirect(&latest_download) {
        Ok(tag) => Ok(tag),
        Err(probe_err) => {
            let url = format!("https://api.github.com/repos/{repo}/releases/latest");
            match curl_github_api(&url) {
                Ok(body) => parse_tag_name_from_json(&body).with_context(|| {
                    format!("parse GitHub release JSON for `{repo}` after redirect probe failed: {probe_err}")
                }),
                Err(api_err) => {
                    bail!(
                        "failed to resolve latest release for {repo} (redirect and API failed)\n\
                         redirect: {probe_err:#}\n\
                         api: {api_err:#}"
                    );
                }
            }
        }
    }
}

pub fn parse_release_tag_from_github_location(location: &str) -> Option<String> {
    const MARKER: &str = "/releases/download/";
    let rest = location
        .find(MARKER)
        .map(|idx| &location[idx + MARKER.len()..])?;
    let tag = rest.split('/').next().filter(|s| !s.is_empty())?;
    Some(tag.to_string())
}

fn probe_release_tag_via_redirect(latest_download_url: &str) -> Result<String> {
    let location = curl_first_redirect_location(latest_download_url)?;
    parse_release_tag_from_github_location(&location)
        .with_context(|| format!("parse release tag from GitHub redirect: {location}"))
}

fn curl_first_redirect_location(url: &str) -> Result<String> {
    let output = Command::new("curl")
        .args(["-fsS", "-o", "/dev/null", "-D", "-", url])
        .output()
        .context("spawn curl")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("curl HEAD failed for {url} ({}): {stderr}", output.status);
    }
    let headers = String::from_utf8_lossy(&output.stdout);
    for line in headers.lines() {
        let trimmed = line.trim();
        if trimmed.len() > 9 && trimmed[..9].eq_ignore_ascii_case("location:") {
            return Ok(trimmed[9..].trim().to_string());
        }
    }
    bail!("no redirect location in GitHub response for {url}");
}

fn parse_tag_name_from_json(body: &str) -> Result<String> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(tag) = value.get("tag_name").and_then(|v| v.as_str()) {
            return Ok(tag.to_string());
        }
    }
    for line in body.lines() {
        if let Some(start) = line.find("\"tag_name\":") {
            let rest = line[start + 11..].trim_start();
            if rest.starts_with('"') {
                let rest = &rest[1..];
                if let Some(end) = rest.find('"') {
                    return Ok(rest[..end].to_string());
                }
            }
        }
    }
    bail!("could not parse tag_name from GitHub API response");
}

pub fn github_api_token() -> Option<String> {
    for key in ["GITHUB_TOKEN", "GH_TOKEN", "GITHUB_PERSONAL_ACCESS_TOKEN"] {
        if let Ok(val) = std::env::var(key) {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn curl_github_api(url: &str) -> Result<String> {
    let auth = github_api_token();
    let auth_header = auth.as_ref().map(|token| format!("Bearer {token}"));
    let mut args: Vec<String> = vec![
        "-fsSL".into(),
        "-H".into(),
        "Accept: application/vnd.github+json".into(),
        "-H".into(),
        "User-Agent: autonomic-ai".into(),
    ];
    if let Some(ref header) = auth_header {
        args.push("-H".into());
        args.push(header.clone());
    }
    args.push(url.to_string());
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    curl_stdout(&arg_refs).map_err(|err| enrich_github_api_error(err, url, auth.is_some()))
}

fn enrich_github_api_error(err: anyhow::Error, url: &str, authenticated: bool) -> anyhow::Error {
    let msg = err.to_string();
    let rate_limited = msg.contains("403")
        && (msg.contains("rate limit")
            || msg.contains("API rate limit exceeded")
            || msg.contains("secondary rate limit"));
    if !rate_limited {
        return err;
    }
    let hint = if authenticated {
        "GitHub API rate limit exceeded. Wait a few minutes and retry."
    } else {
        "GitHub API rate limit exceeded (unauthenticated). \
         Updates normally use release redirects and avoid the API; this fallback only runs when redirect probing fails. \
         Set GITHUB_TOKEN or GH_TOKEN (public repo read is enough) and retry."
    };
    anyhow::anyhow!("curl failed for {url}: {msg}\n{hint}")
}

fn curl_stdout(args: &[&str]) -> Result<String> {
    let output = Command::new("curl")
        .args(args)
        .output()
        .context("spawn curl")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let url = args.iter().rev().find(|a| a.starts_with("http")).copied();
        if let Some(url) = url {
            bail!("curl failed for {url} ({}): {stderr}", output.status);
        }
        bail!("curl failed ({}): {stderr}", output.status);
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
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
pub fn adhoc_sign_macos(path: &Path) {
    let path_str = path.to_string_lossy();
    let _ = Command::new("xattr")
        .args(["-cr", &path_str])
        .status();
    let _ = Command::new("codesign")
        .args(["--force", "--sign", "-", &path_str])
        .status();
}

#[cfg(not(target_os = "macos"))]
pub fn adhoc_sign_macos(_path: &Path) {}

pub fn download_release_binary(repo: &str, binary: &str, dest: &Path, tag: &str) -> Result<()> {
    let Some(target) = detect_target() else {
        bail!(
            "unsupported platform: {}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        );
    };

    let artifact = release_artifact_name(binary, target);
    let url = format!("https://github.com/{repo}/releases/download/{tag}/{artifact}");
    let tmp = dest.with_extension("download");
    let tmp_str = tmp.to_string_lossy().to_string();

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
    adhoc_sign_macos(dest);
    Ok(())
}

pub fn run_organ_self_update(
    repo: &str,
    binary: &str,
    current_version: &str,
    force: bool,
) -> Result<bool> {
    let latest = fetch_latest_tag(repo, binary)?;
    let latest_ver = latest.trim_start_matches('v');
    let current_ver = current_version.trim_start_matches('v');

    if !force && !version_is_newer(latest_ver, current_ver) {
        println!("{binary} already at latest version ({current_version})");
        return Ok(false);
    }

    let exe = std::env::current_exe().context("get current exe path")?;
    let tmp = exe.with_extension("download");

    println!("Downloading {binary} {latest}...");
    download_release_binary(repo, binary, &tmp, &latest)?;
    std::fs::rename(&tmp, &exe).context("replace binary")?;
    adhoc_sign_macos(&exe);

    println!("Updated {binary} from v{current_version} to v{latest}");
    Ok(true)
}

pub fn ensure_release_binary(
    repo: &str,
    binary: &str,
    dest: &Path,
    force: bool,
) -> Result<bool> {
    let latest = fetch_latest_tag(repo, binary)?;
    let latest_ver = latest.trim_start_matches('v');

    if dest.is_file() {
        if let Some(current) = read_installed_version(dest) {
            let current_ver = current.trim_start_matches('v');
            if !force && !version_is_newer(latest_ver, current_ver) {
                return Ok(false);
            }
        } else if !force {
            return Ok(false);
        }
    }

    download_release_binary(repo, binary, dest, &latest)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tag_from_location() {
        let loc = "https://github.com/autonomic-ai-dev/agent-body/releases/download/v0.5.11/agent-body-aarch64-apple-darwin";
        assert_eq!(
            parse_release_tag_from_github_location(loc).as_deref(),
            Some("v0.5.11")
        );
    }

    #[test]
    fn version_compare() {
        assert!(version_is_newer("0.5.2", "0.5.1"));
        assert!(!version_is_newer("0.5.1", "0.5.2"));
    }
}
