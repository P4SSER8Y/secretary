export type Metadata = {
    id: string,
    kind: "File" | "Text",
    name: string,
    expiration: string,
    size: number,
    public: boolean,
}
