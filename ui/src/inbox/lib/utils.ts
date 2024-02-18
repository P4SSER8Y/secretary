// @ts-ignore: 7016
import { P5Message, P5Notification } from 'p5-ui';
import { getCurrentInstance } from 'vue';

export function p5_message(type: 'start' | 'failed' | 'clear') {
    switch (type) {
        case 'clear':
            P5Message({ type: 'clear' });
            break;
        case 'failed':
            P5Message({ type: 'fail' });
            break;
        case 'start':
            P5Message({ type: 'default' });
            break;
    }
}

const boats = [
    "mona",
    "ryuji",
    "ann",
    "yusuke",
    "makoto",
    "futaba",
    "haru",
    "akechi",
    "kasumi",
    "sumire",
    "lavenza",
];

export function p5_notify(msg: string) {
    let index = Math.floor(Math.random() * boats.length);
    P5Notification({
        content: msg,
        character: boats[index],
    });
}

export function get_api() {
    return getCurrentInstance()?.appContext.config.globalProperties.$api;
}
