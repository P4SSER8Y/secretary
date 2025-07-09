use anyhow::Result;
use figment::Figment;

mod agent;

pub async fn init(figment: &Figment) -> Result<()> {
    let config: agent::Config = figment.find_value("let_server_run")?.deserialize()?;
    tokio::spawn(agent::go(config));
    Ok(())
}
