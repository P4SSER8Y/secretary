use rocket::{
    delete, get, http::ContentType, post, response::content::RawJson, serde::json::Json, Build, routes, FromForm, Rocket, State,
};
use serde::Serialize;

use crate::config::AlbumConfig;
use crate::converter::{ConversionParams, CropAnchor};
use crate::storage::ImageMetadata;
use crate::{discovery, sender, storage};

// ---- Shared state ----

struct AlbumState {
    config: AlbumConfig,
}

// ---- Response types ----

#[derive(Serialize)]
struct UploadResponse {
    ok: bool,
    id: String,
    meta: storage::ImageMetadata,
}

#[derive(Serialize)]
struct ListResponse {
    images: Vec<storage::ImageMetadata>,
}

#[derive(Serialize)]
struct SwitchResponse {
    ok: bool,
    message: String,
}

#[derive(Serialize)]
struct StatusResponse {
    ip: Option<String>,
    online: bool,
    status: Option<String>,
    last_seen: Option<String>,
}

// ---- Upload form ----

#[derive(FromForm)]
struct UploadForm<'r> {
    file: rocket::fs::TempFile<'r>,
    #[field(name = "crop_mode")]
    crop_mode: Option<String>,
    dither: Option<bool>,
    rotate_cw: Option<bool>,
    rotate_ccw: Option<bool>,
    invert: Option<bool>,
}

// ---- Build ----

pub async fn build(
    build: Rocket<Build>,
    base: &str,
    config: AlbumConfig,
) -> anyhow::Result<Rocket<Build>> {
    Ok(build.manage(AlbumState { config }).mount(
        base,
        routes![
            preview,
            display,
            upload,
            list,
            get_preview,
            get_raw,
            switch,
            switch_next,
            switch_prev,
            switch_random,
            delete_image,
            device_status,
            current_image,
        ],
    ))
}

// ---- Handlers ----

#[post("/preview", data = "<form>", format = "multipart/form-data")]
async fn preview(
    form: rocket::form::Form<UploadForm<'_>>,
    state: &State<AlbumState>,
) -> Result<(ContentType, Vec<u8>), RawJson<String>> {
    let (file_data, params, _, _) = extract_form(form).await?;
    let cfg = state.config.clone();

    let converted = tokio::task::spawn_blocking(move || {
        crate::converter::convert_to_epd(&file_data, &params, &cfg)
    })
    .await
    .map_err(|e| err_json(&format!("conversion panicked: {}", e)))?
    .map_err(|e| err_json(&format!("conversion failed: {}", e)))?;

    Ok((ContentType::PNG, converted.epd_png))
}

#[post("/display", data = "<form>", format = "multipart/form-data")]
async fn display(
    form: rocket::form::Form<UploadForm<'_>>,
    state: &State<AlbumState>,
) -> Result<Json<SwitchResponse>, RawJson<String>> {
    let (file_data, params, _, _) = extract_form(form).await?;
    let cfg = state.config.clone();

    let converted = tokio::task::spawn_blocking(move || {
        crate::converter::convert_to_epd(&file_data, &params, &cfg)
    })
    .await
    .map_err(|e| err_json(&format!("conversion panicked: {}", e)))?
    .map_err(|e| err_json(&format!("conversion failed: {}", e)))?;

    sender::send_to_device(&state.config, &converted.raw_4bpp)
        .await
        .map_err(|e| err_json(&format!("send failed: {}", e)))?;

    Ok(Json(SwitchResponse {
        ok: true,
        message: "image sent to device".to_string(),
    }))
}

#[post("/upload", data = "<form>", format = "multipart/form-data")]
async fn upload(
    form: rocket::form::Form<UploadForm<'_>>,
    state: &State<AlbumState>,
) -> Result<Json<UploadResponse>, RawJson<String>> {
    let (file_data, params, original_filename, mime_type) = extract_form(form).await?;
    let id = uuid::Uuid::new_v4().to_string();
    let cfg = state.config.clone();
    let meta_params = params.clone();

    let converted = {
        let file_data = file_data.clone();
        tokio::task::spawn_blocking(move || {
            crate::converter::convert_to_epd(&file_data, &params, &cfg)
        })
        .await
        .map_err(|e| err_json(&format!("conversion panicked: {}", e)))?
        .map_err(|e| err_json(&format!("conversion failed: {}", e)))?
    };

    let meta = storage::ImageMetadata {
        id: id.clone(),
        original_filename: original_filename.unwrap_or_else(|| "unknown".to_string()),
        mime_type: mime_type.unwrap_or_else(|| "application/octet-stream".to_string()),
        created_at: chrono::Local::now().to_rfc3339(),
        params: meta_params,
    };

    storage::save_image(&state.config, &id, &converted, &meta)
        .map_err(|e| err_json(&format!("save failed: {}", e)))?;

    Ok(Json(UploadResponse {
        ok: true,
        id,
        meta,
    }))
}

#[get("/list")]
fn list() -> Json<ListResponse> {
    let images = storage::list_images();
    Json(ListResponse { images })
}

#[get("/preview/<id>")]
fn get_preview(
    id: &str,
    state: &State<AlbumState>,
) -> Result<(ContentType, Vec<u8>), RawJson<String>> {
    let data = storage::load_preview(&state.config, id)
        .map_err(|e| err_json(&format!("preview not found: {}", e)))?;
    Ok((ContentType::PNG, data))
}

#[get("/raw/<id>")]
fn get_raw(id: &str, state: &State<AlbumState>) -> Result<Vec<u8>, RawJson<String>> {
    storage::load_raw_binary(&state.config, id)
        .map_err(|e| err_json(&format!("raw not found: {}", e)))
}

