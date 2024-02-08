<script setup lang="ts">
import { Ref, computed, onMounted, ref } from 'vue';
import ItemList from './components/ItemList.vue';
import { Metadata } from './lib/structs';
import { getCurrentInstance } from 'vue';

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
</script>

<template>
    <div class="navbar bg-base-300">
        <div class="flex-1">
            <p5-icon type="party" name="p5"></p5-icon>
            <p5-title content="INBOX" size="extra-large" font_color="#FFF" selected_bg_color="#ff0022" selected_font_color="black">
            </p5-title>
        </div>
        <div class="flex-none gap-2">
            <input v-model="prefix" type="tel" placeholder="code" class="input w-24 md:w-auto" />
        </div>
    </div>
    <ItemList :data="filteredData" @update="reload"></ItemList>
</template>

<style scoped lang="postcss"></style>
