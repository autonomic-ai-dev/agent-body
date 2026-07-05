use anyhow::{bail, Result};

use agent_body_core::ecosystem_config;

pub fn run_git(sub: &[String]) -> Result<()> {
    let sync = ecosystem_config::effective_sync(None)?;
    if sub.is_empty() {
        bail!("usage: autonomic sync git <init|status|pull|push>");
    }
    match sub[0].as_str() {
        "init" => {
            agent_body_core::git_init(
                (!sync.git.remote.is_empty()).then_some(sync.git.remote.as_str()),
                Some(sync.git.branch.as_str()),
            )?;
            println!(
                "Initialized workspace git sync at {}",
                agent_body_core::autonomic_root().display()
            );
        }
        "status" => {
            println!("{}", agent_body_core::git_status()?);
        }
        "pull" => {
            agent_body_core::git_pull()?;
            println!("pulled workspace sync");
        }
        "push" => {
            agent_body_core::git_push()?;
            println!("pushed workspace sync");
        }
        other => bail!("unknown sync git subcommand: {other}"),
    }
    Ok(())
}

pub fn run(args: &[String]) -> Result<()> {
    if args.is_empty() {
        bail!("usage: autonomic sync git <init|status|pull|push>");
    }
    match args[0].as_str() {
        "git" => run_git(&args[1..]),
        other => bail!("unknown sync target: {other} (only `git` is supported)"),
    }
}
