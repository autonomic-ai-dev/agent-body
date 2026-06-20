use anyhow::Result;
use std::process::Command;

use crate::router::{self, ORGANS};

pub fn run_update() -> Result<()> {
    println!("Updating all Autonomic organs from latest GitHub releases...");
    
    let mut child = Command::new("bash")
        .arg("-c")
        .arg("curl -fsSL https://raw.githubusercontent.com/autonomic-ai-dev/agent-body/main/scripts/install-all-organs.sh | bash")
        .spawn()?;
        
    let status = child.wait()?;
    
    if status.success() {
        println!("\nSuccessfully updated all organs.\n");
    } else {
        println!("\nUpdate failed.\n");
    }
    
    show_versions()
}

pub fn show_versions() -> Result<()> {
    println!("Autonomic organ versions on PATH:\n");
    println!("{:<10} {:<16} status", "organ", "binary");
    println!("{}", "-".repeat(42));

    for (alias, binary) in ORGANS {
        match router::organ_version(alias)? {
            Some(version) => println!("{alias:<10} {binary:<16} {version}"),
            None => println!("{alias:<10} {binary:<16} not installed"),
        }
    }

    println!();
    println!("Install missing organs from GitHub releases or `cargo install --path <repo>`.");
    Ok(())
}
