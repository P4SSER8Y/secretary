<script setup lang="ts">
import { onMounted, onBeforeUnmount, computed } from 'vue';
import { Meta } from '../lib/struct';

// const api = getCurrentInstance()?.appContext.config.globalProperties.$api;
const props = defineProps<{
    meta: Meta;
}>();
const emit = defineEmits<{ end: [] }>();
const img_src = computed(() => `i/raw/${props.meta.uuid}`);

const handle_key_press = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
        close();
    }
};

function close() {
    emit('end');
}

onMounted(() => {
    document.ondragenter = () => {
        window.addEventListener('keydown', handle_key_press);
    };
});

onBeforeUnmount(() => {
    window.removeEventListener('keydown', handle_key_press);
});
</script>

<template>
    <div class="cover fullScreenUpload flex" @click="close">
        <img :src="img_src" class="max-h-[calc(60dvh)] max-w-[calc(90dvw)] object-contain" />
    </div>
</template>

<style scoped lang="postcss">
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
    @apply backdrop-filter backdrop-blur-sm;
}
</style>
