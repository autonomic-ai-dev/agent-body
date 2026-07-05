use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use agent_body_core::memory_dir;
use agent_body_core::organ_state_dir;
use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};

use crate::degradation::DegradationState;
use agent_body_core::ui::ProgressRun;

use crate::nats_config;

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
        args: &[],
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
    let mut specs: Vec<_> = DAEMONS.iter().collect();
    specs.sort_by_key(|s| s.start_order);
    let mut progress =
        ProgressRun::new("Starting Autonomic daemons").with_total_hint(specs.len() + 2);

    let workspace = progress.step("workspace");
    agent_body_core::ensure_dirs()?;
    let supervisor_dir = supervisor_dir();
    fs::create_dir_all(&supervisor_dir)?;
    workspace.done();

    let nats_step = progress.step("nats security");
    let nats_bootstrap = match nats_config::ensure_nats_security() {
        Ok(b) => {
            nats_step.done();
            Some(b)
        }
        Err(e) => {
            nats_step.warn(format!("{e:#}"));
            None
        }
    };

    for spec in specs.iter().filter(|s| s.start_order == 0) {
        let step = progress.step(spec.name);
        match start_daemon(spec, &supervisor_dir, nats_bootstrap.as_ref()) {
            Ok(pid) => match wait_for_health(spec, 15) {
                Ok(()) => step.done(),
                Err(e) => step.warn(format!("pid {pid} — {e:#}")),
            },
            Err(e) if e.to_string().contains("already running") => step.cached(),
            Err(e) => step.fail(format!("{e:#}")),
        }
    }

    let brain_step = progress.step("brain index");
    match wait_for_brain_index(30) {
        Ok(()) => brain_step.done(),
        Err(e) => brain_step.warn(format!("{e:#} — continuing boot")),
    }

    for spec in specs.iter().filter(|s| s.start_order > 0) {
        let step = progress.step(spec.name);
        match start_daemon(spec, &supervisor_dir, nats_bootstrap.as_ref()) {
            Ok(pid) => match wait_for_health(spec, 15) {
                Ok(()) => step.done(),
                Err(e) => step.warn(format!("pid {pid} — {e:#}")),
            },
            Err(e) if e.to_string().contains("already running") => step.cached(),
            Err(e) => step.fail(format!("{e:#}")),
        }
    }

    let snapshot = progress.step("status snapshot");
    write_status_snapshot()?;
    snapshot.done();

    let summary = progress.finish()?;
    if summary.failed > 0 {
        anyhow::bail!("one or more daemons failed to start");
    }
    if std::env::var("AUTONOMIC_NATS_URL").is_err() {
        eprintln!("hint: export AUTONOMIC_NATS_URL=nats://localhost:4222");
    }
    Ok(())
}

