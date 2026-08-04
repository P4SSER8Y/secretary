import { now } from '@vueuse/core';

export { raven, ravenOpen, ravenWait, ravenUrl };

function sleep(timeout: number) {
    return new Promise((resolve) => {
        setTimeout(resolve, timeout);
    });
}

/** 构造认证页 URL：gate 认证完成后会把窗口跳转到 raven.html?token=xxx */
function ravenUrl(gate: string, expire?: number): URL {
    let cb = new URL('raven.html', window.location.href);
    let url = new URL(gate, window.location.origin);
    url.searchParams.set('c', cb.href);
    url.searchParams.set('f', 'meme');
    url.searchParams.set('t', '1');
    if (expire) {
        url.searchParams.set('e', expire.toString());
    }
    return url;
}

/**
 * 同步打开认证窗口。
 * 必须在用户手势的同步调用栈内调用，否则 Safari 弹窗拦截器会让 window.open 返回 null
 * （Edge/Chrome 对 await 后的短暂异步更宽容，Safari 不会）。
 * gate 未知时先打开同源 raven.html 占位，由调用方在拿到 gate 后设置 win.location 导航。
 */
function ravenOpen(gate?: string, expire?: number): Window {
    localStorage.removeItem('token');
    const url = gate ? ravenUrl(gate, expire).href : new URL('raven.html', window.location.href).href;
    const width = 300;
    const height = 400;
    const left = (screen.width - width) / 2;
    const top = (screen.height - height) / 2;
    let win = window.open(
        url,
        '_blank',
        `menubar=no,toolbar=no,location=no,scrollbars=no,resizable=no,width=${width},height=${height},left=${left},top=${top}`
    );
    if (!win) throw Error("Raven doesn't known where to fly");
    return win;
}

/** 轮询等待认证窗口写入 token（认证完成后 gate 跳转 raven.html 写入 localStorage） */
async function ravenWait(win: Window, timeout: number = Infinity): Promise<string> {
    let deadline = timeout + now();
    while (now() < deadline) {
        await sleep(100);
        let token = localStorage.getItem('token');
        if (token && token.length > 0) {
            win.close();
            return token;
        }
        if (win.closed) {
            throw Error('Raven was killed');
        }
    }
    win.close();
    throw Error('Raven is not home in time');
}

/** 兼容旧接口：直接打开并等待 */
async function raven(gate?: string, expire?: number, timeout: number = Infinity): Promise<string> {
    if (!gate) throw Error("Raven doesn't known where to fly");
    const win = ravenOpen(gate, expire);
    return ravenWait(win, timeout);
}
