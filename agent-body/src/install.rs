use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostTarget {
    Cursor { global: bool },
    ClaudeDesktop,
    VsCode { user: bool },
    ClaudeCode { user: bool },
    OpenCode { user: bool },
    Codex { user: bool },
    Gemini { user: bool },
    Antigravity { user: bool },
    All,
}

impl HostTarget {
    #[allow(clippy::too_many_arguments)]
    pub fn parse(
        host: Option<&str>,
        all: bool,
        global: bool,
        cursor: bool,
        claude_desktop: bool,
        vscode: bool,
        claude_code: bool,
        opencode: bool,
        codex: bool,
        gemini: bool,
        antigravity: bool,
    ) -> Self {
        if all || host == Some("all") {
            return Self::All;
        }
        if cursor || host == Some("cursor") {
            return Self::Cursor { global };
        }
        if claude_desktop || host == Some("claude-desktop") || host == Some("claude") {
            return Self::ClaudeDesktop;
        }
        if vscode || host == Some("vscode") {
            return Self::VsCode { user: global };
        }
        if claude_code || host == Some("claude-code") {
            return Self::ClaudeCode { user: global };
        }
        if opencode || host == Some("opencode") {
            return Self::OpenCode { user: global };
        }
        if codex || host == Some("codex") {
            return Self::Codex { user: global };
        }
        if gemini || host == Some("gemini") {
            return Self::Gemini { user: global };
        }
        if antigravity || host == Some("antigravity") {
            return Self::Antigravity { user: global };
        }
        // Default to Cursor with global flag
        Self::Cursor { global }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Cursor { global } if global => "cursor (global)",
            Self::Cursor { .. } => "cursor (workspace)",
            Self::ClaudeDesktop => "claude-desktop",
            Self::VsCode { user: true } => "vscode (user)",
            Self::VsCode { .. } => "vscode (workspace)",
            Self::ClaudeCode { user: true } => "claude-code (user)",
            Self::ClaudeCode { .. } => "claude-code (project)",
            Self::OpenCode { user: true } => "opencode (user)",
            Self::OpenCode { .. } => "opencode (project)",
            Self::Codex { user: true } => "codex (user)",
            Self::Codex { .. } => "codex (project)",
            Self::Gemini { user: true } => "gemini (user)",
            Self::Gemini { .. } => "gemini (project)",
            Self::Antigravity { user: true } => "antigravity (user)",
            Self::Antigravity { .. } => "antigravity (project)",
            Self::All => "all agent hosts",
        }
    }
}

pub fn resolve_mcp_command_path() -> PathBuf {
    if Path::new("/opt/homebrew/bin/agent-body").exists() {
        PathBuf::from("/opt/homebrew/bin/agent-body")
    } else if Path::new("/usr/local/bin/agent-body").exists() {
        PathBuf::from("/usr/local/bin/agent-body")
    } else if let Ok(current) = std::env::current_exe() {
        current
    } else {
        PathBuf::from("agent-body")
    }
}

pub fn mcp_server_entry(exe: &Path) -> Value {
    json!({
        "command": exe.display().to_string(),
        "args": ["serve-mcp"],
        "env": {
            "RUST_LOG": "info"
        }
    })
}

pub fn vscode_server_entry(exe: &Path) -> Value {
    json!({
        "type": "stdio",
        "command": exe.display().to_string(),
        "args": ["serve-mcp"],
        "env": {
            "RUST_LOG": "info"
        }
    })
}

pub fn opencode_server_entry(exe: &Path) -> Value {
    json!({
        "type": "local",
        "command": [exe.display().to_string(), "serve-mcp".to_string()],
        "enabled": true,
        "environment": {
            "RUST_LOG": "info"
        }
    })
}

pub fn codex_mcp_toml_block(exe: &Path) -> String {
    format!(
        "\n[mcp.servers.agent-body]\ncommand = \"{}\"\nargs = [\"serve-mcp\"]\n[mcp.servers.agent-body.env]\nRUST_LOG = \"info\"\n",
        exe.display()
    )
}

pub fn run(target: HostTarget, print_only: bool) -> Result<()> {
    let exe = resolve_mcp_command_path();

    if print_only {
        match target {
            HostTarget::VsCode { .. } => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "servers": { "agent-body": vscode_server_entry(&exe) }
                    }))?
                );
            }
            HostTarget::OpenCode { .. } => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "mcp": { "agent-body": opencode_server_entry(&exe) }
                    }))?
                );
            }
            HostTarget::Codex { .. } => {
                println!("{}", codex_mcp_toml_block(&exe));
            }
            _ => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "mcpServers": { "agent-body": mcp_server_entry(&exe) }
                    }))?
                );
            }
        }
        return Ok(());
    }

    let paths = install_host(target, &exe, false)?;
    println!("⚡ agent-body MCP configured for {}", target.label());
    println!("Binary executable: {}", exe.display());
    for path in &paths {
        println!("  ✓ {}", path.display());
    }

    // Compose agent guidelines if present
    if let Ok(agents_md) = agent_body_core::compose_agents_md() {
        println!("AGENTS.md composed: {}", agents_md.display());
    }
    if let Ok(links) = agent_body_core::install_host_agents_md_links() {
        for link in links {
            println!("  linked AGENTS.md -> {}", link.display());
        }
    }

    println!("\nRestart your editor or trigger MCP refresh to connect agent-body.");
    Ok(())
}

