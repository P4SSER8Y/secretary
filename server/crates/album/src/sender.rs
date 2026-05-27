use base64::Engine;

use crate::config::AlbumConfig;
use crate::discovery;

pub async fn send_to_device(cfg: &AlbumConfig, raw_binary: &[u8]) -> anyhow::Result<()> {
    let state = discovery::get_device_state();
    let ip = state.ip.ok_or(anyhow::anyhow!("device IP unknown"))?;
    if !state.online {
        log::warn!("device appears offline, attempting send anyway");
    }

    let url = cfg.device_http_endpoint.replace("{ip}", &ip);
    let b64 = base64::engine::general_purpose::STANDARD.encode(raw_binary);

    log::info!("sending {} bytes (base64: {} chars) to {}", raw_binary.len(), b64.len(), url);

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Content-Type", "text/plain")
        .body(b64)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("device returned {} {}: {}", status.as_u16(), status, body));
    }
    log::info!("image sent successfully");
    Ok(())
}
