<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref, Ref, watch } from 'vue';
import { MemeList } from '../lib/struct';
const props = defineProps<{ data: MemeList | null }>();
const emits = defineEmits<{ commit: [string]; quit: [void] }>();

interface tag_info {
    name: string;
    count: number;
}
let info: Ref<tag_info[]> = ref([]);

function update_tag_list() {
    if (props.data?.meta) {
        let map: Map<string, number> = new Map();
        for (let i = 0; i < props.data.meta.length; i++) {
            for (let j = 0; j < (props.data.meta[i].tags?.length ?? 0); j++) {
                let key = props.data.meta[i].tags![j];
                map.set(key, (map.get(key) ?? 0) + 1);
            }
        }
        info.value = [];
        map.forEach((v, k) => {
            if (v > 1) {
                info.value.push({ name: k, count: v });
            }
        });
        info.value.sort((a, b) => b.count - a.count);
        console.log(info);
    }
}

let selected = reactive<Set<string>>(new Set());
function toggle(name: string) {
    if (selected.has(name)) {
        selected.delete(name);
    } else {
        selected.add(name);
    }
}

const output = computed(() => Array.from(selected.keys()).join(','));

const handle_key_press = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
        close();
    }
};

watch(() => props.data, update_tag_list);
onMounted(() => {
    update_tag_list();
    document.ondragenter = () => {
        window.addEventListener('keydown', handle_key_press);
    };
});

onBeforeUnmount(() => {
    window.removeEventListener('keydown', handle_key_press);
});
</script>

<template>
    <div class="cover">
        <div class="grid grid-cols-1 max-h-[calc(100dvh-4rem)] w-[calc(100dvw-4rem)]">
            <div class="flex">
                <span class="flex-1 text-center flex items-center">{{ output }}</span>
                <div class="join join-vertical">
                    <button class="btn btn-outline btn-xs join-item" @click="emits('quit')">⎋</button>
                    <button class="btn btn-outline btn-xs join-item" @click="selected.clear()">⨉</button>
                    <button class="btn btn-outline btn-xs join-item" @click="emits('commit', output)">✓</button>
                </div>
            </div>
            <div class="divider"></div>
            <div class="flex flex-wrap gap-1 justify-between">
                <button v-for="item in info" :key="item.name" class="btn btn-outline btn-sm badge" @click="() => toggle(item.name)">
                    {{ item.name }}
                    <span v-if="item.count > 1" class="text-xs">{{ item.count }}</span>
                </button>
            </div>
        </div>
    </div>
</template>

<style lang="postcss" scoped>
.cover {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    right: 0;

    display: flex;
    align-items: center;
    justify-content: center;

    background-color: rgba(0, 0, 0, 0.65);
    z-index: 50;
    @apply backdrop-filter backdrop-blur-sm;
}
</style>