pub fn install_host(target: HostTarget, exe: &Path, quiet: bool) -> Result<Vec<PathBuf>> {
    match target {
        HostTarget::All => {
            let mut paths = Vec::new();
            let mut ok_hosts = Vec::new();
            let mut errors = Vec::new();

            let hosts: &[(HostTarget, &str)] = &[
                (HostTarget::Cursor { global: true }, "cursor"),
                (HostTarget::ClaudeDesktop, "claude-desktop"),
                (HostTarget::VsCode { user: true }, "vscode"),
                (HostTarget::ClaudeCode { user: true }, "claude-code"),
                (HostTarget::OpenCode { user: true }, "opencode"),
                (HostTarget::Codex { user: true }, "codex"),
                (HostTarget::Gemini { user: true }, "gemini"),
                (HostTarget::Antigravity { user: true }, "antigravity"),
            ];

            for (target, name) in hosts {
                match install_host(*target, exe, true) {
                    Ok(p) => {
                        paths.extend(p);
                        ok_hosts.push(*name);
                    }
                    Err(e) => errors.push(format!("  ✗ {name}: {e}")),
                }
            }

            if !quiet {
                if !ok_hosts.is_empty() {
                    println!("Installed agent-body MCP for: {}", ok_hosts.join(", "));
                }
                for err in &errors {
                    eprintln!("{err}");
                }
            }

            Ok(paths)
        }
        HostTarget::Cursor { global } => install_cursor(exe, global, quiet),
        HostTarget::ClaudeDesktop => install_claude_desktop(exe, quiet),
        HostTarget::VsCode { user } => install_vscode(exe, user, quiet),
        HostTarget::ClaudeCode { user } => install_claude_code(exe, user, quiet),
        HostTarget::OpenCode { user } => install_opencode(exe, user, quiet),
        HostTarget::Codex { user } => install_codex(exe, user, quiet),
        HostTarget::Gemini { user } => install_gemini(exe, user, quiet),
        HostTarget::Antigravity { user } => install_antigravity(exe, user, quiet),
    }
}

