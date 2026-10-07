//! 导航页（PHANTOM NAVI）的链接配置
//!
//! 页面本体是 `ui/public/index.html` + `ui/public/nav/`（纯静态，`/` 路径由 FileServer 服务），
//! 但链接列表不走仓库：由本模块用 `GET /nav/links.json` 直接吐出来。
//!
//! 文件路径来自配置 `nav.links`：
//!   - 相对路径 → 相对 `data_path`（例：`links = "nav/links.json"` + `data_path = "/data"` ⇒ `/data/nav/links.json`）
//!   - 也可以直接写绝对路径
//! 文件不存在时返回 404，前端会退回去读 `/nav/links.example.json`（仓库内示例）。

use std::path::{Path, PathBuf};

use log::{info, warn};
use rocket::figment::Figment;
use rocket::http::ContentType;
use rocket::{Build, Rocket, State};

pub struct NavLinks {
    path: PathBuf,
}

#[get("/nav/links.json")]
async fn links(config: &State<NavLinks>) -> Option<(ContentType, String)> {
    match tokio::fs::read_to_string(&config.path).await {
        Ok(body) => Some((ContentType::JSON, body)),
        Err(err) => {
            warn!("nav links {:?} is unavailable: {}", config.path, err);
            None
        }
    }
}

pub fn build(build: Rocket<Build>, config: &Figment) -> Rocket<Build> {
    let data_path = config
        .find_value("data_path")
        .ok()
        .and_then(|value| value.into_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "data".to_string());

    let configured = config
        .find_value("nav.links")
        .ok()
        .and_then(|value| value.into_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "nav/links.json".to_string());

    let path = Path::new(&configured);
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        Path::new(&data_path).join(path)
    };

    info!("nav links file: {:?}", path);

    build.manage(NavLinks { path }).mount("/", routes![links])
}
