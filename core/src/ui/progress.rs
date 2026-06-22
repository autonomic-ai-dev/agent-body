use std::fmt::Write as _;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::global_workspace;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressMode {
    Auto,
    Plain,
    Quiet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressStatus {
    Running,
    Done,
    Cached,
    Warn,
    Failed,
}

pub fn resolve_progress_mode(flag: Option<ProgressMode>) -> ProgressMode {
    if let Some(mode) = flag {
        return mode;
    }
    if let Ok(val) = std::env::var("AUTONOMIC_PROGRESS") {
        return parse_mode_str(&val).unwrap_or(ProgressMode::Auto);
    }
    ProgressMode::Auto
}

fn parse_mode_str(raw: &str) -> Option<ProgressMode> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "auto" => Some(ProgressMode::Auto),
        "plain" => Some(ProgressMode::Plain),
        "quiet" => Some(ProgressMode::Quiet),
        _ => None,
    }
}

fn effective_mode(requested: ProgressMode) -> ProgressMode {
    match requested {
        ProgressMode::Auto if std::io::stdout().is_terminal() => ProgressMode::Auto,
        ProgressMode::Auto => ProgressMode::Plain,
        other => other,
    }
}

pub struct ProgressRun {
    title: String,
    mode: ProgressMode,
    total_hint: Option<usize>,
    steps: Vec<StepRecord>,
    log_path: PathBuf,
    log_buf: String,
    started: Instant,
}

struct StepRecord {
    index: usize,
    name: String,
    status: ProgressStatus,
    duration: Duration,
    detail: Option<String>,
}

impl ProgressRun {
    pub fn new(title: impl Into<String>) -> Self {
        Self::with_mode(title, resolve_progress_mode(None))
    }

    pub fn with_mode(title: impl Into<String>, mode: ProgressMode) -> Self {
        let title = title.into();
        let mode = effective_mode(mode);
        let ts = chrono_like_timestamp();
        let log_path = default_log_path(&title, &ts);
        let mut log_buf = String::new();
        let _ = writeln!(log_buf, "[{ts}] {title}");
        if mode != ProgressMode::Quiet {
            println!("[+] {title}");
        }
        Self {
            title,
            mode,
            total_hint: None,
            steps: Vec::new(),
            log_path,
            log_buf,
            started: Instant::now(),
        }
    }

    pub fn with_total_hint(mut self, total: usize) -> Self {
        self.total_hint = Some(total);
        self
    }

    pub fn step<'a>(&'a mut self, name: impl Into<String>) -> ProgressStep<'a> {
        let index = self.steps.len() + 1;
        let name = name.into();
        let _ = writeln!(self.log_buf, "  START [{index}] {name}");
        ProgressStep {
            run: self,
            index,
            name,
            started: Instant::now(),
        }
    }

    pub fn finish(mut self) -> Result<ProgressSummary> {
        let total = self.started.elapsed();
        let failed = self
            .steps
            .iter()
            .filter(|s| s.status == ProgressStatus::Failed)
            .count();
        let warned = self
            .steps
            .iter()
            .filter(|s| s.status == ProgressStatus::Warn)
            .count();

        let _ = writeln!(
            self.log_buf,
            "SUMMARY failed={failed} warned={warned} elapsed={:.1}s",
            total.as_secs_f64()
        );
        write_log_file(&self.log_path, &self.log_buf)?;

        if self.mode != ProgressMode::Quiet {
            println!();
            println!(
                "Summary: {} — {} step(s), {failed} failed, {warned} warning(s) · {:.1}s",
                self.title,
                self.steps.len(),
                total.as_secs_f64()
            );
            println!("Log: {}", self.log_path.display());
        } else if failed > 0 || warned > 0 {
            eprintln!("Log: {}", self.log_path.display());
        }

        Ok(ProgressSummary { failed, warned })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ProgressSummary {
    pub failed: usize,
    pub warned: usize,
}

pub struct ProgressStep<'a> {
    run: &'a mut ProgressRun,
    index: usize,
    name: String,
    started: Instant,
}

impl<'a> ProgressStep<'a> {
    pub fn done(self) {
        self.complete(ProgressStatus::Done, None);
    }

    pub fn cached(self) {
        self.complete(ProgressStatus::Cached, None);
    }

    pub fn warn(self, detail: impl Into<String>) {
        self.complete(ProgressStatus::Warn, Some(detail.into()));
    }

    pub fn fail(self, detail: impl Into<String>) {
        self.complete(ProgressStatus::Failed, Some(detail.into()));
    }

    pub fn run<F>(self, f: F) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        match f() {
            Ok(()) => {
                self.done();
                Ok(())
            }
            Err(err) => {
                self.fail(format!("{err:#}"));
                Err(err)
            }
        }
    }

    fn complete(self, status: ProgressStatus, detail: Option<String>) {
        let duration = self.started.elapsed();
        let record = StepRecord {
            index: self.index,
            name: self.name.clone(),
            status,
            duration,
            detail: detail.clone(),
        };
        let _ = writeln!(
            self.run.log_buf,
            "  END   [{}] {} {:?} {:.2}s {:?}",
            record.index,
            record.name,
            record.status,
            record.duration.as_secs_f64(),
            record.detail
        );
        emit_step_line(self.run.mode, self.run.total_hint, &record);
        if let Some(detail) = detail {
            if matches!(status, ProgressStatus::Failed | ProgressStatus::Warn) {
                let _ = writeln!(self.run.log_buf, "        {detail}");
                if self.run.mode != ProgressMode::Quiet {
                    for line in detail.lines() {
                        println!("        {line}");
                    }
                }
            }
        }
        self.run.steps.push(record);
    }
}

fn emit_step_line(mode: ProgressMode, total_hint: Option<usize>, step: &StepRecord) {
    if mode == ProgressMode::Quiet {
        return;
    }
    let label = match step.status {
        ProgressStatus::Running => "…",
        ProgressStatus::Done => "DONE",
        ProgressStatus::Cached => "CACHED",
        ProgressStatus::Warn => "WARN",
        ProgressStatus::Failed => "ERROR",
    };
    let total = total_hint
        .map(|t| t.to_string())
        .unwrap_or_else(|| "?".into());
    let line = format!(
        " => [{}/{}] {:<24} {} {:.1}s",
        step.index,
        total,
        step.name,
        label,
        step.duration.as_secs_f64()
    );
    println!("{line}");
}

fn default_log_path(title: &str, ts: &str) -> PathBuf {
    let slug = title
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .to_ascii_lowercase();
    global_workspace::autonomic_root()
        .join("logs")
        .join("runs")
        .join(format!("{ts}-{slug}.log"))
}

fn write_log_file(path: &PathBuf, body: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, body)?;
    Ok(())
}

fn chrono_like_timestamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}
