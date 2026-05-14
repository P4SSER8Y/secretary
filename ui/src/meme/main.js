import { createApp } from 'vue';
import './style.css';
import App from './App.vue';
import axios from 'axios';
import { createPinia } from 'pinia';

const app = createApp(App);
const pinia = createPinia();
const api = axios.create({
    baseURL: 'i/',
    timeout: 15000,
});
api.interceptors.request.use((config) => {
    const password = sessionStorage.getItem('password');
    if (password) {
        config.headers.password = password;
    }
    return config;
});
app.config.globalProperties.$api = api;
app.use(pinia);
app.mount('#app');
