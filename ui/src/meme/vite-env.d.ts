/// <reference types="vite/client" />

import axios from "axios";

declare module 'vue' {
    interface ComponentCustomProperties {
        $api: typeof axios
    }
}
