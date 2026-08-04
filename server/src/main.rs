use std::{collections::HashSet, path::Path};

use anyhow::anyhow;
use chrono::Local;
use clap::{Parser, Subcommand};
use log::{info, warn};
use minisign_verify::{PublicKey, Signature};
use rocket::figment::{
    providers::{Format, Toml},
    Figment,
};

#[macro_use]
extern crate rocket;

mod decrypt;
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
    /// verify signature
    Verify { files: Vec<String> },
    /// decrypt meme-encrypted data from stdin
    Decrypt {
        #[arg(short, long)]
        owner: String,
        #[arg(short, long)]
        password: String,
    },
    /// Gate WebAuthn credential & invite management (CLI only)
    Gate {
        #[command(subcommand)]
        command: gate::cli::GateCommand,
    },
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

    if is_enabled(&config, "mqtt", false) {
        if let Ok(host) = config.find_value("mqtt.host") {
            if let Some(host) = host.as_str() {
                if !host.is_empty() {
                    let port = config
                        .find_value("mqtt.port")
                        .ok()
                        .and_then(|x| x.to_i128())
                        .unwrap_or(1883) as u16;
                    let username = config
                        .find_value("mqtt.username")
                        .ok()
                        .and_then(|x| x.into_string());
                    let password = config
                        .find_value("mqtt.password")
                        .ok()
                        .and_then(|x| x.into_string());
                    let client_id = config
                        .find_value("mqtt.client_id")
                        .ok()
                        .and_then(|x| x.into_string())
                        .unwrap_or_else(|| "secretary".to_string());
                    mqtt::init(mqtt::MqttConfig {
                        host: host.to_string(),
                        port,
                        username,
                        password,
                        client_id,
                    });
                }
            }
        }
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
        wtf = kindle::build("/kindle/", wtf, &config);
    }
    if is_enabled(&config, "meme", false) {
        wtf = meme::build("/meme/", wtf, &config).await.unwrap();
    }
    if is_enabled(&config, "album", false) {
        wtf = album::build("/album/api/", wtf, &config).await.unwrap();
    }
    if is_enabled(&config, "inbox", false) {
        wtf = inbox::build("/inbox/api/", wtf, &config).await.unwrap();
    }
    if is_enabled(&config, "recipe", false) {
        wtf = recipe::build("/recipe/api/", wtf, &config).await.unwrap();
    }
    if is_enabled(&config, "gate", false) {
        wtf = gate::build("/gate/api/", wtf, &config).await.unwrap();
    }
    if let Ok(ui) = config.find_value("ui_path") {
        if let Some(ui) = ui.as_str() {
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

fn iter_load_config(file: &str) -> Figment {
    let mut config = Figment::new();
    let mut set = HashSet::new();
    let mut data_path = "".to_string();
    let mut file = file.to_string();
    while !set.contains(&file) {
        let file_path = Path::new(&data_path).join(&file);
        eprintln!(">>> Load and merge {:?}", file_path);
        set.insert(file.clone());
        config = config
            .merge(Toml::file(&file_path).nested())
            .select(PROFILE);

        if let Ok(path) = config.find_value("data_path") {
            if let Some(path) = path.as_str() {
                data_path = path.to_string();
            }
        }
        let local = config.find_value("local");
        if local.is_err() {
            break;
        }
        let local = local.unwrap();
        if let Some(path) = local.as_str() {
            file = path.to_string();
            continue;
        }
        break;
    }
    config
}

fn verify(config: &Figment, files: &Vec<String>) -> anyhow::Result<()> {
    if files.len() == 0 {
        return Err(anyhow!("No files to verify"));
    }
    let public_key = config
        .find_value("verify.public_key")?
        .as_str()
        .ok_or(anyhow!("verify.public_key is not a valid string"))?
        .to_owned();
    log::info!("Verifying files with public key: {}", public_key);
    let public_key = PublicKey::from_base64(&public_key).map_err(|e| anyhow!("invalid public key: {}", e))?;
    for file in files {
        let content = std::fs::read(Path::new(file))
            .map_err(|e| anyhow!("Failed to read file {}: {}", file, e))?;
        let sign = Signature::from_file(Path::new(&format!("{}.minisig", file)))
            .map_err(|e| anyhow!("Failed to read signature from {}.minisig: {}", file, e))?;
        let _ = public_key
            .verify(&content, &sign, false)
            .map_err(|e| anyhow!("Failed to verify {}: {}", file, e))?;
        log::info!("verify {} success", file);
    }
    Ok(())
}

/// 42
#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let config = iter_load_config("Rocket.toml");
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
        Some(Commands::Verify { files }) => {
            if let Err(e) = verify(&config, &files) {
                log::error!("Verify failed: {}", e);
            }
            Ok(())
        }
        Some(Commands::Decrypt { owner, password }) => decrypt::run(&owner, &password),
        Some(Commands::Gate { command }) => {
            // Ensure data_path is initialized (normally done in go())
            if let Ok(data) = config.find_value("data_path") {
                if let Some(data) = data.as_str() {
                    utils::init_data_path(data);
                }
            }
            if let Err(e) = gate::cli::handle(command) {
                eprintln!("Error: {}", e);
            }
            Ok(())
        }
        Some(Commands::Go) | None => go(&config).await,
    }
}
