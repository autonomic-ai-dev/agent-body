use anyhow::Result;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

pub fn run_dashboard(refresh_secs: u64) -> Result<()> {
    let refresh = Duration::from_secs(refresh_secs.max(1));
    loop {
        print!("\x1b[2J\x1b[H");
        print_header();
        print_process_table();
        print!(
            "Press Ctrl+C to exit. Refreshing every {}s.",
            refresh.as_secs()
        );
        io::stdout().flush()?;
        thread::sleep(refresh);
    }
}

fn print_header() {
    println!("Autonomic TUI — organ process monitor");
    println!("workspace: {}", agent_body_core::autonomic_root().display());
    println!();
}

fn print_process_table() {
    use sysinfo::{ProcessesToUpdate, System};

    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);

    println!(
        "{:<8} {:<22} {:>8} {:>10}",
        "PID", "PROCESS", "CPU%", "MEM (MB)"
    );
    println!("{}", "-".repeat(54));

    let mut rows: Vec<(u32, String, f32, u64)> = system
        .processes()
        .iter()
        .filter_map(|(pid, process)| {
            let name = process.name().to_string_lossy().to_string();
            if !name.starts_with("agent-") && name != "autonomic" && name != "nats-server" {
                return None;
            }
            let cpu = process.cpu_usage();
            let mem_mb = process.memory() / 1024 / 1024;
            Some((pid.as_u32(), name, cpu, mem_mb))
        })
        .collect();

    rows.sort_by(|a, b| a.1.cmp(&b.1));

    if rows.is_empty() {
        println!("(no autonomic processes running — try `autonomic start`)");
    } else {
        for (pid, name, cpu, mem_mb) in rows {
            println!("{pid:<8} {name:<22} {cpu:>7.1} {mem_mb:>9}");
        }
    }
    println!();
}
