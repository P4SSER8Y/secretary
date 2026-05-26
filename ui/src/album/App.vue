<script setup lang="ts">
import { onMounted, onBeforeUnmount, getCurrentInstance, ref } from 'vue';
import GalleryGrid from './components/GalleryGrid.vue';
import UploadPanel from './components/UploadPanel.vue';
import DeviceStatus from './components/DeviceStatus.vue';
import PreviewModal from './components/PreviewModal.vue';
import { useDeviceStore } from './lib/store';
import type { ImageMeta } from './lib/structs';

const vm = getCurrentInstance()!;
const api = vm.appContext.config.globalProperties.$api;
const deviceStore = useDeviceStore();

const images = ref<ImageMeta[]>([]);
const previewId = ref<string | null>(null);
const showUpload = ref(false);
const errorMsg = ref('');

async function loadImages() {
    try {
        const res = await api.get('list');
        images.value = res.data.images;
    } catch {
        // handle silently
    }
}

function showError(msg: string) {
    errorMsg.value = msg;
    setTimeout(() => (errorMsg.value = ''), 4000);
}

onMounted(() => {
    loadImages();
    deviceStore.startPolling(api, 5000);
});

onBeforeUnmount(() => {
    deviceStore.stopPolling();
});

async function onSwitchImage(id: string) {
    try {
        const res = await api.post(`switch/${id}`);
        if (!res.data.ok) {
            showError(res.data.message);
        }
    } catch (e: any) {
        showError(e?.response?.data?.message || 'Switch failed');
    }
}

async function onDeleteImage(id: string) {
    try {
        const res = await api.delete(`image/${id}`);
        if (res.data.ok) {
            await loadImages();
            if (previewId.value === id) previewId.value = null;
        } else {
            showError(res.data.message);
        }
    } catch (e: any) {
        showError(e?.response?.data?.message || 'Delete failed');
    }
}

async function onSwitchNext() {
    try {
        const res = await api.post('switch/next');
        if (!res.data.ok) showError(res.data.message);
    } catch (e: any) {
        showError(e?.response?.data?.message || 'Failed');
    }
}

async function onSwitchPrev() {
    try {
        const res = await api.post('switch/prev');
        if (!res.data.ok) showError(res.data.message);
    } catch (e: any) {
        showError(e?.response?.data?.message || 'Failed');
    }
}

async function onSwitchRandom() {
    try {
        const res = await api.post('switch/random');
        if (!res.data.ok) showError(res.data.message);
    } catch (e: any) {
        showError(e?.response?.data?.message || 'Failed');
    }
}
</script>

<template>
    <div class="min-h-dvh bg-base-100 text-base-content">
        <DeviceStatus />

        <!-- nav -->
        <div class="navbar bg-base-200 shadow-sm px-4 gap-2 max-md:flex-wrap">
            <h1 class="text-xl font-bold flex-none">Album</h1>
            <div class="flex gap-1 flex-none max-md:order-3 max-md:w-full max-md:justify-center">
                <button class="btn btn-xs btn-ghost" @click="onSwitchPrev" title="上一张">◀</button>
                <button class="btn btn-xs btn-ghost" @click="onSwitchRandom" title="随机">🎲</button>
                <button class="btn btn-xs btn-ghost" @click="onSwitchNext" title="下一张">▶</button>
            </div>
            <div class="flex-1"></div>
            <button
                class="btn btn-sm btn-primary"
                @click="showUpload = !showUpload"
            >
                {{ showUpload ? 'Cancel' : 'Upload' }}
            </button>
        </div>

        <!-- error toast -->
        <div v-if="errorMsg" class="toast toast-top toast-end z-50">
            <div class="alert alert-error text-sm">{{ errorMsg }}</div>
        </div>

        <UploadPanel
            v-if="showUpload"
            @uploaded="() => { showUpload = false; loadImages(); }"
            @error="showError"
        />

        <GalleryGrid
            :images="images"
            @preview="(id: string) => (previewId = id)"
            @switch="onSwitchImage"
            @delete="onDeleteImage"
        />

        <PreviewModal
            v-if="previewId"
            :image-id="previewId"
            :images="images"
            @close="previewId = null"
            @switch="onSwitchImage"
            @delete="onDeleteImage"
        />
    </div>
</template>

<style scoped lang="postcss">
@reference "tailwindcss";
</style>
