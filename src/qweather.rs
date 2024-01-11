use rocket::{Build, Rocket, figment::Figment};

pub async fn build(build: Rocket<Build>, figment: &Figment) -> Rocket<Build> {
    qweather::init(figment).await;
    return build;
}
