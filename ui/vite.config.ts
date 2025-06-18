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
        kindle: resolve(__dirname, 'kindle/debug/index.html'),
        meme: resolve(__dirname, 'meme/index.html'),
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
      },
      '^/meme/i': {
        target: 'https://s.32323235.xyz',
        secure: false,
        changeOrigin: true,
      }
    }
  }
})
