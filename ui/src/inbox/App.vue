<script setup lang="ts">
import { Ref, computed, onMounted, ref } from 'vue';
import ItemList from './components/ItemList.vue';
import { Metadata, get_link } from './lib/structs';
import { getCurrentInstance } from 'vue';
import Uploader from './components/Uploader.vue';
import { p5_message } from './lib/utils';

const api = getCurrentInstance()?.appContext.config.globalProperties.$api;
const prefix = ref('');
const data: Ref<Metadata[]> = ref([]);
const filteredData = computed(() => {
    return data.value.filter((item) => item.id.startsWith(prefix.value));
});
const currentId = ref('');

onMounted(async () => {
    await reload();
});

async function reload() {
    let raw = await api?.get('list');
    data.value = raw?.data;
}

function enter_code() {
    if (filteredData.value.length == 1) {
        get_file(filteredData.value[0]);
    } else {
        p5_message('failed');
    }
}

function get_file(data: { id: string }) {
    let link = get_link(data);
    window.location.href = link;
}

async function close_uploader() {
    let el = document.getElementById('uploader') as HTMLDialogElement | null;
    el?.close();
    await reload();
}

function uploaded(id: string) {
    currentId.value = id;
}
</script>

<template>
    <div class="navbar bg-base-300">
        <div class="flex-1">
            <p5-icon type="party" name="p5"></p5-icon>
            <p5-title content="INBOX" size="extra-large" font_color="#FFF" selected_bg_color="#ff0022" selected_font_color="black">
            </p5-title>
        </div>
        <div class="flex-none gap-2">
            <input v-model="prefix" type="tel" placeholder="code" class="input input-ghost w-24 md:w-auto" @keyup.enter="enter_code" />
            <button class="btn" onclick="uploader.showModal()">+</button>
        </div>
    </div>
    <div class="grid grid-col-1">
        <p5-title
            v-if="currentId.length > 0"
            :content="currentId"
            size="extra-large"
            selected_bg_color="#FFF"
            font_color="#ff0022"
            selected_font_color="#000"
            :animation="true"
            class="place-self-center"
            @click="() => (prefix = currentId)"
        >
        </p5-title>
        <p5-divider v-if="currentId.length > 0"></p5-divider>
        <ItemList v-if="filteredData.length > 0" :data="filteredData" @update="reload"></ItemList>
        <div v-else class="hero">
            <div class="hero-content text-center">
                <p5-title
                    content="TAKE YOUR HEART"
                    size="extra-large"
                    :animation="prefix.length > 0"
                    @click="get_file({ id: prefix })"
                ></p5-title>
            </div>
        </div>
    </div>
    <dialog id="uploader" class="modal">
        <Uploader @close="close_uploader" @uploaded="uploaded"></Uploader>
    </dialog>
</template>

<style scoped lang="postcss"></style>
