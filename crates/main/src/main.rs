use chrono::Local;
use clap::{Parser, Subcommand};
use log::{info, warn};
use rocket::{Build, Rocket};

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

fn is_enabled(build: &Rocket<Build>, name: &str, default: bool) -> bool {
    if let Ok(value) = build.figment().find_value(&format!("switches.{}", name)) {
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

async fn go() -> Result<(), rocket::Error> {
    let mut wtf = rocket::build();
    info!("build version: {}", VERSION);

    if is_enabled(&wtf, "bark", false) {
        bark::build(wtf.figment());
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

    if is_enabled(&wtf, "let_server_run", false) {
        wtf = let_server_run::build(wtf).await;
    }
    if is_enabled(&wtf, "weather", false) {
        wtf = qweather::build(wtf).await;
    }
    if is_enabled(&wtf, "kindle", false) {
        wtf = kindle::build("/kindle", wtf);
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