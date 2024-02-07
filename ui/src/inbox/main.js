import { createApp } from 'vue'
import './style.css'
import App from './App.vue'

import P5UI from 'p5-ui'
import 'p5-ui/dist/style.css'

const app = createApp(App);
app.use(P5UI);
app.mount('#app')
