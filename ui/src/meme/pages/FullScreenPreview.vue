<script setup lang="ts">
import { onMounted, onBeforeUnmount, computed, inject, ref, Ref, getCurrentInstance } from 'vue';
import { Meta } from '../lib/struct';

const api = getCurrentInstance()?.appContext.config.globalProperties.$api;
const props = defineProps<{
    meta: Meta;
}>();
const emit = defineEmits<{ end: []; deleted: [uuid: string]; 'tags-updated': [] }>();
const password = inject<Ref<string>>('password', ref(''));
const img_src = computed(() => {
    let url = `i/raw/${props.meta.uuid}`;
    if (password.value) {
        url += `?pwd=${encodeURIComponent(password.value)}`;
    }
    return url;
});
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

// ---- tag editing ----

const mime_parts = new Set(props.meta.mime.split('/').map(t => t.toLowerCase()));
const user_tags = (props.meta.tags ?? []).filter(t => !mime_parts.has(t.toLowerCase()));

const tags = ref<string[]>([...user_tags]);
const new_tag_input = ref('');
const is_saving = ref(false);

async function save_tags() {
    is_saving.value = true;
    try {
        await api?.post(`tag/${props.meta.uuid}`, { tags: tags.value });
        props.meta.tags = [...tags.value];
        emit('tags-updated');
        console.log(`tags saved for ${props.meta.uuid}: ${tags.value}`);
    } catch (e) {
        console.error('failed to save tags', e);
        tags.value = [...(props.meta.tags ?? [])];
    } finally {
        is_saving.value = false;
    }
}

function add_tags() {
    const raw = new_tag_input.value.trim();
    if (!raw) return;
    const incoming = raw
        .split(/[,，;；]/)
        .map(t => t.trim())
        .filter(t => t.length > 0);
    if (incoming.length === 0) return;
    const existing = new Set(tags.value.map(t => t.toLowerCase()));
    for (const t of incoming) {
        if (!existing.has(t.toLowerCase())) {
            tags.value.push(t);
            existing.add(t.toLowerCase());
        }
    }
    new_tag_input.value = '';
    save_tags();
}

function remove_tag(idx: number) {
    tags.value.splice(idx, 1);
    save_tags();
}

onMounted(() => {
    is_hold_on.value = false;
    delete_code.value = null;
    window.addEventListener('keydown', handle_key_press);
});

onBeforeUnmount(() => {
    window.removeEventListener('keydown', handle_key_press);
});
</script>

<template>
    <div class="cover fullScreenUpload flex max-h-[calc(100dvh - 4rem)] max-w-[calc(100dvw - 4rem)] p-8" @click="close">
        <img v-if="props.meta.mime.startsWith('image/')" :src="img_src" class="object-contain w-full h-full" />
        <video v-else-if="props.meta.mime.startsWith('video/')" :src="img_src" class="object-contain w-full h-full" autoplay loop/>
    </div>

    <!-- tag bar -->
    <div class="fixed bottom-4 left-1/2 -translate-x-1/2 z-50 flex flex-wrap items-center gap-1.5 max-w-[80dvw] bg-base-100/90 backdrop-blur-md border border-base-300/50 rounded-box p-2" @click.stop>
        <span v-for="(tag, idx) in tags" :key="idx" class="badge badge-soft badge-info gap-1 pr-0.5">
            {{ tag }}
            <button class="btn btn-ghost btn-xs px-0.5 hover:btn-error" @click="remove_tag(idx)" :disabled="is_saving">&times;</button>
        </span>
        <div class="join join-horizontal">
            <input
                v-model="new_tag_input"
                type="text"
                placeholder="+ tag"
                class="input input-xs input-ghost w-20 focus:w-32 transition-all duration-200"
                :disabled="is_saving"
                @keyup.enter="add_tags"
            />
            <button v-if="new_tag_input.trim().length > 0" class="join-item btn btn-xs btn-ghost" @click="add_tags" :disabled="is_saving">OK</button>
        </div>
    </div>

    <!-- delete bar -->
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
