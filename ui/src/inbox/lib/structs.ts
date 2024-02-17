export type Metadata = {
    id: string,
    kind: "File" | "Text",
    name: string,
    expiration: string,
    size: number,
    public: boolean,
}

export function get_link(item: Metadata) {
    return 'api/get/' + item.id;
}
