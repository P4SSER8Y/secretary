<script setup lang="ts">

import { getCurrentInstance, onMounted, Ref, ref, watch } from 'vue';
import Waterfall from './pages/Waterfall.vue'
import Gallery from './pages/Gallery.vue';
import { raven } from "./raven"
import { MemeList } from './lib/struct';
import { debounce } from 'lodash';

const api = getCurrentInstance()?.appContext.config.globalProperties.$api;
let token: Ref<string | null> = ref(null);
let name: Ref<string | null> = ref(null);
let data: Ref<MemeList | null> = ref(null);
let filter: Ref<string> = ref("");
let waterfall_pagnition: Ref<number> = ref(30);

enum Mode {
    Waterfall,
    Gallery,
};
let mode: Ref<Mode> = ref(Mode.Waterfall);

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

async function logout() {
    filter.value = "";
    token.value = null;
    data.value = null;
}

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
    <div class="fixed navbar bg-base-100 z-50 opacity-0 hover:opacity-90 rounded-3xl">
        <div class="flex-1 min-w-0">
            <input type="text" placeholder="" class="input input-ghost w-full" v-model="filter" />
        </div>
        <div v-if="token" class="dropdown dropdown-end">
            <div tabindex="0" role="button" class="btn btn-ghost">
                <div class="w-2">
                    ↓
                </div>
            </div>
            <ul tabindex="0" class="menu menu-sm dropdown-content bg-base-100 rounded-box z-[1] mt-3 w-52 p-2 shadow">
                <li>
                    <span>user</span>
                    <ul>
                        <li>
                            <a>{{ name }}</a>
                        </li>
                        <li><a @click="logout">logout</a></li>
                        <li><a @click="update">update</a></li>
                    </ul>
                </li>
                <li>
                    <span>mode</span>
                    <ul>
                        <li>
                            <div @click="() => mode = Mode.Waterfall">
                                <input type="radio" class="radio" :checked="mode == Mode.Waterfall" />
                                <span>waterfall</span>
                            </div>
                        </li>
                        <li class="flex">
                            <div class="flex" @click="() => mode = Mode.Gallery">
                                <input type="radio" class="radio" :checked="mode == Mode.Gallery" />
                                <span>gallery</span>
                            </div>
                        </li>
                    </ul>
                </li>
                <li v-if="mode == Mode.Waterfall">
                    <span>range</span>
                    <ul>
                        <li>
                            <div>
                                <input type="range" class="range range-xs" min="10" max="50" step="5"
                                    v-model.number="waterfall_pagnition" />
                                <div>{{ waterfall_pagnition }}</div>
                            </div>
                        </li>
                    </ul>
                </li>
            </ul>
        </div>
        <div v-else class="btn btn-ghost" @click="() => { raven('https://hodor.32323235.xyz/').then((res) => token = res) }">⊙</div>
    </div>
    <Waterfall v-if="token && mode == Mode.Waterfall" class="main-entry-container" :data="data"
        :pagnition="waterfall_pagnition">
    </Waterfall>
    <Gallery v-else-if="token && mode == Mode.Gallery" class="main-entry-container" :data="data"></Gallery>
</template>

<style scoped lang="postcss"></style>
