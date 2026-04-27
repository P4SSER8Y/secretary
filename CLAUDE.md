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
- **tsdb** — Thin InfluxDB2 wrapper for time-series metrics.
- **utils** — Shared utilities: sled-backed key-value store (`database::Db`), configurable data path.

### UI (`ui/`)

Vite 5 + Vue 3 + TypeScript + Tailwind CSS + daisyUI. Multi-page app with three entry points (defined in `vite.config.ts`):

- **inbox** — `/inbox/index.html`, pastebin frontend
- **meme** — `/meme/index.html`, media gallery with waterfall/gallery/tag-cloud views
- **kindle** — `/kindle/debug/index.html`, Kindle dashboard debug preview

Dev server proxies `/inbox/api` and `/meme/i` to configurable backends.

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
