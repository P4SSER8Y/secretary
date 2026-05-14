<script setup lang="ts">
import { computed, inject, onMounted, ref, watch, type Ref } from 'vue';
import { MemeList } from '../lib/struct';

const props = defineProps<{
    data: MemeList | null;
}>();
const password = inject<Ref<string>>('password', ref(''));

function raw_src(uuid: string) {
    let url = `i/raw/${uuid}`;
    if (password.value) {
        url += `?pwd=${encodeURIComponent(password.value)}`;
    }
    return url;
}
function thumbnail_src(uuid: string) {
    let url = `i/thumbnail/${uuid}`;
    if (password.value) {
        url += `?pwd=${encodeURIComponent(password.value)}`;
    }
    return url;
}

let index = ref(0);
let idx_random = ref(0);
let idx_next = ref(0);
let idx_previous = ref(0);

function is_img_valid(idx: number) {
    return props.data && idx >= 0 && idx < props.data.meta.length;
}

function get_img_mime(idx: number) {
    return is_img_valid(idx) ? props.data!.meta[idx].mime : '';
}

let img_center_valid = computed(() => is_img_valid(index.value));

let img_center_src = computed(() => raw_src(props.data!.meta[index.value]?.uuid ?? ''));
let img_random_src = computed(() => raw_src(props.data!.meta[idx_random.value]?.uuid ?? ''));
let img_next_src = computed(() => raw_src(props.data!.meta[idx_next.value]?.uuid ?? ''));
let img_previous_src = computed(() => raw_src(props.data!.meta[idx_previous.value]?.uuid ?? ''));

let img_center_mime = computed(() => get_img_mime(index.value));
let video_center_thumbnail_src = computed(() => is_img_valid(index.value) ? thumbnail_src(props.data!.meta[index.value].uuid) : '');

function next() {
    idx_previous.value = index.value;
    index.value = idx_next.value;
    if (props.data) {
        if (idx_next.value < props.data.meta.length - 1) {
            idx_next.value++;
        } else {
            idx_next.value = 0;
        }
    } else {
        idx_next.value = 0;
    }
}

function previous() {
    idx_next.value = index.value;
    index.value = idx_previous.value;
    if (props.data) {
        if (idx_previous.value > 0) {
            idx_previous.value--;
        } else {
            idx_previous.value = props.data.meta.length - 1;
        }
    } else {
        idx_previous.value = 0;
    }
}

function random() {
    index.value = idx_random.value;
    if (props.data) {
        let length = props.data.meta.length;
        idx_random.value = Math.floor(Math.random() * length);
        idx_next.value = (idx_random.value + 1) % length;
        idx_previous.value = (idx_random.value - 1 + length) % length;
    } else {
        idx_random.value = 0;
        idx_next.value = 0;
        idx_previous.value = 0;
    }
}

onMounted(random);
watch(
    () => props.data,
    () => {
        if (!is_img_valid(idx_random.value)) {
            random();
        }
        random();
    }
);
</script>

<template>
    <div class="flex items-center justify-center h-dvh" v-if="img_center_valid" @click="random">
        <Transition>
            <img v-if="img_center_mime.startsWith('image/')" :src="img_center_src" :key="img_center_src"
                class="absolute rounded-lg m-8 transition-shadow duration-250 ease-in-out" :style="{
                    'max-width': 'calc(100dvw - 4rem)',
                    'max-height': 'calc(100dvh - 4rem)',
                    'object-fit': 'contain',
                }" />
            <video v-else-if="img_center_mime.startsWith('video/')" :src="img_center_src" :key="img_center_src"
                :poster="video_center_thumbnail_src"
                class="absolute rounded-lg m-8 transition-shadow duration-250 ease-in-out" :style="{
                    'max-width': 'calc(100dvw - 4rem)',
                    'max-height': 'calc(100dvh - 4rem)',
                    'object-fit': 'contain',
                }" autoplay loop />
        </Transition>
        <img v-show="false" :src="img_next_src" />
        <img v-show="false" :src="img_previous_src" />
        <img v-show="false" :src="img_random_src" />
        <div class="join fixed bottom-4 left-1/2 transform -translate-x-1/2 opacity-0 hover:opacity-80">
            <button class="join-item btn" @click="previous">&lt;&lt;&lt;</button>
            <button class="join-item btn text-3xl" @click="random">⚄</button>
            <button class="join-item btn" @click="next">&gt;&gt;&gt;</button>
        </div>
    </div>
</template>

<style lang="postcss" scoped>
.v-enter-active,
.v-leave-active {
    transition: opacity 0.5s ease;
}

.v-enter-from,
.v-leave-to {
    opacity: 0;
}
</style>
