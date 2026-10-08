/* ==========================================================================
 * PHANTOM NAVI — 导航页（secretary 首页，挂在 / 路径）
 *
 * 这个文件是 Vite 的入口（对应的 html 是 ui/index.html → dist/index.html）：
 * 样式、示例配置都在这里 import，构建时由 Vite 打包 + esbuild 压缩 + 内容哈希。
 *
 * 配置是动态的：启动时拉 /nav/links.json（不在仓库里，跟部署走 —— 由服务端
 * nav.links 配置指向，默认 data_path 下的 nav/links.json）；拉不到就用编译进来的
 * 示例配置（nav/links.example.json），页面不会白屏。
 *
 * 条目两种形态：带非空 items 的 = 分组（二级菜单入口），只有 url 的 = 普通链接。
 * 层级用栈实现，所以 items 里再套 items 就是三级。
 * ========================================================================== */
import './p5.css';
import LINKS_EXAMPLE from './links.example.json';

(function () {
  'use strict';

  var CONFIG_URL = '/nav/links.json';
  var FALLBACK = LINKS_EXAMPLE;
  var DEFAULTS = {
    title: 'NAVI',
    subtitle: '',
    watermark: 'Take Your Heart',
    newTab: false,    /* false = 同标签页打开（先播擦除再跳）；true = 交给浏览器开新标签 */
    theme: 'p5',      /* 主菜单主题 */
    subTheme: '',     /* 二级及以下主题；留空 = 跟 theme 同款 */
    links: []
  };

  function fetchJson(url) {
    return fetch(url, { cache: 'no-store' }).then(function (r) {
      if (!r.ok) throw new Error(url + ' -> HTTP ' + r.status);
      return r.json();
    });
  }

  /* ----------------------------------------------------------- 图标 */
  var ICON_PATHS = {
    bowl:   '<path d="M2.6 11.2h18.8c0 5.2-4.2 8.6-9.4 8.6s-9.4-3.4-9.4-8.6Z"/><path d="M7.4 7.8c0-1.5 1.3-1.7 1.3-3.1M12 7c0-1.7 1.4-2 1.4-3.7M16.6 7.8c0-1.5 1.3-1.7 1.3-3.1"/>',
    shield: '<path d="M12 2.6 20 6v6.1c0 4.9-3.4 8.4-8 9.3-4.6-.9-8-4.4-8-9.3V6l8-3.4Z"/><path d="M8.4 12.2 11 14.9l4.7-5.3"/>',
    server: '<rect x="3" y="4" width="18" height="7" rx="1.6"/><rect x="3" y="13" width="18" height="7" rx="1.6"/><path d="M7 7.5h.01M7 16.5h.01"/>',
    globe:  '<circle cx="12" cy="12" r="9.2"/><path d="M2.8 12h18.4M12 2.8c2.6 3.1 2.6 15.3 0 18.4M12 2.8c-2.6 3.1-2.6 15.3 0 18.4"/>',
    lock:   '<rect x="4.4" y="10.4" width="15.2" height="10.6" rx="2.2"/><path d="M7.9 10.4V7.5a4.1 4.1 0 0 1 8.2 0v2.9"/><path d="M12 14.6v2.4"/>',
    film:   '<rect x="3" y="4.6" width="18" height="14.8" rx="2.2"/><path d="M8 4.6v14.8M16 4.6v14.8M3 9.6h18M3 14.4h18"/>',
    note:   '<path d="M5 3.6h9l5 5v11.8H5z"/><path d="M14 3.6v5h5"/><path d="M8.4 13h7.2M8.4 16.4h4.2"/>',
    star:   '<path d="M12 3.4 14.4 9.4h6.4l-5.1 3.9 1.9 6.3L12 16l-5.6 3.6 1.9-6.3L3.2 9.4h6.4Z"/>',
    spark:  '<path d="M12 3v3.6M12 17.4V21M3 12h3.6M17.4 12H21M6 6l2.6 2.6M15.4 15.4 18 18M18 6l-2.6 2.6M8.6 15.4 6 18"/>',
    git:    '<circle cx="6.6" cy="6.6" r="2.4"/><circle cx="17.4" cy="6.6" r="2.4"/><path d="M6.6 9v4.2c0 2.4 1.9 4.3 4.3 4.3h6.5"/><path d="M17.4 9v3.4"/>',
    home:   '<path d="M3.4 10.6 12 3.6l8.6 7"/><path d="M5.6 9.9V20.4h12.8V9.9"/><path d="M9.7 20.4v-5.5h4.6v5.5"/>'
  };
  var STAR_SVG =
    '<svg viewBox="0 0 100 100" aria-hidden="true">' +
    '<path d="M50 2 57.6 36.2 87 19 66.4 46.9 98 50 66.4 53.1 87 81 57.6 63.8 50 98 42.4 63.8 13 81 33.6 53.1 2 50 33.6 46.9 13 19 42.4 36.2Z" ' +
    'fill="currentColor" stroke="#fff" stroke-width="2.5" stroke-linejoin="miter"/></svg>';
  /* P3 的选中标记：CSS 画的水面涟漪（见 .star .ripple），不引额外素材 */
  var RIPPLE_MARKUP = '<span class="ripple"></span>';

  function iconMarkup(icon) {
    if (!icon) return '';
    if (ICON_PATHS[icon]) {
      return '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" ' +
             'stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">' + ICON_PATHS[icon] + '</svg>';
    }
    return '<span style="font-size:.92em;line-height:1">' + icon + '</span>';
  }

  /* 分组：带非空 items 的条目就是二级菜单的入口；叶子是普通链接。
     两种可以混着放，顺序照配置来。 */
  function isGroup(l) { return !!(l && l.items && l.items.length); }
  function usable(l) { return !!l && (!!l.url || isGroup(l)); }

  /* ----------------------------------------------------------- 主题色表 */
  /* 每个主题一组：斜条擦除用的主色 + 暗色 */
  var THEME_COLORS = {
    p5: { main: '#e60012', ink: '#0a0a0e' },
    p3: { main: '#1a5fd8', ink: '#050a18' }
  };
  /* 选中标记（星 / 涟漪）：键同时用来判断主题名是否合法 */
  var MARK = { p5: STAR_SVG, p3: RIPPLE_MARKUP };

  /* ----------------------------------------------------------- 主流程 */
  function main(raw) {
    var CFG = Object.assign({}, DEFAULTS, raw || {});
    var links = (CFG.links || []).filter(usable);

    var menuEl = document.getElementById('menu');
    var crumbEl = document.getElementById('crumb');
    var crumbTitleEl = document.getElementById('crumbTitle');
    var crumbPathEl = document.getElementById('crumbPath');
    var backBtn = document.getElementById('crumbBack');

    /* 层级栈：depth = 0 是主菜单；进二级就把子项数组压栈，返回时弹栈。
       用栈而不是写死两层，所以想套三层也能直接用。 */
    var stack = [links];   /* 每一层的条目数组 */
    var titles = [];       /* 每一层的分组名（面包屑用） */
    var origins = [];      /* 从哪个父项进来的（返回时恢复选中） */
    var depth = 0;
    var rows = [];         /* 当前层的 DOM（行） */
    var pos = -1;          /* 当前选中项下标 */
    var busy = false;      /* 进/出层的动画期间锁住输入 */

    function pad2(n) { return String(n).padStart(2, '0'); }

    /* ------------------------------------------------- 主题（P5 / P3 切换）
       主菜单用 theme，二级及以下用 subTheme。切主题时先用斜条擦过屏幕，
       盖住之后才换配色，避免整页颜色硬切。 */
    var flashEl = document.createElement('div');
    flashEl.className = 'flash';
    flashEl.setAttribute('aria-hidden', 'true');
    for (var fi = 0; fi < 5; fi++) flashEl.appendChild(document.createElement('span'));
    document.body.appendChild(flashEl);

    var themeNow = null;
    var enterDelay = 0;    /* 主题擦除期间，新行要等它盖上来再入场 */

    function themeFor(level) {
      var want = (level === 0)
        ? (CFG.theme || DEFAULTS.theme)
        : (CFG.subTheme || CFG.theme || DEFAULTS.theme);
      return MARK[want] ? want : DEFAULTS.theme;
    }

    function applyTheme(level) {
      var want = themeFor(level);
      enterDelay = 0;
      if (themeNow === want) return want;
      if (themeNow === null) {                       /* 首次：直接定色，不闪 */
        themeNow = want;
        document.documentElement.setAttribute('data-theme', want);
        return want;
      }
      var col = THEME_COLORS[want] || THEME_COLORS.p5;
      flashEl.style.setProperty('--flash-c', col.main);
      flashEl.style.setProperty('--flash-ink', col.ink);
      flashEl.classList.remove('play');
      void flashEl.offsetWidth;
      flashEl.classList.add('play');
      themeNow = want;
      enterDelay = 220;
      setTimeout(function () { document.documentElement.setAttribute('data-theme', want); }, 230);
      return want;
    }

    function buildRows(list, level) {
      var th = applyTheme(level);
      var mark = MARK[th] || STAR_SVG;
      var frag = document.createDocumentFragment();
      rows = [];
      list.forEach(function (link, i) {
        var grp = isGroup(link);
        var kids = grp ? link.items.filter(usable) : [];
        var li = document.createElement('li');
        li.className = 'row';

        var a = document.createElement('a');
        a.className = 'item' + (grp ? ' grp' : '') + (level > 0 ? ' kid' : '');
        if (grp) {
          /* 分组不是链接：右键新标签页打开这种操作对它没意义 */
          a.setAttribute('role', 'button');
          a.setAttribute('aria-haspopup', 'true');
          a.setAttribute('tabindex', '0');
        } else {
          a.href = link.url;
          if (CFG.newTab !== false) { a.target = '_blank'; a.rel = 'noopener noreferrer'; }
        }
        if (link.accent) a.style.setProperty('--accent', link.accent);
        a.setAttribute('aria-label', (link.name || '') + (link.desc ? ' — ' + link.desc : '') + (grp ? '（子菜单）' : ''));

        a.innerHTML =
          '<span class="star" aria-hidden="true">' + mark + '</span>' +
          '<span class="poly p-white"></span>' +
          '<span class="poly p-red"></span>' +
          '<span class="poly p-ink"></span>' +
          '<span class="idx">' + pad2(i + 1) + '</span>' +
          '<span class="ic">' + iconMarkup(link.icon) + '</span>' +
          '<span class="txt">' +
            '<span class="name">' + (link.name || '') + '</span>' +
            '<span class="desc">' + (link.desc || link.tag || '') + '</span>' +
          '</span>' +
          ((link.tag) ? '<span class="tag">' + link.tag + '</span>' : '') +
          (grp ? '<span class="cnt">' + pad2(kids.length) + '</span>' : '') +
          '<span class="go" aria-hidden="true">' + (grp ? '»' : '▶') + '</span>';

        a.addEventListener('mouseenter', function () { select(i); });
        a.addEventListener('focus', function () { select(i); });
        a.addEventListener('click', function (ev) {
          if (grp) { ev.preventDefault(); enter(i); return; }
          launch(link, ev);
        });

        li.appendChild(a);
        frag.appendChild(li);
        rows.push(a);
      });

      menuEl.innerHTML = '';
      menuEl.classList.toggle('sub', level > 0);
      menuEl.appendChild(frag);

      if (!rows.length) {
        var hint = document.createElement('li');
        hint.className = 'row';
        hint.innerHTML = '<div class="empty">' + (level > 0
          ? '这个分组里没有条目'
          : '没有导航项 —— 部署时提供 /nav/links.json（服务端配置 nav.links 指向，默认 data_path/nav/links.json）') + '</div>';
        menuEl.appendChild(hint);
      }
      pos = -1;
    }

    function select(i) {
      if (i < 0 || i >= rows.length || i === pos) return;
      pos = i;
      rows.forEach(function (el, k) {
        el.classList.toggle('on', k === i);
        if (k === i) {
          el.classList.remove('wob');
          void el.offsetWidth;
          el.classList.add('wob');
        }
      });
    }

    /* 出层：行错峰向左飘走（.out），返回「全部飘完约需多久」 */
    function staggerOut(list, step) {
      list.forEach(function (el, k) {
        setTimeout(function () {
          el.classList.remove('on', 'wob', 'in');
          el.classList.add('out');
        }, k * step);
      });
      return 150 + list.length * step;
    }
    function staggerIn(list, step, base) {
      list.forEach(function (el, k) {
        setTimeout(function () { el.classList.add('in'); }, base + k * step);
      });
    }

    /* 面包屑：depth = 0 时收起来 */
    function setCrumb() {
      if (depth === 0) { crumbEl.classList.remove('on'); return; }
      crumbTitleEl.textContent = titles[titles.length - 1] || '';
      crumbPathEl.textContent = String(CFG.title || DEFAULTS.title) +
        ' / ' + titles.join(' / ') + ' · ' + pad2(rows.length);
      crumbEl.classList.add('on');
    }

    /* 进二级：父项让位 → 子项从右侧滑入，首项自动选中 */
    function enter(i) {
      if (busy || depth !== stack.length - 1) return;
      var g = stack[depth][i];
      if (!isGroup(g)) return;
      var kids = g.items.filter(usable);
      if (!kids.length) return;
      busy = true;
      var wait = staggerOut(rows, 26);
      setTimeout(function () {
        origins.push(i);
        titles.push(g.name || '');
        stack.push(kids);
        depth += 1;
        buildRows(kids, depth);
        setCrumb();
        var base = 40 + enterDelay;
        staggerIn(rows, 68, base);
        setTimeout(function () {
          select(0);
          busy = false;
        }, base + rows.length * 68 + 90);
      }, wait);
    }

    /* 返回上一层：恢复进层前选中的那个父项 */
    function back() {
      if (busy || depth === 0) return;
      busy = true;
      var from = origins[origins.length - 1];
      var wait = staggerOut(rows, 22);
      setTimeout(function () {
        origins.pop();
        titles.pop();
        stack.pop();
        depth -= 1;
        buildRows(stack[depth], depth);
        setCrumb();
        var base = 30 + enterDelay;
        staggerIn(rows, 68, base);
        setTimeout(function () {
          select((from >= 0 && from < rows.length) ? from : 0);
          busy = false;
        }, base + rows.length * 68 + 90);
      }, wait);
    }

    if (backBtn) backBtn.addEventListener('click', back);

    /* --------------------------------------------------------- 擦除 / 爆闪 */
    var wipeEl = document.getElementById('wipe');
    function playWipe() {
      wipeEl.classList.remove('play');
      void wipeEl.offsetWidth;
      wipeEl.classList.add('play');
    }

    var splashEl = document.getElementById('splash');
    var splashChild = splashEl.firstElementChild;
    function splashAt(x, y) {
      splashEl.style.transform = 'translate(' + x + 'px,' + y + 'px)';
      splashChild.classList.remove('blow');
      void splashChild.offsetWidth;
      splashChild.classList.add('blow');
    }

    function launch(link, ev) {
      if (!link) return;
      var x = (ev && ev.clientX) || window.innerWidth * 0.5;
      var y = (ev && ev.clientY) || window.innerHeight * 0.5;
      if (typeof x !== 'number' || !x) x = window.innerWidth * 0.5;
      if (typeof y !== 'number' || !y) y = window.innerHeight * 0.5;

      splashAt(x, y);

      if (CFG.newTab !== false) return;       /* 交给浏览器原生新标签页打开 */
      if (ev) ev.preventDefault();            /* 同标签页：先播擦除再跳 */
      playWipe();
      setTimeout(function () { window.location.href = link.url; }, 430);
    }

    /* --------------------------------------------------------- 键盘
       ↑↓/jk 选行；Enter / → 进分组（→ 在普通行上仍是「下一行」）；
       Esc / Backspace / ← 退回上一层（← 在主菜单里仍是「上一行」）。 */
    document.addEventListener('keydown', function (ev) {
      if (ev.metaKey || ev.ctrlKey || ev.altKey) return;
      var k = ev.key;

      if (k === 'Escape' || k === 'Backspace') {
        if (depth > 0) { ev.preventDefault(); back(); }
        return;
      }
      if (!rows.length) return;

      if (k === 'ArrowDown' || k === 'j') {
        ev.preventDefault(); select((pos + 1) % rows.length);
      } else if (k === 'ArrowUp' || k === 'k') {
        ev.preventDefault(); select((pos - 1 + rows.length) % rows.length);
      } else if (k === 'ArrowRight') {
        ev.preventDefault();
        if (pos >= 0 && isGroup(stack[depth][pos])) enter(pos);
        else select((pos + 1) % rows.length);
      } else if (k === 'ArrowLeft') {
        ev.preventDefault();
        if (depth > 0) back();
        else select((pos - 1 + rows.length) % rows.length);
      } else if (k === 'Enter' || k === ' ') {
        if (pos >= 0) { ev.preventDefault(); rows[pos].click(); }
      } else if (/^[1-9]$/.test(k)) {
        var idx = parseInt(k, 10) - 1;
        if (idx < rows.length) { ev.preventDefault(); select(idx); rows[idx].click(); }
      }
    });

    /* --------------------------------------------------------- 时钟 */
    var WD = ['日', '一', '二', '三', '四', '五', '六'];
    var clkDay = document.getElementById('clkDay');
    var clkTime = document.getElementById('clkTime');
    function pad(n) { return n < 10 ? '0' + n : '' + n; }
    function tick() {
      var d = new Date();
      clkDay.textContent = (d.getMonth() + 1) + '月' + pad(d.getDate()) + '日 周' + WD[d.getDay()];
      clkTime.textContent = pad(d.getHours()) + ':' + pad(d.getMinutes()) + ':' + pad(d.getSeconds());
    }
    tick();
    setInterval(tick, 1000);

    /* --------------------------------------------------------- 标题 / 水印 / 视差 */
    /* 主标题：按空格切词，最后一个词用主色（"NAVI" 就是整体主色） */
    var titleEl = document.getElementById('title');
    var words = String(CFG.title || DEFAULTS.title).trim().split(/\s+/);
    titleEl.textContent = '';
    words.forEach(function (w, i) {
      var s = document.createElement('span');
      s.className = 'w' + (i === words.length - 1 ? ' red' : '');
      s.textContent = w;
      if (i) titleEl.appendChild(document.createTextNode(' '));
      titleEl.appendChild(s);
    });
    document.getElementById('subText').textContent = CFG.subtitle || '';
    document.title = CFG.title + (CFG.subtitle ? ' // ' + CFG.subtitle : '');
    if (typeof CFG.watermark === 'string') document.querySelector('.wm').textContent = CFG.watermark;

    /* 竖排水印自适应：字多了就自动缩小，保证不超出视口高度 */
    var wmEl = document.querySelector('.wm');
    function fitWatermark() {
      var txt = (wmEl.textContent || '').trim();
      if (!txt) { wmEl.style.display = 'none'; return; }
      wmEl.style.display = '';
      var base = Math.min(124, window.innerWidth * 0.074);   /* 约等于 CSS 里的 7.4vmax，上限 124px */
      var limit = window.innerHeight * 0.80;
      wmEl.style.fontSize = base.toFixed(1) + 'px';
      var h = wmEl.getBoundingClientRect().height;
      if (h > limit) wmEl.style.fontSize = Math.max(16, base * limit / h).toFixed(1) + 'px';
    }
    fitWatermark();
    /* 展示字体加载完成后重量一次：否则会按回退字体的高度算，字号偏小 */
    if (document.fonts && document.fonts.ready) document.fonts.ready.then(fitWatermark);
    window.addEventListener('resize', fitWatermark);

    var reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    if (!reduce && window.matchMedia('(pointer:fine)').matches) {
      var px = 0, py = 0, tx = 0, ty = 0, raf = 0;
      var root = document.documentElement;
      function loop() {
        px += (tx - px) * 0.08;
        py += (ty - py) * 0.08;
        root.style.setProperty('--px', px.toFixed(4));
        root.style.setProperty('--py', py.toFixed(4));
        if (Math.abs(tx - px) > 0.001 || Math.abs(ty - py) > 0.001) {
          raf = requestAnimationFrame(loop);
        } else { raf = 0; }
      }
      document.addEventListener('pointermove', function (ev) {
        tx = (ev.clientX / window.innerWidth - 0.5) * -2;
        ty = (ev.clientY / window.innerHeight - 0.5) * -2;
        if (!raf) raf = requestAnimationFrame(loop);
      }, { passive: true });
    }

    /* --------------------------------------------------------- 开场 */
    buildRows(links, 0);
    playWipe();
    staggerIn(rows, 95, 520);
    setTimeout(function () { select(0); }, 560 + rows.length * 95);
  }

  fetchJson(CONFIG_URL)['catch'](function (e) {
    console.warn('[nav] ' + e.message + '，改用编译进来的示例配置');
    return FALLBACK;
  }).then(main)['catch'](function (e) {
    console.error('[nav] 初始化失败', e);
  });
})();
