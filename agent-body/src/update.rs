use anyhow::Result;

use crate::router::{self, ORGANS};

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
