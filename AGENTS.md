# AGENTS.md

This file provides guidance to coding agents (Claude Code / Codex / Hermes, …) when working with code in this repository.

## Build & Development

```sh
# Build everything (UI + server)
make all

# Build with cross-compilation target
TARGET=aarch64-unknown-linux-musl make -j server

# Build UI only
make ui

# Package into a tarball (after building)
TARGET=aarch64-unknown-linux-musl make -j package

# Update dependencies
make update

# Run the Rocket server (for development)
cd server && cargo run --release

# Run the Vite dev server (for UI development)
cd ui && yarn dev
```

## Architecture

Monorepo with two top-level components: a **Rust Rocket web server** (`server/`) and a **Vite + Vue 3 SPA** (`ui/`).

### Server (`server/`)

A Rocket (v0.5) web server organized as a Cargo workspace. The main binary at `server/src/main.rs` uses a `Figment`-based config merging chain: `Rocket.toml` → nested overrides via `local` field → profile selection. Each module is feature-gated via `switches.*` in config and conditionally mounted.

**Workspace crates** (under `server/crates/`):

- **bark** — iOS push notifications via [Bark](https://github.com/finb/bark) service. Sends HTTP POST with JSON payload.
- **inbox** — Pastebin with file/text upload, expiration, 4-digit code retrieval. Sled-based metadata storage.
- **kindle** — Kindle screensaver generator. Renders grayscale dashboards (date, weather, battery) as `GrayImage`. Three style variants (`alpha`, `bravo`, `charlie`).
- **let_server_run** — WeChat bot agent powered by [LetServerRun](https://letserver.run/). Long-polling job loop with extensible executors (echo, shell).
- **meme** — Personal media gallery with S3-backed storage, JWT auth via external gate, image processing (thumbnails, dithering for e-ink, video thumbnails via ffmpeg).
- **qweather** — Weather forecast fetcher, powered by [QWeather API](https://dev.qweather.com/). Cron-driven periodic updates.
- **recipe** — Shared meal ordering + step-guided cooking + online recipe editor. Markdown-based recipe storage with YAML frontmatter (Chinese `name` as unique key), per-dish portion scaling, ingredient aggregation, polling-based multi-device sync. S3-backed storage with push-then-sync write model.
- **tsdb** — Thin InfluxDB2 wrapper for time-series metrics.
- **utils** — Shared utilities: sled-backed key-value store (`database::Db`), configurable data path.

### UI (`ui/`)

Vite 5 + Vue 3 + TypeScript + Tailwind CSS + daisyUI. Multi-page app with three entry points (defined in `vite.config.ts`):

- **inbox** — `/inbox/index.html`, pastebin frontend
- **meme** — `/meme/index.html`, media gallery with waterfall/gallery/tag-cloud views
- **kindle** — `/kindle/debug/index.html`, Kindle dashboard debug preview
- **recipe** — `/recipe/index.html`, meal ordering (MenuPage) + step-guided cooking (CookingPage) with caramellatte daisyUI theme

Dev server proxies `/inbox/api`, `/meme/i`, `/album/api`, and `/recipe/api` to configurable backends. Proxy context strings must NOT include `^` prefix — it would be treated as a literal character by http-proxy-middleware, breaking path matching.

### CI/CD

One workflow does everything: `.github/workflows/build.yml` cross-compiles for `aarch64-unknown-linux-musl` and `x86_64-unknown-linux-musl`, uploads artifacts on branch pushes, and publishes a GitHub release on `v*` tags (same run, dispatched by `github.ref`).

A tag push must cost exactly one workflow run (2 jobs, one per target). Two traps already fixed once — don't reintroduce them:

- Splitting the release path back into a second workflow with a bare `on: push:` makes every tag push run the whole matrix twice.
- Adding a `make` step before `make package` recompiles UI + Rust: `package` already depends on `all` (`ui server configure`).

Also don't install `rsign2` in CI — only `make sign` uses it. `VITE_HODOR_ENTRY` is a build-time secret for meme auth.

### Nav page (`/`)

`ui/index.html` + `ui/nav/`（Vite 入口，和其它页面一样走 `yarn build`）：项目根 html 构建成 `dist/index.html`，被 Rocket 的 `FileServer`（`Options::Index`，rank 999）服务在 `/`。样式（`nav/p5.css`）和示例配置都在 `nav/app.js` 里 `import`，生产构建走 esbuild 压缩 + CSS 压缩 + 文件名内容哈希（`assets/xxx-<hash>.{js,css,woff2}`）；所以它不再是 `public/` 里的原样拷贝。

链接列表不进仓库，由服务端直接吐：

- `server/src/nav.rs` 提供 `GET /nav/links.json`，读配置 `nav.links` 指定的文件；相对路径按 `data_path` 解析（`links = "nav/links.json"` + `data_path = "/data"` ⇒ `/data/nav/links.json`），文件不存在返回 404。
- 页面启动时 `fetch('/nav/links.json', {cache:'no-store'})`；取不到就用**编译进 bundle 的**示例配置（`ui/nav/links.example.json` 在构建时被 import，不再单独发请求），不会白屏。
- 部署：把 `links.json` 放到实例数据目录（如 `~/ws/data/<实例>/nav/links.json`），并在 `Local.toml` 里写 `[default.nav]` / `[release.nav]` 的 `links`（默认值见 `server/Rocket.toml`）。配置文件放 data 目录，`app/update.sh` 升级不会动它。
- 字段：`title` / `subtitle` / `watermark` / `newTab` / `theme` / `subTheme` / `links[...]`。条目带非空 `items` 就是二级菜单入口（子层可再套 `items` 做三级），叶子字段是 `{name,url,desc,icon,tag,accent}`；`icon` 支持 bowl|shield|server|globe|lock|film|note|star|spark|git|home，也可直接写 emoji。
- 主题按「页」生效：`theme` 管主菜单、`subTheme` 管二级及以下（留空 = 跟 theme 同款），可选 `"p5"`（怪盗红 + 尖刺星）/ `"p3"`（深蓝水面 + 涟漪标记）。**同一页所有条目都用该页主题色**（包括进子菜单的那一项）；条目级 `accent` 会覆盖它，跨主题写死颜色会让那条看起来“跑到了别的主题”，一般别写。切主题时先用斜条擦过整屏再换配色（`.flash` 元素由 JS 建）。
- `newTab`：`false`（推荐）= 同标签页打开，点击先播擦除动画再跳转；`true` = 交给浏览器开新标签（擦除动画就播不出来）。
- ℹ️ release 二进制由本仓库 CI 从 `v*` tag 构建，本身就含 `nav` 路由，所以 `app/update.sh` 升级后 `/nav/links.json` 照常可用；只有换成不含该路由的第三方包时才会重新 404（页面退回示例配置）。

### Packaging

`make package` bundles `Rocket.toml`, the UI `dist/`, and the server binary into a tarball. Optional signing via `rsign2`.

### Configuration

`Rocket.toml` at the server root is the primary config. A `local` field can point to an override file (e.g., `Local.toml`, `Debug.toml`). The server merges these and selects the profile section (`debug` / `release`). Data directory defaults to `data/`.

### Key patterns

- Modules expose a `build(base, rocket, config) -> Rocket<Build>` function that registers routes under a base path
- `OnceCell`/`OnceLock` for lazy static initialization from config throughout
- `sled` for embedded key-value persistence (inbox metadata, launch timestamps)
- S3 (S3-compatible object store) for meme media storage
- Version string generated at build time via `build.rs` using NATO phonetic alphabet suffixes
- **Recipe ingredients** support nesting via `sub_ingredients` field — compound items (e.g. "浓盐葱姜水" → [盐, 葱, 姜, 水]) display as grouped cards in UI, with sub-ingredients scaled and aggregated independently
- **Recipe menus** support `custom_dishes` — freeform dishes added directly to the order without a backing recipe file. Stored in menu frontmatter YAML, displayed with portion controls in summary and cooking views
- **MQTT** is gated by `switches.mqtt` in config; when disabled, `mqtt::subscribe()` is a silent no-op (publish functions already were). Debug.toml sets it off by default
- **Recipe editing** — full online editing for recipe markdown and cover images:
  - `name` (Chinese dish name) is the sole unique identifier — no separate `id` field; old `id:` fields map via YAML preprocessing in `markdown::parse_recipe` and `menus::parse_menu_yaml`
  - API: `GET /recipes/<name>/raw`, `PUT /recipes/<name>/raw`, `POST /recipes/<name>/image`
  - Editor: `md-editor-v3` fullscreen overlay, launched from detail modal "✏️ 编辑此菜谱" or drawer "📝 新建菜谱"; cover image via separate modal dialog
  - Write model: **push-then-sync** — `save_and_sync()` pushes to S3, then `manual_sync_and_reload()` pulls back to keep local cache and index consistent; S3 unavailable = save rejected
