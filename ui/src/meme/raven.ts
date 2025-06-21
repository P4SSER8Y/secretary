import { now } from '@vueuse/core';

export { raven };

function sleep(timeout: number) {
    return new Promise((resolve) => {
        setTimeout(resolve, timeout);
    });
}

async function raven(gate?: string, expire?: number, timeout: number = Infinity): Promise<string> {
    if (!gate) throw Error("Raven doesn't known where to fly");
    localStorage.removeItem('token');
    let cb = new URL('raven.html', window.location.href);
    let url = new URL('https://hodor.32323235.xyz');
    url.searchParams.set('c', cb.href);
    url.searchParams.set('f', 'meme');
    url.searchParams.set('t', '1');
    if (expire) {
        url.searchParams.set('e', expire.toString());
    }

    const width = 300;
    const height = 400;
    const left = (screen.width - width) / 2;
    const top = (screen.height - height) / 2;

    let win = window.open(
        url.href,
        '_blank',
        `menubar=no,toolbar=no,location=no,scrollbars=no,resizable=no,width=${width},height=${height},left=${left},top=${top}`
    );
    if (!win) throw Error("Raven doesn't known where to fly");

    timeout = timeout + now();
    while (now() < timeout) {
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
