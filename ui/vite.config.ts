import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'path';

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [vue()],
  build: {
    outDir: '../dist/ui',
    emptyOutDir: true,
    copyPublicDir: true,
    rollupOptions: {
      input: {
        inbox: resolve(__dirname, 'inbox/index.html'),
      }
    }
  },
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
  },
  server: {
    host: true,
    proxy: {
      '^/inbox/api': {
        target: 'http://192.168.31.41:8000',
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/inbox\/api/, "/inbox"),
      }
    }
  }
})
