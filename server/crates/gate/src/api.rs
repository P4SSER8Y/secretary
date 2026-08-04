use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

use anyhow::Context;
use rocket::{
    get, post, routes,
    serde::json::Json,
    Build, Rocket,
};
use serde::{Deserialize, Serialize};
use webauthn_rs::prelude::*;

use crate::{
    storage::{self, delete_invite, get_invite, store_credential, StoredCredential},
    token::{generate_code, hash_code, load_signing_key, sign_jwt},
};

static WEBAUTHN: OnceLock<Webauthn> = OnceLock::new();
static SIGNING_KEY: OnceLock<jsonwebtoken::EncodingKey> = OnceLock::new();
static INVITE_CODE_LENGTH: OnceLock<usize> = OnceLock::new();
static INVITE_TTL: OnceLock<u64> = OnceLock::new();
static DEFAULT_EXPIRE: OnceLock<u64> = OnceLock::new();

fn get_default_expire() -> Option<u64> {
    DEFAULT_EXPIRE.get().copied()
}

// ── In-memory challenge store (single-process server: no need to persist) ─

const CHALLENGE_TTL: i64 = 60; // seconds

enum PendingChallenge {
    Registration {
        family: String,
        code_hash: String,
        state: PasskeyRegistration,
        created_at: i64,
    },
    Authentication {
        state: PasskeyAuthentication,
        created_at: i64,
    },
}

impl PendingChallenge {
    fn created_at(&self) -> i64 {
        match self {
            PendingChallenge::Registration { created_at, .. }
            | PendingChallenge::Authentication { created_at, .. } => *created_at,
        }
    }
}

static CHALLENGES: OnceLock<Mutex<HashMap<String, PendingChallenge>>> = OnceLock::new();

fn challenges() -> &'static Mutex<HashMap<String, PendingChallenge>> {
    CHALLENGES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn put_challenge(key: &str, pending: PendingChallenge) {
    let now = chrono::Utc::now().timestamp();
    let mut map = challenges().lock().unwrap();
    map.retain(|_, c| now - c.created_at() <= CHALLENGE_TTL);
    map.insert(key.to_string(), pending);
}

fn take_challenge(key: &str) -> Option<PendingChallenge> {
    let now = chrono::Utc::now().timestamp();
    let mut map = challenges().lock().unwrap();
    map.retain(|_, c| now - c.created_at() <= CHALLENGE_TTL);
    map.remove(key)
}

pub fn init_webauthn(rp_id: &str, origin: &str) -> Result<(), anyhow::Error> {
    let origin_url = url::Url::parse(origin)?;
    let web = WebauthnBuilder::new(rp_id, &origin_url)?
        .rp_name(rp_id)
        .timeout(std::time::Duration::from_secs(60))
        .build()?;
    WEBAUTHN.get_or_init(|| web);
    Ok(())
}

pub fn init_signing_key(key_b64: &str) -> Result<(), anyhow::Error> {
    let key = load_signing_key(key_b64)?;
    SIGNING_KEY.get_or_init(|| key);
    Ok(())
}

pub fn init_config(code_length: usize, ttl: u64, jwt_expire: u64) {
    INVITE_CODE_LENGTH.get_or_init(|| code_length);
    INVITE_TTL.get_or_init(|| ttl);
    DEFAULT_EXPIRE.get_or_init(|| jwt_expire);
}

fn get_web() -> &'static Webauthn {
    WEBAUTHN.get().expect("Webauthn not initialized")
}

fn get_key() -> &'static jsonwebtoken::EncodingKey {
    SIGNING_KEY.get().expect("Signing key not initialized")
}

fn get_code_length() -> usize {
    *INVITE_CODE_LENGTH.get().unwrap_or(&8)
}

fn get_ttl() -> u64 {
    *INVITE_TTL.get().unwrap_or(&60)
}

// ── Response types ──────────────────────────────────────

#[derive(Serialize)]
struct AuthCompleteResponse {
    token: String,
    name: String,
    family: String,
}

