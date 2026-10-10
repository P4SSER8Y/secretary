# server — 架构、各模块 crate、关键模式

> 根 [`AGENTS.md`](../AGENTS.md) 的分册，只在 `server/` 下干活时需要读。

## Server (`server/`)

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

（静态页面的服务方式见 [`../ui/AGENTS.md`](../ui/AGENTS.md)；导航页的路由 `server/src/nav.rs` 见 [`../ui/nav/AGENTS.md`](../ui/nav/AGENTS.md)。）

## Key patterns

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
