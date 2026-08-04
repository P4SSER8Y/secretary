pub mod api;
pub mod cli;
pub mod storage;
pub mod token;

use rocket::{figment::Figment, Build, Rocket};

pub async fn build(
    base: &'static str,
    build: Rocket<Build>,
    config: &Figment,
) -> Result<Rocket<Build>, anyhow::Error> {
    // Read config
    let rp_id = config
        .find_value("gate.rp_id")
        .ok()
        .and_then(|v| v.into_string())
        .unwrap_or_default();
    let origin = config
        .find_value("gate.origin")
        .ok()
        .and_then(|v| v.into_string())
        .unwrap_or_default();
    let jwt_secret_key = config
        .find_value("gate.jwt_secret_key")
        .ok()
        .and_then(|v| v.into_string())
        .unwrap_or_default();
    let code_length: usize = config
        .find_value("gate.invite_code_length")
        .ok()
        .and_then(|v| v.to_i128())
        .unwrap_or(8) as usize;
    let ttl: u64 = config
        .find_value("gate.invite_ttl")
        .ok()
        .and_then(|v| v.to_i128())
        .unwrap_or(60) as u64;
    // token 默认超时（秒）—— 配置在 meme 段
    let jwt_expire: u64 = config
        .find_value("meme.jwt_expire")
        .ok()
        .and_then(|v| v.to_i128())
        .unwrap_or(3600) as u64;

    // Init webauthn
    if !rp_id.is_empty() && !origin.is_empty() {
        api::init_webauthn(&rp_id, &origin)?;
        log::info!("Gate: WebAuthn initialized (rp_id={}, origin={})", rp_id, origin);
    } else {
        log::warn!("Gate: rp_id or origin not configured, WebAuthn disabled");
    }

    // Init JWT signing key
    if !jwt_secret_key.is_empty() {
        api::init_signing_key(&jwt_secret_key)?;
        log::info!("Gate: JWT signing key loaded");
    } else {
        log::warn!("Gate: jwt_secret_key not configured, JWT signing disabled");
    }

    // Init config
    api::init_config(code_length, ttl, jwt_expire);

    // Clean up expired invites on startup
    if let Err(e) = storage::cleanup_expired_invites() {
        log::warn!("Gate: failed to clean up expired invites: {}", e);
    }

    // Store meta in sled
    let db = utils::database::Db::new();
    let meta = storage::GateMeta {
        rp_name: config
            .find_value("gate.rp_name")
            .ok()
            .and_then(|v| v.into_string())
            .unwrap_or_default(),
        rp_id,
        origin,
    };
    let _ = storage::set_meta(&db, &meta);

    // Mount routes
    let build = api::build(base, build, config)?;
    Ok(build)
}
