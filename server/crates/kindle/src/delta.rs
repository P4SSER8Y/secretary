use super::shared::*;
use anyhow::{anyhow, Result};
use image::DynamicImage;
use log::info;

pub async fn generate(context: &Context) -> Result<DynamicImage> {
    let path = context
        .dashboard_path
        .as_deref()
        .ok_or(anyhow!("dashboard_path not provided"))?;

    info!("loading dashboard from: {}", path);
    let img = image::open(path)
        .map_err(|e| anyhow!("failed to load dashboard image: {}", e))?;

    Ok(img)
}
