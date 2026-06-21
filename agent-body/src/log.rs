use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};

/// Resolve the supervisor log directory.
fn log_dir() -> PathBuf {
    agent_body_core::organ_state_dir("supervisor").join("logs")
}

/// List available log files (stem names).
pub fn list_logs() -> Result<Vec<String>> {
    let dir = log_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(&dir).with_context(|| format!("read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "log") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                names.push(stem.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}

/// Print the full log for a given daemon name.
pub fn print_log(name: &str) -> Result<()> {
    let path = log_dir().join(format!("{name}.log"));
    if !path.exists() {
        anyhow::bail!("no log found for '{name}'. Available: {}", list_logs()?.join(", "));
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;
    print!("{content}");
    Ok(())
}

/// Follow (tail -f) the log for a given daemon name.
pub fn follow_log(name: &str) -> Result<()> {
    let path = log_dir().join(format!("{name}.log"));
    if !path.exists() {
        anyhow::bail!("no log found for '{name}'. Available: {}", list_logs()?.join(", "));
    }

    let file = fs::File::open(&path).with_context(|| format!("open {}", path.display()))?;
    let mut reader = BufReader::new(file);

    // Print existing content first
    let mut buf = String::new();
    loop {
        buf.clear();
        let bytes = reader.read_line(&mut buf)?;
        if bytes == 0 {
            break;
        }
        print!("{buf}");
    }

    // Follow new lines
    loop {
        buf.clear();
        let bytes = reader.read_line(&mut buf)?;
        if bytes == 0 {
            thread::sleep(Duration::from_millis(200));
            continue;
        }
        print!("{buf}");
    }
}
