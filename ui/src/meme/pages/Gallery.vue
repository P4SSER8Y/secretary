<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { MemeList } from '../lib/struct';

const props = defineProps<{
    data: MemeList | null,
}>();

let index = ref(0);
let img_center_valid = computed(() => ((props.data) && (index.value >= 0) && (index.value < props.data.meta.length)));
let img_center_src = computed(() => img_center_valid ? ("i/raw/" + props.data!.meta[index.value].uuid) : "");

function next() {
    if (props.data) {
        if (index.value < props.data.meta.length - 1) {
            index.value++;
        }
        else {
            index.value = 0;
        }
    }
    else {
        index.value = 0;
    }
}

function previous() {
    if (props.data) {
        if (index.value > 0) {
            index.value--;
        }
        else {
            index.value = props.data.meta.length - 1;
        }
    }
    else {
        index.value = 0;
    }
}

function random() {
    if (props.data) {
        index.value = Math.floor(Math.random() * props.data.meta.length);
    }
    else {
        index.value = 0;
    }
}

onMounted(random);
watch(() => props.data, random);

</script>

<template>
    <div class="flex items-start justify-center h-screen">
        <img v-if="img_center_valid" :src="img_center_src" class="rounded-lg mx-8 my-8" :style="{
            'max-width': 'calc(100vw - 4rem)',
            'max-height': 'calc(100vh - 8rem)',
            'width': 'auto',
            'height': 'auto',
            'object-fit': 'contain'
        }" @click="random" />
    </div>
    <div class="join fixed bottom-4 left-1/2 transform -translate-x-1/2 opacity-0 hover:opacity-80">
        <button class="join-item btn" @click="previous">
            &lt;&lt;&lt;
        </button>
        <button class="join-item btn" @click="random">
            🎲
        </button>
        <button class="join-item btn" @click="next">
            &gt;&gt;&gt;
        </button>
    </div>
</template>

<style lang="postcss" scoped></style>
