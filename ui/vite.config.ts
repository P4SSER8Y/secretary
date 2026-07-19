import { defineConfig, loadEnv } from 'vite';
import vue from '@vitejs/plugin-vue';
import { resolve } from 'path';

// https://vitejs.dev/config/
export default defineConfig(({ mode }) => {
    const env = loadEnv(mode, process.cwd(), '');
    return {
        plugins: [vue()],
        build: {
            outDir: 'dist',
            emptyOutDir: true,
            copyPublicDir: true,
            rollupOptions: {
                input: {
                    album: resolve(__dirname, 'album/index.html'),
                    inbox: resolve(__dirname, 'inbox/index.html'),
                    kindle: resolve(__dirname, 'kindle/debug/index.html'),
                    meme: resolve(__dirname, 'meme/index.html'),
                    recipe: resolve(__dirname, 'recipe/index.html'),
                },
            },
        },
        resolve: {
            alias: {
                '@': resolve(__dirname, 'src'),
            },
        },
        server: {
            host: true,
            proxy: {
                '/album/api': {
                    target: env.VITE_PROXY_ALBUM_API,
                    changeOrigin: true,
                },
                '/inbox/api': {
                    target: env.VITE_PROXY_INBOX_API,
                    changeOrigin: true,
                },
                '/meme/i': {
                    target: env.VITE_PROXY_MEME_API,
                    secure: false,
                    changeOrigin: true,
                },
                '/recipe/api': {
                    target: env.VITE_PROXY_RECIPE_API || 'http://127.0.0.1:8000',
                    changeOrigin: true,
                },
            },
        },
    };
});
