import { createApp } from 'vue'
import { createPinia } from 'pinia'
import axios from 'axios'
import './style.css'
import App from './App.vue'

const app = createApp(App)
const pinia = createPinia()

const api = axios.create({
    baseURL: 'api/',
    timeout: 10000,
})

app.config.globalProperties.$api = api
app.use(pinia)
app.mount('#app')
