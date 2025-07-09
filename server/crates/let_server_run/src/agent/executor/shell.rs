use anyhow::anyhow;
use tokio::{
    process::Command,
    time::{timeout, Duration},
};

use super::parsers;

pub async fn execute(
    alias: &str,
    _args: &Vec<&str>,
    context: &str,
) -> Result<String, anyhow::Error> {
    let args = parsers::parser(context)?;
    let mut process = Command::new(args.0)
        .args(args.1)
        .kill_on_drop(true)
        .spawn()?;
    match timeout(Duration::from_secs(60), process.wait()).await {
        Ok(_) => Ok(format!("run {} succeeded", alias)),
        Err(_) => Err(anyhow!("run {} timeout", alias)),
    }
}
