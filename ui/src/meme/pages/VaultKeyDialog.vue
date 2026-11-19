<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue';
import SecretInput from '../components/SecretInput.vue';

const emits = defineEmits<{ quit: []; done: [key: string] }>();

const key = ref('');
let finished = false;

function submit() {
    finished = true;
    emits('done', key.value);
}

function cancel() {
    if (finished) return;
    finished = true;
    emits('quit');
}

function on_key(e: KeyboardEvent) {
    if (e.key === 'Escape') cancel();
}

onMounted(() => window.addEventListener('keydown', on_key));
onBeforeUnmount(() => window.removeEventListener('keydown', on_key));
</script>

<template>
    <div class="cover">
        <div class="w-full max-w-sm mx-4 rounded-box bg-base-200 p-2 shadow-xl">
            <SecretInput v-model="key" autofocus placeholder="key" @keyup.enter="submit" @blur="cancel" />
        </div>
    </div>
</template>

<style scoped>
.cover {
    position: absolute;
    inset: 0;

    display: flex;
    align-items: center;
    justify-content: center;

    background-color: rgba(0, 0, 0, 0.5);
    z-index: 50;
    backdrop-filter: blur(4px);
}
</style>