#[derive(Serialize)]
struct InviteExistsResponse {
    has_invites: bool,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

// ── Registration ────────────────────────────────────────

/// Registration body — Step 1 is `{ "code": "..." }`, Step 2 is the raw
/// RegistrationResponseJSON from the browser. Hodor distinguishes them by shape.
#[derive(Deserialize)]
#[serde(untagged)]
enum RegisterBody {
    Begin { code: String },
    Complete(RegisterPublicKeyCredential),
}

/// POST /gate/register?name=<name>&family=<family>
/// (family query param ignored — the invite determines the real family)
#[post("/register?<name>", data = "<body>")]
async fn register(
    name: &str,
    body: Json<RegisterBody>,
) -> Result<Json<serde_json::Value>, (rocket::http::Status, Json<ErrorResponse>)> {
    match body.into_inner() {
        RegisterBody::Begin { code } => register_begin(name, &code).await,
        RegisterBody::Complete(response) => register_complete(name, response).await,
    }
}

/// Step 1: verify code → return registration options JSON (unwrapped)
async fn register_begin(
    name: &str,
    code: &str,
) -> Result<Json<serde_json::Value>, (rocket::http::Status, Json<ErrorResponse>)> {
    let db = utils::database::Db::new();
    let code_hash = hash_code(code);

    let invite = get_invite(&code_hash)
        .map_err(|e| err500(&e.to_string()))?
        .ok_or_else(|| err(rocket::http::Status::Unauthorized, "invalid_code"))?;

    let now = chrono::Utc::now().timestamp();
    if now > invite.expires_at {
        let _ = delete_invite(&code_hash);
        return Err(err(rocket::http::Status::Gone, "invite_expired"));
    }

    if storage::credential_exists(&db, name, &invite.family)
        .map_err(|e| err500(&e.to_string()))?
    {
        return Err(err(rocket::http::Status::Conflict, "already_registered"));
    }

    let user_id = uuid::Uuid::new_v4();
    let (challenge, state) = get_web()
        .start_passkey_registration(user_id, name, name, None)
        .map_err(|e| err500(&format!("WebAuthn error: {}", e)))?;

    // Serialize ONLY the public_key options (Hodor/@simplewebauthn expect unwrapped)
    let challenge_json = serde_json::to_value(&challenge.public_key)
        .map_err(|e| err500(&format!("JSON error: {}", e)))?;

    // Store in-memory, keyed by name (step 2 has no code, only name)
    put_challenge(
        &format!("reg:{}", name),
        PendingChallenge::Registration {
            family: invite.family.clone(),
            code_hash,
            state,
            created_at: now,
        },
    );

    Ok(Json(challenge_json))
}

/// Step 2: complete registration → store credential → delete invite + challenge
async fn register_complete(
    name: &str,
    response: RegisterPublicKeyCredential,
) -> Result<Json<serde_json::Value>, (rocket::http::Status, Json<ErrorResponse>)> {
    let db = utils::database::Db::new();

    // Step 2 body has no code — take the in-memory pending challenge by name
    let (family, code_hash, state) = match take_challenge(&format!("reg:{}", name)) {
        Some(PendingChallenge::Registration {
            family,
            code_hash,
            state,
            ..
        }) => (family, code_hash, state),
        _ => return Err(err(rocket::http::Status::BadRequest, "no_active_challenge")),
    };

    let result = get_web()
        .finish_passkey_registration(&response, &state)
        .context("WebAuthn registration failed");

    // Clean up invite regardless of outcome (用完即删)
    let _ = delete_invite(&code_hash);

    match result {
        Ok(passkey) => {
            let passkey_bytes = serde_json::to_vec(&passkey)
                .map_err(|e| err500(&format!("serde_json: {}", e)))?;
            let credential = StoredCredential {
                name: name.to_string(),
                family: family.clone(),
                credential_id: format!("{:?}", passkey.cred_id()),
                passkey_bytes,
                origin: String::new(),
                enabled: true,
                created_at: chrono::Utc::now().to_rfc3339(),
            };
            store_credential(&db, &credential).map_err(|e| err500(&e.to_string()))?;
            // 返回注册时的真实 family，供前端跳转登录页时自动带上
            Ok(Json(serde_json::json!({ "status": "ok", "family": family })))
        }
        Err(e) => Err(err(
            rocket::http::Status::BadRequest,
            &format!("registration_failed: {}", e),
        )),
    }
}

// ── Authentication ──────────────────────────────────────

/// GET /gate/auth?name=<name>&family=<family> → authentication options JSON (unwrapped)
#[get("/auth?<name>&<family>")]
async fn auth_begin(
    name: &str,
    family: &str,
) -> Result<Json<serde_json::Value>, (rocket::http::Status, Json<ErrorResponse>)> {
    let db = utils::database::Db::new();

    let creds = storage::list_credentials(&db, Some(family))
        .map_err(|e| err500(&e.to_string()))?;
    let mut passkeys = Vec::new();
    for (_, c) in &creds {
        if c.name == name && c.enabled {
            if let Ok(passkey) = serde_json::from_slice::<Passkey>(&c.passkey_bytes) {
                passkeys.push(passkey);
            }
        }
    }

    if passkeys.is_empty() {
        return Err(err(rocket::http::Status::Unauthorized, "no_credentials"));
    }

    let (challenge, state) = get_web()
        .start_passkey_authentication(&passkeys)
        .map_err(|e| err500(&format!("WebAuthn error: {}", e)))?;

    // Unwrapped options (matches @simplewebauthn generateAuthenticationOptions)
    let challenge_json = serde_json::to_value(&challenge.public_key)
        .map_err(|e| err500(&format!("JSON error: {}", e)))?;

    put_challenge(
        &format!("auth:{}:{}", name, family),
        PendingChallenge::Authentication {
            state,
            created_at: chrono::Utc::now().timestamp(),
        },
    );

    Ok(Json(challenge_json))
}

/// POST /gate/auth?name=<name>&family=<family> → verify assertion, return JWT
/// body 是 WebAuthn 认证响应；Hodor 前端可能附带 `expire`（秒）字段控制 token 有效期
#[post("/auth?<name>&<family>", data = "<body>")]
async fn auth_complete(
    name: &str,
    family: &str,
    body: Json<serde_json::Value>,
) -> Result<Json<AuthCompleteResponse>, (rocket::http::Status, Json<ErrorResponse>)> {
    let state = match take_challenge(&format!("auth:{}:{}", name, family)) {
        Some(PendingChallenge::Authentication { state, .. }) => state,
        _ => return Err(err(rocket::http::Status::BadRequest, "no_active_challenge")),
    };

    // ttl 完全由后端 meme.jwt_expire 配置决定，前端不可覆盖
    let expire = get_default_expire();
    let credential: PublicKeyCredential = serde_json::from_value(body.into_inner())
        .map_err(|e| err500(&format!("invalid auth response: {}", e)))?;

    get_web()
        .finish_passkey_authentication(&credential, &state)
        .map_err(|e| {
            err(
                rocket::http::Status::Unauthorized,
                &format!("authentication_failed: {}", e),
            )
        })?;

    let token =
        sign_jwt(name, family, get_key(), expire).map_err(|e| err500(&format!("JWT: {}", e)))?;

    Ok(Json(AuthCompleteResponse {
        token,
        name: name.to_string(),
        family: family.to_string(),
    }))
}

// ── Invite check (for frontend default-login logic) ─────

/// GET /gate/invite/exists → whether any pending (non-expired) invite exists
#[get("/invite/exists")]
async fn invite_exists() -> Result<Json<InviteExistsResponse>, (rocket::http::Status, Json<ErrorResponse>)> {
    let has = storage::has_invites().map_err(|e| err500(&e.to_string()))?;
    Ok(Json(InviteExistsResponse { has_invites: has }))
}

/// Health check
#[get("/ping")]
fn ping() -> &'static str {
    "pong"
}