pub fn stop_all() -> Result<()> {
    let supervisor_dir = supervisor_dir();
    if !supervisor_dir.exists() {
        println!("No supervisor state at {}", supervisor_dir.display());
        return Ok(());
    }

    let mut specs: Vec<_> = DAEMONS.iter().collect();
    specs.sort_by_key(|s| std::cmp::Reverse(s.start_order));
    let mut progress =
        ProgressRun::new("Stopping Autonomic daemons").with_total_hint(specs.len() + 1);

    for spec in specs {
        let step = progress.step(spec.name);
        let pid_path = supervisor_dir.join(format!("{}.pid", spec.name));
        if read_running_pid(&pid_path)?.is_none() {
            step.cached();
        } else {
            stop_daemon(spec, true)?;
            step.done();
        }
    }

    let snapshot = progress.step("status snapshot");
    write_status_snapshot()?;
    snapshot.done();
    progress.finish()?;
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
    let hdr_log = "LOG";
    println!(
        "{:<8} {:<6} {:<8} {:<8} {}",
        "ORGAN", "PID", "RUNNING", "HEALTHY", hdr_log
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
    let nats_bootstrap = nats_config::ensure_nats_security().ok();
    loop {
        for spec in DAEMONS {
            let pid_path = supervisor_dir().join(format!("{}.pid", spec.name));
            let running = read_running_pid(&pid_path)?.is_some();
            let healthy = running && check_health(spec.health_url);

            if !running || !healthy {
                if running {
                    record_restart(spec.name);
                    eprintln!("  ! {} unhealthy — restarting", spec.name);
                    stop_daemon(spec, false).ok();
                } else {
                    record_restart(spec.name);
                    eprintln!("  ! {} not running — starting", spec.name);
                }
                match start_daemon(spec, &supervisor_dir(), nats_bootstrap.as_ref()) {
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

pub fn collect_status() -> Result<Vec<DaemonStatus>> {
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

pub fn supervisor_dir() -> PathBuf {
    organ_state_dir("supervisor")
}

fn start_daemon(
    spec: &DaemonSpec,
    supervisor_dir: &Path,
    nats_bootstrap: Option<&nats_config::NatsBootstrap>,
) -> Result<u32> {
    let pid_path = supervisor_dir.join(format!("{}.pid", spec.name));
    if let Some(existing) = read_running_pid(&pid_path)? {
        if check_health(spec.health_url) {
            anyhow::bail!("already running and healthy (pid {existing})");
        }
        stop_daemon(spec, false).ok();
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

    let mut cmd = Command::new(spec.binary);
    if spec.name == "nats" {
        if let Some(bootstrap) = nats_bootstrap {
            cmd.arg("-c").arg(&bootstrap.config_path);
        } else {
            cmd.args(["-js", "-m", "8222", "-sd"])
                .arg(agent_body_core::broker_dir());
        }
    } else {
        cmd.args(spec.args);
    }

    cmd.env("RUST_LOG", "debug");
    if let Some(bootstrap) = nats_bootstrap {
        if spec.name != "nats" {
            for (key, value) in agent_body_core::organ_env_vars(&bootstrap.bundle, spec.name) {
                cmd.env(key, value);
            }
        }
        if std::env::var("AUTONOMIC_NATS_URL").is_err() {
            let scheme = if bootstrap.bundle.tls_enabled {
                "tls"
            } else {
                "nats"
            };
            cmd.env(
                "AUTONOMIC_NATS_URL",
                format!("{}://127.0.0.1:{}", scheme, bootstrap.bundle.port),
            );
        }
    }

    let child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::from(log_file.try_clone()?))
        .stderr(Stdio::from(log_file))
        .spawn()
        .with_context(|| format!("spawn {}", spec.binary))?;

    let pid = child.id();
    fs::write(&pid_path, pid.to_string())?;
    Ok(pid)
}

fn stop_daemon(spec: &DaemonSpec, quiet: bool) -> Result<()> {
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
            if !quiet {
                println!("  stopped {} (pid {pid})", spec.name);
            }
            return Ok(());
        }
        thread::sleep(Duration::from_millis(300));
    }

    let _ = Command::new("kill")
        .arg("-KILL")
        .arg(pid.to_string())
        .status();
    fs::remove_file(&pid_path).ok();
    if !quiet {
        println!("  force-stopped {} (pid {pid})", spec.name);
    }
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

pub fn check_health_with_latency(url: &str) -> (bool, u64) {
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(_) => return (false, 0),
    };
    let start = std::time::Instant::now();
    let ok = client
        .get(url)
        .send()
        .map(|r| r.status().is_success())
        .unwrap_or(false);
    (ok, start.elapsed().as_millis() as u64)
}

fn check_health(url: &str) -> bool {
    check_health_with_latency(url).0
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

pub const EVICTION_SCORE_THRESHOLD: u8 = 30;
const RSS_WARN_KB: u64 = 512 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganHealth {
    pub name: String,
    pub running: bool,
    pub healthy: bool,
    pub health_score: u8,
    pub latency_ms: u64,
    pub restart_count: u32,
    pub rss_kb: u64,
    pub should_evict: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgansHealthReport {
    pub timestamp: String,
    pub mesh_score: u8,
    pub organs: Vec<OrganHealth>,
    pub degradation: DegradationState,
}

#[derive(Debug, Clone, Copy)]
pub struct HealthScoreInput {
    pub running: bool,
    pub healthy: bool,
    pub latency_ms: u64,
    pub restart_count: u32,
    pub rss_kb: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct HealthScoreResult {
    pub score: u8,
    pub should_evict: bool,
}

pub fn compute_health_score(input: HealthScoreInput) -> HealthScoreResult {
    if !input.running {
        return HealthScoreResult {
            score: 0,
            should_evict: true,
        };
    }
    let mut score: i32 = 100;
    if !input.healthy {
        score -= 50;
    }
    if input.latency_ms > 2000 {
        score -= 30;
    } else if input.latency_ms > 500 {
        score -= 15;
    } else if input.latency_ms > 200 {
        score -= 5;
    }
    score -= (input.restart_count as i32).saturating_mul(5).min(40);
    if input.rss_kb > RSS_WARN_KB {
        score -= 20;
    } else if input.rss_kb > RSS_WARN_KB / 2 {
        score -= 10;
    }
    let score = score.clamp(0, 100) as u8;
    HealthScoreResult {
        score,
        should_evict: score < EVICTION_SCORE_THRESHOLD,
    }
}

pub fn read_restart_counts() -> HashMap<String, u32> {
    let path = supervisor_dir().join("restart_counts.json");
    if !path.exists() {
        return HashMap::new();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn record_restart(name: &str) {
    let mut counts = read_restart_counts();
    *counts.entry(name.to_string()).or_insert(0) += 1;
    let path = supervisor_dir().join("restart_counts.json");
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(
        &path,
        serde_json::to_string_pretty(&counts).unwrap_or_default(),
    );
}

pub fn rss_kb_for_pid(pid: u32) -> u64 {
    let mut sys = System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::everything()),
    );
    sys.refresh_processes(
        sysinfo::ProcessesToUpdate::Some(&[Pid::from_u32(pid)]),
        true,
    );
    sys.process(Pid::from_u32(pid))
        .map(|proc| proc.memory() / 1024)
        .unwrap_or(0)
}

pub fn organ_health_from_status(status: &DaemonStatus, restarts: u32) -> OrganHealth {
    let (healthy, latency_ms) = if status.running {
        check_health_with_latency(&status.health_url)
    } else {
        (false, 0)
    };
    let rss_kb = status.pid.map(rss_kb_for_pid).unwrap_or(0);
    let scored = compute_health_score(HealthScoreInput {
        running: status.running,
        healthy,
        latency_ms,
        restart_count: restarts,
        rss_kb,
    });
    OrganHealth {
        name: status.name.clone(),
        running: status.running,
        healthy,
        health_score: scored.score,
        latency_ms,
        restart_count: restarts,
        rss_kb,
        should_evict: scored.should_evict,
    }
}

pub fn build_organs_health_report() -> Result<OrgansHealthReport> {
    let statuses = collect_status()?;
    let restarts = read_restart_counts();
    let organs: Vec<OrganHealth> = statuses
        .iter()
        .map(|s| organ_health_from_status(s, restarts.get(&s.name).copied().unwrap_or(0)))
        .collect();
    let mesh_score = organs.iter().map(|o| o.health_score).min().unwrap_or(0);
    Ok(OrgansHealthReport {
        timestamp: chrono::Utc::now().to_rfc3339(),
        mesh_score,
        degradation: DegradationState::from_mesh_score(mesh_score),
        organs,
    })
}

pub fn wait_for_brain_index(timeout_secs: u64) -> Result<()> {
    let db = memory_dir().join("brain.db");
    let deadline = Duration::from_secs(timeout_secs);
    let start = std::time::Instant::now();
    while start.elapsed() < deadline {
        if brain_index_ready(&db) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(400));
    }
    anyhow::bail!(
        "brain index not ready at {} within {}s",
        db.display(),
        timeout_secs
    )
}

pub fn brain_index_ready(db_path: &Path) -> bool {
    db_path.is_file()
        && fs::metadata(db_path)
            .map(|m| m.len() > 4096)
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_score_eviction() {
        let bad = compute_health_score(HealthScoreInput {
            running: true,
            healthy: false,
            latency_ms: 2500,
            restart_count: 8,
            rss_kb: RSS_WARN_KB + 1,
        });
        assert!(bad.should_evict);
        assert!(bad.score < EVICTION_SCORE_THRESHOLD);

        let good = compute_health_score(HealthScoreInput {
            running: true,
            healthy: true,
            latency_ms: 40,
            restart_count: 0,
            rss_kb: 50_000,
        });
        assert!(!good.should_evict);
        assert!(good.score >= 80);
    }
}
