use clap::{Parser, Subcommand, ValueEnum};

use agent_body_core::ui::ProgressMode;

#[derive(Parser)]
#[command(name = "autonomic", about = "Autonomic AI ecosystem manager", version)]
struct Cli {
    /// Progress output style (Docker BuildKit-like): auto, plain, or quiet
    #[arg(long, value_enum, global = true, default_value = "auto")]
    progress: ProgressArg,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Clone, Copy, ValueEnum)]
enum ProgressArg {
    Auto,
    Plain,
    Quiet,
}

impl From<ProgressArg> for ProgressMode {
    fn from(value: ProgressArg) -> Self {
        match value {
            ProgressArg::Auto => ProgressMode::Auto,
            ProgressArg::Plain => ProgressMode::Plain,
            ProgressArg::Quiet => ProgressMode::Quiet,
        }
    }
}

#[derive(Subcommand)]
enum DoctorCommands {
    /// Workspace + organ version summary (no log scan)
    Stats {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum AgentsCommands {
    /// Rebuild ~/.autonomic/AGENTS.md
    Compose {
        /// Install host symlinks to composed AGENTS.md
        #[arg(long)]
        install: bool,
    },
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
        /// Update a single organ alias (brain, spine, heart, ...)
        #[arg(long)]
        organ: Option<String>,
    },
    /// Git sync for whole ~/.autonomic workspace
    Sync {
        #[arg(required = true)]
        target: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Compose AGENTS.md from ~/.autonomic/agents fragments
    Agents {
        #[command(subcommand)]
        command: AgentsCommands,
    },
    /// Verify organ binaries, workspace, and check logs for errors
    Doctor {
        #[command(subcommand)]
        command: Option<DoctorCommands>,
        /// Binaries and workspace only — skip supervisor log scan
        #[arg(long)]
        quick: bool,
        /// Alias for --quick
        #[arg(long, hide = true)]
        binaries_only: bool,
    },
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
    Top {
        #[arg(long, default_value_t = 2)]
        refresh: u64,
    },
    /// Launch the Autonomic Terminal Dashboard
    Tui {
        /// Re-download agent-tui even if already installed
        #[arg(short, long)]
        force: bool,
    },
    /// Start the MCP Gateway server over stdio (aggregates all organ MCP tools)
    ServeMcp,
    /// Launch the Autonomic Web Dashboard
    Ui {
        /// Open the hosted dashboard without starting the local NATS relay
        #[arg(long)]
        open_only: bool,
        /// Pull the latest agent-ui relay and reinstall dependencies
        #[arg(short, long)]
        force: bool,
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
    apply_progress_env(cli.progress);

    match cli.command {
        Some(Commands::Init { name }) => agent_body::init::init_project(name.as_deref())?,
        Some(Commands::Start) => agent_body::supervisor::start_all()?,
        Some(Commands::Stop) => agent_body::supervisor::stop_all()?,
        Some(Commands::Restart) => agent_body::supervisor::restart_all()?,
        Some(Commands::Supervise { interval }) => agent_body::supervisor::supervise(interval)?,
        Some(Commands::Update { force, organ }) => {
            agent_body::update::run_update(force, organ.as_deref())?
        }
        Some(Commands::Sync { target, args }) => {
            let mut argv = vec![target];
            argv.extend(args);
            agent_body::sync_cmd::run(&argv)?;
        }
        Some(Commands::Agents { command }) => match command {
            AgentsCommands::Compose { install } => {
                if install {
                    agent_body::agents::compose_and_link()?;
                } else {
                    agent_body::agents::compose()?;
                }
            }
        },
        Some(Commands::Doctor {
            command,
            quick,
            binaries_only,
        }) => match command {
            Some(DoctorCommands::Stats { json }) => {
                rt.block_on(agent_body::doctor::run_stats(json))?;
            }
            None => {
                rt.block_on(agent_body::doctor::run(quick || binaries_only))?;
            }
        },
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
        Some(Commands::Top { refresh }) => {
            agent_body::tui::run_dashboard(refresh)?;
        }
        Some(Commands::Tui { force }) => agent_body::tui_install::run(force)?,
        Some(Commands::Ui { open_only, force }) => agent_body::ui_relay::run(open_only, force)?,
        Some(Commands::ServeMcp) => rt.block_on(agent_body::mcp_gateway::McpGateway::run())?,
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

fn apply_progress_env(mode: ProgressArg) {
    let value = match ProgressMode::from(mode) {
        ProgressMode::Auto => "auto",
        ProgressMode::Plain => "plain",
        ProgressMode::Quiet => "quiet",
    };
    std::env::set_var("AUTONOMIC_PROGRESS", value);
}
