use rocket::{Build, Rocket};

pub async fn build(build: Rocket<Build>) -> Rocket<Build> {
    qweather::init(build.figment()).await;
    return build;
}
