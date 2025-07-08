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
                    inbox: resolve(__dirname, 'inbox/index.html'),
                    kindle: resolve(__dirname, 'kindle/debug/index.html'),
                    meme: resolve(__dirname, 'meme/index.html'),
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
                '^/inbox/api': {
                    target: env.VITE_PROXY_INBOX_API,
                    changeOrigin: true,
                },
                '^/meme/i': {
                    target: env.VITE_PROXY_MEME_API,
                    secure: false,
                    changeOrigin: true,
                },
            },
        },
    };
});
