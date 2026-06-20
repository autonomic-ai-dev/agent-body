use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use agent_body_core::organ_state_dir;

#[derive(Debug, Clone)]
struct DaemonSpec {
    name: &'static str,
    binary: &'static str,
    args: &'static [&'static str],
}

const DAEMONS: &[DaemonSpec] = &[
    DaemonSpec {
        name: "nerves",
        binary: "agent-nerves",
        args: &["serve"],
    },
    DaemonSpec {
        name: "heart",
        binary: "agent-heart",
        args: &["serve"],
    },
];

pub fn start_all() -> Result<()> {
    agent_body_core::ensure_dirs()?;
    let supervisor_dir = organ_state_dir("supervisor");
    fs::create_dir_all(&supervisor_dir)?;

    println!("Starting autonomic daemons...");
    for spec in DAEMONS {
        match start_daemon(spec, &supervisor_dir) {
            Ok(pid) => println!("  ✓ {} (pid {pid})", spec.name),
            Err(e) => println!("  ✗ {} — {e}", spec.name),
        }
    }

    if std::env::var("AUTONOMIC_NATS_URL").is_err() {
        println!("  hint: export AUTONOMIC_NATS_URL=nats://localhost:4222");
    }

    Ok(())
}

pub fn stop_all() -> Result<()> {
    let supervisor_dir = organ_state_dir("supervisor");
    if !supervisor_dir.exists() {
        println!("No supervisor state at {}", supervisor_dir.display());
        return Ok(());
    }

    for entry in fs::read_dir(&supervisor_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("pid") {
            continue;
        }
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
        let pid_text = fs::read_to_string(&path).unwrap_or_default();
        let pid: i32 = pid_text.trim().parse().unwrap_or(0);
        if pid > 0 {
            let _ = Command::new("kill").arg(pid.to_string()).status();
            println!("  stopped {name} (pid {pid})");
        }
        fs::remove_file(path).ok();
    }
    Ok(())
}

fn start_daemon(spec: &DaemonSpec, supervisor_dir: &Path) -> Result<u32> {
    let pid_path = supervisor_dir.join(format!("{}.pid", spec.name));
    if let Some(existing) = read_running_pid(&pid_path)? {
        anyhow::bail!("already running (pid {existing})");
    }

    if Command::new("which")
        .arg(spec.binary)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| !s.success())
        .unwrap_or(true)
    {
        anyhow::bail!("{binary} not found on PATH", binary = spec.binary);
    }

    let log_dir = organ_state_dir("supervisor").join("logs");
    fs::create_dir_all(&log_dir)?;
    let log_path = log_dir.join(format!("{}.log", spec.name));
    let log_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .with_context(|| format!("open log {}", log_path.display()))?;

    let child = Command::new(spec.binary)
        .args(spec.args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log_file.try_clone()?))
        .stderr(Stdio::from(log_file))
        .spawn()
        .with_context(|| format!("spawn {}", spec.binary))?;

    let pid = child.id();
    fs::write(&pid_path, pid.to_string())?;
    Ok(pid)
}

fn read_running_pid(path: &PathBuf) -> Result<Option<i32>> {
    if !path.exists() {
        return Ok(None);
    }
    let pid: i32 = fs::read_to_string(path)?.trim().parse().unwrap_or(0);
    if pid <= 0 {
        fs::remove_file(path).ok();
        return Ok(None);
    }
    let alive = Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if alive {
        Ok(Some(pid))
    } else {
        fs::remove_file(path).ok();
        Ok(None)
    }
}
