<script setup lang="ts">

import { getCurrentInstance, onMounted, Ref, ref, watch } from 'vue';
import Waterfall from './pages/Waterfall.vue'
import { raven } from "./raven"
import { MemeList } from './lib/struct';
import { debounce } from 'lodash';

const api = getCurrentInstance()?.appContext.config.globalProperties.$api;
let token: Ref<string | null> = ref(null);
let name: Ref<string | null> = ref(null);
let data: Ref<MemeList | null> = ref(null);
let filter: Ref<string> = ref("");
let waterfall_pagnition: Ref<number> = ref(30);

const update = debounce(
    async function update() {
        if (!token.value)
            return;
        try {
            let res = await api?.get("list", { headers: { 'token': token.value }, params: { filter: filter.value } });
            data.value = res?.data;
        }
        catch {
            data.value = null;
        }
    },
    500
);

onMounted(() => {
    if (localStorage.getItem('token')?.length ?? 0 > 0) {
        let temp = localStorage.getItem('token');
        if (temp) {
            const payload = JSON.parse(atob(temp.split('.')[1]));
            if (payload.exp > Date.now() / 1000) {
                token.value = temp;
            }
        }
    }
});

watch(token, (newVal) => {
    if (newVal) {
        const payload = JSON.parse(atob(newVal.split('.')[1]));
        name.value = payload.n;
        console.log("new name: ", name.value);
        document.cookie = `token=${newVal}`;
        update();
    }
    else {
        document.cookie = `token=`;
        name.value = null;
    }
});

watch(filter, update);

</script>

<template>
    <div class="navbar bg-base-100">
        <div class="flex-none">
            <a class="btn btn-ghost text-xl">meme</a>
        </div>
        <div class="flex-1 min-w-0">
            <input type="text" placeholder="filter" class="input input-ghost w-full" v-model="filter" />
        </div>
        <div class="flex-none gap-2">
            <div class="dropdown dropdown-end">
                <div tabindex="0" role="button" class="btn btn-ghost">
                    <div class="w-10">
                        {{ name ?? "WTF" }}
                    </div>
                </div>
                <ul tabindex="0"
                    class="menu menu-sm dropdown-content bg-base-100 rounded-box z-[1] mt-3 w-52 p-2 shadow">
                    <li v-if="token"><a @click="() => { token = null }">Logout</a></li>
                    <li v-else><a
                            @click="() => { raven('https://hodor.32323235.xyz/').then((res) => token = res) }">Login</a>
                    </li>
                    <li>
                        <div>
                            <input type="range" class="range range-xs" min="10" max="50" step="5"
                                v-model.number="waterfall_pagnition" />
                            <div>{{ waterfall_pagnition }}</div>
                        </div>
                    </li>
                </ul>
            </div>
        </div>
    </div>
    <Waterfall class="main-entry-container" :data="data" :pagnition="waterfall_pagnition"></Waterfall>
</template>

<style scoped lang="postcss"></style>
