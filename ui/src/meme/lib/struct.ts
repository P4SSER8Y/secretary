interface Meta {
    uuid: string,
    timestamp?: string,
    tags?: string[],
}

export interface MemeList {
    meta: Meta[];
}

export enum Mode {
    Waterfall,
    Gallery,
};
