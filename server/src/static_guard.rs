//! 未使能模块的静态页面同样 404
//!
//! `ui_path` 指向的整个 ui/ 目录被 `FileServer` 挂在 `/`（rank 999），它与 `switches.*`
//! 是两条互不相干的线：关掉某模块只是不挂它的 API 路由（`/meme/i/*` 之类），
//! `ui/<模块>/index.html` 依旧照发 200 —— 于是没使能的模块也能打开页面，页面里所有
//! 接口全是 404，看起来像是坏了。
//!
//! 这里不动 ui/ 目录、也不改 `FileServer` 的行为：把它包一层 [`Handler`]，
//! 请求的第一段路径命中「已关闭模块」就直接 404，其余原样转给 `FileServer`。
//! 判断用的是与 `FileServer` 相同的解码后路径段（并同样处理 `..`）：`/meme/`、
//! `/meme/index.html`、`/meme/raven.html`、`/me%6De/` 都会 404，而 `/meme/../recipe/`
//! 按 `FileServer` 的解析理解成 `recipe`。

use rocket::fs::{FileServer, Options};
use rocket::http::{Method, Status};
use rocket::route::{Handler, Outcome, Route};
use rocket::{Data, Request};

/// 有自带静态页面的模块：数组里的名字 = `switches` 里的开关名 = `ui/` 下的目录名
/// （也是 URL 的第一段）。weather / tsdb / let_server_run / bark / mqtt 没有静态页，不在此列。
pub const STATIC_MODULES: [&'static str; 6] = ["album", "gate", "inbox", "kindle", "meme", "recipe"];

/// 与 main.rs 里原本的 `FileServer` 一致的优先级：低于所有模块路由，高于 404。
const RANK: isize = 999;

/// `switches` 里的模块开关：名字 + 是否使能
pub type Switch<'a> = (&'a str, bool);

#[derive(Clone)]
struct GuardedFiles {
    inner: FileServer,
    disabled: Vec<&'static str>,
}

#[rocket::async_trait]
impl Handler for GuardedFiles {
    async fn handle<'r>(&self, req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r> {
        // 与 FileServer 一致：解码后的路径段，`..` 往回退
        let mut segments: Vec<&str> = Vec::new();
        for segment in req.uri().path().segments() {
            if segment == ".." {
                segments.pop();
            } else {
                segments.push(segment);
            }
        }
        if let Some(first) = segments.first() {
            if let Some(module) = self.disabled.iter().copied().find(|module| *module == *first) {
                log::warn!("{}: module {} is disabled, static page blocked", req.uri(), module);
                return Outcome::error(Status::NotFound);
            }
        }
        self.inner.handle(req, data).await
    }
}

impl From<GuardedFiles> for Vec<Route> {
    fn from(server: GuardedFiles) -> Self {
        let mut route = Route::ranked(RANK, Method::Get, "/<path..>", server);
        route.name = Some("Guarded FileServer".into());
        vec![route]
    }
}

/// 用 `ui_path` 建静态页服务：`switches` 里关掉的模块（`STATIC_MODULES` 范围内）整段 404。
pub fn build(ui_path: &str, switches: &[Switch<'_>]) -> Vec<Route> {
    // 没配的开关按关闭处理，与 main.rs 的 `is_enabled(.., false)` 一致
    let enabled = |module: &str| {
        switches
            .iter()
            .any(|(name, enabled)| *name == module && *enabled)
    };
    let disabled: Vec<&'static str> = STATIC_MODULES
        .into_iter()
        .filter(|module| !enabled(module))
        .collect();

    if disabled.is_empty() {
        log::info!("static pages of all modules are served");
    } else {
        log::info!("static pages of disabled modules will 404: {:?}", disabled);
    }

    let inner = FileServer::new(ui_path, Options::Index | Options::NormalizeDirs).rank(RANK);
    GuardedFiles { inner, disabled }.into()
}
