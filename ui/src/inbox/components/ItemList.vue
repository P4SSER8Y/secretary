<script setup lang="ts">
import { Metadata, get_link } from '../lib/structs';
import dayjs from 'dayjs';
import relativeTime from 'dayjs/plugin/relativeTime';
import { get_api, p5_message } from '../lib/utils';
dayjs.extend(relativeTime);

const api = get_api();
const props = defineProps<{
    data: Metadata[];
}>();
const emit = defineEmits<{ update: [] }>();

async function drop(id: string) {
    if (!api) {
        p5_message('failed');
        return;
    }
    await api.get('drop/' + id);
    p5_message('clear');
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
                <!-- <div class="text-base-content text-nowrap text-md text-clip overflow-hidden ...">{{ item.name }}</div> -->
                <p5-title :content="item.name" font_color="#FFF"></p5-title>
            </div>
            <div class="collapse-content bg-base-300 flex gap-5">
                <button @click="drop(item.id)" class="min-w-20">
                    <p5-title content="DROP" :animation="true" font_color="#FFF" selected_bg_color="#ff0022"> </p5-title>
                </button>
                <div class="tooltip tooltip-warning" :data-tip="dayjs(item.expiration).format('YYYY-MM-DD dd HH:mm:ss')">
                    <p5-title :content="dayjs(item.expiration).fromNow(true)" font_color="cyan"></p5-title>
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped></style>
