use anyhow::Result;

pub fn compose() -> Result<()> {
    let path = agent_body_core::compose_agents_md()?;
    println!("Composed {}", path.display());
    Ok(())
}

pub fn compose_and_link() -> Result<()> {
    let links = agent_body_core::install_host_agents_md_links()?;
    let path = agent_body_core::agents_md_path();
    println!("Composed {}", path.display());
    for link in links {
        println!("  linked {}", link.display());
    }
    Ok(())
}
