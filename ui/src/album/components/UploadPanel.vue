<script setup lang="ts">
import { ref, getCurrentInstance } from 'vue';

const emit = defineEmits<{ uploaded: [id: string]; error: [msg: string] }>();

const vm = getCurrentInstance()!;
const api = vm.appContext.config.globalProperties.$api;

const fileInput = ref<HTMLInputElement | null>(null);
const originalUrl = ref<string | null>(null);
const epdUrl = ref<string | null>(null);
const uploading = ref(false);
const previewing = ref(false);
const sending = ref(false);

const params = ref({
    crop_mode: 'center',
    dither: true,
    rotate_cw: false,
    rotate_ccw: false,
});

const cropOptions = [
    { v: 'none', l: 'Fit' },
    { v: 'top-left', l: '↖' },
    { v: 'top', l: '↑' },
    { v: 'top-right', l: '↗' },
    { v: 'left', l: '←' },
    { v: 'center', l: '⊙' },
    { v: 'right', l: '→' },
    { v: 'bottom-left', l: '↙' },
    { v: 'bottom', l: '↓' },
    { v: 'bottom-right', l: '↘' },
];

function onFileChange(e: Event) {
    const input = e.target as HTMLInputElement;
    if (input.files?.length) {
        if (originalUrl.value) URL.revokeObjectURL(originalUrl.value);
        if (epdUrl.value) URL.revokeObjectURL(epdUrl.value);
        originalUrl.value = URL.createObjectURL(input.files[0]);
        epdUrl.value = null;
    }
}

async function doPreview() {
    const file = fileInput.value?.files?.[0];
    if (!file) return;

    previewing.value = true;
    try {
        const form = new FormData();
        form.append('file', file);
        form.append('crop_mode', params.value.crop_mode === 'none' ? '' : params.value.crop_mode);
        form.append('dither', String(params.value.dither));
        form.append('rotate_cw', String(params.value.rotate_cw));
        form.append('rotate_ccw', String(params.value.rotate_ccw));

        const res = await api.post('preview', form, { responseType: 'blob' });
        if (epdUrl.value) URL.revokeObjectURL(epdUrl.value);
        epdUrl.value = URL.createObjectURL(res.data);
    } catch (e: any) {
        emit('error', 'Preview failed');
    } finally {
        previewing.value = false;
    }
}

async function doDisplay() {
    const file = fileInput.value?.files?.[0];
    if (!file) return;

    sending.value = true;
    try {
        const form = new FormData();
        form.append('file', file);
        form.append('crop_mode', params.value.crop_mode === 'none' ? '' : params.value.crop_mode);
        form.append('dither', String(params.value.dither));
        form.append('rotate_cw', String(params.value.rotate_cw));
        form.append('rotate_ccw', String(params.value.rotate_ccw));

        await api.post('display', form);
    } catch (e: any) {
        emit('error', e?.response?.data?.message || 'Send failed');
    } finally {
        sending.value = false;
    }
}

async function doUpload() {
    const file = fileInput.value?.files?.[0];
    if (!file) return;

    uploading.value = true;
    try {
        const form = new FormData();
        form.append('file', file);
        form.append('crop_mode', params.value.crop_mode === 'none' ? '' : params.value.crop_mode);
        form.append('dither', String(params.value.dither));
        form.append('rotate_cw', String(params.value.rotate_cw));
        form.append('rotate_ccw', String(params.value.rotate_ccw));

        const res = await api.post('upload', form);
        if (res.data.ok) {
            const id = res.data.id;
            // 保存后直接发送到设备
            try {
                await api.post(`switch/${id}`);
            } catch (_) { /* send failure is non-fatal */ }
            emit('uploaded', id);
            // reset
            if (originalUrl.value) URL.revokeObjectURL(originalUrl.value);
            if (epdUrl.value) URL.revokeObjectURL(epdUrl.value);
            originalUrl.value = null;
            epdUrl.value = null;
            if (fileInput.value) fileInput.value.value = '';
        }
    } catch (e: any) {
        emit('error', e?.response?.data?.message || 'Upload failed');
    } finally {
        uploading.value = false;
    }
}
</script>

<template>
    <div class="p-4 bg-base-200/50 border-b border-base-300">
        <div class="max-w-4xl mx-auto flex flex-col gap-4">
            <!-- file input -->
            <div class="flex gap-3 items-center max-md:flex-col">
                <input
                    ref="fileInput"
                    type="file"
                    accept="image/*"
                    class="file-input file-input-bordered file-input-sm w-full max-w-xs"
                    @change="onFileChange"
                />
                <div class="flex gap-1 flex-wrap">
                    <button class="btn btn-sm btn-outline" :disabled="previewing" @click="doPreview">
                        {{ previewing ? '...' : '预览' }}
                    </button>
                    <button class="btn btn-sm btn-primary" :disabled="uploading" @click="doUpload">
                        {{ uploading ? '...' : '保存' }}
                    </button>
                    <button class="btn btn-sm btn-accent" :disabled="sending" @click="doDisplay">
                        {{ sending ? '...' : '发送' }}
                    </button>
                </div>
            </div>

            <!-- toggles -->
            <div class="flex gap-3 text-xs items-start">
                <div class="grid grid-cols-3 gap-1 w-fit">
                    <button
                        v-for="o in cropOptions.filter(x => x.v !== 'none')" :key="o.v"
                        class="btn btn-xs"
                        :class="params.crop_mode === o.v ? 'btn-primary' : 'btn-ghost'"
                        @click="params.crop_mode = o.v"
                    >{{ o.l }}</button>
                    <button
                        class="btn btn-xs col-span-3"
                        :class="params.crop_mode === 'none' ? 'btn-primary' : 'btn-ghost'"
                        @click="params.crop_mode = 'none'"
                    >Fit</button>
                </div>

                <div class="flex flex-col gap-1">
                    <label class="flex items-center gap-1 cursor-pointer">
                        <input type="checkbox" v-model="params.dither" class="toggle toggle-xs" />
                        <span>dither</span>
                    </label>

                    <label class="flex items-center gap-1 cursor-pointer">
                        <input type="checkbox" v-model="params.rotate_cw" class="toggle toggle-xs"
                            @change="params.rotate_ccw = false" />
                        <span>cw</span>
                    </label>

                    <label class="flex items-center gap-1 cursor-pointer">
                        <input type="checkbox" v-model="params.rotate_ccw" class="toggle toggle-xs"
                            @change="params.rotate_cw = false" />
                        <span>ccw</span>
                    </label>
                </div>
            </div>

            <!-- preview: two columns -->
            <div v-if="originalUrl" class="grid grid-cols-2 gap-3 max-md:grid-cols-1">
                <div class="text-center">
                    <div class="text-xs text-base-content/50 mb-1">原图</div>
                    <img :src="originalUrl" class="max-w-full max-h-64 mx-auto rounded-lg shadow object-contain" />
                </div>
                <div class="text-center">
                    <div class="text-xs text-base-content/50 mb-1">7 色预览</div>
                    <img
                        v-if="epdUrl"
                        :src="epdUrl"
                        class="max-w-full max-h-64 mx-auto rounded-lg shadow object-contain"
                        style="image-rendering: pixelated;"
                    />
                    <div v-else class="flex items-center justify-center h-32 bg-base-300 rounded-lg text-base-content/30 text-sm">
                        点击「预览」生成
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>
