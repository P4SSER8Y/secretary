use rocket::{Build, Rocket};
use log::error;

pub async fn build(build: Rocket<Build>) -> Rocket<Build> {
    if let Err(err) = let_server_run::init(build.figment()).await {
        error!("init let_server_run failed: {}", err);
    }
    return build;
}
