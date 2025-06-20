export interface Meta {
    uuid: string;
    timestamp?: string;
    tags?: string[];
}

export interface MemeList {
    meta: Meta[];
}

export enum PageType {
    Waterfall,
    Gallery,
}

export enum SortKey {
    timestamp = 'ts',
    uuid = 'uuid',
}
