/// <reference types="vite/client" />

declare module 'vue' {
    interface ComponentCustomProperties {
        $api: import('axios').AxiosInstance;
    }
}

export {};
