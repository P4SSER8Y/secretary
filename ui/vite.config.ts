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
      '^(/inbox/api|/meme/i)': {
        target: 'https://s.32323235.xyz/',
        changeOrigin: true,
        secure: false,
      }
    }
  }
})
