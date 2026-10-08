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
/* 图标用**自托管**的 Material Symbols（outlined 可变字体，连字名版）：JSON 里直接写图标名
   （wifi / hard_drive / router / network_wired …），名字就是元素里的文本 → 任意名字都能用、
   加图标不用重新构建。不依赖 CDN，内网/离线同样显示。
   ⚠️ 完整字体 ≈3.9MB（一次下载、之后内容哈希长缓存）；嫌重可以再子集化（代价：加图标要重构建）。 */
import 'material-symbols/outlined.css';
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
    subTheme: '',     /* 兼容字段：分组条目自己没写 theme 时，子菜单用它；一般直接写在条目上 */
    links: []
  };

  function fetchJson(url) {
    return fetch(url, { cache: 'no-store' }).then(function (r) {
      if (!r.ok) throw new Error(url + ' -> HTTP ' + r.status);
      return r.json();
    });
  }

  /* ----------------------------------------------------------- 图标
     Material Symbols（自托管 outlined）。icon 直接写**图标集自己的名字**，不做任何改写/映射：
     名字就是元素里的文本，交给字体的连字渲染 —— 写什么就是什么，加图标也不用改代码。
     - 用 Material 原生的下划线写法：hard_drive / network_check / account_tree / photo_camera …
       完整清单：https://fonts.google.com/icons（搜索框里可以搜中文）
     - 名字不对（或差一个字母）时连字不命中：下面的 checkIcons 会把那个位置留空 + 控制台提示，
       不会画成一串字母
     - 非 ASCII 字符串（如 "📶"）原样当 emoji 画
     ⚠️ Material Symbols 没有品牌 logo（docker/github 这类），要品牌图标得另加一套（如 Simple Icons）。 */
  var warnedIcon = 0;
  function iconMarkup(icon) {
    if (!icon || typeof icon !== 'string') return '';
    if (/^[\x21-\x7e]{1,40}$/.test(icon)) {
      /* ASCII：原样交给字体按连字名渲染（成不成由字体决定，写错的会被 checkIcons 藏掉） */
      return '<span class="material-symbols-outlined" aria-hidden="true">' + icon + '</span>';
    }
    return '<span style="font-size:.92em;line-height:1">' + icon + '</span>';   /* emoji / 汉字原样 */
  }
  function iconsReady() {
    return !!(document.fonts && document.fonts.check &&
      document.fonts.check('24px "Material Symbols Outlined"'));
  }
  /* 连字名写错时浏览器会把字母原样画出来（看着像图标坏了）。字体加载完量一下宽度：
     真图标 ≈1em（这里用 offsetWidth = 布局宽度，不受 .ic 的 skew 影响，getBoundingClientRect
     会把斜切后的外框算进去导致误判），没命中的名字明显更宽 → 藏掉 + 提示一次。 */
  function checkIcons(root) {
    var els = (root || document).querySelectorAll('.material-symbols-outlined');
    for (var i = 0; i < els.length; i++) {
      var el = els[i];
      if (el.getAttribute('data-ms') === '1') continue;
      el.setAttribute('data-ms', '1');
      var fs = parseFloat(getComputedStyle(el).fontSize) || 24;
      if (el.offsetWidth > fs * 1.5) {
        el.style.visibility = 'hidden';
        if (warnedIcon++ < 3) {
          console.warn('[nav] 图标名 "' + el.textContent + '" 不是 Material Symbols 的名字，这个位置已留空。' +
            '请用图标集的原名（下划线写法，如 hard_drive / network_check / account_tree），清单见 fonts.google.com/icons');
        }
      }
    }
  }
  var STAR_SVG =
    '<svg viewBox="0 0 100 100" aria-hidden="true">' +
    '<path d="M50 2 57.6 36.2 87 19 66.4 46.9 98 50 66.4 53.1 87 81 57.6 63.8 50 98 42.4 63.8 13 81 33.6 53.1 2 50 33.6 46.9 13 19 42.4 36.2Z" ' +
    'fill="currentColor" stroke="#fff" stroke-width="2.5" stroke-linejoin="miter"/></svg>';
  /* P3 的选中标记：CSS 画的水面涟漪（见 .star .ripple），不引额外素材 */
  var RIPPLE_MARKUP = '<span class="ripple"></span>';

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
       主题是「一层一份」：主菜单用页面配置里的 theme；进哪个分组，子菜单就用
       那一项自己写的 theme（写在分组条目上，所以不同分组可以各用各的；兼容
       subTheme）。条目上没写就沿用父层的主题。切主题时先用斜条擦过屏幕，
       盖住之后才换配色，避免整页颜色硬切。 */
    var flashEl = document.createElement('div');
    flashEl.className = 'flash';
    flashEl.setAttribute('aria-hidden', 'true');
    for (var fi = 0; fi < 5; fi++) flashEl.appendChild(document.createElement('span'));
    document.body.appendChild(flashEl);

    function validTheme(t) { return MARK[t] ? t : null; }
    /* 每一层的主题（下标 = 层号）：0 是主菜单，进分组时往里压一项 */
    var themes = [validTheme(CFG.theme) || DEFAULTS.theme];
    /* 分组的子菜单主题：条目自带的 theme（兼容 subTheme）→ 全局 subTheme → 父层 */
    function childTheme(g, parentLevel) {
      return validTheme(g && (g.theme || g.subTheme)) ||
             validTheme(CFG.subTheme) || themeFor(parentLevel);
    }

    var themeNow = null;
    var enterDelay = 0;    /* 主题擦除期间，新行要等它盖上来再入场 */

    function themeFor(level) { return themes[level] || themes[0]; }

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
            '<span class="desc">' + (link.desc || '') + '</span>' +
          '</span>' +
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
      if (iconsReady()) checkIcons(menuEl);   /* 字体已就绪就立刻量一次，写错的图标名直接藏掉 */

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
        themes.push(childTheme(g, depth - 1));   /* 子菜单主题跟这一项走 */
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
        themes.pop();                            /* 回到父层，主题也跟着退回去 */
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
    /* 字体可能比首屏慢一点到：加载完成后再量一次图标（只量没量过的） */
    if (document.fonts && document.fonts.ready) {
      document.fonts.ready.then(function () { checkIcons(menuEl); });
    }
  }

  fetchJson(CONFIG_URL)['catch'](function (e) {
    console.warn('[nav] ' + e.message + '，改用编译进来的示例配置');
    return FALLBACK;
  }).then(main)['catch'](function (e) {
    console.error('[nav] 初始化失败', e);
  });
})();
