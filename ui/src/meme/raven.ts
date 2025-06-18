import { now } from '@vueuse/core';

export { raven };

function sleep(timeout: number) {
    return new Promise((resolve) => {
        setTimeout(resolve, timeout);
    });
}

async function raven(gate?: string, timeout: number = Infinity): Promise<string> {
    if (!gate) throw Error("Raven doesn't known where to fly");
    localStorage.removeItem('token');
    let url = new URL('raven.html', window.location.href);
    url.searchParams.set('gate', gate);
    url.searchParams.set('family', 'meme');
    let win = window.open(url.href, '_blank', 'menubar=no,toolbar=no,location=yes');
    if (!win) throw Error("Raven doesn't known where to fly");

    timeout = timeout + now();
    while (now() < timeout) {
        await sleep(1000);
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
