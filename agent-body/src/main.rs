use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "autonomic", about = "Autonomic AI ecosystem manager", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Scaffold workspace and optional project directory
    Init {
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Start background daemons (nerves, heart)
    Start,
    /// Stop background daemons started by autonomic
    Stop,
    /// Restart all supervised daemons
    Restart,
    /// Watch daemons and restart unhealthy processes
    Supervise {
        #[arg(long, default_value_t = 5)]
        interval: u64,
    },
    /// Update all organs to latest GitHub releases
    Update {
        /// Force re-download even if already at latest
        #[arg(short, long)]
        force: bool,
    },
    /// Verify organ binaries, workspace, and check logs for errors
    Doctor,
    /// Show workspace paths and daemon supervisor status
    Status,
    /// Display or follow daemon logs
    Log {
        /// Daemon name (e.g. spine, nerves, heart) or "all"
        name: Option<String>,
        /// Follow log output (tail -f)
        #[arg(short, long)]
        follow: bool,
        /// List available log files
        #[arg(short, long)]
        list: bool,
    },
    /// Live CPU/RAM monitor for autonomic processes
    Tui {
        #[arg(long, default_value_t = 2)]
        refresh: u64,
    },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() && !agent_body::router::is_builtin(&args[0]) {
        return agent_body::router::exec_organ(&args);
    }

    let rt = tokio::runtime::Runtime::new()?;
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Init { name }) => agent_body::init::init_project(name.as_deref())?,
        Some(Commands::Start) => agent_body::supervisor::start_all()?,
        Some(Commands::Stop) => agent_body::supervisor::stop_all()?,
        Some(Commands::Restart) => agent_body::supervisor::restart_all()?,
        Some(Commands::Supervise { interval }) => agent_body::supervisor::supervise(interval)?,
        Some(Commands::Update { force }) => agent_body::update::run_update(force)?,
        Some(Commands::Doctor) => {
            let healthy = rt.block_on(agent_body::doctor::check_all())?;
            println!();
            let logs_ok = agent_body::doctor::check_logs()?;
            if healthy && logs_ok {
                println!("\nAll systems healthy.");
            } else {
                println!("\nSome checks failed. Run `autonomic update` for version details.");
                std::process::exit(1);
            }
        }
        Some(Commands::Status) => {
            let _ = agent_body::config::Config::load()?;
            println!("autonomic status");
            println!("  config: {}", agent_body_core::config_path().display());
            println!(
                "  workspace: {}",
                agent_body_core::autonomic_root().display()
            );
            println!("  memory: {}", agent_body_core::memory_dir().display());
            println!("  broker: {}", agent_body_core::broker_dir().display());
            println!("  route organs: {}", agent_body::router::organ_list());
            println!();
            agent_body::supervisor::print_status()?;
        }
        Some(Commands::Tui { refresh }) => {
            agent_body::tui::run_dashboard(refresh)?;
        }
        Some(Commands::Log { name, follow, list }) => {
            if list {
                let logs = agent_body::log::list_logs()?;
                if logs.is_empty() {
                    println!(
                        "No log files found in {}",
                        agent_body_core::organ_state_dir("supervisor")
                            .join("logs")
                            .display()
                    );
                } else {
                    println!("Available logs:");
                    for log in &logs {
                        println!("  {log}");
                    }
                }
                return Ok(());
            }
            let name = match name {
                Some(n) => n,
                None => anyhow::bail!(
                    "usage: autonomic log <name> [--follow]  (or --list to see available logs)"
                ),
            };
            if follow {
                agent_body::log::follow_log(&name)?;
            } else {
                agent_body::log::print_log(&name)?;
            }
        }
        None => {
            Cli::parse_from(["autonomic", "--help"]);
        }
    }

    Ok(())
}