#[post("/switch/<id>")]
async fn switch(
    id: &str,
    state: &State<AlbumState>,
) -> Result<Json<SwitchResponse>, RawJson<String>> {
    do_switch(&state.config, id).await
}

#[post("/switch/next")]
async fn switch_next(
    state: &State<AlbumState>,
) -> Result<Json<SwitchResponse>, RawJson<String>> {
    let images = storage::list_images();
    if images.is_empty() {
        return Err(err_json("no images"));
    }
    let current_id = storage::get_current().ok().flatten();
    let mut sorted: Vec<_> = images.iter().collect();
    sorted.sort_by(|a, b| a.created_at.cmp(&b.created_at));

    let next = if let Some(ref cid) = current_id {
        let pos = sorted.iter().position(|m| m.id == *cid);
        let picked: &ImageMetadata = match pos {
            Some(p) if p + 1 < sorted.len() => sorted[p + 1],
            _ => sorted.first().unwrap(),
        };
        picked.clone()
    } else {
        images[0].clone()
    };

    do_switch(&state.config, &next.id).await
}

#[post("/switch/prev")]
async fn switch_prev(
    state: &State<AlbumState>,
) -> Result<Json<SwitchResponse>, RawJson<String>> {
    let images = storage::list_images();
    if images.is_empty() {
        return Err(err_json("no images"));
    }
    let current_id = storage::get_current().ok().flatten();
    let mut sorted: Vec<_> = images.iter().collect();
    sorted.sort_by(|a, b| a.created_at.cmp(&b.created_at));

    let prev = if let Some(ref cid) = current_id {
        let pos = sorted.iter().position(|m| m.id == *cid);
        let picked: &ImageMetadata = match pos {
            Some(p) if p > 0 => sorted[p - 1],
            _ => sorted.last().unwrap(),
        };
        picked.clone()
    } else {
        images[0].clone()
    };

    do_switch(&state.config, &prev.id).await
}

#[post("/switch/random")]
async fn switch_random(
    state: &State<AlbumState>,
) -> Result<Json<SwitchResponse>, RawJson<String>> {
    let images = storage::list_images();
    if images.is_empty() {
        return Err(err_json("no images"));
    }
    use rand::Rng;
    let idx = rand::rng().random_range(0..images.len());
    do_switch(&state.config, &images[idx].id).await
}

#[delete("/image/<id>")]
fn delete_image(
    id: &str,
    state: &State<AlbumState>,
) -> Result<Json<SwitchResponse>, RawJson<String>> {
    storage::delete_image(&state.config, id)
        .map_err(|e| err_json(&format!("delete failed: {}", e)))?;
    Ok(Json(SwitchResponse {
        ok: true,
        message: "image deleted".to_string(),
    }))
}

#[get("/device-status")]
fn device_status() -> Json<StatusResponse> {
    let s = discovery::get_device_state();
    Json(StatusResponse {
        ip: s.ip,
        online: s.online,
        status: s.status,
        last_seen: s.last_seen,
    })
}

#[get("/current")]
fn current_image(
    _state: &State<AlbumState>,
) -> Result<Json<serde_json::Value>, RawJson<String>> {
    match storage::get_current().ok().flatten() {
        Some(id) => match storage::get_metadata(&id) {
            Ok(meta) => Ok(Json(serde_json::json!({"ok": true, "image": meta}))),
            Err(_) => Ok(Json(serde_json::json!({
                "ok": true,
                "image": null,
                "message": "current image file missing"
            }))),
        },
        None => Ok(Json(serde_json::json!({"ok": true, "image": null}))),
    }
}

// ---- Helpers ----

async fn extract_form(
    mut form: rocket::form::Form<UploadForm<'_>>,
) -> Result<
    (
        Vec<u8>,
        ConversionParams,
        Option<String>,
        Option<String>,
    ),
    RawJson<String>,
> {
    let original_filename = form
        .file
        .raw_name()
        .map(|n| n.dangerous_unsafe_unsanitized_raw().to_string());

    let mime_type = form
        .file
        .content_type()
        .map(|ct| ct.to_string());

    // Read file into memory
    let path = form.file.path().ok_or_else(|| err_json("no file path"))?;
    let file_data = tokio::fs::read(path)
        .await
        .map_err(|e| err_json(&format!("failed to read upload: {}", e)))?;

    let params = ConversionParams {
        crop_mode: form
            .crop_mode
            .take()
            .and_then(|s| CropAnchor::from_str(&s)),
        dither: form.dither.unwrap_or(true),
        rotate_cw: form.rotate_cw.unwrap_or(false),
        rotate_ccw: form.rotate_ccw.unwrap_or(false),
        invert: form.invert.unwrap_or(false),
        ..Default::default()
    };

    Ok((file_data, params, original_filename, mime_type))
}

fn err_json(msg: &str) -> RawJson<String> {
    RawJson(serde_json::json!({"ok": false, "message": msg}).to_string())
}

async fn do_switch(
    config: &AlbumConfig,
    id: &str,
) -> Result<Json<SwitchResponse>, RawJson<String>> {
    let raw = storage::load_raw_binary(config, id)
        .map_err(|e| err_json(&format!("image not found: {}", e)))?;

    sender::send_to_device(config, &raw)
        .await
        .map_err(|e| err_json(&format!("send failed: {}", e)))?;

    storage::set_current(id).ok();

    Ok(Json(SwitchResponse {
        ok: true,
        message: format!("switched to {}", id),
    }))
}
