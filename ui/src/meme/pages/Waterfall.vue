<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { MemeList } from '../lib/struct';
import { storeToRefs } from 'pinia';
import { useConfigStore } from '../lib/configStore';
import 'wc-waterfall';
import { useWindowSize } from '@vueuse/core';
import { Meta } from '../lib/struct';

const window = useWindowSize();

const props = defineProps<{
    data: MemeList | null;
}>();
const emits = defineEmits<{ show: [meta: Meta] }>();
const config = useConfigStore();
const { waterfall_pagnition } = storeToRefs(config);

let min_idx = ref(0);
let cols = computed(() => {
    if (window.width.value < 640) {
        return 2;
    } else if (window.width.value < 768) {
        return 3;
    } else if (window.width.value < 1024) {
        return 4;
    } else if (window.width.value < 1280) {
        return 5;
    } else if (window.width.value < 1536) {
        return 6;
    } else {
        return 7;
    }
});

watch(
    () => props.data,
    () => {
        min_idx.value = 0;
    }
);

function next() {
    if ((props.data?.meta.length ?? 0) > min_idx.value + waterfall_pagnition.value) {
        min_idx.value = min_idx.value + waterfall_pagnition.value;
    }
}

function previous() {
    if (min_idx.value >= waterfall_pagnition.value) {
        min_idx.value -= waterfall_pagnition.value;
    } else {
        min_idx.value = 0;
    }
}
</script>

<template>
    <div class="h-dvh w-dvw p-4 overflow-auto overflow-x-hidden">
        <wc-waterfall :cols="cols">
            <div v-for="item in props.data?.meta.slice(min_idx, min_idx + waterfall_pagnition)" :key="item.uuid" class="mx-auto ease-in-out">
                <div
                        class="tooltip tooltip-bottom tooltip-info gap-1 m-1"
                        :data-tip="item.tags?.join('/') ?? 'wtf'"
                        @click="() => $emit('show', item)"
                        >
                        <img :src="'i/thumbnail/' + item.uuid" class="rounded-xl" />
                </div>
            </div>
        </wc-waterfall>
        <div class="join fixed bottom-4 left-1/2 transform -translate-x-1/2 opacity-0 hover:opacity-80">
            <button class="join-item btn" @click="previous">&lt;&lt;&lt;</button>
            <button class="join-item btn">{{ min_idx + 1 }} ··· {{ min_idx + waterfall_pagnition }}</button>
            <button class="join-item btn" @click="next">&gt;&gt;&gt;</button>
        </div>
    </div>
</template>

<style lang="postcss" scoped>
@keyframes fadeIn {
    from {
        opacity: 0;
    }
    to {
        opacity: 1;
    }
}

img {
    animation: fadeIn 0.25s ease-in;
    opacity: 0;
    animation-fill-mode: forwards;
}
</style>
