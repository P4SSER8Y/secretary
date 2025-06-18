interface Meta {
    uuid: string,
    timestamp?: string,
    tags?: string[],
}

export interface MemeList {
    meta: Meta[];
}
