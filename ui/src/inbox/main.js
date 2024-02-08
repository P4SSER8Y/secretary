import { createApp } from 'vue';
import './style.css';
import App from './App.vue';
import axios from 'axios';

import P5UI from 'p5-ui';
import 'p5-ui/dist/style.css';

const app = createApp(App);
app.config.globalProperties.$api = axios.create({
    baseURL: 'api/',
    timeout: 5000,
});
app.use(P5UI);
app.mount('#app');
