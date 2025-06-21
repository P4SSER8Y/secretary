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

export interface ApiListParams {
    filter?: string;
    asc?: boolean;
    sort?: SortKey;
}

export interface TokenPayload {
    n: string;
    f: string;
    exp: number;
}
