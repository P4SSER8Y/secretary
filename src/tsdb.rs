use rocket::{figment::Figment, Build, Rocket};

pub async fn build(build: Rocket<Build>, figment: &Figment) -> Rocket<Build> {
    tsdb::init(figment).await;
    build
}
