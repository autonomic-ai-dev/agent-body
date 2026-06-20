use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "autonomic", about = "Autonomic AI ecosystem manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scaffold a new project and initialize required organs
    Init {
        /// Project name
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Start all required background daemons
    Start,
    /// Upgrade all ecosystem binaries to the latest compatible versions
    Update,
    /// Verify daemon health and MCP connections
    Doctor,
    /// Show configuration and status
    Status,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    match cli.command {
        Commands::Init { name } => {
            let project = name.unwrap_or_else(|| "my-ai-project".to_string());
            println!("Scaffolding project '{}'...", project);
            println!("  (not yet implemented)");
        }
        Commands::Start => {
            println!("autonomic start (not yet implemented)");
        }
        Commands::Update => {
            println!("autonomic update (not yet implemented)");
        }
        Commands::Doctor => {
            let healthy = agent_body::doctor::check_all().await?;
            if healthy {
                println!("All systems healthy.");
            } else {
                println!("Some checks failed. Run `autonomic status` for details.");
            }
        }
        Commands::Status => {
            let config = agent_body::config::Config::load()?;
            println!("autonomic status");
            println!(
                "  config: {}",
                agent_body_core::config_path().display()
            );
            println!(
                "  workspace: {}",
                agent_body_core::autonomic_root().display()
            );
            println!(
                "  memory: {}",
                agent_body_core::memory_dir().display()
            );
            println!(
                "  broker: {}",
                agent_body_core::broker_dir().display()
            );
            let _ = config;
        }
    }
    Ok(())
}
