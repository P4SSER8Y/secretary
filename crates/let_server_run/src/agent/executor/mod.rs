use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};

mod echo;
mod parsers;
mod shell;

#[derive(Deserialize, Debug)]
#[serde(tag = "t", content = "c", rename_all = "lowercase")]
pub enum ExecutorType {
    Echo(String),
    Shell(String),
}

pub async fn execute(
    message: &str,
    config: Arc<HashMap<String, ExecutorType>>,
) -> Result<String, anyhow::Error> {
    match parsers::parser(message) {
        Ok((alias, args)) => {
            let alias_lower = alias.to_lowercase();
            match config.get(&alias_lower) {
                Some(cfg) => match cfg {
                    ExecutorType::Echo(_) => echo::execute(alias, &args, "").await,
                    ExecutorType::Shell(c) => shell::execute(alias, &args, c).await,
                },
                None => Err(anyhow::anyhow!("executor \"{}\" not matched", alias)),
            }
        }
        Err(err) => Err(err),
    }
}