fn write_mcp_servers_file(path: &Path, entry: Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let mut root = if path.exists() {
        let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        serde_json::from_str(&raw).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };
    if !root.is_object() {
        root = json!({});
    }
    if let Some(obj) = root.as_object_mut() {
        let servers = obj.entry("mcpServers").or_insert_with(|| json!({}));
        if let Some(servers_obj) = servers.as_object_mut() {
            servers_obj.insert("agent-body".to_string(), entry);
        }
    }
    let pretty = serde_json::to_string_pretty(&root)?;
    fs::write(path, format!("{pretty}\n")).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn write_vscode_servers_file(path: &Path, entry: Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let mut root = if path.exists() {
        let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        serde_json::from_str(&raw).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };
    if !root.is_object() {
        root = json!({});
    }
    if let Some(obj) = root.as_object_mut() {
        let servers = obj.entry("servers").or_insert_with(|| json!({}));
        if let Some(servers_obj) = servers.as_object_mut() {
            servers_obj.insert("agent-body".to_string(), entry);
        }
    }
    let pretty = serde_json::to_string_pretty(&root)?;
    fs::write(path, format!("{pretty}\n")).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn write_opencode_servers_file(path: &Path, entry: Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let mut root = if path.exists() {
        let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        serde_json::from_str(&raw).unwrap_or_else(|_| json!({}))
    } else {
        json!({
            "$schema": "https://opencode.ai/config.json",
            "mcp": {}
        })
    };
    if !root.is_object() {
        root = json!({ "$schema": "https://opencode.ai/config.json", "mcp": {} });
    }
    if let Some(obj) = root.as_object_mut() {
        let mcp = obj.entry("mcp").or_insert_with(|| json!({}));
        if let Some(mcp_obj) = mcp.as_object_mut() {
            mcp_obj.insert("agent-body".to_string(), entry);
        }
    }
    let pretty = serde_json::to_string_pretty(&root)?;
    fs::write(path, format!("{pretty}\n")).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn install_cursor(exe: &Path, global: bool, _quiet: bool) -> Result<Vec<PathBuf>> {
    let path = if global {
        let home = dirs::home_dir().context("home directory")?;
        home.join(".cursor").join("mcp.json")
    } else {
        std::env::current_dir()?.join(".cursor").join("mcp.json")
    };
    write_mcp_servers_file(&path, mcp_server_entry(exe))?;
    Ok(vec![path])
}

fn install_claude_desktop(exe: &Path, _quiet: bool) -> Result<Vec<PathBuf>> {
    let home = dirs::home_dir().context("home directory")?;
    #[cfg(target_os = "macos")]
    let path = home
        .join("Library")
        .join("Application Support")
        .join("Claude")
        .join("claude_desktop_config.json");
    #[cfg(target_os = "windows")]
    let path = dirs::config_dir()
        .unwrap_or_else(|| home.join("AppData").join("Roaming"))
        .join("Claude")
        .join("claude_desktop_config.json");
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let path = home
        .join(".config")
        .join("Claude")
        .join("claude_desktop_config.json");

    write_mcp_servers_file(&path, mcp_server_entry(exe))?;
    Ok(vec![path])
}

fn install_vscode(exe: &Path, user: bool, _quiet: bool) -> Result<Vec<PathBuf>> {
    let path = if user {
        let home = dirs::home_dir().context("home directory")?;
        #[cfg(target_os = "macos")]
        {
            home.join("Library")
                .join("Application Support")
                .join("Code")
                .join("User")
                .join("mcp.json")
        }
        #[cfg(target_os = "windows")]
        {
            dirs::config_dir()
                .unwrap_or_else(|| home.join("AppData").join("Roaming"))
                .join("Code")
                .join("User")
                .join("mcp.json")
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            home.join(".config")
                .join("Code")
                .join("User")
                .join("mcp.json")
        }
    } else {
        let cwd = std::env::current_dir()?;
        cwd.join(".vscode").join("mcp.json")
    };
    write_vscode_servers_file(&path, vscode_server_entry(exe))?;
    Ok(vec![path])
}

fn install_claude_code(exe: &Path, user: bool, _quiet: bool) -> Result<Vec<PathBuf>> {
    let path = if user {
        let home = dirs::home_dir().context("home directory")?;
        home.join(".claude.json")
    } else {
        std::env::current_dir()?.join(".mcp.json")
    };
    write_mcp_servers_file(&path, mcp_server_entry(exe))?;
    Ok(vec![path])
}

fn install_opencode(exe: &Path, user: bool, _quiet: bool) -> Result<Vec<PathBuf>> {
    let path = if user {
        let home = dirs::home_dir().context("home directory")?;
        home.join(".config").join("opencode").join("opencode.json")
    } else {
        std::env::current_dir()?.join("opencode.json")
    };
    write_opencode_servers_file(&path, opencode_server_entry(exe))?;
    Ok(vec![path])
}

fn install_codex(exe: &Path, user: bool, _quiet: bool) -> Result<Vec<PathBuf>> {
    let path = if user {
        let home = dirs::home_dir().context("home directory")?;
        home.join(".codex").join("config.toml")
    } else {
        std::env::current_dir()?.join(".codex").join("config.toml")
    };

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let existing = if path.exists() {
        fs::read_to_string(&path)?
    } else {
        String::new()
    };

    if !existing.contains("[mcp.servers.agent-body]") {
        let block = codex_mcp_toml_block(exe);
        let updated = format!("{}{}", existing, block);
        fs::write(&path, updated)?;
    }
    Ok(vec![path])
}

fn install_gemini(exe: &Path, user: bool, _quiet: bool) -> Result<Vec<PathBuf>> {
    let path = if user {
        let home = dirs::home_dir().context("home directory")?;
        home.join(".gemini").join("settings.json")
    } else {
        std::env::current_dir()?
            .join(".gemini")
            .join("settings.json")
    };
    write_mcp_servers_file(&path, mcp_server_entry(exe))?;
    Ok(vec![path])
}

fn install_antigravity(exe: &Path, user: bool, _quiet: bool) -> Result<Vec<PathBuf>> {
    let entry = mcp_server_entry(exe);
    let paths = if user {
        let home = dirs::home_dir().context("home directory")?;
        vec![
            home.join(".gemini")
                .join("antigravity")
                .join("mcp_config.json"),
            home.join(".gemini").join("config").join("mcp_config.json"),
        ]
    } else {
        vec![std::env::current_dir()?
            .join(".gemini")
            .join("antigravity")
            .join("mcp_config.json")]
    };

    let mut written = Vec::new();
    for p in paths {
        if write_mcp_servers_file(&p, entry.clone()).is_ok() {
            written.push(p);
        }
    }

    if written.is_empty() {
        bail!("Failed to write antigravity mcp_config.json");
    }
    Ok(written)
}
