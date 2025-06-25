/// <reference types="vite/client" />

interface ImportMeteEnv {
    readonly VITE_HODOR_ENTRY: string;
}

interface ImportMeta {
    readonly env: ImportMeteEnv;
}
