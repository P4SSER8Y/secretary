<script setup lang="ts">
import { onMounted, onBeforeUnmount, computed, ref, Ref, getCurrentInstance } from 'vue';
import { Meta } from '../lib/struct';

const api = getCurrentInstance()?.appContext.config.globalProperties.$api;
const props = defineProps<{
    meta: Meta;
}>();
const emit = defineEmits<{ end: []; deleted: [uuid: string] }>();
const img_src = computed(() => `i/raw/${props.meta.uuid}`);
let delete_code: Ref<string | null> = ref(null);
let is_hold_on = ref(true);
let timer: ReturnType<typeof setTimeout> | number = 0;

const handle_key_press = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
        close();
    }
};

function close() {
    emit('end');
}

function request_delete() {
    api?.get('delete', { params: { id: props.meta.uuid } }).then((res) => {
        clearTimeout(timer);
        delete_code.value = res.data;
        is_hold_on.value = true;
        timer = setTimeout(() => {
            clearTimeout(timer);
            is_hold_on.value = false;
            timer = setTimeout(() => {
                clearTimeout(timer);
                delete_code.value = null;
                is_hold_on.value = false;
            }, 3000);
        }, 2000);
    });
}

function confirm_delete() {
    clearTimeout(timer);
    emit('deleted', props.meta.uuid);
    api?.get('delete', { params: { code: delete_code.value } })
        .then((res) => {
            console.log(`${res.data}`);
            console.log(`deleted ${props.meta.uuid}`);
            emit('deleted', props.meta.uuid);
            emit('end');
        })
        .finally(() => {
            delete_code.value = null;
        });
}

function cancel_delete() {
    clearTimeout(timer);
    delete_code.value = null;
    is_hold_on.value = false;
}

onMounted(() => {
    is_hold_on.value = false;
    delete_code.value = null;
    document.ondragenter = () => {
        window.addEventListener('keydown', handle_key_press);
    };
});

onBeforeUnmount(() => {
    window.removeEventListener('keydown', handle_key_press);
});
</script>

<template>
    <div class="cover fullScreenUpload flex max-h-[calc(100dvh - 4rem)] max-w-[calc(100dvw - 4rem)] p-8" @click="close">
        <img :src="img_src" class="object-contain w-full h-full" />
    </div>
    <div class="fixed bottom-0 right-0 pb-4 z-50">
        <div class="join">
            <button v-if="!delete_code" class="join-item btn btn-xs btn-ghost duration-300 ease-in-out"
                @click.stop="request_delete">
                🗑️
            </button>
            <button v-else class="join-item btn btn-xs btn-error duration-300 ease-in-out" @click.stop="confirm_delete"
                :disabled="is_hold_on">
                delete
            </button>
            <button v-if="delete_code" class="join-item btn btn-xs btn-success duration-300 ease-in-out"
                @click.stop="cancel_delete">
                cancel
            </button>
        </div>
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
    z-index: 45;
    @apply backdrop-filter backdrop-blur-sm;
}
</style>
