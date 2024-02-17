<script setup lang="ts">
import { Ref, computed, onMounted, ref } from 'vue';
import ItemList from './components/ItemList.vue';
import { Metadata, get_link } from './lib/structs';
import { getCurrentInstance } from 'vue';
// @ts-ignore: 7016
import { P5Message } from 'p5-ui';

const api = getCurrentInstance()?.appContext.config.globalProperties.$api;
const prefix = ref('');
const data: Ref<Metadata[]> = ref([]);
const filteredData = computed(() => {
    return data.value.filter((item) => item.id.startsWith(prefix.value));
});

onMounted(async () => {
    await reload();
});

async function reload() {
    let raw = await api?.get('list');
    data.value = raw?.data;
}

async function enter_code() {
    if (filteredData.value.length == 1) {
        let link = get_link(filteredData.value[0]);
        window.location.href = link;
    } else {
        P5Message({ type: 'fail' });
    }
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
            <input v-model="prefix" type="tel" placeholder="code" class="input w-24 md:w-auto" @keyup.enter="enter_code" />
        </div>
    </div>
    <ItemList v-if="filteredData.length > 0" :data="filteredData" @update="reload"></ItemList>
    <div v-else class="hero">
        <div class="hero-content text-center">
            <p5-icon type="party" name="hifumi" />
            <p5-title content="TAKE YOUR HEART" size="extra-large"></p5-title>
        </div>
    </div>
</template>

<style scoped lang="postcss"></style>
