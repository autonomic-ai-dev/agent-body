use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use agent_body_core::organ_state_dir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub name: String,
    pub binary: String,
    pub pid: Option<u32>,
    pub running: bool,
    pub healthy: bool,
    pub health_url: String,
    pub log_path: String,
}

#[derive(Debug, Clone)]
struct DaemonSpec {
    name: &'static str,
    binary: &'static str,
    args: &'static [&'static str],
    health_url: &'static str,
    start_order: u8,
}

const DAEMONS: &[DaemonSpec] = &[
    DaemonSpec {
        name: "nats",
        binary: "nats-server",
        args: &["-js", "-m", "8222"],
        health_url: "http://127.0.0.1:8222/healthz",
        start_order: 0,
    },
    DaemonSpec {
        name: "spine",
        binary: "agent-spine",
        args: &["serve"],
        health_url: "http://127.0.0.1:3001/",
        start_order: 1,
    },
    DaemonSpec {
        name: "nerves",
        binary: "agent-nerves",
        args: &["serve"],
        health_url: "http://127.0.0.1:3102/health",
        start_order: 2,
    },
    DaemonSpec {
        name: "heart",
        binary: "agent-heart",
        args: &["serve"],
        health_url: "http://127.0.0.1:3101/health",
        start_order: 3,
    },
];

pub fn start_all() -> Result<()> {
    agent_body_core::ensure_dirs()?;
    let supervisor_dir = supervisor_dir();
    fs::create_dir_all(&supervisor_dir)?;

    println!("Starting autonomic daemons (ordered)...");
    let mut specs: Vec<_> = DAEMONS.iter().collect();
    specs.sort_by_key(|s| s.start_order);

    for spec in specs {
        match start_daemon(spec, &supervisor_dir) {
            Ok(pid) => {
                println!("  ✓ {} (pid {pid})", spec.name);
                if let Err(e) = wait_for_health(spec, 15) {
                    eprintln!("  ! {} — {e}", spec.name);
                }
            }
            Err(e) => println!("  ✗ {} — {e}", spec.name),
        }
    }

    write_status_snapshot()?;

    if std::env::var("AUTONOMIC_NATS_URL").is_err() {
        println!("  hint: export AUTONOMIC_NATS_URL=nats://localhost:4222");
    }

    Ok(())
}

pub fn stop_all() -> Result<()> {
    let supervisor_dir = supervisor_dir();
    if !supervisor_dir.exists() {
        println!("No supervisor state at {}", supervisor_dir.display());
        return Ok(());
    }

    println!("Stopping autonomic daemons (reverse order)...");
    let mut specs: Vec<_> = DAEMONS.iter().collect();
    specs.sort_by_key(|s| std::cmp::Reverse(s.start_order));

    for spec in specs {
        stop_daemon(spec)?;
    }

    write_status_snapshot()?;
    Ok(())
}

pub fn restart_all() -> Result<()> {
    stop_all()?;
    thread::sleep(Duration::from_secs(1));
    start_all()
}

pub fn print_status() -> Result<()> {
    let statuses = collect_status()?;
    println!("autonomic supervisor");
    println!("  state: {}", supervisor_dir().display());
    println!();
    println!(
        "{:<8} {:<6} {:<8} {:<8} {}",
        "ORGAN", "PID", "RUNNING", "HEALTHY", "LOG"
    );
    println!("{}", "-".repeat(72));
    for s in &statuses {
        println!(
            "{:<8} {:<6} {:<8} {:<8} {}",
            s.name,
            s.pid.map(|p| p.to_string()).unwrap_or_else(|| "-".into()),
            if s.running { "yes" } else { "no" },
            if s.healthy { "yes" } else { "no" },
            s.log_path
        );
    }
    Ok(())
}

