<script setup lang="ts">

import { ref, watch } from 'vue';
import { MemeList } from '../lib/struct'

const props = defineProps<{
    data: MemeList | null,
    pagnition: number,
}>();

let min_idx = ref(0);

watch(() => props.data, () => {
    min_idx.value = 0;
});

function next() {
    if ((props.data?.meta.length ?? 0) > min_idx.value + props.pagnition) {
        min_idx.value = min_idx.value + props.pagnition;
    }
}

function previous() {
    if (min_idx.value >= props.pagnition) {
        min_idx.value -= props.pagnition;
    }
    else {
        min_idx.value = 0;
    }
}
</script>

<template>
    <div class="box min-h-screen">
        <div v-for="item in props.data?.meta.slice(min_idx, min_idx + props.pagnition)" :key="item.uuid" class="item mx-auto">
            <div class="tooltip tooltip-bottom tooltip-info" :data-tip="item.tags?.join('/') ?? 'wtf'">
                <img :src="'i/thumbnail/' + item.uuid" class="rounded-xl">
                </img>
            </div>
        </div>
    </div>
    <div class="join fixed bottom-4 left-1/2 transform -translate-x-1/2 opacity-0 hover:opacity-80">
        <button class="join-item btn" @click="previous">
            &lt;&lt;&lt;
        </button>
        <button class="join-item btn" @click="next">
            &gt;&gt;&gt;
        </button>
    </div>
</template>

<style lang="postcss" scoped>
.box {
    margin: 10px;
    column-count: 4;
    column-gap: 10px;
}

.item {
    margin-bottom: 10px;
}

.item img {
    width: 100%;
    height: 100%;
}
</style>