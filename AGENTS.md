# AGENTS.md

This file provides guidance to coding agents (Claude Code / Codex / Hermes, …) when working with code in this repository.

它是**索引 + 公共说明**：各模块的细节拆进了就近的子文件（见下面的索引表），在对应目录里干活时会被自动读到。改东西前先看这里，再看对应子文件。

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

## Repo layout

| 目录 | 是什么 |
|---|---|
| `server/` | Rust（Rocket v0.5）后端，Cargo workspace，模块按 `switches.*` 开关挂载 |
| `ui/` | Vite 5 + Vue 3 + TS + Tailwind + daisyUI 多入口前端（打包成 `dist/` 交给 server 静态托管） |
| `scripts/` | 辅助脚本 |

## 细节索引

| 领域 | 看哪 |
|---|---|
| server 架构、各模块 crate（meme/recipe/kindle/…）、关键模式 | [`server/AGENTS.md`](server/AGENTS.md) |
| UI 入口、开发代理、构建产物 | [`ui/AGENTS.md`](ui/AGENTS.md) |
| **导航页**（挂在 `/`）：`links.json` 契约、主题、图标、Service Worker、PWA | [`ui/nav/AGENTS.md`](ui/nav/AGENTS.md) |
| CI/CD、打包、配置加载 | 本文件（下面三节） |

## CI/CD

One workflow does everything: `.github/workflows/build.yml` cross-compiles for `aarch64-unknown-linux-musl` and `x86_64-unknown-linux-musl`, uploads artifacts on branch pushes, and publishes a GitHub release on `v*` tags (same run, dispatched by `github.ref`).

A tag push must cost exactly one workflow run (2 jobs, one per target). Two traps already fixed once — don't reintroduce them:

- Splitting the release path back into a second workflow with a bare `on: push:` makes every tag push run the whole matrix twice.
- Adding a `make` step before `make package` recompiles UI + Rust: `package` already depends on `all` (`ui server configure`).

Also don't install `rsign2` in CI — only `make sign` uses it. `VITE_HODOR_ENTRY` is a build-time secret for meme auth.

## Packaging

`make package` bundles `Rocket.toml`, the UI `dist/`, and the server binary into a tarball. Optional signing via `rsign2`.

## Configuration

`Rocket.toml` at the server root is the primary config. A `local` field can point to an override file (e.g., `Local.toml`, `Debug.toml`). The server merges these and selects the profile section (`debug` / `release`). Data directory defaults to `data/`.