// ── Helpers ─────────────────────────────────────────────

fn err(status: rocket::http::Status, msg: &str) -> (rocket::http::Status, Json<ErrorResponse>) {
    (
        status,
        Json(ErrorResponse {
            error: msg.to_string(),
        }),
    )
}

fn err500(msg: &str) -> (rocket::http::Status, Json<ErrorResponse>) {
    (
        rocket::http::Status::InternalServerError,
        Json(ErrorResponse {
            error: msg.to_string(),
        }),
    )
}

// ── CLI support (used by cli.rs) ────────────────────────

pub fn cli_create_invite(
    family: &str,
    code_length: Option<usize>,
    ttl: Option<u64>,
) -> anyhow::Result<String> {
    use crate::storage::create_invite;
    let len = code_length.unwrap_or_else(get_code_length);
    let ttl_val = ttl.unwrap_or_else(get_ttl);
    let code = generate_code(len);
    let code_hash = hash_code(&code);
    create_invite(&code_hash, family, ttl_val)?;
    Ok(code)
}

// ── Route mounting ──────────────────────────────────────

pub fn build(
    base: &'static str,
    build: Rocket<Build>,
    _config: &rocket::figment::Figment,
) -> Result<Rocket<Build>, anyhow::Error> {
    Ok(build.mount(
        base,
        routes![register, auth_begin, auth_complete, invite_exists, ping],
    ))
}
