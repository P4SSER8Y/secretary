use chrono::Local;
use clap::{Parser, Subcommand};
use log::{info, warn};
use rocket::figment::{
    providers::{Format, Toml},
    Figment,
};

#[macro_use]
extern crate rocket;

mod kindle;
mod let_server_run;
mod logger;
mod qweather;

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

async fn go() -> Result<(), rocket::Error> {
    let config = Figment::new()
        .merge(Toml::file("Rocket.toml").nested())
        .merge(Toml::file("Local.toml").nested())
        .select(PROFILE);

    let mut wtf = rocket::build();
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

    if is_enabled(&config, "let_server_run", false) {
        wtf = let_server_run::build(wtf, &config).await;
    }
    if is_enabled(&config, "weather", false) {
        wtf = qweather::build(wtf, &config).await;
    }
    if is_enabled(&config, "kindle", false) {
        wtf = kindle::build("/kindle", wtf, &config);
    }

    wtf.ignite().await?.launch().await?;
    let _ = db.flush();
    Ok(())
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    logger::init();

    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Version) => {
            println!("{}", VERSION);
            Ok(())
        }
        Some(Commands::Go) => go().await,
        None => go().await,
    }
}
