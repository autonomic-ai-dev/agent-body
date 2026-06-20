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
    /// Show installed organ binary versions
    Update,
    /// Verify organ binaries and workspace
    Doctor,
    /// Show workspace paths and daemon supervisor status
    Status,
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
        Some(Commands::Update) => agent_body::update::show_versions()?,
        Some(Commands::Doctor) => {
            let healthy = rt.block_on(agent_body::doctor::check_all())?;
            if healthy {
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
            println!(
                "  route organs: {}",
                agent_body::router::organ_list()
            );
            println!();
            agent_body::supervisor::print_status()?;
        }
        Some(Commands::Tui { refresh }) => {
            agent_body::tui::run_dashboard(refresh)?;
        }
        None => {
            Cli::parse_from(["autonomic", "--help"]);
        }
    }

    Ok(())
}
