import { createApp } from 'vue';
import './style.css';
import App from './App.vue';
import axios from 'axios';
import { createPinia } from 'pinia';

const app = createApp(App);
const pinia = createPinia();
app.config.globalProperties.$api = axios.create({
    baseURL: 'i/',
    timeout: 5000,
});
app.use(pinia);
app.mount('#app');
