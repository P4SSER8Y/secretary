# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

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

GitHub Actions cross-compiles for `aarch64-unknown-linux-musl` and `x86_64-unknown-linux-musl`. The `develop` workflow runs on every push; `release` triggers on `v*` tags and creates GitHub releases. `VITE_HODOR_ENTRY` is a build-time secret for meme auth.

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
