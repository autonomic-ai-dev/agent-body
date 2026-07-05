use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ecosystem_config;
use crate::global_workspace;

pub const BASE_AGENT_MODE: &str = r#"# Autonomic agent mode

Enforces route_task before every turn — memory, skills, and cross-session context are injected automatically.

## Rules
- Call `agent-brain_route_task` with `user_message`, `current_working_directory`, `open_files` at the start of every turn
- Use agent-brain `grep_search`, `file_summary`, `read_file_head`, `read_file_tail` instead of native Read/Grep
- Call `store_memory` at task end for durable outcomes

## Autonomic utilities (delegate, don't improvise)
- Workflows: `agent-spine run --meta "..."` or `agent-spine init --with @workflow`
- Discovery: `agent-brain registry list` (skills + utilities)
- Upstream MCP: `route_to_mcp` when `suggested_tools` appears in route_task
"#;

pub const FRAGMENT_ORDER: &[&str] = &[
    "brain", "spine", "heart", "nerves", "muscle", "immune", "eyes", "mouth",
];

pub fn default_fragment(organ: &str) -> Option<&'static str> {
    match organ {
        "brain" => Some(
            r#"## agent-brain
- MCP routing, hooks, durable memory under `~/.autonomic/memory`
- `agent-brain doctor`, `agent-brain install --global`, `agent-brain sync git`
"#,
        ),
        "spine" => Some(
            r#"## agent-spine
- YAML workflows, DAG execution, approval gates
- `agent-spine run`, `agent-spine validate`, `agent-spine init`
"#,
        ),
        "heart" => Some(
            r#"## agent-heart
- Scheduled GC, token budget gate for spine
- `agent-heart serve`, `agent-heart gc`, `agent-heart budget check`
"#,
        ),
        "nerves" => Some(
            r#"## agent-nerves
- NATS/JetStream event bridge
- `agent-nerves serve`, `agent-nerves ping`, `agent-nerves stream tail`
"#,
        ),
        "muscle" => Some(
            r#"## agent-muscle
- Sandboxed command execution and training backends
- `agent-muscle run`, `agent-muscle serve`, `agent-muscle train`
"#,
        ),
        "immune" => Some(
            r#"## agent-immune
- Vulnerability scan, sandbox verify, memory growth checks
- `agent-immune scan`, `agent-immune sandbox run`, `agent-immune serve`
"#,
        ),
        "eyes" => Some(
            r#"## agent-eyes
- Capture, DOM index, optional VLM describe
- `agent-eyes capture`, `agent-eyes verify`, `agent-eyes serve`
"#,
        ),
        "mouth" => Some(
            r#"## agent-mouth
- AST command validation, Slack approvals, notifications
- `agent-mouth validate`, `agent-mouth send`, `agent-mouth serve`
"#,
        ),
        _ => None,
    }
}

pub fn scaffold_agents_dir() -> Result<()> {
    let dir = global_workspace::agents_dir();
    fs::create_dir_all(&dir).context("create agents dir")?;
    let base = dir.join("base.md");
    if !base.is_file() {
        fs::write(&base, BASE_AGENT_MODE).context("write agents/base.md")?;
    }
    for organ in FRAGMENT_ORDER {
        let path = dir.join(format!("{organ}.md"));
        if !path.is_file() {
            if let Some(content) = default_fragment(organ) {
                fs::write(&path, content.trim()).context("write organ fragment")?;
            }
        }
    }
    let readme = dir.join("README");
    if !readme.is_file() {
        fs::write(
            &readme,
            "Fragment files compose ~/.autonomic/AGENTS.md via `autonomic agents compose`.\n",
        )?;
    }
    Ok(())
}

pub fn write_fragment(organ: &str, content: &str) -> Result<PathBuf> {
    let dir = global_workspace::agents_dir();
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{organ}.md"));
    fs::write(&path, content.trim()).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

pub fn compose_agents_md() -> Result<PathBuf> {
    compose_agents_md_inner(false)
}

fn compose_agents_md_inner(force: bool) -> Result<PathBuf> {
    let mut cfg = ecosystem_config::agents_config()?;
    if cfg.mode.is_empty() {
        cfg.mode = "agent-brain".into();
    }
    if !force && !cfg.compose_agents_md {
        anyhow::bail!("agents.compose_agents_md is false in config");
    }

    scaffold_agents_dir()?;

    let mut parts = Vec::new();
    let base_path = global_workspace::agents_dir().join("base.md");
    if base_path.is_file() {
        parts.push(fs::read_to_string(&base_path)?);
    } else {
        parts.push(BASE_AGENT_MODE.to_string());
    }

    for organ in FRAGMENT_ORDER {
        let frag = global_workspace::agents_dir().join(format!("{organ}.md"));
        if frag.is_file() {
            parts.push(fs::read_to_string(&frag)?);
        }
    }

    let body = parts
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");

    let out = global_workspace::agents_md_path();
    fs::write(&out, format!("{body}\n")).with_context(|| format!("write {}", out.display()))?;
    Ok(out)
}

#[cfg(unix)]
pub fn symlink_agents_md(target: &Path, link: &Path) -> Result<()> {
    if let Some(parent) = link.parent() {
        fs::create_dir_all(parent)?;
    }
    if link.is_symlink() || link.exists() {
        fs::remove_file(link).ok();
    }
    std::os::unix::fs::symlink(target, link)
        .with_context(|| format!("symlink {} -> {}", link.display(), target.display()))?;
    Ok(())
}

#[cfg(not(unix))]
pub fn symlink_agents_md(target: &Path, link: &Path) -> Result<()> {
    if let Some(parent) = link.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(target, link)
        .with_context(|| format!("copy {} to {}", target.display(), link.display()))?;
    Ok(())
}

pub fn install_host_agents_md_links() -> Result<Vec<PathBuf>> {
    let canonical = compose_agents_md_inner(true)?;
    let home = dirs::home_dir().context("HOME not set")?;
    let mut installed = Vec::new();

    let links: Vec<(&str, PathBuf)> = vec![
        ("opencode", home.join(".config/opencode/AGENTS.md")),
        ("claude", home.join(".claude/AGENTS.md")),
        ("codex", home.join(".codex/AGENTS.md")),
        ("gemini", home.join(".gemini/AGENTS.md")),
        (
            "antigravity",
            home.join(".gemini").join("antigravity").join("AGENTS.md"),
        ),
        ("vscode", home.join(".vscode/AGENTS.md")),
    ];

    for (label, link) in links {
        if symlink_agents_md(&canonical, &link).is_ok() {
            installed.push(link);
        } else {
            eprintln!("could not install AGENTS.md link for {label}");
        }
    }
    Ok(installed)
}
