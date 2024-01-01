use rocket::{Build, Rocket};

pub fn build(build: Rocket<Build>) -> Rocket<Build> {
    qweather::init(build.figment());
    return build;
}
