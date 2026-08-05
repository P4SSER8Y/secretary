<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, Ref, getCurrentInstance } from 'vue';
import { Meta } from '../lib/struct';

enum FileType {
    Unknown,
    Image,
    Video,
};

const api = getCurrentInstance()?.appContext.config.globalProperties.$api;
const emit = defineEmits<{ done: [meta: Meta]; begin: [] }>();
let show_drag = ref(false);
let is_uploading = ref(false);
let file: Ref<File | null> = ref(null);
let source: Ref<any | null> = ref(null);
let tags: Ref<string> = ref('');
let file_type: Ref<FileType> = ref(FileType.Unknown);

function detect_file_type(file: File): FileType {
    if (file.type.startsWith('image/')) {
        return FileType.Image;
    }
    else if (file.type.startsWith('video/')) {
        return FileType.Video;
    }
    return FileType.Unknown;
}

function drop_file(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    show_drag.value = false;
    file.value = e.dataTransfer?.files[0] ?? null;
    if (file.value) {
        file_type.value = detect_file_type(file.value);
        let reader = new FileReader();
        reader.readAsDataURL(file.value);
        reader.onload = () => {
            source.value = reader.result;
        };
    }
}

const handle_key_press = (e: KeyboardEvent) => {
    if (e.key === 'Escape' && !show_drag.value) {
        close();
    }
};

function close() {
    show_drag.value = false;
    file.value = null;
    source.value = null;
    is_uploading.value = false;
    tags.value = '';
    file_type.value = FileType.Unknown;
}

function upload() {
    is_uploading.value = true;
    let form = new FormData();
    form.append('file', file.value!);
    form.append('tags', tags.value);
    form.append('mime', file.value!.type);
    form.append('filename', file.value!.name);
    api?.post('upload', form)
        .then((res) => emit('done', res.data))
        .finally(() => close());
}

onMounted(() => {
    document.ondragenter = () => {
        show_drag.value = true;
        emit('begin');
        file.value = null;
        source.value = null;
        is_uploading.value = false;
        window.addEventListener('keydown', handle_key_press);
    };
});

onBeforeUnmount(() => {
    document.ondragenter = null;
    window.removeEventListener('keydown', handle_key_press);
});
</script>

<template>
    <div class="cover fullScreenUpload flex" v-if="show_drag || file" @dragleave="
        () => {
            show_drag = false;
        }
    " @drop="drop_file" @dragover="
        (e) => {
            e.preventDefault();
            e.stopPropagation();
        }
    ">
        <div v-if="show_drag" class="text-9xl animate-bounce">↑</div>
        <div v-if="file" class="p-4 m-4 flex flex-col gap-6 max-h-[calc(90dvh)]">
            <img v-if="file.type.startsWith('image/')" :src="source"
                class="max-h-[calc(60dvh)] max-w-[calc(90dvw)] object-contain" />
            <video v-else-if="file.type.startsWith('video/')" :type="file.type" :src="source" controls autoplay loop 
                class="max-h-[calc(60dvh)] max-w-[calc(90dvw)] object-contain" />
            <div class="grid items-center align-middle place-items-center">
                <input type="text" placeholder="tags"
                    class="input input-sm w-[calc(80dvw)] bg-transparent m-5 text-center border-none" v-model="tags" />
                <button class="btn btn-ghost w-[calc(80dvw)] bg-transparent" @click="upload" :disabled="is_uploading">
                    <span v-if="is_uploading" class="loading loading-infinity loading-lg"></span>
                    <span v-if="!is_uploading"> upload </span>
                </button>
            </div>
        </div>
        <button class="btn btn-square btn-outline absolute top-4 right-4" @click="close">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24"
                stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
        </button>
    </div>
</template>

<style scoped lang="postcss">
@reference "tailwindcss";

.flex {
    display: flex;
    align-items: center;
    justify-content: center;
}

.cover {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    right: 0;
}

.fullScreenUpload {
    background-color: rgba(0, 0, 0, 0.65);
    z-index: 50;
    @apply backdrop-filter backdrop-blur-xl;
}
</style>
