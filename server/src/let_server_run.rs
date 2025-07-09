use rocket::{Build, Rocket, figment::Figment};
use log::error;

pub async fn build(build: Rocket<Build>, figment: &Figment) -> Rocket<Build> {
    if let Err(err) = let_server_run::init(figment).await {
        error!("init let_server_run failed: {}", err);
    }
    return build;
}