pub fn supervise(interval_secs: u64) -> Result<()> {
    let interval = Duration::from_secs(interval_secs.max(2));
    println!(
        "Supervising autonomic daemons every {}s (Ctrl+C to exit).",
        interval.as_secs()
    );
    loop {
        for spec in DAEMONS {
            let pid_path = supervisor_dir().join(format!("{}.pid", spec.name));
            let running = read_running_pid(&pid_path)?.is_some();
            let healthy = running && check_health(spec.health_url);

            if !running || !healthy {
                if running {
                    eprintln!("  ! {} unhealthy — restarting", spec.name);
                    stop_daemon(spec).ok();
                } else {
                    eprintln!("  ! {} not running — starting", spec.name);
                }
                match start_daemon(spec, &supervisor_dir()) {
                    Ok(pid) => {
                        println!("  ✓ {} (pid {pid})", spec.name);
                        let _ = wait_for_health(spec, 15);
                    }
                    Err(e) => eprintln!("  ✗ {} — {e}", spec.name),
                }
            }
        }
        write_status_snapshot().ok();
        thread::sleep(interval);
    }
}

fn collect_status() -> Result<Vec<DaemonStatus>> {
    let supervisor_dir = supervisor_dir();
    let log_dir = organ_state_dir("supervisor").join("logs");
    let mut out = Vec::new();

    for spec in DAEMONS {
        let pid_path = supervisor_dir.join(format!("{}.pid", spec.name));
        let pid = read_running_pid(&pid_path)?;
        let running = pid.is_some();
        let healthy = running && check_health(spec.health_url);
        out.push(DaemonStatus {
            name: spec.name.to_string(),
            binary: spec.binary.to_string(),
            pid,
            running,
            healthy,
            health_url: spec.health_url.to_string(),
            log_path: log_dir
                .join(format!("{}.log", spec.name))
                .display()
                .to_string(),
        });
    }
    Ok(out)
}

fn write_status_snapshot() -> Result<()> {
    let statuses = collect_status()?;
    let path = supervisor_dir().join("status.json");
    fs::write(&path, serde_json::to_string_pretty(&statuses)?)?;
    Ok(())
}

fn supervisor_dir() -> PathBuf {
    organ_state_dir("supervisor")
}

fn start_daemon(spec: &DaemonSpec, supervisor_dir: &Path) -> Result<u32> {
    let pid_path = supervisor_dir.join(format!("{}.pid", spec.name));
    if let Some(existing) = read_running_pid(&pid_path)? {
        if check_health(spec.health_url) {
            anyhow::bail!("already running and healthy (pid {existing})");
        }
        stop_daemon(spec).ok();
    }

    if !binary_on_path(spec.binary) {
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
        .env("RUST_LOG", "debug")
        .stdin(Stdio::null())
        .stdout(Stdio::from(log_file.try_clone()?))
        .stderr(Stdio::from(log_file))
        .spawn()
        .with_context(|| format!("spawn {}", spec.binary))?;

    let pid = child.id();
    fs::write(&pid_path, pid.to_string())?;
    Ok(pid)
}

fn stop_daemon(spec: &DaemonSpec) -> Result<()> {
    let pid_path = supervisor_dir().join(format!("{}.pid", spec.name));
    let Some(pid) = read_running_pid(&pid_path)? else {
        return Ok(());
    };

    let _ = Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .status();

    for _ in 0..10 {
        if read_running_pid(&pid_path)?.is_none() {
            println!("  stopped {} (pid {pid})", spec.name);
            return Ok(());
        }
        thread::sleep(Duration::from_millis(300));
    }

    let _ = Command::new("kill")
        .arg("-KILL")
        .arg(pid.to_string())
        .status();
    fs::remove_file(&pid_path).ok();
    println!("  force-stopped {} (pid {pid})", spec.name);
    Ok(())
}

fn wait_for_health(spec: &DaemonSpec, timeout_secs: u64) -> Result<()> {
    let deadline = Duration::from_secs(timeout_secs);
    let start = std::time::Instant::now();
    while start.elapsed() < deadline {
        if check_health(spec.health_url) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(400));
    }
    anyhow::bail!(
        "{} did not become healthy within {}s ({})",
        spec.name,
        timeout_secs,
        spec.health_url
    )
}

fn check_health(url: &str) -> bool {
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    client
        .get(url)
        .send()
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

fn binary_on_path(binary: &str) -> bool {
    Command::new("which")
        .arg(binary)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn read_running_pid(path: &PathBuf) -> Result<Option<u32>> {
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
        Ok(Some(pid as u32))
    } else {
        fs::remove_file(path).ok();
        Ok(None)
    }
}
