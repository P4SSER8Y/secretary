<script setup lang="ts">
import { getCurrentInstance } from 'vue';
import { Metadata } from '../lib/structs';
// @ts-ignore: 7016
import { P5Message } from 'p5-ui';

const api = getCurrentInstance()?.appContext.config.globalProperties.$api;
const props = defineProps<{
    data: Metadata[];
}>();
const emit = defineEmits<{ update: [] }>();

function get_link(item: Metadata) {
    return 'api/get/' + item.id;
}

async function drop(id: string) {
    if (!api) {
        P5Message({ type: 'fail' });
        return;
    }
    await api.get('drop/' + id);
    P5Message({ type: 'clear' });
    emit('update');
}
</script>

<template>
    <div v-for="item in props.data" class="m-3">
        <div class="collapse collapse-arrow">
            <input type="radio" name="items" />
            <div class="collapse-title bg-base-200 rounded-md flex gap-5 p-3 place-items-center align-self-center justify-self-center">
                <a :href="get_link(item)" class="p5-hover-animation-mix min-w-20 center">
                    <p5-title
                        :content="item.id"
                        size="extra-large"
                        class="text-nowrap"
                        selected_font_color="#ff0022"
                        font_color="#FFF"
                    ></p5-title>
                </a>
                <div class="text-base-content text-nowrap text-md text-clip overflow-hidden ...">{{ item.name }}</div>
            </div>
            <div class="collapse-content bg-base-300 flex gap-5">
                <div>{{ item.expiration }}</div>
                <button @click="drop(item.id)" class="min-w-20">
                    <p5-title content="DROP" :animation="true" font_color="#FFF" selected_bg_color="#ff0022"> </p5-title>
                </button>
            </div>
        </div>
    </div>
</template>

<style scoped></style>
