import { createApp } from 'vue';
import './style.css';
import App from './App.vue';
import axios from 'axios';

const app = createApp(App);
app.config.globalProperties.$api = axios.create({
    baseURL: 'i/',
    timeout: 5000,
});
app.mount('#app');
