<script setup lang="ts">
import axios from 'axios';
import { Ref, onMounted, ref, watch } from 'vue';
import { useIntervalFn, useThrottleFn } from '@vueuse/core';

const src: Ref<string | null> = ref(null);
const now: Ref<string> = ref('');
function update() {
    axios
        .get('/kindle/', { responseType: 'blob', timeout: 1000, params: { now: now.value } })
        .then((res) => {
            if (src.value) {
                URL.revokeObjectURL(src.value);
            }
            src.value = URL.createObjectURL(res.data);
        })
        .catch((_e) => {
            if (src.value) {
                URL.revokeObjectURL(src.value);
            }
            src.value = null;
        });
}
let f = useThrottleFn(update, 500);

onMounted(() => {
    useIntervalFn(f, 500);
});
watch(now, f);
</script>

<template>
    <div class="flex flex-col gap-4 w-dvw h-dvh p-10">
        <input type="text" placeholder="now" class="input input-bordered flex-none" v-model="now" />
        <div class="flex-1 place-self-center">
            <img v-if="src && src.length > 0" :src="src" />
            <div v-else class="skeleton w-full h-full"></div>
        </div>
    </div>
</template>

<style scoped lang="postcss"></style>
