use chrono::Local;
use clap::{Parser, Subcommand};
use log::{info, warn};
use rocket::figment::{
    providers::{Format, Toml},
    Figment,
};

#[macro_use]
extern crate rocket;

mod fairings;
mod kindle;
mod let_server_run;
mod logger;
mod qweather;
mod tsdb;

pub const VERSION: &'static str = include_str!(concat!(env!("OUT_DIR"), "/version"));

#[derive(Parser)]
#[command(author, version = VERSION)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// launch server
    Go,
    /// print version and exit
    Version,
}

fn is_enabled(config: &Figment, name: &str, default: bool) -> bool {
    if let Ok(value) = config.find_value(&format!("switches.{}", name)) {
        if let Some(value) = value.to_bool() {
            warn!(
                "switches.{} is {}",
                name,
                if value { "enabled" } else { "disabled" }
            );
            return value;
        }
    }
    error!("switches.{} is not provided", name);
    default
}

#[cfg(debug_assertions)]
const PROFILE: &str = "debug";

#[cfg(not(debug_assertions))]
const PROFILE: &str = "release";

async fn go(config: &Figment) -> Result<(), rocket::Error> {
    let config = rocket::Config::figment().merge(config);
    if let Ok(data) = config.find_value("data_path") {
        if let Some(data) = data.as_str() {
            utils::init_data_path(data);
        }
    }
    let data = std::path::Path::new(utils::get_data_path());
    if !data.exists() {
        std::fs::create_dir_all(data).or_else(|err| Err(rocket::error::ErrorKind::Io(err)))?;
    } else if !data.is_dir() {
        use rocket::error::ErrorKind;
        use rocket::figment::error::Kind;
        return Err(ErrorKind::Config(
            Kind::Message(format!(
                "{} is not a valid directory",
                data.to_str().unwrap()
            ))
            .into(),
        )
        .into());
    }
    let mut wtf = rocket::custom(&config);
    info!("build version: {}", VERSION);

    if is_enabled(&config, "bark", false) {
        bark::build(&config);
        tokio::spawn(bark::send(bark::Message {
            body: "Hello World",
            title: Some("Lighter"),
            ..Default::default()
        }));
    }

    let db = utils::database::Db::new();
    if let Ok(launch) = db.get::<String>("launch") {
        info!("last launch at {:?}", launch);
    } else {
        info!("never launched before");
    }
    if let Err(_) = db.set("launch", &Local::now().to_rfc3339()) {
        error!("last launch not found");
    }

    if is_enabled(&config, "tsdb", false) {
        wtf = tsdb::build(wtf, &config).await;
    }

    if is_enabled(&config, "let_server_run", false) {
        wtf = let_server_run::build(wtf, &config).await;
    }
    if is_enabled(&config, "weather", false) {
        wtf = qweather::build(wtf, &config).await;
    }
    if is_enabled(&config, "kindle", false) {
        wtf = kindle::build("/kindle", wtf, &config);
    }
    if is_enabled(&config, "inbox", false) {
        wtf = inbox::build("/inbox/api", wtf, &config).await.unwrap();
    }
    if let Ok(ui) = config.find_value("ui_path") {
        info!("{:#?}", ui);
        if let Some(ui) = ui.as_str() {
            info!("{:#?}", ui);
            use rocket::fs::{FileServer, Options};
            let options = Options::Index | Options::NormalizeDirs;
            wtf = wtf.mount("/", FileServer::new(ui, options).rank(999))
        }
    }

    wtf.attach(fairings::RequestTimer)
        .ignite()
        .await?
        .launch()
        .await?;
    let _ = db.flush();
    Ok(())
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let mut config = Figment::new()
        .merge(Toml::file("Rocket.toml").nested())
        .select(PROFILE);
    if let Ok(local) = config.find_value("local") {
        if let Some(path) = local.as_str() {
            config = config.merge(Toml::file(path).nested()).select(PROFILE);
        }
    }

    let level = config
        .find_value("level")
        .ok()
        .and_then(|x| x.into_string());
    logger::init(level.as_deref());

    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Version) => {
            println!("{}", VERSION);
            Ok(())
        }
        Some(Commands::Go) | None => go(&config).await,
    }
}
