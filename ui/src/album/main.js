import { createApp } from 'vue';
import './style.css';
import App from './App.vue';
import axios from 'axios';
import { createPinia } from 'pinia';

const app = createApp(App);
const pinia = createPinia();
const api = axios.create({
    baseURL: '/album/api/',
    timeout: 30000,
});
app.config.globalProperties.$api = api;
app.use(pinia);
app.mount('#app');
