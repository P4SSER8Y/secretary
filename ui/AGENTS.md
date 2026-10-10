# ui — 入口、开发代理、构建产物

> 根 [`AGENTS.md`](../AGENTS.md) 的分册。导航页（挂在 `/` 的首页）单独一篇：[`nav/AGENTS.md`](nav/AGENTS.md)。

Vite 5 + Vue 3 + TypeScript + Tailwind CSS + daisyUI。Multi-page app with these entry points (defined in `vite.config.ts`):

- **nav** — the project-root `index.html` + `ui/nav/`, the P5-style start page served at `/`（细节见 [`nav/AGENTS.md`](nav/AGENTS.md)）
- **inbox** — `/inbox/index.html`, pastebin frontend
- **meme** — `/meme/index.html`, media gallery with waterfall/gallery/tag-cloud views
- **kindle** — `/kindle/debug/index.html`, Kindle dashboard debug preview
- **recipe** — `/recipe/index.html`, meal ordering (MenuPage) + step-guided cooking (CookingPage) with caramellatte daisyUI theme

Dev server proxies `/inbox/api`, `/meme/i`, `/album/api`, and `/recipe/api` to configurable backends. Proxy context strings must NOT include `^` prefix — it would be treated as a literal character by http-proxy-middleware, breaking path matching.

构建产物是 `ui/dist/`（由 `make ui` / `yarn build` 生成，含 `index.html`、各入口 html、`assets/<名>-<hash>.*`）。

- 放 `ui/public/` 的东西**原样拷进 `dist/` 根**（不改名、不压缩）—— 只有必须使用固定 URL 的文件才放那里（目前是导航页的 `sw.js` / `manifest.json` / 几个图标 favicon，详见 [`nav/AGENTS.md`](nav/AGENTS.md)）。
- 其余资源一律走 Vite 的处理链（内容哈希 + 压缩），页面里用**绝对路径**（`/assets/...`）。
- server 端由 Rocket 的 `FileServer`（`Options::Index`，rank 999）托管 `ui/` 目录，所以 `dist/index.html` 就是 `/`。
