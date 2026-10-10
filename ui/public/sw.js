/* 导航页的 Service Worker —— 只为缓存，别的模块一律不碰
 *
 * 为什么需要它：Rocket 的 FileServer 不发任何缓存头（无 Cache-Control / ETag /
 * Last-Modified），浏览器没有任何缓存依据 → 实测每次打开都要重下 4.0MB 字体。
 * 服务端是上游二进制改不了，SW 是唯一能在客户端修掉它的手段。
 *
 * 拦截白名单（同 origin 下还有 /recipe/ /meme/ /gate/ /album/ /inbox/ 等模块，不能插手）：
 *   导航请求 /            → network-first（离线回缓存）
 *   /assets/*             → cache-first（文件名带内容哈希，内容即版本）
 *   /nav/links.json       → network-first，带超时（改完配置刷新即生效，离线用上次的）
 *   其它一切              → 不调 respondWith，原样走网络
 *
 * ⚠️ 退路：把 KILL 改成 true 再部署一次 → SW 自注销并清掉所有缓存。
 */
const CACHE = 'nav-v3';
const KILL = false;
const SHELL = '/index.html';       /* 导航结果一律以这个稳定键入缓存（URL 上可能带 ?r= 之类的查询串） */
const TIMEOUT = 3500;              /* network-first 的超时：超过就先用缓存顶住 */
/* 和 /assets/ 一样按 cache-first 处理的固定路径（服务端不发缓存头，不缓存就会每次重下） */
const STATIC = ['/favicon.ico', '/favicon-256.png', '/apple-touch-icon.png'];

self.addEventListener('install', (event) => {
  if (KILL) { self.skipWaiting(); return; }
  event.waitUntil((async () => {
    const cache = await caches.open(CACHE);
    try {
      const html = await fetch(SHELL, { cache: 'no-store' });
      if (html.ok) await cache.put(SHELL, html);
      /* 只预缓存 HTML 本体：实测在 install 里批量预取 /assets/*、/favicon* 会静默失败
         （只有 SHELL 写进了缓存，后面的都没跑，连 catch 都不报），所以固定图标改由页面
         在 SW 接管后预热（见 nav/app.js），/assets/* 则在第二次受控访问时自然入缓存。 */
    } catch (e) { /* 安装时离线：跳过预缓存，运行时再补 */ }
    await self.skipWaiting();
  })());
});

self.addEventListener('activate', (event) => {
  event.waitUntil((async () => {
    if (KILL) {
      const keys = await caches.keys();
      await Promise.all(keys.map((k) => caches.delete(k)));
      await self.registration.unregister();
      return;
    }
    const keys = await caches.keys();
    await Promise.all(keys.filter((k) => k !== CACHE).map((k) => caches.delete(k)));
    await self.clients.claim();
  })());
});

async function networkFirst(req, key) {
  const cache = await caches.open(CACHE);
  try {
    const ctrl = new AbortController();
    const timer = setTimeout(() => ctrl.abort(), TIMEOUT);
    const res = await fetch(req, { signal: ctrl.signal });
    clearTimeout(timer);
    if (res && res.ok) cache.put(key, res.clone());
    return res;
  } catch (e) {
    const hit = await cache.match(key);
    if (hit) return hit;
    throw e;
  }
}

async function cacheFirst(req) {
  const cache = await caches.open(CACHE);
  const hit = await cache.match(req);
  if (hit) return hit;
  const res = await fetch(req);
  if (res && res.ok) cache.put(req, res.clone());
  return res;
}

self.addEventListener('fetch', (event) => {
  if (KILL) return;
  const req = event.request;
  if (req.method !== 'GET') return;
  let url;
  try { url = new URL(req.url); } catch (e) { return; }
  if (url.origin !== self.location.origin) return;      /* 跨域一律不碰 */
  if (url.searchParams.has('nosw')) return;             /* 手动绕过：加 ?nosw=1 */

  if (req.mode === 'navigate' && (url.pathname === '/' || url.pathname === '/index.html')) {
    /* 只拦导航页自己的导航请求：/recipe/ /meme/ 等页面的导航原样走网络，
       否则会把它们的响应缓存到 SHELL 键上，污染离线时的首页。 */
    event.respondWith(networkFirst(req, SHELL).catch(async () => {
      const cache = await caches.open(CACHE);
      return (await cache.match(SHELL)) || Response.error();
    }));
    return;
  }

  if (url.pathname.startsWith('/assets/') || STATIC.indexOf(url.pathname) >= 0) {
    event.respondWith(cacheFirst(req));                 /* 带哈希的资源 + 固定图标：长期缓存 */
    return;
  }

  if (url.pathname === '/nav/links.json') {             /* 导航配置：改了要立刻生效，离线用旧的 */
    event.respondWith(networkFirst(req, req));
    return;
  }

  /* 其它路径（/recipe/ /meme/ /gate/ /album/ /inbox/ 以及各模块 API）：不拦，原样走网络 */
});
